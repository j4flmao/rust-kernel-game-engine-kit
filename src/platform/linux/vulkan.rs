//! Minimal Vulkan loader boundary for Linux.
#![allow(unsafe_code)]

use core::ffi::c_void;

use super::dl::{DlError, DynamicLibrary};
use crate::platform::vulkan_policy::{
    QueueCreateRequest, QueueFamilyInfo, QUEUE_COMPUTE, QUEUE_TRANSFER,
};

pub type VkInstance = *mut c_void;
pub type VkPhysicalDevice = *mut c_void;
pub type VkDevice = *mut c_void;
pub type VkQueue = *mut c_void;
pub type VkCommandPool = *mut c_void;
pub type VkCommandBuffer = *mut c_void;
pub type VkBuffer = u64;
pub type VkDeviceMemory = u64;
pub type VkFence = *mut c_void;
pub type VkSemaphore = *mut c_void;
pub type VkSurfaceKHR = u64;
pub type VkSwapchainKHR = u64;
pub type VkImage = u64;
pub type VkResult = i32;
pub type VkGetInstanceProcAddr = unsafe extern "C" fn(VkInstance, *const u8) -> *const c_void;
pub type VkGetDeviceProcAddr = unsafe extern "C" fn(VkDevice, *const u8) -> *const c_void;
pub type VkEnumerateInstanceVersion = unsafe extern "C" fn(*mut u32) -> VkResult;
type VkCreateInstance =
    unsafe extern "C" fn(*const InstanceCreateInfo, *const c_void, *mut VkInstance) -> VkResult;
type VkDestroyInstance = unsafe extern "C" fn(VkInstance, *const c_void);
type VkEnumeratePhysicalDevices =
    unsafe extern "C" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
type VkGetPhysicalDeviceProperties =
    unsafe extern "C" fn(VkPhysicalDevice, *mut PhysicalDeviceProperties);
type VkGetPhysicalDeviceQueueFamilyProperties =
    unsafe extern "C" fn(VkPhysicalDevice, *mut u32, *mut QueueFamilyProperties);
type VkCreateDevice = unsafe extern "C" fn(
    VkPhysicalDevice,
    *const DeviceCreateInfo,
    *const c_void,
    *mut VkDevice,
) -> VkResult;
type VkDestroyDevice = unsafe extern "C" fn(VkDevice, *const c_void);
type VkGetDeviceQueue = unsafe extern "C" fn(VkDevice, u32, u32, *mut VkQueue);
type VkCreateCommandPool = unsafe extern "C" fn(
    VkDevice,
    *const CommandPoolCreateInfo,
    *const c_void,
    *mut VkCommandPool,
) -> VkResult;
type VkDestroyCommandPool = unsafe extern "C" fn(VkDevice, VkCommandPool, *const c_void);
type VkAllocateCommandBuffers = unsafe extern "C" fn(
    VkDevice,
    *const CommandBufferAllocateInfo,
    *mut VkCommandBuffer,
) -> VkResult;
type VkFreeCommandBuffers =
    unsafe extern "C" fn(VkDevice, VkCommandPool, u32, *const VkCommandBuffer);
type VkBeginCommandBuffer =
    unsafe extern "C" fn(VkCommandBuffer, *const CommandBufferBeginInfo) -> VkResult;
type VkEndCommandBuffer = unsafe extern "C" fn(VkCommandBuffer) -> VkResult;
type VkResetCommandBuffer = unsafe extern "C" fn(VkCommandBuffer, u32) -> VkResult;
type VkCreateBuffer = unsafe extern "C" fn(
    VkDevice,
    *const BufferCreateInfo,
    *const c_void,
    *mut VkBuffer,
) -> VkResult;
type VkDestroyBuffer = unsafe extern "C" fn(VkDevice, VkBuffer, *const c_void);
type VkGetBufferMemoryRequirements =
    unsafe extern "C" fn(VkDevice, VkBuffer, *mut MemoryRequirements);
type VkAllocateMemory = unsafe extern "C" fn(
    VkDevice,
    *const MemoryAllocateInfo,
    *const c_void,
    *mut VkDeviceMemory,
) -> VkResult;
type VkFreeMemory = unsafe extern "C" fn(VkDevice, VkDeviceMemory, *const c_void);
type VkBindBufferMemory = unsafe extern "C" fn(VkDevice, VkBuffer, VkDeviceMemory, u64) -> VkResult;
type VkMapMemory =
    unsafe extern "C" fn(VkDevice, VkDeviceMemory, u64, u64, u32, *mut *mut c_void) -> VkResult;
type VkUnmapMemory = unsafe extern "C" fn(VkDevice, VkDeviceMemory);
type VkFlushMappedMemoryRanges =
    unsafe extern "C" fn(VkDevice, u32, *const MappedMemoryRange) -> VkResult;
type VkGetPhysicalDeviceMemoryProperties =
    unsafe extern "C" fn(VkPhysicalDevice, *mut PhysicalDeviceMemoryProperties);
type VkCmdDispatch = unsafe extern "C" fn(VkCommandBuffer, u32, u32, u32);
type VkCmdDrawIndexedIndirect = unsafe extern "C" fn(VkCommandBuffer, VkBuffer, u64, u32, u32);
type VkCmdCopyBuffer =
    unsafe extern "C" fn(VkCommandBuffer, VkBuffer, VkBuffer, u32, *const BufferCopy);
type VkCmdPipelineBarrier = unsafe extern "C" fn(
    VkCommandBuffer,
    u32,
    u32,
    u32,
    u32,
    *const c_void,
    u32,
    *const BufferMemoryBarrier,
    u32,
    *const c_void,
);
type VkCreateFence =
    unsafe extern "C" fn(VkDevice, *const FenceCreateInfo, *const c_void, *mut VkFence) -> VkResult;
type VkDestroyFence = unsafe extern "C" fn(VkDevice, VkFence, *const c_void);
type VkWaitForFences = unsafe extern "C" fn(VkDevice, u32, *const VkFence, u32, u64) -> VkResult;
type VkResetFences = unsafe extern "C" fn(VkDevice, u32, *const VkFence) -> VkResult;
type VkCreateSemaphore = unsafe extern "C" fn(
    VkDevice,
    *const SemaphoreCreateInfo,
    *const c_void,
    *mut VkSemaphore,
) -> VkResult;
type VkDestroySemaphore = unsafe extern "C" fn(VkDevice, VkSemaphore, *const c_void);
type VkQueueSubmit = unsafe extern "C" fn(VkQueue, u32, *const SubmitInfo, VkFence) -> VkResult;

pub const VK_SUCCESS: VkResult = 0;
pub const VK_INCOMPLETE: VkResult = 5;
pub const VK_API_VERSION_1_0: u32 = 0x0040_0000;
pub const VK_SUBOPTIMAL_KHR: VkResult = 1000001003;
pub const VK_ERROR_OUT_OF_DATE_KHR: VkResult = -1000001004;
pub const VK_NOT_READY: VkResult = 1;

pub const VK_FORMAT_B8G8R8A8_UNORM: u32 = 44;
pub const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
pub const VK_FORMAT_UNDEFINED: u32 = 0;
pub const VK_COLOR_SPACE_SRGB_NONLINEAR_KHR: u32 = 0;

pub const VK_PRESENT_MODE_IMMEDIATE_KHR: u32 = 0;
pub const VK_PRESENT_MODE_MAILBOX_KHR: u32 = 1;
pub const VK_PRESENT_MODE_FIFO_KHR: u32 = 2;
pub const VK_PRESENT_MODE_FIFO_RELAXED_KHR: u32 = 3;

pub const VK_IMAGE_USAGE_TRANSFER_DST_BIT: u32 = 0x0000_0008;
pub const VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT: u32 = 0x0000_0010;
pub const VK_IMAGE_LAYOUT_UNDEFINED: u32 = 0;
pub const VK_IMAGE_LAYOUT_GENERAL: u32 = 1;
pub const VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: u32 = 1000001002;
pub const VK_IMAGE_ASPECT_COLOR_BIT: u32 = 0x0000_0001;
pub const VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR: u32 = 0x0000_0001;
pub const VK_SHARING_MODE_EXCLUSIVE: u32 = 0;
pub const VK_IMAGE_TILING_OPTIMAL: u32 = 0;
pub const VK_BUFFER_USAGE_STORAGE_BUFFER_BIT: u32 = 0x0000_0020;
pub const VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT: u32 = 0x0000_0100;
pub const VK_BUFFER_USAGE_TRANSFER_SRC_BIT: u32 = 0x0000_0001;
pub const VK_BUFFER_USAGE_TRANSFER_DST_BIT: u32 = 0x0000_0002;
pub const VK_PIPELINE_STAGE_TRANSFER_BIT: u32 = 0x0000_0100;
pub const VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT: u32 = 0x0000_0800;
pub const VK_PIPELINE_STAGE_DRAW_INDIRECT_BIT: u32 = 0x0000_0200;
pub const VK_ACCESS_TRANSFER_WRITE_BIT: u32 = 0x0000_1000;
pub const VK_ACCESS_SHADER_READ_BIT: u32 = 0x0000_0020;
pub const VK_ACCESS_SHADER_WRITE_BIT: u32 = 0x0000_0040;
pub const VK_ACCESS_INDIRECT_COMMAND_READ_BIT: u32 = 0x0000_0002;
pub const VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT: u32 = 0x0000_0001;
pub const VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT: u32 = 0x0000_0002;
pub const VK_MEMORY_PROPERTY_HOST_COHERENT_BIT: u32 = 0x0000_0004;

pub const VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT: u32 = 0x0000_0001;
pub const VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT: u32 = 0x0000_0400;
pub const VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT: u32 = 0x0000_0010;
pub const VK_SOURCE_STAGE_UNDEFINED: u32 = VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT;

pub const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: u32 = 1000001000;
pub const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: u32 = 1000001001;
pub const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: u32 = 15;

pub const VK_KHR_SURFACE: &[u8] = b"VK_KHR_surface\0";
pub const VK_KHR_XLIB_SURFACE: &[u8] = b"VK_KHR_xlib_surface\0";
pub const VK_KHR_SWAPCHAIN: &[u8] = b"VK_KHR_swapchain\0";

#[repr(C)]
struct ApplicationInfo {
    s_type: u32,
    next: *const c_void,
    application_name: *const u8,
    application_version: u32,
    engine_name: *const u8,
    engine_version: u32,
    api_version: u32,
}
#[repr(C)]
struct InstanceCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    application_info: *const ApplicationInfo,
    layer_count: u32,
    layers: *const *const u8,
    extension_count: u32,
    extensions: *const *const u8,
}

#[repr(C)]
struct DeviceQueueCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    queue_family_index: u32,
    queue_count: u32,
    queue_priorities: *const f32,
}
#[repr(C)]
struct DeviceCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    queue_create_info_count: u32,
    queue_create_infos: *const DeviceQueueCreateInfo,
    enabled_layer_count: u32,
    enabled_layers: *const *const u8,
    enabled_extension_count: u32,
    enabled_extensions: *const *const u8,
    enabled_features: *const c_void,
}

#[repr(C)]
struct CommandPoolCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    queue_family_index: u32,
}
#[repr(C)]
struct CommandBufferAllocateInfo {
    s_type: u32,
    next: *const c_void,
    command_pool: VkCommandPool,
    level: u32,
    command_buffer_count: u32,
}
#[repr(C)]
struct CommandBufferBeginInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    inheritance_info: *const c_void,
}
#[repr(C)]
struct FenceCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
}
#[repr(C)]
struct SemaphoreCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
}
#[repr(C)]
struct SubmitInfo {
    s_type: u32,
    next: *const c_void,
    wait_count: u32,
    waits: *const VkSemaphore,
    wait_stages: *const u32,
    command_count: u32,
    commands: *const VkCommandBuffer,
    signal_count: u32,
    signals: *const VkSemaphore,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
pub struct SwapchainCreateInfoKhr {
    pub s_type: u32,
    pub next: *const c_void,
    pub flags: u32,
    pub surface: VkSurfaceKHR,
    pub min_image_count: u32,
    pub image_format: u32,
    pub image_color_space: u32,
    pub image_extent: Extent2D,
    pub image_array_layers: u32,
    pub image_usage: u32,
    pub image_sharing_mode: u32,
    pub queue_family_index_count: u32,
    pub queue_family_indices: *const u32,
    pub pre_transform: u32,
    pub composite_alpha: u32,
    pub present_mode: u32,
    pub clipped: u32,
    pub old_swapchain: VkSwapchainKHR,
}

#[repr(C)]
pub struct PresentInfoKhr {
    pub s_type: u32,
    pub next: *const c_void,
    pub wait_semaphore_count: u32,
    pub wait_semaphores: *const VkSemaphore,
    pub swapchain_count: u32,
    pub swapchains: *const VkSwapchainKHR,
    pub image_indices: *const u32,
    pub results: *mut VkResult,
}

#[repr(C)]
pub struct ImageSubresourceRange {
    pub aspect_mask: u32,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ClearColorValue {
    pub float32: [f32; 4],
}

#[repr(C)]
pub struct MemoryBarrier {
    pub s_type: u32,
    pub next: *const c_void,
    pub src_access_mask: u32,
    pub dst_access_mask: u32,
}

#[repr(C)]
pub struct BufferMemoryBarrier {
    pub s_type: u32,
    pub next: *const c_void,
    pub src_access_mask: u32,
    pub dst_access_mask: u32,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub buffer: u64,
    pub offset: u64,
    pub size: u64,
}

#[repr(C)]
pub struct ImageMemoryBarrier {
    pub s_type: u32,
    pub next: *const c_void,
    pub src_access_mask: u32,
    pub dst_access_mask: u32,
    pub old_layout: u32,
    pub new_layout: u32,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub image: VkImage,
    pub subresource_range: ImageSubresourceRange,
}
#[repr(C)]
struct PhysicalDeviceProperties {
    bytes: [u8; 824],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct QueueFamilyProperties {
    flags: u32,
    queue_count: u32,
    min_image_transfer_granularity: [u32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalDeviceInfo {
    pub api_version: u32,
    pub driver_version: u32,
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_type: u32,
    pub name: String,
    pub queue_families: Vec<QueueFamilyInfo>,
}

impl PhysicalDeviceInfo {
    pub fn selection_score(&self) -> u8 {
        match self.device_type {
            2 => 4,
            1 => 3,
            3 => 2,
            4 => 1,
            _ => 0,
        }
    }
    pub fn select_preferred(devices: &[Self]) -> Option<&Self> {
        devices.iter().max_by_key(|device| device.selection_score())
    }
}

#[derive(Debug)]
pub enum VulkanLoaderError {
    Library(DlError),
    MissingEntry(DlError),
    ApiQuery(VkResult),
    MissingCommand(String),
    Api(VkResult),
    NoPhysicalDevices,
    InvalidQueuePlan,
    /// The swapchain images are out of date with the surface (resize); the
    /// renderer must recreate the swapchain.
    OutOfDate,
    /// The swapchain works but its configuration is now suboptimal.
    Suboptimal,
    AllocationFailed,
}

impl core::fmt::Display for VulkanLoaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Library(error) => write!(f, "failed to load Linux Vulkan loader: {error}"),
            Self::MissingEntry(error) => write!(f, "Linux Vulkan entry point missing: {error}"),
            Self::ApiQuery(result) => write!(
                f,
                "vkEnumerateInstanceVersion failed with VkResult {result}"
            ),
            Self::MissingCommand(name) => write!(f, "Vulkan command missing: {name}"),
            Self::Api(result) => write!(f, "Vulkan call failed with VkResult {result}"),
            Self::NoPhysicalDevices => write!(f, "Vulkan reported no physical devices"),
            Self::InvalidQueuePlan => {
                f.write_str("Vulkan logical-device queue plan is empty or invalid")
            }
            Self::OutOfDate => {
                f.write_str("swapchain is out of date with the presentation surface")
            }
            Self::Suboptimal => f.write_str("swapchain is suboptimal for the current surface"),
            Self::AllocationFailed => f.write_str("Vulkan enumeration allocation failed"),
        }
    }
}

impl core::error::Error for VulkanLoaderError {}

pub struct LogicalDevice {
    handle: VkDevice,
    destroy: VkDestroyDevice,
    get_queue: VkGetDeviceQueue,
}

/// A live Vulkan instance created with the renderer's required extensions.
/// Kept opaque; the renderer uses `VulkanLoader` and raw handles.
pub struct NativeInstance {
    instance: VkInstance,
    destroy: VkDestroyInstance,
}

impl NativeInstance {
    pub const fn raw(&self) -> VkInstance {
        self.instance
    }
}

impl Drop for NativeInstance {
    fn drop(&mut self) {
        // SAFETY: instance was created successfully and is owned by this wrapper.
        unsafe { (self.destroy)(self.instance, core::ptr::null()) };
    }
}

/// # Safety
/// `buffer` must be a live command buffer owned by a live device.
pub unsafe fn begin_command_buffer(
    loader: &VulkanLoader,
    buffer: VkCommandBuffer,
) -> Result<(), VulkanLoaderError> {
    if buffer.is_null() {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let begin: VkBeginCommandBuffer = loader.command(b"vkBeginCommandBuffer\0")?;
    let info = CommandBufferBeginInfo {
        s_type: 42,
        next: core::ptr::null(),
        flags: 0,
        inheritance_info: core::ptr::null(),
    };
    // SAFETY: buffer and begin info are valid for the synchronous call.
    let result = unsafe { begin(buffer, &info) };
    if result == VK_SUCCESS {
        Ok(())
    } else {
        Err(VulkanLoaderError::Api(result))
    }
}

/// # Safety
/// `buffer` must be a live command buffer currently recording.
pub unsafe fn end_command_buffer(
    loader: &VulkanLoader,
    buffer: VkCommandBuffer,
) -> Result<(), VulkanLoaderError> {
    if buffer.is_null() {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let end: VkEndCommandBuffer = loader.command(b"vkEndCommandBuffer\0")?;
    // SAFETY: buffer is a live recording command buffer.
    let result = unsafe { end(buffer) };
    if result == VK_SUCCESS {
        Ok(())
    } else {
        Err(VulkanLoaderError::Api(result))
    }
}

/// # Safety
/// `buffer` must be a live command buffer not pending execution.
pub unsafe fn reset_command_buffer(
    loader: &VulkanLoader,
    buffer: VkCommandBuffer,
) -> Result<(), VulkanLoaderError> {
    if buffer.is_null() {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let reset: VkResetCommandBuffer = loader.command(b"vkResetCommandBuffer\0")?;
    // SAFETY: buffer is a live command buffer not pending execution.
    let result = unsafe { reset(buffer, 0) };
    if result == VK_SUCCESS {
        Ok(())
    } else {
        Err(VulkanLoaderError::Api(result))
    }
}
#[repr(C)]
struct BufferCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    size: u64,
    usage: u32,
    sharing_mode: u32,
    queue_family_index_count: u32,
    queue_family_indices: *const u32,
}
#[repr(C)]
struct MemoryRequirements {
    size: u64,
    alignment: u64,
    memory_type_bits: u32,
}
#[repr(C)]
struct BufferCopy {
    src_offset: u64,
    dst_offset: u64,
    size: u64,
}
#[repr(C)]
struct MappedMemoryRange {
    s_type: u32,
    next: *const c_void,
    memory: VkDeviceMemory,
    offset: u64,
    size: u64,
}
#[repr(C)]
struct MemoryAllocateInfo {
    s_type: u32,
    next: *const c_void,
    allocation_size: u64,
    memory_type_index: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryType {
    property_flags: u32,
    heap_index: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryHeap {
    size: u64,
    flags: u32,
    _padding: u32,
}
#[repr(C)]
struct PhysicalDeviceMemoryProperties {
    memory_type_count: u32,
    memory_types: [MemoryType; 32],
    memory_heap_count: u32,
    memory_heaps: [MemoryHeap; 16],
}

/// Records a compute dispatch for a live command buffer.
///
/// # Safety
/// `buffer` must be a live command buffer currently recording, and the caller
/// must have bound a compatible compute pipeline and descriptor state.
pub unsafe fn cmd_dispatch(
    loader: &VulkanLoader,
    buffer: VkCommandBuffer,
    groups: [u32; 3],
) -> Result<(), VulkanLoaderError> {
    if buffer.is_null() || groups.contains(&0) {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let dispatch: VkCmdDispatch = loader.command(b"vkCmdDispatch\0")?;
    // SAFETY: caller guarantees a recording command buffer and compatible state.
    unsafe { dispatch(buffer, groups[0], groups[1], groups[2]) };
    Ok(())
}

/// Records a compute dispatch through the logical-device dispatch table.
///
/// This is the preferred entry point for native rendering. The legacy
/// [`cmd_dispatch`] wrapper remains available for loader-level bring-up, but
/// a live device must use this variant so ICD-specific command pointers are
/// resolved through `vkGetDeviceProcAddr`.
///
/// # Safety
/// `device` and `buffer` must be live objects from the same Vulkan device.
/// The command buffer must be recording with a compatible compute pipeline.
pub unsafe fn cmd_dispatch_device(
    loader: &VulkanLoader,
    device: VkDevice,
    buffer: VkCommandBuffer,
    groups: [u32; 3],
) -> Result<(), VulkanLoaderError> {
    if device.is_null() || buffer.is_null() || groups.contains(&0) {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let dispatch: VkCmdDispatch = loader.device_command(device, b"vkCmdDispatch\0")?;
    // SAFETY: caller guarantees a recording command buffer and compatible state.
    unsafe { dispatch(buffer, groups[0], groups[1], groups[2]) };
    Ok(())
}

/// Records indexed indirect draws for a live command buffer.
///
/// # Safety
/// `buffer` and `indirect` must belong to the live device, the command buffer
/// must be recording, and the indirect range must be valid for the bound draw
/// pipeline.
pub unsafe fn cmd_draw_indexed_indirect(
    loader: &VulkanLoader,
    buffer: VkCommandBuffer,
    indirect: VkBuffer,
    offset: u64,
    draw_count: u32,
    stride: u32,
) -> Result<(), VulkanLoaderError> {
    if buffer.is_null() || indirect == 0 || draw_count == 0 || stride == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let draw: VkCmdDrawIndexedIndirect = loader.command(b"vkCmdDrawIndexedIndirect\0")?;
    // SAFETY: caller guarantees a recording command buffer and valid buffer range.
    unsafe { draw(buffer, indirect, offset, draw_count, stride) };
    Ok(())
}

/// Records indexed indirect draws through the logical-device dispatch table.
///
/// # Safety
/// `device`, `buffer`, and `indirect` must belong to the same live device;
/// the command buffer must be recording and the indirect range must be valid
/// for the bound graphics pipeline.
pub unsafe fn cmd_draw_indexed_indirect_device(
    loader: &VulkanLoader,
    device: VkDevice,
    buffer: VkCommandBuffer,
    indirect: VkBuffer,
    offset: u64,
    draw_count: u32,
    stride: u32,
) -> Result<(), VulkanLoaderError> {
    if device.is_null() || buffer.is_null() || indirect == 0 || draw_count == 0 || stride == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let draw: VkCmdDrawIndexedIndirect =
        loader.device_command(device, b"vkCmdDrawIndexedIndirect\0")?;
    // SAFETY: caller guarantees a recording command buffer and valid buffer range.
    unsafe { draw(buffer, indirect, offset, draw_count, stride) };
    Ok(())
}

/// Records one bounded staging-to-device buffer copy.
///
/// # Safety
/// All handles must belong to the same live device and the command buffer must
/// be recording. The caller must provide the required transfer barriers.
pub unsafe fn cmd_copy_buffer_device(
    loader: &VulkanLoader,
    device: VkDevice,
    command: VkCommandBuffer,
    source: VkBuffer,
    destination: VkBuffer,
    source_offset: u64,
    destination_offset: u64,
    size: u64,
) -> Result<(), VulkanLoaderError> {
    if device.is_null() || command.is_null() || source == 0 || destination == 0 || size == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let source_end = source_offset
        .checked_add(size)
        .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
    let destination_end = destination_offset
        .checked_add(size)
        .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
    if source_end < source_offset || destination_end < destination_offset {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let copy: VkCmdCopyBuffer = loader.device_command(device, b"vkCmdCopyBuffer\0")?;
    let region = BufferCopy {
        src_offset: source_offset,
        dst_offset: destination_offset,
        size,
    };
    unsafe { copy(command, source, destination, 1, &region) };
    Ok(())
}

/// Inserts one buffer barrier for a transfer/compute/draw handoff.
///
/// # Safety
/// `command` must be recording and `buffer` must belong to `device`.
pub unsafe fn cmd_buffer_barrier_device(
    loader: &VulkanLoader,
    device: VkDevice,
    command: VkCommandBuffer,
    buffer: VkBuffer,
    offset: u64,
    size: u64,
    src_stage: u32,
    dst_stage: u32,
    src_access: u32,
    dst_access: u32,
) -> Result<(), VulkanLoaderError> {
    if device.is_null()
        || command.is_null()
        || buffer == 0
        || size == 0
        || src_stage == 0
        || dst_stage == 0
        || offset.checked_add(size).is_none()
    {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let barrier: VkCmdPipelineBarrier = loader.device_command(device, b"vkCmdPipelineBarrier\0")?;
    let info = BufferMemoryBarrier {
        s_type: 44,
        next: core::ptr::null(),
        src_access_mask: src_access,
        dst_access_mask: dst_access,
        src_queue_family_index: u32::MAX,
        dst_queue_family_index: u32::MAX,
        buffer,
        offset,
        size,
    };
    unsafe {
        barrier(
            command,
            src_stage,
            dst_stage,
            0,
            0,
            core::ptr::null(),
            1,
            &info,
            0,
            core::ptr::null(),
        )
    };
    Ok(())
}

pub struct NativeCommandPool {
    handle: VkCommandPool,
    destroy: VkDestroyCommandPool,
    device: VkDevice,
}

pub struct NativeFence {
    handle: VkFence,
    device: VkDevice,
    destroy: VkDestroyFence,
    wait: VkWaitForFences,
    reset: VkResetFences,
}
impl NativeFence {
    pub const fn raw(&self) -> VkFence {
        self.handle
    }
    pub fn wait(&self, timeout_ns: u64) -> Result<(), VulkanLoaderError> {
        // SAFETY: fence belongs to the live device.
        let result = unsafe { (self.wait)(self.device, 1, &self.handle, 1, timeout_ns) };
        if result == VK_SUCCESS {
            Ok(())
        } else {
            Err(VulkanLoaderError::Api(result))
        }
    }
    pub fn reset(&self) -> Result<(), VulkanLoaderError> {
        // SAFETY: fence belongs to the live device.
        let result = unsafe { (self.reset)(self.device, 1, &self.handle) };
        if result == VK_SUCCESS {
            Ok(())
        } else {
            Err(VulkanLoaderError::Api(result))
        }
    }
}
impl Drop for NativeFence {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativeSemaphore {
    handle: VkSemaphore,
    device: VkDevice,
    destroy: VkDestroySemaphore,
}
impl NativeSemaphore {
    pub const fn raw(&self) -> VkSemaphore {
        self.handle
    }
}
impl Drop for NativeSemaphore {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

/// Owns a Vulkan buffer and its backing allocation.
pub struct NativeBuffer {
    handle: VkBuffer,
    memory: VkDeviceMemory,
    size: u64,
    device: VkDevice,
    destroy_buffer: VkDestroyBuffer,
    free_memory: VkFreeMemory,
    memory_properties: u32,
}

impl NativeBuffer {
    pub const fn raw(&self) -> VkBuffer {
        self.handle
    }

    pub const fn memory(&self) -> VkDeviceMemory {
        self.memory
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub fn write_host_coherent(
        &self,
        loader: &VulkanLoader,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), VulkanLoaderError> {
        let required = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        if self.memory_properties & required != required {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let length = u64::try_from(bytes.len()).map_err(|_| VulkanLoaderError::InvalidQueuePlan)?;
        let end = offset
            .checked_add(length)
            .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
        if end > self.size {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let map: VkMapMemory = loader.device_command(self.device, b"vkMapMemory\0")?;
        let unmap: VkUnmapMemory = loader.device_command(self.device, b"vkUnmapMemory\0")?;
        let mut mapped = core::ptr::null_mut();
        let result = unsafe { map(self.device, self.memory, offset, length, 0, &mut mapped) };
        if result != VK_SUCCESS || mapped.is_null() {
            return Err(VulkanLoaderError::Api(result));
        }
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped.cast::<u8>(), bytes.len()) };
        unsafe { unmap(self.device, self.memory) };
        Ok(())
    }

    /// Writes host-visible memory and flushes the aligned range when the
    /// allocation is non-coherent. `non_coherent_atom_size` must come from the
    /// physical-device limits and must be non-zero for non-coherent memory.
    pub fn write_host_visible(
        &self,
        loader: &VulkanLoader,
        offset: u64,
        bytes: &[u8],
        non_coherent_atom_size: u64,
    ) -> Result<(), VulkanLoaderError> {
        if self.memory_properties & VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let length = u64::try_from(bytes.len()).map_err(|_| VulkanLoaderError::InvalidQueuePlan)?;
        let end = offset
            .checked_add(length)
            .ok_or(VulkanLoaderError::InvalidQueuePlan)?;
        if end > self.size {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let coherent = self.memory_properties & VK_MEMORY_PROPERTY_HOST_COHERENT_BIT != 0;
        let atom = if coherent { 1 } else { non_coherent_atom_size };
        if atom == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let mapped_offset = offset / atom * atom;
        let mapped_end = end
            .checked_add(atom - 1)
            .ok_or(VulkanLoaderError::InvalidQueuePlan)?
            / atom
            * atom;
        if mapped_end > self.size {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let map: VkMapMemory = loader.device_command(self.device, b"vkMapMemory\0")?;
        let unmap: VkUnmapMemory = loader.device_command(self.device, b"vkUnmapMemory\0")?;
        let mut mapped = core::ptr::null_mut();
        let result = unsafe {
            map(
                self.device,
                self.memory,
                mapped_offset,
                mapped_end - mapped_offset,
                0,
                &mut mapped,
            )
        };
        if result != VK_SUCCESS || mapped.is_null() {
            return Err(VulkanLoaderError::Api(result));
        }
        let relative = usize::try_from(offset - mapped_offset)
            .map_err(|_| VulkanLoaderError::InvalidQueuePlan)?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                mapped.cast::<u8>().add(relative),
                bytes.len(),
            )
        };
        if !coherent {
            let flush: VkFlushMappedMemoryRanges =
                loader.device_command(self.device, b"vkFlushMappedMemoryRanges\0")?;
            let range = MappedMemoryRange {
                s_type: 6,
                next: core::ptr::null(),
                memory: self.memory,
                offset: mapped_offset,
                size: mapped_end - mapped_offset,
            };
            let result = unsafe { flush(self.device, 1, &range) };
            if result != VK_SUCCESS {
                unsafe { unmap(self.device, self.memory) };
                return Err(VulkanLoaderError::Api(result));
            }
        }
        unsafe { unmap(self.device, self.memory) };
        Ok(())
    }
}

impl Drop for NativeBuffer {
    fn drop(&mut self) {
        // SAFETY: the wrapper owns both objects and drops them before the
        // parent logical device.
        unsafe {
            (self.destroy_buffer)(self.device, self.handle, core::ptr::null());
            (self.free_memory)(self.device, self.memory, core::ptr::null());
        }
    }
}
impl NativeCommandPool {
    pub const fn raw(&self) -> VkCommandPool {
        self.handle
    }
    pub fn allocate_command_buffers(
        &self,
        loader: &VulkanLoader,
        count: u32,
    ) -> Result<Vec<VkCommandBuffer>, VulkanLoaderError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let allocate: VkAllocateCommandBuffers = loader.command(b"vkAllocateCommandBuffers\0")?;
        let info = CommandBufferAllocateInfo {
            s_type: 40,
            next: core::ptr::null(),
            command_pool: self.handle,
            level: 0,
            command_buffer_count: count,
        };
        let mut buffers = vec![core::ptr::null_mut(); count as usize];
        // SAFETY: output storage and allocation info are valid for the call.
        let result = unsafe { allocate(self.device, &info, buffers.as_mut_ptr()) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(buffers)
    }
    pub fn free_command_buffers(
        &self,
        loader: &VulkanLoader,
        buffers: &[VkCommandBuffer],
    ) -> Result<(), VulkanLoaderError> {
        if buffers.is_empty() {
            return Ok(());
        }
        let free: VkFreeCommandBuffers = loader.command(b"vkFreeCommandBuffers\0")?;
        // SAFETY: buffers were allocated from this device/pool and remain live.
        unsafe {
            free(
                self.device,
                self.handle,
                buffers.len() as u32,
                buffers.as_ptr(),
            )
        };
        Ok(())
    }
}
impl Drop for NativeCommandPool {
    fn drop(&mut self) {
        // SAFETY: pool and device are live and owned for this drop order.
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}
impl LogicalDevice {
    pub const fn raw(&self) -> VkDevice {
        self.handle
    }
    pub fn queue(&self, family: u32, index: u32) -> Option<VkQueue> {
        let mut queue = core::ptr::null_mut();
        // SAFETY: device is live and Vulkan writes one queue handle.
        unsafe { (self.get_queue)(self.handle, family, index, &mut queue) };
        (!queue.is_null()).then_some(queue)
    }

    /// Creates and binds one bounded Vulkan buffer allocation.
    ///
    /// # Safety
    /// `physical` must be live and paired with this logical device. `size` and
    /// `usage` must be non-zero and valid for the eventual Vulkan commands.
    pub unsafe fn create_buffer(
        &self,
        loader: &VulkanLoader,
        physical: VkPhysicalDevice,
        size: u64,
        usage: u32,
        required_properties: u32,
        preferred_properties: u32,
    ) -> Result<NativeBuffer, VulkanLoaderError> {
        if physical.is_null() || size == 0 || usage == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateBuffer = loader.device_command(self.handle, b"vkCreateBuffer\0")?;
        let destroy_buffer: VkDestroyBuffer =
            loader.device_command(self.handle, b"vkDestroyBuffer\0")?;
        let requirements: VkGetBufferMemoryRequirements =
            loader.device_command(self.handle, b"vkGetBufferMemoryRequirements\0")?;
        let allocate: VkAllocateMemory =
            loader.device_command(self.handle, b"vkAllocateMemory\0")?;
        let free_memory: VkFreeMemory = loader.device_command(self.handle, b"vkFreeMemory\0")?;
        let bind: VkBindBufferMemory =
            loader.device_command(self.handle, b"vkBindBufferMemory\0")?;
        let info = BufferCreateInfo {
            s_type: 12,
            next: core::ptr::null(),
            flags: 0,
            size,
            usage,
            sharing_mode: VK_SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            queue_family_indices: core::ptr::null(),
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        let mut requirements_out = MemoryRequirements {
            size: 0,
            alignment: 0,
            memory_type_bits: 0,
        };
        unsafe { requirements(self.handle, handle, &mut requirements_out) };
        let get_memory_properties: VkGetPhysicalDeviceMemoryProperties =
            loader.command(b"vkGetPhysicalDeviceMemoryProperties\0")?;
        let mut properties = PhysicalDeviceMemoryProperties {
            memory_type_count: 0,
            memory_types: [MemoryType {
                property_flags: 0,
                heap_index: 0,
            }; 32],
            memory_heap_count: 0,
            memory_heaps: [MemoryHeap {
                size: 0,
                flags: 0,
                _padding: 0,
            }; 16],
        };
        unsafe { get_memory_properties(physical, &mut properties) };
        let Some(type_index) = select_memory_type(
            &properties,
            requirements_out.memory_type_bits,
            required_properties,
            preferred_properties,
        ) else {
            unsafe { destroy_buffer(self.handle, handle, core::ptr::null()) };
            return Err(VulkanLoaderError::InvalidQueuePlan);
        };
        let allocation_info = MemoryAllocateInfo {
            s_type: 5,
            next: core::ptr::null(),
            allocation_size: requirements_out.size,
            memory_type_index: type_index,
        };
        let mut memory = 0;
        let result = unsafe {
            allocate(
                self.handle,
                &allocation_info,
                core::ptr::null(),
                &mut memory,
            )
        };
        if result != VK_SUCCESS || memory == 0 {
            unsafe { destroy_buffer(self.handle, handle, core::ptr::null()) };
            return Err(VulkanLoaderError::Api(result));
        }
        let result = unsafe { bind(self.handle, handle, memory, 0) };
        if result != VK_SUCCESS {
            unsafe {
                free_memory(self.handle, memory, core::ptr::null());
                destroy_buffer(self.handle, handle, core::ptr::null());
            }
            return Err(VulkanLoaderError::Api(result));
        }
        let memory_properties = properties.memory_types[type_index as usize].property_flags;
        Ok(NativeBuffer {
            handle,
            memory,
            size,
            device: self.handle,
            destroy_buffer,
            free_memory,
            memory_properties,
        })
    }

    pub fn create_semaphore(
        &self,
        loader: &VulkanLoader,
    ) -> Result<NativeSemaphore, VulkanLoaderError> {
        let create: VkCreateSemaphore = loader.command(b"vkCreateSemaphore\0")?;
        let destroy: VkDestroySemaphore = loader.command(b"vkDestroySemaphore\0")?;
        let info = SemaphoreCreateInfo {
            s_type: 9,
            next: core::ptr::null(),
            flags: 0,
        };
        let mut handle = core::ptr::null_mut();
        // SAFETY: device and create-info are valid for this synchronous call.
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeSemaphore {
            handle,
            device: self.handle,
            destroy,
        })
    }
    /// # Safety
    /// All handles must be live and belong to the same Vulkan device.
    pub unsafe fn queue_submit(
        &self,
        loader: &VulkanLoader,
        queue: VkQueue,
        command: VkCommandBuffer,
        wait: VkSemaphore,
        signal: VkSemaphore,
        fence: VkFence,
    ) -> Result<(), VulkanLoaderError> {
        if queue.is_null()
            || command.is_null()
            || wait.is_null()
            || signal.is_null()
            || fence.is_null()
        {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let submit: VkQueueSubmit = loader.command(b"vkQueueSubmit\0")?;
        let waits = [wait];
        let stages = [0x0000_0400_u32];
        let commands = [command];
        let signals = [signal];
        let info = SubmitInfo {
            s_type: 4,
            next: core::ptr::null(),
            wait_count: 1,
            waits: waits.as_ptr(),
            wait_stages: stages.as_ptr(),
            command_count: 1,
            commands: commands.as_ptr(),
            signal_count: 1,
            signals: signals.as_ptr(),
        };
        // SAFETY: all handles and submit arrays are live for this synchronous call.
        let result = unsafe { submit(queue, 1, &info, fence) };
        if result == VK_SUCCESS {
            Ok(())
        } else {
            Err(VulkanLoaderError::Api(result))
        }
    }

    pub fn create_fence(
        &self,
        loader: &VulkanLoader,
        signaled: bool,
    ) -> Result<NativeFence, VulkanLoaderError> {
        let create: VkCreateFence = loader.command(b"vkCreateFence\0")?;
        let destroy: VkDestroyFence = loader.command(b"vkDestroyFence\0")?;
        let wait: VkWaitForFences = loader.command(b"vkWaitForFences\0")?;
        let reset: VkResetFences = loader.command(b"vkResetFences\0")?;
        let info = FenceCreateInfo {
            s_type: 8,
            next: core::ptr::null(),
            flags: if signaled { 1 } else { 0 },
        };
        let mut handle = core::ptr::null_mut();
        // SAFETY: device and create-info are valid for this synchronous call.
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeFence {
            handle,
            device: self.handle,
            destroy,
            wait,
            reset,
        })
    }
    pub fn create_command_pool(
        &self,
        loader: &VulkanLoader,
        family: u32,
    ) -> Result<NativeCommandPool, VulkanLoaderError> {
        let create: VkCreateCommandPool = loader.command(b"vkCreateCommandPool\0")?;
        let destroy: VkDestroyCommandPool = loader.command(b"vkDestroyCommandPool\0")?;
        let info = CommandPoolCreateInfo {
            s_type: 39,
            next: core::ptr::null(),
            flags: 0,
            queue_family_index: family,
        };
        let mut handle = core::ptr::null_mut();
        // SAFETY: device and create-info are valid for the synchronous call.
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeCommandPool {
            handle,
            destroy,
            device: self.handle,
        })
    }
}
impl Drop for LogicalDevice {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by this wrapper and was created successfully.
        unsafe { (self.destroy)(self.handle, core::ptr::null()) };
    }
}

pub struct VulkanLoader {
    library: DynamicLibrary,
    get_instance_proc_addr: VkGetInstanceProcAddr,
    get_device_proc_addr: VkGetDeviceProcAddr,
}

impl VulkanLoader {
    pub fn load() -> Result<Self, VulkanLoaderError> {
        let library =
            DynamicLibrary::open("libvulkan.so.1", true).map_err(VulkanLoaderError::Library)?;
        let symbol = library
            .symbol::<VkGetInstanceProcAddr>("vkGetInstanceProcAddr")
            .map_err(VulkanLoaderError::MissingEntry)?;
        // SAFETY: Vulkan loader exports this exact ABI.
        let get_instance_proc_addr = unsafe { symbol.as_fn() };
        let symbol = library
            .symbol::<VkGetDeviceProcAddr>("vkGetDeviceProcAddr")
            .map_err(VulkanLoaderError::MissingEntry)?;
        // SAFETY: Vulkan loader exports this exact ABI.
        let get_device_proc_addr = unsafe { symbol.as_fn() };
        Ok(Self {
            library,
            get_instance_proc_addr,
            get_device_proc_addr,
        })
    }

    /// # Safety
    /// `physical_device` must be a live handle owned by a live Vulkan instance.
    pub unsafe fn create_logical_device(
        &self,
        physical_device: VkPhysicalDevice,
        requests: &[QueueCreateRequest; 3],
    ) -> Result<LogicalDevice, VulkanLoaderError> {
        if physical_device.is_null() {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let active: Vec<QueueCreateRequest> = requests
            .iter()
            .copied()
            .filter(|request| request.queue_count != 0)
            .collect();
        if active.is_empty() {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let priorities = [1.0_f32; 3];
        let queue_infos: Vec<DeviceQueueCreateInfo> = active
            .iter()
            .map(|request| DeviceQueueCreateInfo {
                s_type: 2,
                next: core::ptr::null(),
                flags: 0,
                queue_family_index: request.family_index,
                queue_count: request.queue_count,
                queue_priorities: priorities.as_ptr(),
            })
            .collect();
        let create: VkCreateDevice = self.command(b"vkCreateDevice\0")?;
        let destroy: VkDestroyDevice = self.command(b"vkDestroyDevice\0")?;
        let get_queue: VkGetDeviceQueue = self.command(b"vkGetDeviceQueue\0")?;
        // The renderer always targets a presenting device: request the
        // KHR_swapchain device extension so acquire/present are usable.
        let swapchain_extension = VK_KHR_SWAPCHAIN;
        let enabled_extensions: [*const u8; 1] = [swapchain_extension.as_ptr()];
        let info = DeviceCreateInfo {
            s_type: 3,
            next: core::ptr::null(),
            flags: 0,
            queue_create_info_count: queue_infos.len() as u32,
            queue_create_infos: queue_infos.as_ptr(),
            enabled_layer_count: 0,
            enabled_layers: core::ptr::null(),
            enabled_extension_count: enabled_extensions.len() as u32,
            enabled_extensions: enabled_extensions.as_ptr(),
            enabled_features: core::ptr::null(),
        };
        let mut handle = core::ptr::null_mut();
        // SAFETY: all create-info pointers remain live for this synchronous call.
        let result = unsafe { create(physical_device, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(LogicalDevice {
            handle,
            destroy,
            get_queue,
        })
    }

    /// # Safety
    /// The returned address must be called with its exact Vulkan signature.
    pub unsafe fn global_proc(&self, name: &[u8]) -> Option<*const c_void> {
        if name.last().copied() != Some(0) {
            return None;
        }
        // SAFETY: caller upholds the Vulkan command signature contract.
        let pointer =
            unsafe { (self.get_instance_proc_addr)(core::ptr::null_mut(), name.as_ptr()) };
        (!pointer.is_null()).then_some(pointer)
    }

    /// Resolves a device-level Vulkan command for `device`. Device commands
    /// (swapchain, present, pipeline barrier) dispatch through
    /// `vkGetDeviceProcAddr`, not the instance loader.
    ///
    /// # Safety
    /// `device` must be a valid logical device known to the loader, and the
    /// returned pointer must be called with the exact Vulkan signature.
    pub unsafe fn device_proc(&self, device: VkDevice, name: &[u8]) -> Option<*const c_void> {
        if name.last().copied() != Some(0) || device.is_null() {
            return None;
        }
        // SAFETY: caller upholds the Vulkan command signature contract.
        let pointer = unsafe { (self.get_device_proc_addr)(device, name.as_ptr()) };
        (!pointer.is_null()).then_some(pointer)
    }

    /// Creates a persistent Vulkan instance with the given instance
    /// extensions (e.g. `VK_KHR_surface` + `VK_KHR_xlib_surface`). The
    /// instance is destroyed when dropped.
    ///
    /// # Safety
    /// Every extension name must be NUL-terminated and supported by the
    /// loader; extension lists must stay live for the synchronous call.
    pub fn create_instance(
        &self,
        extensions: &[&[u8]],
    ) -> Result<NativeInstance, VulkanLoaderError> {
        let create: VkCreateInstance = self.command(b"vkCreateInstance\0")?;
        let destroy: VkDestroyInstance = self.command(b"vkDestroyInstance\0")?;
        let names: Vec<*const u8> = extensions.iter().map(|name| name.as_ptr()).collect();
        let app_name = b"rust-kernel-game-engine-kit\0";
        let engine_name = b"rust-kernel-game-engine-kit\0";
        let app = ApplicationInfo {
            s_type: 0,
            next: core::ptr::null(),
            application_name: app_name.as_ptr(),
            application_version: 1,
            engine_name: engine_name.as_ptr(),
            engine_version: 1,
            api_version: VK_API_VERSION_1_0,
        };
        let info = InstanceCreateInfo {
            s_type: 1,
            next: core::ptr::null(),
            flags: 0,
            application_info: &app,
            layer_count: 0,
            layers: core::ptr::null(),
            extension_count: names.len() as u32,
            extensions: names.as_ptr(),
        };
        let mut instance = core::ptr::null_mut();
        // SAFETY: all pointers reference live ABI-compatible values.
        let result = unsafe { create(&info, core::ptr::null(), &mut instance) };
        if result != VK_SUCCESS {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeInstance { instance, destroy })
    }

    pub fn library_name(&self) -> &str {
        self.library.name()
    }

    pub fn instance_api_version(&self) -> Result<u32, VulkanLoaderError> {
        let name = b"vkEnumerateInstanceVersion\0";
        // SAFETY: name is NUL-terminated and Vulkan returns a valid command pointer.
        let Some(pointer) = (unsafe { self.global_proc(name) }) else {
            return Ok(VK_API_VERSION_1_0);
        };
        // SAFETY: exact Vulkan ABI.
        let enumerate: VkEnumerateInstanceVersion = unsafe { core::mem::transmute(pointer) };
        let mut version = VK_API_VERSION_1_0;
        // SAFETY: version is writable output storage.
        let result = unsafe { enumerate(&mut version) };
        if result == VK_SUCCESS || result == VK_INCOMPLETE {
            Ok(version)
        } else {
            Err(VulkanLoaderError::ApiQuery(result))
        }
    }

    pub fn discover_physical_devices(&self) -> Result<Vec<PhysicalDeviceInfo>, VulkanLoaderError> {
        let instance = self.create_instance(&[])?;
        let report = unsafe { self.devices_and_handles(instance.raw()) };
        // SAFETY: instance was created successfully and remains live here.
        report.map(|devices| devices.into_iter().map(|(_, info)| info).collect())
    }

    /// Discovers devices and their raw handles for a live instance.
    ///
    /// # Safety
    /// `instance` must be a live Vulkan instance created by this loader.
    pub unsafe fn devices_and_handles(
        &self,
        instance: VkInstance,
    ) -> Result<Vec<(VkPhysicalDevice, PhysicalDeviceInfo)>, VulkanLoaderError> {
        // SAFETY: caller guarantees a live instance.
        let handles = unsafe { self.enumerate_handles(instance) }?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(handles.len())
            .map_err(|_| VulkanLoaderError::AllocationFailed)?;
        for handle in handles {
            // SAFETY: each handle came from the same live instance.
            let info = unsafe { self.describe_device(handle) }?;
            output.push((handle, info));
        }
        Ok(output)
    }

    /// Enumerates raw physical-device handles for `instance`. Used by
    /// render bring-up paths that must keep raw handles to build devices.
    ///
    /// # Safety
    /// `instance` must be a live Vulkan instance created by this loader.
    pub unsafe fn enumerate_handles(
        &self,
        instance: VkInstance,
    ) -> Result<Vec<VkPhysicalDevice>, VulkanLoaderError> {
        let enumerate: VkEnumeratePhysicalDevices =
            self.command(b"vkEnumeratePhysicalDevices\0")?;
        // SAFETY: caller guarantees a live instance.
        unsafe { enumerate_handles(instance, enumerate) }
    }

    /// Reads full [`PhysicalDeviceInfo`] for a raw handle.
    ///
    /// # Safety
    /// `device` must be a live handle owned by a live instance of this loader.
    pub unsafe fn describe_device(
        &self,
        device: VkPhysicalDevice,
    ) -> Result<PhysicalDeviceInfo, VulkanLoaderError> {
        let properties: VkGetPhysicalDeviceProperties =
            self.command(b"vkGetPhysicalDeviceProperties\0")?;
        let queue_properties: VkGetPhysicalDeviceQueueFamilyProperties =
            self.command(b"vkGetPhysicalDeviceQueueFamilyProperties\0")?;
        // SAFETY: caller guarantees a live handle.
        Ok(describe_device(device, properties, queue_properties))
    }

    pub(crate) fn command<T>(&self, name: &'static [u8]) -> Result<T, VulkanLoaderError> {
        // SAFETY: static command names are NUL-terminated.
        let pointer = unsafe { self.global_proc(name) }.ok_or_else(|| {
            VulkanLoaderError::MissingCommand(String::from_utf8_lossy(name).into_owned())
        })?;
        // SAFETY: T matches the Vulkan command ABI at each call site.
        Ok(unsafe { core::mem::transmute_copy(&pointer) })
    }

    pub(crate) fn device_command<T>(
        &self,
        device: VkDevice,
        name: &'static [u8],
    ) -> Result<T, VulkanLoaderError> {
        // SAFETY: static command names are NUL-terminated and the caller
        // supplies the live device whose dispatch table owns the command.
        let pointer = unsafe { self.device_proc(device, name) }.ok_or_else(|| {
            VulkanLoaderError::MissingCommand(String::from_utf8_lossy(name).into_owned())
        })?;
        // SAFETY: T matches the Vulkan command ABI at each call site.
        Ok(unsafe { core::mem::transmute_copy(&pointer) })
    }
}

fn select_memory_type(
    properties: &PhysicalDeviceMemoryProperties,
    compatible_bits: u32,
    required: u32,
    preferred: u32,
) -> Option<u32> {
    let count = (properties.memory_type_count as usize).min(properties.memory_types.len());
    let mut best = None;
    let mut best_score = 0_u32;
    for index in 0..count {
        if compatible_bits & (1_u32 << index) == 0 {
            continue;
        }
        let flags = properties.memory_types[index].property_flags;
        if flags & required != required {
            continue;
        }
        let score = (flags & preferred).count_ones();
        if best.is_none() || score > best_score {
            best = Some(index as u32);
            best_score = score;
        }
    }
    best
}

/// # Safety
/// `instance` must be a live Vulkan instance for `enumerate`.
unsafe fn enumerate_handles(
    instance: VkInstance,
    enumerate: VkEnumeratePhysicalDevices,
) -> Result<Vec<VkPhysicalDevice>, VulkanLoaderError> {
    let mut count = 0;
    // SAFETY: count is a valid output pointer for the live instance.
    let result = unsafe { enumerate(instance, &mut count, core::ptr::null_mut()) };
    if result != VK_SUCCESS && result != VK_INCOMPLETE {
        return Err(VulkanLoaderError::Api(result));
    }
    if count == 0 {
        return Err(VulkanLoaderError::NoPhysicalDevices);
    }
    let mut count_u32 = count;
    let count = count_u32 as usize;
    let mut devices = Vec::new();
    devices
        .try_reserve_exact(count)
        .map_err(|_| VulkanLoaderError::AllocationFailed)?;
    devices.resize(count, core::ptr::null_mut());
    // SAFETY: devices has capacity for the returned handles.
    let result = unsafe { enumerate(instance, &mut count_u32 as *mut u32, devices.as_mut_ptr()) };
    if result != VK_SUCCESS && result != VK_INCOMPLETE {
        return Err(VulkanLoaderError::Api(result));
    }
    devices.truncate(count);
    Ok(devices)
}

fn describe_device(
    device: VkPhysicalDevice,
    properties: VkGetPhysicalDeviceProperties,
    queue_properties: VkGetPhysicalDeviceQueueFamilyProperties,
) -> PhysicalDeviceInfo {
    let mut raw = PhysicalDeviceProperties { bytes: [0; 824] };
    // SAFETY: Vulkan writes its documented properties structure.
    unsafe { properties(device, &mut raw) };
    let read_u32 =
        |offset: usize| u32::from_ne_bytes(raw.bytes[offset..offset + 4].try_into().unwrap());
    let name = &raw.bytes[20..276];
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    PhysicalDeviceInfo {
        api_version: read_u32(0),
        driver_version: read_u32(4),
        vendor_id: read_u32(8),
        device_id: read_u32(12),
        device_type: read_u32(16),
        name: String::from_utf8_lossy(&name[..end]).into_owned(),
        queue_families: enumerate_queue_families(device, queue_properties),
    }
}

fn enumerate_queue_families(
    device: VkPhysicalDevice,
    enumerate: VkGetPhysicalDeviceQueueFamilyProperties,
) -> Vec<QueueFamilyInfo> {
    let mut count = 0;
    // SAFETY: count is a valid output pointer for the live physical device.
    unsafe { enumerate(device, &mut count, core::ptr::null_mut()) };
    let mut raw = vec![
        QueueFamilyProperties {
            flags: 0,
            queue_count: 0,
            min_image_transfer_granularity: [0; 3]
        };
        count as usize
    ];
    if count != 0 {
        // SAFETY: raw has exactly the capacity reported by Vulkan.
        unsafe { enumerate(device, &mut count, raw.as_mut_ptr()) };
    }
    raw.into_iter()
        .enumerate()
        .map(|(index, family)| QueueFamilyInfo {
            index: index as u32,
            flags: family.flags & 0x1
                | if family.flags & 0x2 != 0 {
                    QUEUE_COMPUTE
                } else {
                    0
                }
                | if family.flags & 0x4 != 0 {
                    QUEUE_TRANSFER
                } else {
                    0
                },
            queue_count: family.queue_count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constant_is_abi_correct() {
        assert_eq!(VK_SUCCESS, 0);
        assert_eq!(VK_API_VERSION_1_0, 0x0040_0000);
    }
}
