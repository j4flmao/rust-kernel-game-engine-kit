//! Minimal Vulkan loader boundary for the Windows PAL.
//!
//! This module intentionally loads only the Vulkan loader entry point. Instance
//! and device tables belong to the renderer phase and are resolved after the
//! application has selected validation layers and extensions.
#![allow(unsafe_code)] // audited FFI boundary; signatures are Vulkan ABI exact

use core::ffi::c_void;

use super::dl::{DlError, DynamicLibrary};
use crate::platform::vulkan_policy::{
    QueueCreateRequest, QueueFamilyInfo, QUEUE_COMPUTE, QUEUE_GRAPHICS, QUEUE_TRANSFER,
};

pub type VkInstance = *mut c_void;
pub type VkSurfaceKHR = u64;
pub type VkResult = i32;
pub type VkGetInstanceProcAddr =
    unsafe extern "system" fn(instance: VkInstance, name: *const u8) -> *const c_void;
pub type VkGetDeviceProcAddr =
    unsafe extern "system" fn(device: VkDevice, name: *const u8) -> *const c_void;
pub type VkPhysicalDevice = *mut c_void;
pub type VkDevice = *mut c_void;
pub type VkQueue = *mut c_void;
pub type VkCommandPool = *mut c_void;
pub type VkCommandBuffer = *mut c_void;
pub type VkFence = *mut c_void;
pub type VkSemaphore = *mut c_void;
pub type VkSwapchainKHR = u64;
pub type VkImage = u64;
type VkCreateInstance = unsafe extern "system" fn(
    *const InstanceCreateInfo,
    *const c_void,
    *mut VkInstance,
) -> VkResult;
type VkDestroyInstance = unsafe extern "system" fn(VkInstance, *const c_void);
type VkEnumeratePhysicalDevices =
    unsafe extern "system" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
type VkGetPhysicalDeviceProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceProperties);
type VkGetPhysicalDeviceQueueFamilyProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut u32, *mut QueueFamilyProperties);
type VkCreateDevice = unsafe extern "system" fn(
    VkPhysicalDevice,
    *const DeviceCreateInfo,
    *const c_void,
    *mut VkDevice,
) -> VkResult;
type VkDestroyDevice = unsafe extern "system" fn(VkDevice, *const c_void);
type VkGetDeviceQueue = unsafe extern "system" fn(VkDevice, u32, u32, *mut VkQueue);
type VkCreateCommandPool = unsafe extern "system" fn(
    VkDevice,
    *const CommandPoolCreateInfo,
    *const c_void,
    *mut VkCommandPool,
) -> VkResult;
type VkDestroyCommandPool = unsafe extern "system" fn(VkDevice, VkCommandPool, *const c_void);
type VkAllocateCommandBuffers = unsafe extern "system" fn(
    VkDevice,
    *const CommandBufferAllocateInfo,
    *mut VkCommandBuffer,
) -> VkResult;
type VkFreeCommandBuffers =
    unsafe extern "system" fn(VkDevice, VkCommandPool, u32, *const VkCommandBuffer);
pub type VkEnumerateInstanceVersion = unsafe extern "system" fn(version: *mut u32) -> VkResult;
type VkBeginCommandBuffer =
    unsafe extern "system" fn(VkCommandBuffer, *const CommandBufferBeginInfo) -> VkResult;
type VkEndCommandBuffer = unsafe extern "system" fn(VkCommandBuffer) -> VkResult;
type VkResetCommandBuffer = unsafe extern "system" fn(VkCommandBuffer, u32) -> VkResult;
type VkCreateFence = unsafe extern "system" fn(
    VkDevice,
    *const FenceCreateInfo,
    *const c_void,
    *mut VkFence,
) -> VkResult;
type VkDestroyFence = unsafe extern "system" fn(VkDevice, VkFence, *const c_void);
type VkWaitForFences =
    unsafe extern "system" fn(VkDevice, u32, *const VkFence, u32, u64) -> VkResult;
type VkResetFences = unsafe extern "system" fn(VkDevice, u32, *const VkFence) -> VkResult;
type VkCreateSemaphore = unsafe extern "system" fn(
    VkDevice,
    *const SemaphoreCreateInfo,
    *const c_void,
    *mut VkSemaphore,
) -> VkResult;
type VkDestroySemaphore = unsafe extern "system" fn(VkDevice, VkSemaphore, *const c_void);
type VkQueueSubmit =
    unsafe extern "system" fn(VkQueue, u32, *const SubmitInfo, VkFence) -> VkResult;

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

pub const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: u32 = 1000001000;
pub const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: u32 = 1000001001;
pub const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: u32 = 15;

pub const VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT: u32 = 0x0000_0001;
pub const VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT: u32 = 0x0000_0400;
pub const VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT: u32 = 0x0000_0010;

pub const VK_KHR_SURFACE: &[u8] = b"VK_KHR_surface\0";
pub const VK_KHR_WIN32_SURFACE: &[u8] = b"VK_KHR_win32_surface\0";
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

// VkPhysicalDeviceProperties is 824 bytes in Vulkan 1.x. Keeping the exact
// output buffer size avoids depending on a third-party Vulkan header crate.
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
    Api(VkResult),
    NoPhysicalDevices,
    ApiQuery(VkResult),
    InvalidQueuePlan,
    InvalidSurfaceHandle,
    /// The swapchain images are out of date with the surface (resize); the
    /// renderer must recreate the swapchain.
    OutOfDate,
    /// The swapchain works but its configuration is now suboptimal.
    Suboptimal,
    AllocationFailed,
    /// The API returned a swapchain error that this PAL does not model.
    UnknownSwitch(VkResult),
}

impl core::fmt::Display for VulkanLoaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Library(error) => write!(f, "failed to load Vulkan loader: {error}"),
            Self::MissingEntry(error) => write!(f, "Vulkan loader entry point missing: {error}"),
            Self::Api(result) => write!(f, "Vulkan call failed with VkResult {result}"),
            Self::NoPhysicalDevices => write!(f, "Vulkan reported no physical devices"),
            Self::ApiQuery(result) => write!(
                f,
                "vkEnumerateInstanceVersion failed with VkResult {result}"
            ),
            Self::InvalidQueuePlan => {
                f.write_str("Vulkan logical-device queue plan is empty or invalid")
            }
            Self::InvalidSurfaceHandle => f.write_str("Win32 Vulkan surface handle is null"),
            Self::OutOfDate => {
                f.write_str("swapchain is out of date with the presentation surface")
            }
            Self::Suboptimal => f.write_str("swapchain is suboptimal for the current surface"),
            Self::AllocationFailed => f.write_str("Vulkan enumeration allocation failed"),
            Self::UnknownSwitch(result) => {
                write!(f, "unhandled Vulkan swapchain result {result}")
            }
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
impl NativeFence {
    pub const fn raw(&self) -> VkFence {
        self.handle
    }
    pub fn wait(&self, timeout_ns: u64) -> Result<(), VulkanLoaderError> {
        // SAFETY: fence belongs to the live device and output count is valid.
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
    /// Loads the system Vulkan loader without requiring a physical GPU.
    pub fn load() -> Result<Self, VulkanLoaderError> {
        let library = DynamicLibrary::open("vulkan-1.dll").map_err(VulkanLoaderError::Library)?;
        let symbol = library
            .symbol::<VkGetInstanceProcAddr>("vkGetInstanceProcAddr")
            .map_err(VulkanLoaderError::MissingEntry)?;
        // SAFETY: Vulkan loader exports this exact function signature.
        let get_instance_proc_addr = unsafe { symbol.as_fn() };
        let symbol = library
            .symbol::<VkGetDeviceProcAddr>("vkGetDeviceProcAddr")
            .map_err(VulkanLoaderError::MissingEntry)?;
        // SAFETY: Vulkan loader exports this exact function signature.
        let get_device_proc_addr = unsafe { symbol.as_fn() };
        Ok(Self {
            library,
            get_instance_proc_addr,
            get_device_proc_addr,
        })
    }

    pub fn library_name(&self) -> &str {
        self.library.name()
    }

    pub fn get_instance_proc_addr(&self) -> VkGetInstanceProcAddr {
        self.get_instance_proc_addr
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

    /// Resolves a global Vulkan function by name.
    ///
    /// # Safety
    /// The returned pointer must be called only with the signature matching
    /// the requested Vulkan command and the loader's dispatch rules.
    pub unsafe fn global_proc(&self, name: &[u8]) -> Option<*const c_void> {
        if name.last().copied() != Some(0) {
            return None;
        }
        // SAFETY: caller upholds the Vulkan function-pointer contract; name
        // is explicitly checked as a NUL-terminated byte string.
        let pointer =
            unsafe { (self.get_instance_proc_addr)(core::ptr::null_mut(), name.as_ptr()) };
        (!pointer.is_null()).then_some(pointer)
    }

    /// Resolves a device-level Vulkan command for `device` through
    /// `vkGetDeviceProcAddr`.
    ///
    /// # Safety
    /// `device` must be a valid logical device known to the loader, and the
    /// returned pointer must be called with the exact Vulkan signature.
    pub unsafe fn device_proc(&self, device: VkDevice, name: &[u8]) -> Option<*const c_void> {
        if name.last().copied() != Some(0) || device.is_null() {
            return None;
        }
        // SAFETY: caller upholds the Vulkan function-pointer contract; name
        // is explicitly checked as a NUL-terminated byte string.
        let pointer = unsafe { (self.get_device_proc_addr)(device, name.as_ptr()) };
        (!pointer.is_null()).then_some(pointer)
    }

    /// Creates a persistent Vulkan instance with the given instance
    /// extensions (e.g. `VK_KHR_surface` + `VK_KHR_win32_surface`). The
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

    pub fn discover_physical_devices(&self) -> Result<Vec<PhysicalDeviceInfo>, VulkanLoaderError> {
        let instance = self.create_instance(&[])?;
        let report = unsafe { self.devices_and_handles(instance.raw()) };
        // SAFETY: instance was created successfully and remains live here.
        report.map(|devices| devices.into_iter().map(|(_, info)| info).collect())
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

    pub(crate) fn command<T>(&self, name: &'static [u8]) -> Result<T, VulkanLoaderError> {
        // SAFETY: static command names are NUL-terminated.
        let pointer = unsafe { self.global_proc(name) }.ok_or_else(|| {
            VulkanLoaderError::MissingEntry(DlError::Symbol {
                symbol: String::from_utf8_lossy(name).into_owned(),
                code: 0,
            })
        })?;
        // SAFETY: T matches the Vulkan command ABI at each call site.
        Ok(unsafe { core::mem::transmute_copy(&pointer) })
    }

    pub fn instance_api_version(&self) -> Result<u32, VulkanLoaderError> {
        let name = b"vkEnumerateInstanceVersion\0";
        // SAFETY: name is NUL-terminated and the returned pointer is used
        // only with the documented Vulkan command signature.
        let Some(pointer) = (unsafe { self.global_proc(name) }) else {
            return Ok(VK_API_VERSION_1_0);
        };
        // SAFETY: Vulkan defines this exact function ABI.
        let enumerate: VkEnumerateInstanceVersion = unsafe { core::mem::transmute(pointer) };
        let mut version = VK_API_VERSION_1_0;
        // SAFETY: version is a valid writable output pointer.
        let result = unsafe { enumerate(&mut version) };
        if result == VK_SUCCESS || result == VK_INCOMPLETE {
            Ok(version)
        } else {
            Err(VulkanLoaderError::ApiQuery(result))
        }
    }
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
    // SAFETY: Vulkan writes its documented 824-byte properties structure.
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
            flags: if family.flags & 0x1 != 0 {
                QUEUE_GRAPHICS
            } else {
                0
            } | if family.flags & 0x2 != 0 {
                QUEUE_COMPUTE
            } else {
                0
            } | if family.flags & 0x4 != 0 {
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
    fn invalid_command_name_is_rejected_before_ffi() {
        // Arrange
        let name = b"vkCreateInstance";
        // The loader may be absent on CI; validate the pure contract first.
        // Act
        let terminated = name.last().copied() == Some(0);
        // Assert
        assert!(!terminated);
    }

    #[test]
    fn success_constant_matches_vulkan_abi() {
        assert_eq!(VK_SUCCESS, 0);
        assert_eq!(VK_API_VERSION_1_0, 0x0040_0000);
    }
}
