//! Windows GPU backend facade.
//!
//! The Vulkan ABI implementation stays isolated in `vulkan`; this module is
//! the stable platform entry point consumed by the renderer.
pub use super::surface::{create as create_surface, Win32Surface};
pub use super::swapchain::{
    cmd_clear_image, cmd_image_layout_transition, default_image_count, query_capabilities,
    query_formats, query_present_modes, select_format, surface_supported, SurfaceCapabilities,
    SurfaceFormat, Swapchain,
};
pub use super::vulkan::{
    begin_command_buffer, end_command_buffer, reset_command_buffer, LogicalDevice,
    NativeCommandPool, NativeFence, NativeInstance, NativeSemaphore, PhysicalDeviceInfo,
    VkCommandBuffer, VkCommandPool, VkDevice, VkFence, VkImage, VkInstance, VkPhysicalDevice,
    VkQueue, VkSemaphore, VkSurfaceKHR, VkSwapchainKHR, VulkanLoader, VulkanLoaderError,
    VK_FORMAT_B8G8R8A8_UNORM, VK_FORMAT_R8G8B8A8_UNORM, VK_IMAGE_LAYOUT_GENERAL,
    VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_USAGE_TRANSFER_DST_BIT, VK_KHR_SURFACE,
    VK_KHR_WIN32_SURFACE,
};
