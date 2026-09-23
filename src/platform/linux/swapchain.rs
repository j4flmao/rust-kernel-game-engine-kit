//! Native Vulkan KHR_swapchain facade for the Linux/X11 presentation path.
//!
//! All swapchain commands are device-level extension commands resolved
//! through `vkGetDeviceProcAddr`. The renderer owns the `Swapchain`; images
//! are presentable, so the engine can clear+presents them without any
//! framebuffer or pipeline state.
#![allow(unsafe_code)] // justified: raw Vulkan ABI facade

use core::ffi::c_void;

use super::vulkan::*;

const VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR: u32 = 0x0000_0001;
const VK_UINT64_MAX: u64 = u64::MAX;

/// Image format + color space pair, as enumerated for a surface.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceFormat {
    pub format: u32,
    pub color_space: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SurfaceCapabilities {
    pub min_image_count: u32,
    pub max_image_count: u32,
    pub current_extent: Extent2D,
    pub min_image_extent: Extent2D,
    pub max_image_extent: Extent2D,
    pub max_image_array_layers: u32,
    pub supported_transforms: u32,
    pub current_transform: u32,
    pub supported_composite_alpha: u32,
    pub supported_usage_flags: u32,
}

type VkGetPhysicalDeviceSurfaceSupportKHR =
    unsafe extern "C" fn(VkPhysicalDevice, u32, VkSurfaceKHR, *mut u32) -> VkResult;
type VkGetPhysicalDeviceSurfaceCapabilitiesKHR =
    unsafe extern "C" fn(VkPhysicalDevice, VkSurfaceKHR, *mut SurfaceCapabilities) -> VkResult;
type VkGetPhysicalDeviceSurfaceFormatsKHR =
    unsafe extern "C" fn(VkPhysicalDevice, VkSurfaceKHR, *mut u32, *mut SurfaceFormat) -> VkResult;
type VkGetPhysicalDeviceSurfacePresentModesKHR =
    unsafe extern "C" fn(VkPhysicalDevice, VkSurfaceKHR, *mut u32, *mut u32) -> VkResult;
type VkCreateSwapchainKHR = unsafe extern "C" fn(
    VkDevice,
    *const SwapchainCreateInfoKhr,
    *const c_void,
    *mut VkSwapchainKHR,
) -> VkResult;
type VkDestroySwapchainKHR = unsafe extern "C" fn(VkDevice, VkSwapchainKHR, *const c_void);
type VkGetSwapchainImagesKHR =
    unsafe extern "C" fn(VkDevice, VkSwapchainKHR, *mut u32, *mut VkImage) -> VkResult;
type VkAcquireNextImageKHR =
    unsafe extern "C" fn(VkDevice, VkSwapchainKHR, u64, VkSemaphore, VkFence, *mut u32) -> VkResult;
type VkQueuePresentKHR = unsafe extern "C" fn(VkQueue, *const PresentInfoKhr) -> VkResult;
type VkCmdPipelineBarrier = unsafe extern "C" fn(
    VkCommandBuffer,
    u32,
    u32,
    u32,
    u32,
    *const MemoryBarrier,
    u32,
    *const BufferMemoryBarrier,
    u32,
    *const ImageMemoryBarrier,
);
type VkCmdClearColorImage = unsafe extern "C" fn(
    VkCommandBuffer,
    VkImage,
    u32,
    *const ClearColorValue,
    u32,
    *const ImageSubresourceRange,
);

/// # Safety
/// `name` must be NUL-terminated and the returned address must carry the
/// Vulkan command signature `F`.
unsafe fn resolve_device<T>(
    loader: &VulkanLoader,
    device: VkDevice,
    name: &[u8],
) -> Result<T, VulkanLoaderError> {
    // SAFETY: enforced by the caller + this module's usage sites.
    let pointer = unsafe { loader.device_proc(device, name) }.ok_or_else(|| {
        VulkanLoaderError::MissingCommand(String::from_utf8_lossy(name).into_owned())
    })?;
    // SAFETY: F is the Vulkan ABI at each call site.
    Ok(unsafe { core::mem::transmute_copy(&pointer) })
}

/// Does `queue_family` support presenting to `surface` on `physical_device`?
/// # Safety
/// `physical_device`, `surface`, and `family` must be live.
pub unsafe fn surface_supported(
    loader: &VulkanLoader,
    physical_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
    queue_family: u32,
) -> Result<bool, VulkanLoaderError> {
    // SAFETY: physical_device and surface are live.
    let get_support: VkGetPhysicalDeviceSurfaceSupportKHR =
        loader.command(b"vkGetPhysicalDeviceSurfaceSupportKHR\0")?;
    let mut supported = 0u32;
    // SAFETY: all handles live and borrowed data scatter is safe.
    let result = unsafe { get_support(physical_device, queue_family, surface, &mut supported) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(supported != 0)
}

/// # Safety
/// `physical_device` and `surface` must be live.
pub unsafe fn query_capabilities(
    loader: &VulkanLoader,
    physical_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> Result<SurfaceCapabilities, VulkanLoaderError> {
    // SAFETY: handles live.
    let get: VkGetPhysicalDeviceSurfaceCapabilitiesKHR =
        loader.command(b"vkGetPhysicalDeviceSurfaceCapabilitiesKHR\0")?;
    let mut caps = SurfaceCapabilities {
        min_image_count: 0,
        max_image_count: 0,
        current_extent: Extent2D {
            width: 0,
            height: 0,
        },
        min_image_extent: Extent2D {
            width: 0,
            height: 0,
        },
        max_image_extent: Extent2D {
            width: 0,
            height: 0,
        },
        max_image_array_layers: 0,
        supported_transforms: 0,
        current_transform: 0,
        supported_composite_alpha: 0,
        supported_usage_flags: 0,
    };
    // SAFETY: caps has the documented ABI layout.
    let result = unsafe { get(physical_device, surface, &mut caps) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(caps)
}

/// # Safety
/// `physical_device` and `surface` must be live.
pub unsafe fn query_formats(
    loader: &VulkanLoader,
    physical_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> Result<Vec<SurfaceFormat>, VulkanLoaderError> {
    // SAFETY: handles live.
    let get: VkGetPhysicalDeviceSurfaceFormatsKHR =
        loader.command(b"vkGetPhysicalDeviceSurfaceFormatsKHR\0")?;
    let mut count = 0u32;
    // SAFETY: two-call discovery.
    let result = unsafe { get(physical_device, surface, &mut count, core::ptr::null_mut()) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    let mut formats = vec![
        SurfaceFormat {
            format: 0,
            color_space: 0
        };
        count as usize
    ];
    // SAFETY: capacity matches `count`; the driver fills at most that many.
    let result = unsafe { get(physical_device, surface, &mut count, formats.as_mut_ptr()) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    formats.truncate(count as usize);
    Ok(formats)
}

/// # Safety
/// `physical_device` and `surface` must be live.
pub unsafe fn query_present_modes(
    loader: &VulkanLoader,
    physical_device: VkPhysicalDevice,
    surface: VkSurfaceKHR,
) -> Result<Vec<u32>, VulkanLoaderError> {
    // SAFETY: handles live.
    let get: VkGetPhysicalDeviceSurfacePresentModesKHR =
        loader.command(b"vkGetPhysicalDeviceSurfacePresentModesKHR\0")?;
    let mut count = 0u32;
    // SAFETY: two-call discovery.
    let result = unsafe { get(physical_device, surface, &mut count, core::ptr::null_mut()) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    let mut modes = vec![0u32; count as usize];
    // SAFETY: capacity matches `count`.
    let result = unsafe { get(physical_device, surface, &mut count, modes.as_mut_ptr()) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    modes.truncate(count as usize);
    Ok(modes)
}

/// Picks the first requested format present in `formats`, else the driver's
/// first entry.
pub fn select_format(formats: &[SurfaceFormat], preferred: &[(u32, u32)]) -> SurfaceFormat {
    for &(format, color_space) in preferred {
        if formats
            .iter()
            .any(|candidate| candidate.format == format && candidate.color_space == color_space)
        {
            return SurfaceFormat {
                format,
                color_space,
            };
        }
    }
    formats.first().copied().unwrap_or(SurfaceFormat {
        format: VK_FORMAT_B8G8R8A8_UNORM,
        color_space: VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
    })
}

/// The renderer's visible swapchain. Images are presentable; the engine
/// clears them and presents without pipeline state.
pub struct Swapchain {
    device: VkDevice,
    handle: VkSwapchainKHR,
    destroy: VkDestroySwapchainKHR,
    get_images: VkGetSwapchainImagesKHR,
    acquire: VkAcquireNextImageKHR,
    images: Vec<VkImage>,
}

impl Swapchain {
    #[allow(clippy::too_many_arguments)]
    /// # Safety
    /// `loader`, `device`, and `surface` must be live; `queue_family` must support presenting.
    pub unsafe fn create(
        loader: &VulkanLoader,
        device: VkDevice,
        surface: VkSurfaceKHR,
        queue_family: u32,
        format: SurfaceFormat,
        extent: Extent2D,
        present_mode: u32,
        composite_alpha: u32,
        min_image_count: u32,
        old_swapchain: VkSwapchainKHR,
    ) -> Result<Self, VulkanLoaderError> {
        // SAFETY: device is live.
        let create: VkCreateSwapchainKHR =
            unsafe { resolve_device(loader, device, b"vkCreateSwapchainKHR\0")? };
        let destroy: VkDestroySwapchainKHR =
            unsafe { resolve_device(loader, device, b"vkDestroySwapchainKHR\0")? };
        let get_images: VkGetSwapchainImagesKHR =
            unsafe { resolve_device(loader, device, b"vkGetSwapchainImagesKHR\0")? };
        let acquire: VkAcquireNextImageKHR =
            unsafe { resolve_device(loader, device, b"vkAcquireNextImageKHR\0")? };
        let queue_families = [queue_family];
        let info = SwapchainCreateInfoKhr {
            s_type: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            next: core::ptr::null(),
            flags: 0,
            surface,
            min_image_count,
            image_format: format.format,
            image_color_space: format.color_space,
            image_extent: Extent2D {
                width: extent.width,
                height: extent.height,
            },
            image_array_layers: 1,
            image_usage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT,
            image_sharing_mode: VK_SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 1,
            queue_family_indices: queue_families.as_ptr(),
            pre_transform: VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR,
            composite_alpha,
            present_mode,
            clipped: 1,
            old_swapchain,
        };
        let mut handle = 0u64;
        // SAFETY: info fully described with live borrows; handle out-param live.
        let result = unsafe { create(device, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        let mut image_count = 0u32;
        // SAFETY: discovery call, image_count out-param live.
        let result = unsafe { get_images(device, handle, &mut image_count, core::ptr::null_mut()) };
        if result != VK_SUCCESS {
            // SAFETY: handle owned by this construction path so far.
            unsafe { destroy(device, handle, core::ptr::null()) };
            return Err(VulkanLoaderError::Api(result));
        }
        let mut images = vec![0u64; image_count as usize];
        // SAFETY: capacity matches; driver fills these VkImage handles.
        let result = unsafe { get_images(device, handle, &mut image_count, images.as_mut_ptr()) };
        if result != VK_SUCCESS {
            // SAFETY: same as above.
            unsafe { destroy(device, handle, core::ptr::null()) };
            return Err(VulkanLoaderError::Api(result));
        }
        images.truncate(image_count as usize);
        Ok(Self {
            device,
            handle,
            destroy,
            get_images,
            acquire,
            images,
        })
    }

    pub const fn image_count(&self) -> usize {
        self.images.len()
    }

    pub fn images(&self) -> &[VkImage] {
        &self.images
    }

    pub fn handle(&self) -> VkSwapchainKHR {
        self.handle
    }

    /// Blocks until an image is ready (UINT64_MAX timeout); returns its index.
    /// # Safety
    /// `frame` handles belong to a live swapchain/device.
    pub unsafe fn acquire(
        &mut self,
        ready: VkSemaphore,
        fence: VkFence,
    ) -> Result<u32, VulkanLoaderError> {
        let mut index = 0u32;
        // SAFETY: device/handles live; index out-param live.
        let result = unsafe {
            (self.acquire)(
                self.device,
                self.handle,
                VK_UINT64_MAX,
                ready,
                fence,
                &mut index,
            )
        };
        match result {
            VK_SUCCESS => Ok(index),
            VK_SUBOPTIMAL_KHR => Err(VulkanLoaderError::Suboptimal),
            VK_ERROR_OUT_OF_DATE_KHR => Err(VulkanLoaderError::OutOfDate),
            other => Err(VulkanLoaderError::Api(other)),
        }
    }

    /// Presents `image_index` on `queue` after `wait_semaphores` signal.
    /// # Safety
    /// `queue`,`wait` semaphores and `image_index` belong to this live swapchain.
    pub unsafe fn present(
        &mut self,
        loader: &VulkanLoader,
        queue: VkQueue,
        wait_semaphores: &[VkSemaphore],
        image_index: u32,
    ) -> Result<(), VulkanLoaderError> {
        // SAFETY: device is live.
        let present: VkQueuePresentKHR =
            unsafe { resolve_device(loader, self.device, b"vkQueuePresentKHR\0")? };
        let swapchains = [self.handle];
        let indices = [image_index];
        let info = PresentInfoKhr {
            s_type: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            next: core::ptr::null(),
            wait_semaphore_count: wait_semaphores.len() as u32,
            wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            swapchains: swapchains.as_ptr(),
            image_indices: indices.as_ptr(),
            results: core::ptr::null_mut(),
        };
        // SAFETY: info fully described; queue live.
        let result = unsafe { present(queue, &info) };
        match result {
            VK_SUCCESS => Ok(()),
            VK_SUBOPTIMAL_KHR => Err(VulkanLoaderError::Suboptimal),
            VK_ERROR_OUT_OF_DATE_KHR => Err(VulkanLoaderError::OutOfDate),
            other => Err(VulkanLoaderError::Api(other)),
        }
    }

    /// Rebuilds this swapchain against the same device/surface.
    #[allow(clippy::too_many_arguments)]
    /// # Safety
    /// `loader`/`device`/`surface` live; `queue_family` still supports presenting.
    pub unsafe fn recreate(
        &mut self,
        loader: &VulkanLoader,
        surface: VkSurfaceKHR,
        queue_family: u32,
        format: SurfaceFormat,
        extent: Extent2D,
        present_mode: u32,
        composite_alpha: u32,
        min_image_count: u32,
    ) -> Result<(), VulkanLoaderError> {
        let old = self.handle;
        // SAFETY: parameters mirror the live swapchain; old handed off to default.
        let mut replacement = unsafe {
            Self::create(
                loader,
                self.device,
                surface,
                queue_family,
                format,
                extent,
                present_mode,
                composite_alpha,
                min_image_count,
                old,
            )
        }?;
        self.destroy = replacement.destroy;
        self.get_images = replacement.get_images;
        self.acquire = replacement.acquire;
        self.handle = replacement.handle;
        self.images.clear();
        self.images.extend(core::mem::take(&mut replacement.images));
        // old swapchain is destroyed by vkCreateSwapchainKHR; don't double free.
        let _ = old;
        Ok(())
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // SAFETY: device alive for as long as self; fine to destroy before device teardown.
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

/// Transitions `image` between layouts for presentable clear/present.
/// In the raw path the engine always re-acquires with an undefined-layout
/// image (the driver resets it to UNDEFINED on acquire), so no ownership
/// assumptions are needed.
/// # Safety
/// `image`,`command_buffer` live; `loader`/`device` handles bound.
pub unsafe fn cmd_image_layout_transition(
    loader: &VulkanLoader,
    device: VkDevice,
    command_buffer: VkCommandBuffer,
    image: VkImage,
    old_layout: u32,
    new_layout: u32,
) -> Result<(), VulkanLoaderError> {
    // SAFETY: device is live.
    let barrier: VkCmdPipelineBarrier =
        unsafe { resolve_device(loader, device, b"vkCmdPipelineBarrier\0")? };
    let range = ImageSubresourceRange {
        aspect_mask: VK_IMAGE_ASPECT_COLOR_BIT,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };
    let image_barrier = ImageMemoryBarrier {
        s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
        next: core::ptr::null(),
        src_access_mask: 0,
        dst_access_mask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
        old_layout,
        new_layout,
        src_queue_family_index: 0,
        dst_queue_family_index: 0,
        image,
        subresource_range: range,
    };
    // SAFETY: barrier describes the image with matching layout range.
    unsafe {
        barrier(
            command_buffer,
            VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
            VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            0,
            0,
            core::ptr::null(),
            0,
            core::ptr::null(),
            1,
            &image_barrier,
        )
    };
    Ok(())
}

/// Records a full-image clear into `command_buffer`.
/// # Safety
/// `image`,`command_buffer` live; `loader`/`device` handles bound.
pub unsafe fn cmd_clear_image(
    loader: &VulkanLoader,
    device: VkDevice,
    command_buffer: VkCommandBuffer,
    image: VkImage,
    color: [f32; 4],
) -> Result<(), VulkanLoaderError> {
    // SAFETY: device is live.
    let clear: VkCmdClearColorImage =
        unsafe { resolve_device(loader, device, b"vkCmdClearColorImage\0")? };
    let range = ImageSubresourceRange {
        aspect_mask: VK_IMAGE_ASPECT_COLOR_BIT,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    };
    let value = ClearColorValue { float32: color };
    // SAFETY: image layout GENERAL, ranges valid.
    unsafe {
        clear(
            command_buffer,
            image,
            VK_IMAGE_LAYOUT_GENERAL,
            &value,
            1,
            &range,
        )
    };
    Ok(())
}

/// Number of swapchain images to request for a given width (>= 3 when the
/// driver allows it, else clamped).
pub fn default_image_count(caps: &SurfaceCapabilities) -> u32 {
    if caps.min_image_count == 0 {
        return 3;
    }
    let desired = caps.min_image_count + 1;
    if caps.max_image_count != 0 {
        desired.clamp(caps.min_image_count, caps.max_image_count)
    } else {
        desired.max(caps.min_image_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyCaps(SurfaceCapabilities);

    impl DummyCaps {
        fn new() -> Self {
            Self(SurfaceCapabilities {
                min_image_count: 0,
                max_image_count: 0,
                current_extent: Extent2D {
                    width: 1920,
                    height: 1080,
                },
                min_image_extent: Extent2D {
                    width: 1,
                    height: 1,
                },
                max_image_extent: Extent2D {
                    width: 8192,
                    height: 8192,
                },
                max_image_array_layers: 1,
                supported_transforms: 0,
                current_transform: 0,
                supported_composite_alpha: 0,
                supported_usage_flags: 0,
            })
        }
    }

    #[test]
    fn caps_with_unlimited_max_requests_three() {
        let caps = DummyCaps::new().0;
        assert_eq!(default_image_count(&caps), 3);
    }

    #[test]
    fn caps_with_max_clamp_respects_driver_limit() {
        let caps = SurfaceCapabilities {
            min_image_count: 2,
            max_image_count: 3,
            ..DummyCaps::new().0
        };
        assert_eq!(default_image_count(&caps), 3);
    }

    #[test]
    fn selects_preferred_format_when_available() {
        let formats = [
            SurfaceFormat {
                format: VK_FORMAT_R8G8B8A8_UNORM,
                color_space: VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
            },
            SurfaceFormat {
                format: VK_FORMAT_B8G8R8A8_UNORM,
                color_space: VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
            },
        ];
        let picked = select_format(
            &formats,
            &[(VK_FORMAT_B8G8R8A8_UNORM, VK_COLOR_SPACE_SRGB_NONLINEAR_KHR)],
        );
        assert_eq!(picked.format, VK_FORMAT_B8G8R8A8_UNORM);
    }

    #[test]
    fn falls_back_to_first_surface_format() {
        let formats = [SurfaceFormat {
            format: VK_FORMAT_R8G8B8A8_UNORM,
            color_space: VK_COLOR_SPACE_SRGB_NONLINEAR_KHR,
        }];
        let picked = select_format(
            &formats,
            &[(VK_FORMAT_B8G8R8A8_UNORM, VK_COLOR_SPACE_SRGB_NONLINEAR_KHR)],
        );
        assert_eq!(picked.format, VK_FORMAT_R8G8B8A8_UNORM);
        let _ = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    }
}
