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
const UI_UPLOAD_CAPACITY: u64 = 16 * 1024 * 1024;

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
    ui_pipeline: Option<UiNativePipeline>,
    ui_targets: Option<UiFrameTargets>,
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
    ui_staging: NativeBuffer,
    ui_device: NativeBuffer,
    ui_upload: Vec<u8>,
    ui_draw_commands: Vec<crate::subsystems::renderer::UiDrawCommand>,
    loader: VulkanLoader,
    _device: LogicalDevice,
    _instance: NativeInstance,
}

#[allow(dead_code)]
struct UiFrameTargets {
    render_pass: NativeRenderPass,
    image_views: Vec<NativeImageView>,
    framebuffers: Vec<NativeFramebuffer>,
}

impl UiFrameTargets {
    const fn render_pass(&self) -> VkRenderPass {
        self.render_pass.raw()
    }

    fn framebuffer(&self, index: usize) -> Option<VkFramebuffer> {
        self.framebuffers.get(index).map(NativeFramebuffer::raw)
    }
}

struct UiNativePipeline {
    pipeline: NativeGraphicsPipeline,
    pipeline_layout: NativePipelineLayout,
    descriptor_set: VkDescriptorSet,
    _descriptor_pool: NativeDescriptorPool,
    _descriptor_layout: NativeDescriptorSetLayout,
    _fragment_shader: NativeShaderModule,
    _vertex_shader: NativeShaderModule,
}

impl UiNativePipeline {
    unsafe fn build(
        loader: &VulkanLoader,
        device: &LogicalDevice,
        targets: &UiFrameTargets,
        ui_buffer: VkBuffer,
        extent: Extent2D,
    ) -> Result<Option<Self>, PresentError> {
        let vertex_words = crate::platform::ui_shaders::UI_VERTEX_SPIRV;
        let fragment_words = crate::platform::ui_shaders::UI_FRAGMENT_SPIRV;
        if vertex_words.is_empty() || fragment_words.is_empty() {
            return Ok(None);
        }
        let vertex_shader = unsafe { device.create_shader_module(loader, vertex_words) }
            .map_err(PresentError::Vulkan)?;
        let fragment_shader = unsafe { device.create_shader_module(loader, fragment_words) }
            .map_err(PresentError::Vulkan)?;
        let descriptor_layout = unsafe { device.create_ui_descriptor_set_layout(loader) }
            .map_err(PresentError::Vulkan)?;
        let descriptor_pool =
            unsafe { device.create_ui_descriptor_pool(loader) }.map_err(PresentError::Vulkan)?;
        let descriptor_set = unsafe {
            descriptor_pool.allocate_ui_set(
                loader,
                descriptor_layout.raw(),
                ui_buffer,
                UI_UPLOAD_CAPACITY,
            )
        }
        .map_err(PresentError::Vulkan)?;
        let pipeline_layout =
            unsafe { device.create_ui_pipeline_layout(loader, descriptor_layout.raw()) }
                .map_err(PresentError::Vulkan)?;
        let pipeline = unsafe {
            device.create_ui_graphics_pipeline(
                loader,
                vertex_shader.raw(),
                fragment_shader.raw(),
                pipeline_layout.raw(),
                targets.render_pass(),
                extent,
            )
        }
        .map_err(PresentError::Vulkan)?;
        Ok(Some(Self {
            pipeline,
            pipeline_layout,
            descriptor_set,
            _descriptor_pool: descriptor_pool,
            _descriptor_layout: descriptor_layout,
            _fragment_shader: fragment_shader,
            _vertex_shader: vertex_shader,
        }))
    }
}

impl UiFrameTargets {
    unsafe fn build(
        loader: &VulkanLoader,
        device: &LogicalDevice,
        swapchain: &Swapchain,
        format: SurfaceFormat,
        extent: Extent2D,
    ) -> Result<Self, PresentError> {
        let render_pass = unsafe { device.create_ui_render_pass(loader, format.format) }
            .map_err(PresentError::Vulkan)?;
        let mut image_views = Vec::new();
        image_views
            .try_reserve_exact(swapchain.image_count())
            .map_err(|_| PresentError::AllocationFailed)?;
        for &image in swapchain.images() {
            image_views.push(
                unsafe { device.create_color_image_view(loader, image, format.format) }
                    .map_err(PresentError::Vulkan)?,
            );
        }
        let mut framebuffers = Vec::new();
        framebuffers
            .try_reserve_exact(image_views.len())
            .map_err(|_| PresentError::AllocationFailed)?;
        for view in &image_views {
            framebuffers.push(
                unsafe { device.create_framebuffer(loader, render_pass.raw(), view.raw(), extent) }
                    .map_err(PresentError::Vulkan)?,
            );
        }
        Ok(Self {
            render_pass,
            image_views,
            framebuffers,
        })
    }
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
        let ui_targets =
            Some(unsafe { UiFrameTargets::build(&loader, &device, &swapchain, format, extent)? });
        let ui_staging = unsafe {
            device.create_buffer(
                &loader,
                physical,
                UI_UPLOAD_CAPACITY,
                VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
                VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
                VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
            )?
        };
        let ui_device = unsafe {
            device.create_buffer(
                &loader,
                physical,
                UI_UPLOAD_CAPACITY,
                VK_BUFFER_USAGE_TRANSFER_DST_BIT
                    | VK_BUFFER_USAGE_VERTEX_BUFFER_BIT
                    | VK_BUFFER_USAGE_STORAGE_BUFFER_BIT,
                0,
                VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
            )?
        };
        let ui_pipeline = Some(unsafe {
            UiNativePipeline::build(
                &loader,
                &device,
                ui_targets.as_ref().expect("UI targets were just created"),
                ui_device.raw(),
                extent,
            )?
        })
        .flatten();
        let frames = build_frame_states(&loader, &device, family, swapchain.image_count() as u32)?;
        Ok(Self {
            ui_pipeline,
            ui_targets,
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
            ui_staging,
            ui_device,
            ui_upload: Vec::new(),
            ui_draw_commands: Vec::new(),
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

    pub fn set_ui_upload(&mut self, bytes: &[u8]) -> Result<(), PresentError> {
        if u64::try_from(bytes.len()).map_err(|_| PresentError::AllocationFailed)?
            > UI_UPLOAD_CAPACITY
        {
            return Err(PresentError::AllocationFailed);
        }
        self.ui_upload
            .try_reserve(bytes.len().saturating_sub(self.ui_upload.capacity()))
            .map_err(|_| PresentError::AllocationFailed)?;
        self.ui_upload.clear();
        self.ui_upload.extend_from_slice(bytes);
        Ok(())
    }

    pub fn set_ui_upload_ranges(
        &mut self,
        ranges: &[crate::subsystems::ui::UiUploadRange],
        total_bytes: usize,
    ) -> Result<(), PresentError> {
        if u64::try_from(total_bytes).map_err(|_| PresentError::AllocationFailed)?
            > UI_UPLOAD_CAPACITY
        {
            return Err(PresentError::AllocationFailed);
        }
        self.ui_upload
            .try_reserve(total_bytes.saturating_sub(self.ui_upload.capacity()))
            .map_err(|_| PresentError::AllocationFailed)?;
        self.ui_upload.resize(total_bytes, 0);
        for range in ranges {
            let end = range
                .offset
                .checked_add(range.bytes.len())
                .ok_or(PresentError::AllocationFailed)?;
            if end > total_bytes {
                return Err(PresentError::AllocationFailed);
            }
            self.ui_upload[range.offset..end].copy_from_slice(&range.bytes);
        }
        Ok(())
    }

    pub fn set_ui_draw_commands(
        &mut self,
        commands: &[crate::subsystems::renderer::UiDrawCommand],
    ) -> Result<(), PresentError> {
        self.ui_draw_commands
            .try_reserve(
                commands
                    .len()
                    .saturating_sub(self.ui_draw_commands.capacity()),
            )
            .map_err(|_| PresentError::AllocationFailed)?;
        self.ui_draw_commands.clear();
        self.ui_draw_commands.extend_from_slice(commands);
        Ok(())
    }

    pub const fn ui_upload_size(&self) -> usize {
        self.ui_upload.len()
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
            if !self.ui_upload.is_empty() {
                self.ui_staging
                    .write_host_visible(&self.loader, 0, &self.ui_upload, 256)?;
                vulkan::cmd_copy_buffer_device(
                    &self.loader,
                    self._device.raw(),
                    frame.buffer,
                    self.ui_staging.raw(),
                    self.ui_device.raw(),
                    0,
                    0,
                    u64::try_from(self.ui_upload.len())
                        .map_err(|_| VulkanLoaderError::InvalidQueuePlan)?,
                )?;
                vulkan::cmd_buffer_barrier_device(
                    &self.loader,
                    self._device.raw(),
                    frame.buffer,
                    self.ui_device.raw(),
                    0,
                    u64::try_from(self.ui_upload.len())
                        .map_err(|_| VulkanLoaderError::InvalidQueuePlan)?,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    if self.ui_pipeline.is_some() && !self.ui_draw_commands.is_empty() {
                        VK_PIPELINE_STAGE_VERTEX_SHADER_BIT
                    } else {
                        VK_PIPELINE_STAGE_VERTEX_INPUT_BIT
                    },
                    VK_ACCESS_TRANSFER_WRITE_BIT,
                    if self.ui_pipeline.is_some() && !self.ui_draw_commands.is_empty() {
                        VK_ACCESS_SHADER_READ_BIT
                    } else {
                        VK_ACCESS_VERTEX_ATTRIBUTE_READ_BIT
                    },
                )?;
            }
            if !self.ui_draw_commands.is_empty()
                && self.ui_pipeline.is_some()
                && self.ui_targets.is_some()
            {
                let pipeline = self
                    .ui_pipeline
                    .as_ref()
                    .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
                let targets = self
                    .ui_targets
                    .as_ref()
                    .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
                let framebuffer = targets
                    .framebuffer(image as usize)
                    .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
                self._device.cmd_ui_draw(
                    &self.loader,
                    frame.buffer,
                    targets.render_pass(),
                    framebuffer,
                    pipeline.pipeline.raw(),
                    pipeline.pipeline_layout.raw(),
                    pipeline.descriptor_set,
                    self.extent,
                    self.color,
                    &self.ui_draw_commands,
                )?;
            } else {
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
            }
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
        self.ui_pipeline = None;
        self.ui_targets = None;
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
        self.ui_targets = Some(unsafe {
            UiFrameTargets::build(&self.loader, &self._device, &self.swapchain, format, extent)?
        });
        self.ui_pipeline = unsafe {
            UiNativePipeline::build(
                &self.loader,
                &self._device,
                self.ui_targets
                    .as_ref()
                    .expect("UI targets were just rebuilt"),
                self.ui_device.raw(),
                extent,
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
