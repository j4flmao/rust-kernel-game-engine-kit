//! Win32 presentation engine: the minimal real Vulkan pipeline that owns a
//! device + swapchain and presents cleared frames.
//!
//! Mirrors `platform::linux::present` with the Windows calling convention and
//! `VK_KHR_win32_surface`. Field order is teardown order: swapchain, surface,
//! logical device, instance.
#![allow(unsafe_code)] // raw Vulkan + Win32 handle plumbing

use core::ffi::c_void;

use super::surface::Win32Surface;
use super::vulkan::{self, *};
use crate::platform::vulkan_policy::{QueueCreateRequest, QUEUE_GRAPHICS};

pub use super::swapchain::{
    cmd_clear_image, cmd_image_layout_transition, default_image_count, query_capabilities,
    query_formats, query_present_modes, select_format, surface_supported, SurfaceCapabilities,
    SurfaceFormat, Swapchain,
};

const UINT64_MAX: u64 = u64::MAX;
const OPAQUE_COMPOSITE_ALPHA: u32 = 0x0000_0001;

/// Native window handles the engine presents to.
///
/// - Linux: `device` is the X11 `Display*`; `window` carries the X11 window
///   id.
/// - Windows: `device` is the module instance and `window` the `HWND`.
#[derive(Clone, Copy, Debug)]
pub struct PresentHandles {
    pub device: *const c_void,
    pub window: *const c_void,
}

impl PresentHandles {
    pub const fn win32(hinstance: *mut c_void, hwnd: *mut c_void) -> Self {
        Self {
            device: hinstance as *const c_void,
            window: hwnd as *const c_void,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum PresentError {
    Vulkan(VulkanLoaderError),
    NoSuitableDevice,
    NullWindowHandle,
    FormatUnavailable,
    AllocationFailed,
}

impl core::fmt::Display for PresentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Vulkan(error) => write!(f, "vulkan present failure: {error}"),
            Self::NoSuitableDevice => {
                f.write_str("no physical device has a presentable graphics queue")
            }
            Self::NullWindowHandle => f.write_str("present window handles were null"),
            Self::FormatUnavailable => f.write_str("no supported swapchain format was found"),
            Self::AllocationFailed => f.write_str("present frame allocation failed"),
        }
    }
}

impl core::error::Error for PresentError {}

impl From<VulkanLoaderError> for PresentError {
    fn from(value: VulkanLoaderError) -> Self {
        Self::Vulkan(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentStatus {
    Presented,
    OutOfDate,
}

pub struct PresentEngine {
    swapchain: Swapchain,
    surface: Win32Surface,
    queue: VkQueue,
    family: u32,
    frames: Vec<FrameState>,
    image_index: u32,
    color: [f32; 4],
    extent: Extent2D,
    format: SurfaceFormat,
    present_mode: u32,
    loader: VulkanLoader,
    _device: LogicalDevice,
    _instance: NativeInstance,
}

struct FrameState {
    _pool: NativeCommandPool,
    _command_buffers: Vec<VkCommandBuffer>,
    buffer: VkCommandBuffer,
    image_ready: NativeSemaphore,
    render_done: NativeSemaphore,
    fence: NativeFence,
}

// SAFETY: the engine is owned and driven strictly by the single kernel
// scheduler thread; raw handles are never shared across threads.
unsafe impl Send for PresentEngine {}

impl PresentEngine {
    /// Brings up the full native present path against `handles`.
    pub fn build(
        handles: PresentHandles,
        size: PresentSize,
        color: [f32; 4],
    ) -> Result<Self, PresentError> {
        if handles.device.is_null() || handles.window.is_null() {
            return Err(PresentError::NullWindowHandle);
        }
        let loader = VulkanLoader::load()?;
        let instance = loader.create_instance(&[VK_KHR_SURFACE, VK_KHR_WIN32_SURFACE])?;
        // SAFETY: handles outlive the engine per the module contract.
        let surface = unsafe {
            super::surface::create_with_handles(
                &loader,
                instance.raw(),
                handles.device,
                handles.window,
            )?
        };
        let devices = unsafe { loader.devices_and_handles(instance.raw()) }?;
        let (physical, _info, family) = pick_present_device(&loader, &devices, surface.raw())?;
        let requests = [QueueCreateRequest {
            family_index: family,
            queue_count: 1,
        }; 3];
        // SAFETY: handles live.
        let device = unsafe { loader.create_logical_device(physical, &requests)? };
        let Some(queue) = device.queue(family, 0) else {
            return Err(PresentError::NoSuitableDevice);
        };
        let composite_alpha = pick_composite_alpha(&loader, physical, surface.raw())?;
        let (format, present_mode, extent, image_count) =
            configure_surface(&loader, physical, surface.raw(), size)?;
        let swapchain = unsafe {
            Swapchain::create(
                &loader,
                device.raw(),
                surface.raw(),
                family,
                format,
                extent,
                present_mode,
                composite_alpha,
                image_count,
                0,
            )?
        };
        let frames = build_frame_states(&loader, &device, family, swapchain.image_count() as u32)?;
        Ok(Self {
            swapchain,
            surface,
            queue,
            family,
            frames,
            image_index: 0,
            color,
            extent,
            format,
            present_mode,
            loader,
            _device: device,
            _instance: instance,
        })
    }

    pub const fn extent(&self) -> Extent2D {
        self.extent
    }

    pub const fn format(&self) -> SurfaceFormat {
        self.format
    }

    pub const fn present_mode(&self) -> u32 {
        self.present_mode
    }

    pub const fn image_count(&self) -> usize {
        self.frames.len()
    }

    pub fn set_clear_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    /// Acquires the next image, clears it, and presents it.
    pub fn present_frame(&mut self) -> Result<PresentStatus, PresentError> {
        let frame_index = self.image_index as usize % self.frames.len();
        let frame = &self.frames[frame_index];
        frame.fence.wait(UINT64_MAX)?;
        frame.fence.reset()?;

        // SAFETY: handles belong to this live swapchain/device.
        let image = match unsafe {
            self.swapchain
                .acquire(frame.image_ready.raw(), core::ptr::null_mut())
        } {
            Ok(image) => image,
            Err(VulkanLoaderError::OutOfDate) => return Ok(PresentStatus::OutOfDate),
            Err(error) => return Err(PresentError::Vulkan(error)),
        };
        self.image_index = self.image_index.wrapping_add(1);

        // SAFETY: the buffer belongs to this engine's pool/device.
        unsafe {
            vulkan::reset_command_buffer(&self.loader, frame.buffer)?;
            vulkan::begin_command_buffer(&self.loader, frame.buffer)?;
            cmd_image_layout_transition(
                &self.loader,
                self._device.raw(),
                frame.buffer,
                self.swapchain.images()[image as usize],
                VK_IMAGE_LAYOUT_UNDEFINED,
                VK_IMAGE_LAYOUT_GENERAL,
            )?;
            cmd_clear_image(
                &self.loader,
                self._device.raw(),
                frame.buffer,
                self.swapchain.images()[image as usize],
                self.color,
            )?;
            cmd_image_layout_transition(
                &self.loader,
                self._device.raw(),
                frame.buffer,
                self.swapchain.images()[image as usize],
                VK_IMAGE_LAYOUT_GENERAL,
                VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
            )?;
            vulkan::end_command_buffer(&self.loader, frame.buffer)?;
        }
        // SAFETY: all handles belong to the same live device.
        unsafe {
            self._device.queue_submit(
                &self.loader,
                self.queue,
                frame.buffer,
                frame.image_ready.raw(),
                frame.render_done.raw(),
                frame.fence.raw(),
            )?;
        }
        // SAFETY: handles belong to this live swapchain/device.
        match unsafe {
            self.swapchain
                .present(&self.loader, self.queue, &[frame.render_done.raw()], image)
        } {
            Ok(()) => Ok(PresentStatus::Presented),
            Err(VulkanLoaderError::OutOfDate) => Ok(PresentStatus::OutOfDate),
            Err(error) => Err(PresentError::Vulkan(error)),
        }
    }

    /// Rebuilds the swapchain (typically after `OutOfDate` from a resize).
    pub fn recreate(&mut self, size: PresentSize) -> Result<(), PresentError> {
        let physical = self._device.raw();
        let (format, present_mode, extent, image_count) =
            configure_surface(&self.loader, physical, self.surface.raw(), size)?;
        self.format = format;
        self.present_mode = present_mode;
        self.extent = extent;
        unsafe {
            self.swapchain.recreate(
                &self.loader,
                self.surface.raw(),
                self.family,
                format,
                extent,
                present_mode,
                pick_composite_alpha(&self.loader, physical, self.surface.raw())?,
                image_count,
            )?
        };
        Ok(())
    }
}

fn pick_composite_alpha(
    loader: &VulkanLoader,
    physical: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> Result<u32, PresentError> {
    let caps = unsafe { query_capabilities(loader, physical, surface)? };
    if caps.supported_composite_alpha & OPAQUE_COMPOSITE_ALPHA != 0 {
        return Ok(OPAQUE_COMPOSITE_ALPHA);
    }
    let mask = caps.supported_composite_alpha;
    if mask == 0 {
        return Ok(OPAQUE_COMPOSITE_ALPHA);
    }
    Ok(1u32 << mask.trailing_zeros())
}

fn pick_present_device(
    loader: &VulkanLoader,
    devices: &[(VkPhysicalDevice, PhysicalDeviceInfo)],
    surface: VkSurfaceKHR,
) -> Result<(VkPhysicalDevice, PhysicalDeviceInfo, u32), PresentError> {
    let mut best: Option<(VkPhysicalDevice, PhysicalDeviceInfo, u32)> = None;
    for (handle, info) in devices {
        for family in &info.queue_families {
            if family.flags & QUEUE_GRAPHICS == 0 || family.queue_count == 0 {
                continue;
            }
            if unsafe { surface_supported(loader, *handle, surface, family.index) }
                .map_err(PresentError::Vulkan)?
            {
                let score = info.selection_score();
                if best
                    .as_ref()
                    .map(|(_, current, _)| score > current.selection_score())
                    .unwrap_or(true)
                {
                    best = Some((*handle, info.clone(), family.index));
                }
                break;
            }
        }
    }
    best.ok_or(PresentError::NoSuitableDevice)
}

fn configure_surface(
    loader: &VulkanLoader,
    physical: VkPhysicalDevice,
    surface: VkSurfaceKHR,
    size: PresentSize,
) -> Result<(SurfaceFormat, u32, Extent2D, u32), PresentError> {
    // SAFETY: loader/physical/surface live for the session.
    let caps = unsafe { query_capabilities(loader, physical, surface)? };
    let formats = unsafe { query_formats(loader, physical, surface)? };
    let format = select_format(
        &formats,
        &[
            (VK_FORMAT_B8G8R8A8_UNORM, VK_COLOR_SPACE_SRGB_NONLINEAR_KHR),
            (VK_FORMAT_R8G8B8A8_UNORM, VK_COLOR_SPACE_SRGB_NONLINEAR_KHR),
        ],
    );
    let modes = unsafe { query_present_modes(loader, physical, surface)? };
    let present_mode = pick_present_mode(&modes).ok_or(PresentError::FormatUnavailable)?;
    Ok((
        format,
        present_mode,
        pick_extent(&caps, size),
        default_image_count(&caps).max(2),
    ))
}

fn pick_extent(caps: &SurfaceCapabilities, size: PresentSize) -> Extent2D {
    if caps.current_extent.width != u32::MAX && caps.current_extent.height != u32::MAX {
        return caps.current_extent;
    }
    Extent2D {
        width: size
            .width
            .clamp(caps.min_image_extent.width, caps.max_image_extent.width),
        height: size
            .height
            .clamp(caps.min_image_extent.height, caps.max_image_extent.height),
    }
}

fn pick_present_mode(modes: &[u32]) -> Option<u32> {
    for preferred in [
        VK_PRESENT_MODE_MAILBOX_KHR,
        VK_PRESENT_MODE_IMMEDIATE_KHR,
        VK_PRESENT_MODE_FIFO_KHR,
        VK_PRESENT_MODE_FIFO_RELAXED_KHR,
    ] {
        if modes.contains(&preferred) {
            return Some(preferred);
        }
    }
    modes.first().copied()
}

fn build_frame_states(
    loader: &VulkanLoader,
    device: &LogicalDevice,
    family: u32,
    frames: u32,
) -> Result<Vec<FrameState>, PresentError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(frames as usize)
        .map_err(|_| PresentError::AllocationFailed)?;
    for _ in 0..frames {
        let pool = device.create_command_pool(loader, family)?;
        let command_buffers = pool.allocate_command_buffers(loader, 1)?;
        let image_ready = device.create_semaphore(loader)?;
        let render_done = device.create_semaphore(loader)?;
        let fence = device.create_fence(loader, true)?;
        output.push(FrameState {
            _command_buffers: command_buffers.clone(),
            buffer: command_buffers[0],
            _pool: pool,
            image_ready,
            render_done,
            fence,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_mode_prefers_mailbox_over_fifo() {
        assert_eq!(
            pick_present_mode(&[VK_PRESENT_MODE_FIFO_KHR, VK_PRESENT_MODE_MAILBOX_KHR]),
            Some(VK_PRESENT_MODE_MAILBOX_KHR)
        );
        assert_eq!(
            pick_present_mode(&[VK_PRESENT_MODE_FIFO_KHR]),
            Some(VK_PRESENT_MODE_FIFO_KHR)
        );
        assert_eq!(pick_present_mode(&[]), None);
    }

    #[test]
    fn extent_falls_back_to_request_size_clamped() {
        let caps = SurfaceCapabilities {
            min_image_count: 0,
            max_image_count: 0,
            current_extent: Extent2D {
                width: u32::MAX,
                height: u32::MAX,
            },
            min_image_extent: Extent2D {
                width: 320,
                height: 240,
            },
            max_image_extent: Extent2D {
                width: 4096,
                height: 2160,
            },
            max_image_array_layers: 1,
            supported_transforms: 0,
            current_transform: 0,
            supported_composite_alpha: 0,
            supported_usage_flags: 0,
        };
        let pick = pick_extent(
            &caps,
            PresentSize {
                width: 10_000,
                height: 100,
            },
        );
        assert_eq!(pick.width, 4096);
        assert_eq!(pick.height, 240);
    }

    #[test]
    fn composite_alpha_masks_are_laid_out_as_bit_flags() {
        assert_eq!(OPAQUE_COMPOSITE_ALPHA, 1);
        assert_eq!(1u32 << 3, 8);
    }

    #[test]
    fn win32_handles_round_trip() {
        let handle = 0x1234_5678usize as *const c_void;
        let handles = PresentHandles::win32(handle as *mut c_void, handle as *mut c_void);
        assert_eq!(handles.device as usize, 0x1234_5678);
        assert_eq!(handles.window as usize, 0x1234_5678);
    }
}
