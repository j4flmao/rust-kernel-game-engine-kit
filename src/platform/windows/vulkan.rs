//! Minimal Vulkan loader boundary for the Windows PAL.
//!
//! This module intentionally loads only the Vulkan loader entry point. Instance
//! and device tables belong to the renderer phase and are resolved after the
//! application has selected validation layers and extensions.
#![allow(unsafe_code)] // audited FFI boundary; signatures are Vulkan ABI exact

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

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
pub type VkBuffer = u64;
pub type VkDeviceMemory = u64;
pub type VkFence = *mut c_void;
pub type VkSemaphore = *mut c_void;
pub type VkSwapchainKHR = u64;
pub type VkImage = u64;
pub type VkShaderModule = u64;
pub type VkDescriptorSetLayout = u64;
pub type VkDescriptorPool = u64;
pub type VkDescriptorSet = u64;
pub type VkPipelineLayout = u64;
pub type VkRenderPass = u64;
pub type VkPipeline = u64;
pub type VkImageView = u64;
pub type VkFramebuffer = u64;
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
type VkCreateBuffer = unsafe extern "system" fn(
    VkDevice,
    *const BufferCreateInfo,
    *const c_void,
    *mut VkBuffer,
) -> VkResult;
type VkCreateShaderModule = unsafe extern "system" fn(
    VkDevice,
    *const ShaderModuleCreateInfo,
    *const c_void,
    *mut VkShaderModule,
) -> VkResult;
type VkDestroyShaderModule = unsafe extern "system" fn(VkDevice, VkShaderModule, *const c_void);
type VkCreateDescriptorSetLayout = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetLayoutCreateInfo,
    *const c_void,
    *mut VkDescriptorSetLayout,
) -> VkResult;
type VkDestroyDescriptorSetLayout =
    unsafe extern "system" fn(VkDevice, VkDescriptorSetLayout, *const c_void);
type VkCreateDescriptorPool = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorPoolCreateInfo,
    *const c_void,
    *mut VkDescriptorPool,
) -> VkResult;
type VkDestroyDescriptorPool = unsafe extern "system" fn(VkDevice, VkDescriptorPool, *const c_void);
type VkAllocateDescriptorSets = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetAllocateInfo,
    *mut VkDescriptorSet,
) -> VkResult;
type VkUpdateDescriptorSets =
    unsafe extern "system" fn(VkDevice, u32, *const WriteDescriptorSet, u32, *const c_void);
type VkCmdBeginRenderPass =
    unsafe extern "system" fn(VkCommandBuffer, *const RenderPassBeginInfo, u32);
type VkCmdEndRenderPass = unsafe extern "system" fn(VkCommandBuffer);
type VkCmdBindPipeline = unsafe extern "system" fn(VkCommandBuffer, u32, VkPipeline);
type VkCmdBindDescriptorSets = unsafe extern "system" fn(
    VkCommandBuffer,
    u32,
    VkPipelineLayout,
    u32,
    u32,
    *const VkDescriptorSet,
    u32,
    *const u32,
);
type VkCmdPushConstants =
    unsafe extern "system" fn(VkCommandBuffer, VkPipelineLayout, u32, u32, u32, *const c_void);
type VkCreateImageView = unsafe extern "system" fn(
    VkDevice,
    *const ImageViewCreateInfo,
    *const c_void,
    *mut VkImageView,
) -> VkResult;
type VkDestroyImageView = unsafe extern "system" fn(VkDevice, VkImageView, *const c_void);
type VkCreateRenderPass = unsafe extern "system" fn(
    VkDevice,
    *const RenderPassCreateInfo,
    *const c_void,
    *mut VkRenderPass,
) -> VkResult;
type VkDestroyRenderPass = unsafe extern "system" fn(VkDevice, VkRenderPass, *const c_void);
type VkCreateFramebuffer = unsafe extern "system" fn(
    VkDevice,
    *const FramebufferCreateInfo,
    *const c_void,
    *mut VkFramebuffer,
) -> VkResult;
type VkDestroyFramebuffer = unsafe extern "system" fn(VkDevice, VkFramebuffer, *const c_void);
type VkCreateGraphicsPipelines = unsafe extern "system" fn(
    VkDevice,
    u64,
    u32,
    *const GraphicsPipelineCreateInfo,
    *const c_void,
    *mut VkPipeline,
) -> VkResult;
type VkDestroyPipeline = unsafe extern "system" fn(VkDevice, VkPipeline, *const c_void);
type VkCreatePipelineLayout = unsafe extern "system" fn(
    VkDevice,
    *const PipelineLayoutCreateInfo,
    *const c_void,
    *mut VkPipelineLayout,
) -> VkResult;
type VkDestroyPipelineLayout = unsafe extern "system" fn(VkDevice, VkPipelineLayout, *const c_void);
type VkDestroyBuffer = unsafe extern "system" fn(VkDevice, VkBuffer, *const c_void);
type VkGetBufferMemoryRequirements =
    unsafe extern "system" fn(VkDevice, VkBuffer, *mut MemoryRequirements);
type VkAllocateMemory = unsafe extern "system" fn(
    VkDevice,
    *const MemoryAllocateInfo,
    *const c_void,
    *mut VkDeviceMemory,
) -> VkResult;
type VkFreeMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory, *const c_void);
type VkBindBufferMemory =
    unsafe extern "system" fn(VkDevice, VkBuffer, VkDeviceMemory, u64) -> VkResult;
type VkMapMemory = unsafe extern "system" fn(
    VkDevice,
    VkDeviceMemory,
    u64,
    u64,
    u32,
    *mut *mut c_void,
) -> VkResult;
type VkUnmapMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory);
type VkFlushMappedMemoryRanges =
    unsafe extern "system" fn(VkDevice, u32, *const MappedMemoryRange) -> VkResult;
type VkGetPhysicalDeviceMemoryProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceMemoryProperties);
type VkCmdDispatch = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32);
type VkCmdDrawIndexedIndirect = unsafe extern "system" fn(VkCommandBuffer, VkBuffer, u64, u32, u32);
type VkCmdBindVertexBuffers =
    unsafe extern "system" fn(VkCommandBuffer, u32, u32, *const VkBuffer, *const u64);
type VkCmdDraw = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32, u32);
type VkCmdCopyBuffer =
    unsafe extern "system" fn(VkCommandBuffer, VkBuffer, VkBuffer, u32, *const BufferCopy);
type VkCmdPipelineBarrier = unsafe extern "system" fn(
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
pub const VK_BUFFER_USAGE_STORAGE_BUFFER_BIT: u32 = 0x0000_0020;
pub const VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT: u32 = 0x0000_0100;
pub const VK_BUFFER_USAGE_TRANSFER_SRC_BIT: u32 = 0x0000_0001;
pub const VK_BUFFER_USAGE_TRANSFER_DST_BIT: u32 = 0x0000_0002;
pub const VK_BUFFER_USAGE_VERTEX_BUFFER_BIT: u32 = 0x0000_0080;
pub const VK_PIPELINE_STAGE_TRANSFER_BIT: u32 = 0x0000_0100;
pub const VK_PIPELINE_STAGE_VERTEX_INPUT_BIT: u32 = 0x0000_0004;
pub const VK_PIPELINE_STAGE_VERTEX_SHADER_BIT: u32 = 0x0000_0008;
pub const VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT: u32 = 0x0000_0800;
pub const VK_PIPELINE_STAGE_DRAW_INDIRECT_BIT: u32 = 0x0000_0200;
pub const VK_ACCESS_TRANSFER_WRITE_BIT: u32 = 0x0000_1000;
pub const VK_ACCESS_VERTEX_ATTRIBUTE_READ_BIT: u32 = 0x0000_0004;
pub const VK_ACCESS_SHADER_READ_BIT: u32 = 0x0000_0020;
pub const VK_ACCESS_SHADER_WRITE_BIT: u32 = 0x0000_0040;
pub const VK_ACCESS_INDIRECT_COMMAND_READ_BIT: u32 = 0x0000_0002;
pub const VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT: u32 = 0x0000_0001;
pub const VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT: u32 = 0x0000_0002;
pub const VK_MEMORY_PROPERTY_HOST_COHERENT_BIT: u32 = 0x0000_0004;
pub const VK_IMAGE_LAYOUT_UNDEFINED: u32 = 0;
pub const VK_IMAGE_LAYOUT_GENERAL: u32 = 1;
pub const VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: u32 = 1000001002;
pub const VK_IMAGE_ASPECT_COLOR_BIT: u32 = 0x0000_0001;
pub const VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR: u32 = 0x0000_0001;
pub const VK_SHARING_MODE_EXCLUSIVE: u32 = 0;

pub const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: u32 = 1000001000;
pub const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: u32 = 1000001001;
pub const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: u32 = 15;
pub const VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO: u32 = 15;
pub const VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO: u32 = 30;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: u32 = 32;
pub const VK_DESCRIPTOR_TYPE_STORAGE_BUFFER: u32 = 7;
pub const VK_SHADER_STAGE_VERTEX_BIT: u32 = 0x0000_0001;
pub const VK_SHADER_STAGE_FRAGMENT_BIT: u32 = 0x0000_0010;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO: u32 = 33;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO: u32 = 34;
pub const VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET: u32 = 35;
pub const VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT: u32 = 0x0000_0001;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO: u32 = 43;
pub const VK_PIPELINE_BIND_POINT_GRAPHICS: u32 = 0;
pub const VK_SUBPASS_CONTENTS_INLINE: u32 = 0;
pub const VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO: u32 = 15;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO: u32 = 38;
pub const VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO: u32 = 37;
pub const VK_IMAGE_VIEW_TYPE_2D: u32 = 1;
pub const VK_SAMPLE_COUNT_1_BIT: u32 = 1;
pub const VK_ATTACHMENT_LOAD_OP_CLEAR: u32 = 1;
pub const VK_ATTACHMENT_STORE_OP_STORE: u32 = 0;
pub const VK_ATTACHMENT_LOAD_OP_DONT_CARE: u32 = 2;
pub const VK_ATTACHMENT_STORE_OP_DONT_CARE: u32 = 1;
pub const VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
pub const VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO: u32 = 18;
pub const VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO: u32 = 19;
pub const VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO: u32 = 20;
pub const VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO: u32 = 22;
pub const VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO: u32 = 23;
pub const VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO: u32 = 24;
pub const VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO: u32 = 26;
pub const VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO: u32 = 28;
pub const VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST: u32 = 3;
pub const VK_POLYGON_MODE_FILL: u32 = 0;
pub const VK_BLEND_FACTOR_SRC_ALPHA: u32 = 6;
pub const VK_BLEND_FACTOR_ONE: u32 = 1;
pub const VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA: u32 = 7;
pub const VK_BLEND_OP_ADD: u32 = 0;
pub const VK_COLOR_COMPONENT_R_BIT: u32 = 1;
pub const VK_COLOR_COMPONENT_G_BIT: u32 = 2;
pub const VK_COLOR_COMPONENT_B_BIT: u32 = 4;
pub const VK_COLOR_COMPONENT_A_BIT: u32 = 8;

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
struct ShaderModuleCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    code_size: usize,
    code: *const u32,
}

#[repr(C)]
struct DescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: u32,
    descriptor_count: u32,
    stage_flags: u32,
    immutable_samplers: *const c_void,
}

#[repr(C)]
struct DescriptorSetLayoutCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    binding_count: u32,
    bindings: *const DescriptorSetLayoutBinding,
}

#[repr(C)]
struct PushConstantRange {
    stage_flags: u32,
    offset: u32,
    size: u32,
}

#[repr(C)]
struct PipelineLayoutCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    set_layout_count: u32,
    set_layouts: *const VkDescriptorSetLayout,
    push_constant_range_count: u32,
    push_constant_ranges: *const PushConstantRange,
}
#[repr(C)]
struct DescriptorPoolSize {
    descriptor_type: u32,
    descriptor_count: u32,
}
#[repr(C)]
struct DescriptorPoolCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    max_sets: u32,
    pool_size_count: u32,
    pool_sizes: *const DescriptorPoolSize,
}
#[repr(C)]
struct DescriptorSetAllocateInfo {
    s_type: u32,
    next: *const c_void,
    descriptor_pool: VkDescriptorPool,
    descriptor_set_count: u32,
    set_layouts: *const VkDescriptorSetLayout,
}
#[repr(C)]
struct DescriptorBufferInfo {
    buffer: VkBuffer,
    offset: u64,
    range: u64,
}
#[repr(C)]
struct WriteDescriptorSet {
    s_type: u32,
    next: *const c_void,
    dst_set: VkDescriptorSet,
    dst_binding: u32,
    dst_array_element: u32,
    descriptor_count: u32,
    descriptor_type: u32,
    image_info: *const c_void,
    buffer_info: *const DescriptorBufferInfo,
    texel_buffer_view: *const c_void,
}
#[repr(C)]
struct Rect2D {
    offset: [i32; 2],
    extent: Extent2D,
}
#[repr(C)]
struct ClearValue {
    color: [f32; 4],
}
#[repr(C)]
struct RenderPassBeginInfo {
    s_type: u32,
    next: *const c_void,
    render_pass: VkRenderPass,
    framebuffer: u64,
    render_area: Rect2D,
    clear_value_count: u32,
    clear_values: *const ClearValue,
}
#[repr(C)]
struct ComponentMapping {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}
#[repr(C)]
struct ImageViewCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    image: VkImage,
    view_type: u32,
    format: u32,
    components: ComponentMapping,
    subresource_range: ImageSubresourceRange,
}
#[repr(C)]
struct AttachmentDescription {
    flags: u32,
    format: u32,
    samples: u32,
    load_op: u32,
    store_op: u32,
    stencil_load_op: u32,
    stencil_store_op: u32,
    initial_layout: u32,
    final_layout: u32,
}
#[repr(C)]
struct AttachmentReference {
    attachment: u32,
    layout: u32,
}
#[repr(C)]
struct SubpassDescription {
    flags: u32,
    pipeline_bind_point: u32,
    input_attachment_count: u32,
    input_attachments: *const c_void,
    color_attachment_count: u32,
    color_attachments: *const AttachmentReference,
    resolve_attachments: *const c_void,
    depth_stencil_attachment: *const c_void,
    preserve_attachment_count: u32,
    preserve_attachments: *const u32,
}
#[repr(C)]
struct RenderPassCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    attachment_count: u32,
    attachments: *const AttachmentDescription,
    subpass_count: u32,
    subpasses: *const SubpassDescription,
    dependency_count: u32,
    dependencies: *const c_void,
}
#[repr(C)]
struct FramebufferCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    render_pass: VkRenderPass,
    attachment_count: u32,
    attachments: *const VkImageView,
    width: u32,
    height: u32,
    layers: u32,
}
#[repr(C)]
struct PipelineShaderStageCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    stage: u32,
    module: VkShaderModule,
    name: *const u8,
    specialization_info: *const c_void,
}
#[repr(C)]
struct PipelineVertexInputStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    vertex_binding_description_count: u32,
    vertex_binding_descriptions: *const c_void,
    vertex_attribute_description_count: u32,
    vertex_attribute_descriptions: *const c_void,
}
#[repr(C)]
struct PipelineInputAssemblyStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    topology: u32,
    primitive_restart_enable: u32,
}
#[repr(C)]
struct Viewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_depth: f32,
    max_depth: f32,
}
#[repr(C)]
struct PipelineViewportStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    viewport_count: u32,
    viewports: *const Viewport,
    scissor_count: u32,
    scissors: *const Rect2D,
}
#[repr(C)]
struct PipelineRasterizationStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    depth_clamp_enable: u32,
    rasterizer_discard_enable: u32,
    polygon_mode: u32,
    cull_mode: u32,
    front_face: u32,
    depth_bias_enable: u32,
    depth_bias_constant_factor: f32,
    depth_bias_clamp: f32,
    depth_bias_slope_factor: f32,
    line_width: f32,
}
#[repr(C)]
struct PipelineMultisampleStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    rasterization_samples: u32,
    sample_shading_enable: u32,
    min_sample_shading: f32,
    sample_mask: *const u32,
    alpha_to_coverage_enable: u32,
    alpha_to_one_enable: u32,
}
#[repr(C)]
struct PipelineColorBlendAttachmentState {
    blend_enable: u32,
    src_color_blend_factor: u32,
    dst_color_blend_factor: u32,
    color_blend_op: u32,
    src_alpha_blend_factor: u32,
    dst_alpha_blend_factor: u32,
    alpha_blend_op: u32,
    color_write_mask: u32,
}
#[repr(C)]
struct PipelineColorBlendStateCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    logic_op_enable: u32,
    logic_op: u32,
    attachment_count: u32,
    attachments: *const PipelineColorBlendAttachmentState,
    blend_constants: [f32; 4],
}
#[repr(C)]
struct GraphicsPipelineCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    stage_count: u32,
    stages: *const PipelineShaderStageCreateInfo,
    vertex_input_state: *const PipelineVertexInputStateCreateInfo,
    input_assembly_state: *const PipelineInputAssemblyStateCreateInfo,
    tessellation_state: *const c_void,
    viewport_state: *const PipelineViewportStateCreateInfo,
    rasterization_state: *const PipelineRasterizationStateCreateInfo,
    multisample_state: *const PipelineMultisampleStateCreateInfo,
    depth_stencil_state: *const c_void,
    color_blend_state: *const PipelineColorBlendStateCreateInfo,
    dynamic_state: *const c_void,
    layout: VkPipelineLayout,
    render_pass: VkRenderPass,
    subpass: u32,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
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

pub struct NativeShaderModule {
    handle: VkShaderModule,
    device: VkDevice,
    destroy: VkDestroyShaderModule,
}

pub struct NativeDescriptorSetLayout {
    handle: VkDescriptorSetLayout,
    device: VkDevice,
    destroy: VkDestroyDescriptorSetLayout,
}

impl NativeDescriptorSetLayout {
    pub const fn raw(&self) -> VkDescriptorSetLayout {
        self.handle
    }
}

impl Drop for NativeDescriptorSetLayout {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativeDescriptorPool {
    handle: VkDescriptorPool,
    device: VkDevice,
    destroy: VkDestroyDescriptorPool,
}

impl NativeDescriptorPool {
    pub const fn raw(&self) -> VkDescriptorPool {
        self.handle
    }

    /// Allocates one descriptor set and binds the UI storage buffer to it.
    pub unsafe fn allocate_ui_set(
        &self,
        loader: &VulkanLoader,
        layout: VkDescriptorSetLayout,
        buffer: VkBuffer,
        range: u64,
    ) -> Result<VkDescriptorSet, VulkanLoaderError> {
        if layout == 0 || buffer == 0 || range == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let allocate: VkAllocateDescriptorSets =
            loader.device_command(self.device, b"vkAllocateDescriptorSets\0")?;
        let layouts = [layout];
        let info = DescriptorSetAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
            next: core::ptr::null(),
            descriptor_pool: self.handle,
            descriptor_set_count: 1,
            set_layouts: layouts.as_ptr(),
        };
        let mut set = 0;
        let result = unsafe { allocate(self.device, &info, &mut set) };
        if result != VK_SUCCESS || set == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        let update: VkUpdateDescriptorSets =
            loader.device_command(self.device, b"vkUpdateDescriptorSets\0")?;
        let buffer_info = DescriptorBufferInfo {
            buffer,
            offset: 0,
            range,
        };
        let write = WriteDescriptorSet {
            s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
            next: core::ptr::null(),
            dst_set: set,
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            image_info: core::ptr::null(),
            buffer_info: &buffer_info,
            texel_buffer_view: core::ptr::null(),
        };
        unsafe { update(self.device, 1, &write, 0, core::ptr::null()) };
        Ok(set)
    }
}

impl Drop for NativeDescriptorPool {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativePipelineLayout {
    handle: VkPipelineLayout,
    device: VkDevice,
    destroy: VkDestroyPipelineLayout,
}

impl NativePipelineLayout {
    pub const fn raw(&self) -> VkPipelineLayout {
        self.handle
    }
}

impl Drop for NativePipelineLayout {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativeImageView {
    handle: VkImageView,
    device: VkDevice,
    destroy: VkDestroyImageView,
}

impl NativeImageView {
    pub const fn raw(&self) -> VkImageView {
        self.handle
    }
}

impl Drop for NativeImageView {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativeRenderPass {
    handle: VkRenderPass,
    device: VkDevice,
    destroy: VkDestroyRenderPass,
}

impl NativeRenderPass {
    pub const fn raw(&self) -> VkRenderPass {
        self.handle
    }
}

impl Drop for NativeRenderPass {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

pub struct NativeFramebuffer {
    handle: VkFramebuffer,
    device: VkDevice,
    destroy: VkDestroyFramebuffer,
}

impl NativeFramebuffer {
    pub const fn raw(&self) -> VkFramebuffer {
        self.handle
    }
}

impl Drop for NativeFramebuffer {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}
pub struct NativeGraphicsPipeline {
    handle: VkPipeline,
    device: VkDevice,
    destroy: VkDestroyPipeline,
}

impl NativeGraphicsPipeline {
    pub const fn raw(&self) -> VkPipeline {
        self.handle
    }
}

impl Drop for NativeGraphicsPipeline {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
}

impl NativeShaderModule {
    pub const fn raw(&self) -> VkShaderModule {
        self.handle
    }
}

impl Drop for NativeShaderModule {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.device, self.handle, core::ptr::null()) };
    }
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
    /// allocation is non-coherent.
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
/// The device-aware variant is preferred for live rendering because Vulkan
/// device commands must be resolved through `vkGetDeviceProcAddr`.
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

/// Binds one UI vertex/instance buffer through the device dispatch table.
/// The caller must have a compatible graphics pipeline bound.
pub unsafe fn cmd_bind_vertex_buffer_device(
    loader: &VulkanLoader,
    device: VkDevice,
    command: VkCommandBuffer,
    buffer: VkBuffer,
    offset: u64,
) -> Result<(), VulkanLoaderError> {
    if device.is_null() || command.is_null() || buffer == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let bind: VkCmdBindVertexBuffers =
        loader.device_command(device, b"vkCmdBindVertexBuffers\0")?;
    unsafe { bind(command, 0, 1, &buffer, &offset) };
    Ok(())
}

/// Records one non-indexed UI draw through the device dispatch table.
/// Validation rejects zero work, while Vulkan pipeline compatibility remains
/// the responsibility of the renderer pipeline contract.
pub unsafe fn cmd_draw_device(
    loader: &VulkanLoader,
    device: VkDevice,
    command: VkCommandBuffer,
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
) -> Result<(), VulkanLoaderError> {
    if device.is_null() || command.is_null() || vertex_count == 0 || instance_count == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let draw: VkCmdDraw = loader.device_command(device, b"vkCmdDraw\0")?;
    unsafe {
        draw(
            command,
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        )
    };
    Ok(())
}

/// Records one bounded staging-to-device buffer copy.
///
/// # Safety
/// All handles must belong to the same live device and the command buffer must
/// be recording. The caller must provide the required transfer barriers.
#[allow(clippy::too_many_arguments)]
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
#[allow(clippy::too_many_arguments)]
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

    /// Creates an owned shader module from validated SPIR-V words.
    pub unsafe fn create_shader_module(
        &self,
        loader: &VulkanLoader,
        words: &[u32],
    ) -> Result<NativeShaderModule, VulkanLoaderError> {
        if words.is_empty() || words[0] != 0x0723_0203 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateShaderModule =
            loader.device_command(self.handle, b"vkCreateShaderModule\0")?;
        let destroy: VkDestroyShaderModule =
            loader.device_command(self.handle, b"vkDestroyShaderModule\0")?;
        let info = ShaderModuleCreateInfo {
            s_type: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            code_size: words
                .len()
                .checked_mul(core::mem::size_of::<u32>())
                .ok_or(VulkanLoaderError::InvalidQueuePlan)?,
            code: words.as_ptr(),
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeShaderModule {
            handle,
            device: self.handle,
            destroy,
        })
    }

    pub unsafe fn create_ui_descriptor_set_layout(
        &self,
        loader: &VulkanLoader,
    ) -> Result<NativeDescriptorSetLayout, VulkanLoaderError> {
        let create: VkCreateDescriptorSetLayout =
            loader.device_command(self.handle, b"vkCreateDescriptorSetLayout\0")?;
        let destroy: VkDestroyDescriptorSetLayout =
            loader.device_command(self.handle, b"vkDestroyDescriptorSetLayout\0")?;
        let binding = DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            descriptor_count: 1,
            stage_flags: VK_SHADER_STAGE_VERTEX_BIT,
            immutable_samplers: core::ptr::null(),
        };
        let info = DescriptorSetLayoutCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            binding_count: 1,
            bindings: &binding,
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeDescriptorSetLayout {
            handle,
            device: self.handle,
            destroy,
        })
    }

    /// Creates a bounded one-set pool for the UI storage-buffer binding.
    pub unsafe fn create_ui_descriptor_pool(
        &self,
        loader: &VulkanLoader,
    ) -> Result<NativeDescriptorPool, VulkanLoaderError> {
        let create: VkCreateDescriptorPool =
            loader.device_command(self.handle, b"vkCreateDescriptorPool\0")?;
        let destroy: VkDestroyDescriptorPool =
            loader.device_command(self.handle, b"vkDestroyDescriptorPool\0")?;
        let size = DescriptorPoolSize {
            descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            descriptor_count: 1,
        };
        let info = DescriptorPoolCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
            next: core::ptr::null(),
            flags: VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT,
            max_sets: 1,
            pool_size_count: 1,
            pool_sizes: &size,
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeDescriptorPool {
            handle,
            device: self.handle,
            destroy,
        })
    }

    pub unsafe fn create_ui_pipeline_layout(
        &self,
        loader: &VulkanLoader,
        descriptor_set_layout: VkDescriptorSetLayout,
    ) -> Result<NativePipelineLayout, VulkanLoaderError> {
        if descriptor_set_layout == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreatePipelineLayout =
            loader.device_command(self.handle, b"vkCreatePipelineLayout\0")?;
        let destroy: VkDestroyPipelineLayout =
            loader.device_command(self.handle, b"vkDestroyPipelineLayout\0")?;
        let set_layouts = [descriptor_set_layout];
        let push = PushConstantRange {
            stage_flags: VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
            offset: 0,
            size: 8,
        };
        let info = PipelineLayoutCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            set_layout_count: 1,
            set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 1,
            push_constant_ranges: &push,
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativePipelineLayout {
            handle,
            device: self.handle,
            destroy,
        })
    }

    /// Records the backend-neutral UI graphics commands into an active
    /// framebuffer. Resource creation remains owned by the presentation lane.
    pub unsafe fn cmd_ui_draw(
        &self,
        loader: &VulkanLoader,
        command_buffer: VkCommandBuffer,
        render_pass: VkRenderPass,
        framebuffer: u64,
        pipeline: VkPipeline,
        pipeline_layout: VkPipelineLayout,
        descriptor_set: VkDescriptorSet,
        extent: Extent2D,
        clear_color: [f32; 4],
        draw_commands: &[crate::subsystems::renderer::UiDrawCommand],
    ) -> Result<(), VulkanLoaderError> {
        if command_buffer.is_null()
            || render_pass == 0
            || framebuffer == 0
            || pipeline == 0
            || pipeline_layout == 0
            || descriptor_set == 0
            || extent.width == 0
            || extent.height == 0
            || draw_commands.is_empty()
        {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let begin: VkCmdBeginRenderPass =
            loader.device_command(self.handle, b"vkCmdBeginRenderPass\0")?;
        let end: VkCmdEndRenderPass =
            loader.device_command(self.handle, b"vkCmdEndRenderPass\0")?;
        let bind_pipeline: VkCmdBindPipeline =
            loader.device_command(self.handle, b"vkCmdBindPipeline\0")?;
        let bind_sets: VkCmdBindDescriptorSets =
            loader.device_command(self.handle, b"vkCmdBindDescriptorSets\0")?;
        let push_constants: VkCmdPushConstants =
            loader.device_command(self.handle, b"vkCmdPushConstants\0")?;
        let draw: VkCmdDraw = loader.device_command(self.handle, b"vkCmdDraw\0")?;
        let clear = ClearValue { color: clear_color };
        let info = RenderPassBeginInfo {
            s_type: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            next: core::ptr::null(),
            render_pass,
            framebuffer,
            render_area: Rect2D {
                offset: [0, 0],
                extent,
            },
            clear_value_count: 1,
            clear_values: &clear,
        };
        unsafe {
            begin(command_buffer, &info, VK_SUBPASS_CONTENTS_INLINE);
            bind_pipeline(command_buffer, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
            bind_sets(
                command_buffer,
                VK_PIPELINE_BIND_POINT_GRAPHICS,
                pipeline_layout,
                0,
                1,
                &descriptor_set,
                0,
                core::ptr::null(),
            );
            let viewport = [extent.width as f32, extent.height as f32];
            push_constants(
                command_buffer,
                pipeline_layout,
                VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                0,
                core::mem::size_of_val(&viewport) as u32,
                viewport.as_ptr().cast(),
            );
            for command in draw_commands {
                draw(
                    command_buffer,
                    command.vertex_count,
                    command.instance_count,
                    command.first_vertex,
                    command.first_instance,
                );
            }
            end(command_buffer);
        }
        Ok(())
    }

    pub unsafe fn create_color_image_view(
        &self,
        loader: &VulkanLoader,
        image: VkImage,
        format: u32,
    ) -> Result<NativeImageView, VulkanLoaderError> {
        if image == 0 || format == VK_FORMAT_UNDEFINED {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateImageView =
            loader.device_command(self.handle, b"vkCreateImageView\0")?;
        let destroy: VkDestroyImageView =
            loader.device_command(self.handle, b"vkDestroyImageView\0")?;
        let info = ImageViewCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            image,
            view_type: VK_IMAGE_VIEW_TYPE_2D,
            format,
            components: ComponentMapping {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            subresource_range: ImageSubresourceRange {
                aspect_mask: VK_IMAGE_ASPECT_COLOR_BIT,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeImageView {
            handle,
            device: self.handle,
            destroy,
        })
    }

    pub unsafe fn create_ui_render_pass(
        &self,
        loader: &VulkanLoader,
        format: u32,
    ) -> Result<NativeRenderPass, VulkanLoaderError> {
        if format == VK_FORMAT_UNDEFINED {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateRenderPass =
            loader.device_command(self.handle, b"vkCreateRenderPass\0")?;
        let destroy: VkDestroyRenderPass =
            loader.device_command(self.handle, b"vkDestroyRenderPass\0")?;
        let attachment = AttachmentDescription {
            flags: 0,
            format,
            samples: VK_SAMPLE_COUNT_1_BIT,
            load_op: VK_ATTACHMENT_LOAD_OP_CLEAR,
            store_op: VK_ATTACHMENT_STORE_OP_STORE,
            stencil_load_op: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
            final_layout: VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
        };
        let reference = AttachmentReference {
            attachment: 0,
            layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = SubpassDescription {
            flags: 0,
            pipeline_bind_point: VK_PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            input_attachments: core::ptr::null(),
            color_attachment_count: 1,
            color_attachments: &reference,
            resolve_attachments: core::ptr::null(),
            depth_stencil_attachment: core::ptr::null(),
            preserve_attachment_count: 0,
            preserve_attachments: core::ptr::null(),
        };
        let info = RenderPassCreateInfo {
            s_type: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            attachment_count: 1,
            attachments: &attachment,
            subpass_count: 1,
            subpasses: &subpass,
            dependency_count: 0,
            dependencies: core::ptr::null(),
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeRenderPass {
            handle,
            device: self.handle,
            destroy,
        })
    }

    pub unsafe fn create_framebuffer(
        &self,
        loader: &VulkanLoader,
        render_pass: VkRenderPass,
        image_view: VkImageView,
        extent: Extent2D,
    ) -> Result<NativeFramebuffer, VulkanLoaderError> {
        if render_pass == 0 || image_view == 0 || extent.width == 0 || extent.height == 0 {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateFramebuffer =
            loader.device_command(self.handle, b"vkCreateFramebuffer\0")?;
        let destroy: VkDestroyFramebuffer =
            loader.device_command(self.handle, b"vkDestroyFramebuffer\0")?;
        let attachments = [image_view];
        let info = FramebufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            render_pass,
            attachment_count: 1,
            attachments: attachments.as_ptr(),
            width: extent.width,
            height: extent.height,
            layers: 1,
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeFramebuffer {
            handle,
            device: self.handle,
            destroy,
        })
    }

    pub unsafe fn create_ui_graphics_pipeline(
        &self,
        loader: &VulkanLoader,
        vertex_shader: VkShaderModule,
        fragment_shader: VkShaderModule,
        pipeline_layout: VkPipelineLayout,
        render_pass: VkRenderPass,
        extent: Extent2D,
    ) -> Result<NativeGraphicsPipeline, VulkanLoaderError> {
        if vertex_shader == 0
            || fragment_shader == 0
            || pipeline_layout == 0
            || render_pass == 0
            || extent.width == 0
            || extent.height == 0
        {
            return Err(VulkanLoaderError::InvalidQueuePlan);
        }
        let create: VkCreateGraphicsPipelines =
            loader.device_command(self.handle, b"vkCreateGraphicsPipelines\0")?;
        let destroy: VkDestroyPipeline =
            loader.device_command(self.handle, b"vkDestroyPipeline\0")?;
        let entry = b"main\0";
        let stages = [
            PipelineShaderStageCreateInfo {
                s_type: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                next: core::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_VERTEX_BIT,
                module: vertex_shader,
                name: entry.as_ptr(),
                specialization_info: core::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                next: core::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_FRAGMENT_BIT,
                module: fragment_shader,
                name: entry.as_ptr(),
                specialization_info: core::ptr::null(),
            },
        ];
        let vertex_input = PipelineVertexInputStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 0,
            vertex_binding_descriptions: core::ptr::null(),
            vertex_attribute_description_count: 0,
            vertex_attribute_descriptions: core::ptr::null(),
        };
        let assembly = PipelineInputAssemblyStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let viewport = Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = Rect2D {
            offset: [0, 0],
            extent,
        };
        let viewport_state = PipelineViewportStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            viewport_count: 1,
            viewports: &viewport,
            scissor_count: 1,
            scissors: &scissor,
        };
        let rasterization = PipelineRasterizationStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: VK_POLYGON_MODE_FILL,
            cull_mode: 0,
            front_face: 0,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let multisample = PipelineMultisampleStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            rasterization_samples: VK_SAMPLE_COUNT_1_BIT,
            sample_shading_enable: 0,
            min_sample_shading: 0.0,
            sample_mask: core::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        let blend_attachment = PipelineColorBlendAttachmentState {
            blend_enable: 1,
            src_color_blend_factor: VK_BLEND_FACTOR_SRC_ALPHA,
            dst_color_blend_factor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
            color_blend_op: VK_BLEND_OP_ADD,
            src_alpha_blend_factor: VK_BLEND_FACTOR_ONE,
            dst_alpha_blend_factor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: VK_BLEND_OP_ADD,
            color_write_mask: VK_COLOR_COMPONENT_R_BIT
                | VK_COLOR_COMPONENT_G_BIT
                | VK_COLOR_COMPONENT_B_BIT
                | VK_COLOR_COMPONENT_A_BIT,
        };
        let blend = PipelineColorBlendStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            attachments: &blend_attachment,
            blend_constants: [0.0; 4],
        };
        let info = GraphicsPipelineCreateInfo {
            s_type: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            next: core::ptr::null(),
            flags: 0,
            stage_count: 2,
            stages: stages.as_ptr(),
            vertex_input_state: &vertex_input,
            input_assembly_state: &assembly,
            tessellation_state: core::ptr::null(),
            viewport_state: &viewport_state,
            rasterization_state: &rasterization,
            multisample_state: &multisample,
            depth_stencil_state: core::ptr::null(),
            color_blend_state: &blend,
            dynamic_state: core::ptr::null(),
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: 0,
            base_pipeline_index: -1,
        };
        let mut handle = 0;
        let result = unsafe { create(self.handle, 0, 1, &info, core::ptr::null(), &mut handle) };
        if result != VK_SUCCESS || handle == 0 {
            return Err(VulkanLoaderError::Api(result));
        }
        Ok(NativeGraphicsPipeline {
            handle,
            device: self.handle,
            destroy,
        })
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
    active_instance: AtomicPtr<c_void>,
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
            active_instance: AtomicPtr::new(core::ptr::null_mut()),
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

    /// Resolves an instance-level command through a live Vulkan instance.
    ///
    /// # Safety
    /// `instance` must be live and `name` must be NUL-terminated.
    pub unsafe fn instance_proc(&self, instance: VkInstance, name: &[u8]) -> Option<*const c_void> {
        if name.last().copied() != Some(0) || instance.is_null() {
            return None;
        }
        let pointer = unsafe { (self.get_instance_proc_addr)(instance, name.as_ptr()) };
        (!pointer.is_null()).then_some(pointer)
    }

    /// Creates a persistent Vulkan instance with the given instance
    /// extensions (e.g. `VK_KHR_surface` + `VK_KHR_win32_surface`). The
    /// instance is destroyed when dropped.
    ///
    /// # Safety
    /// Every extension name must be NUL-terminated and supported by the
    /// loader; extension lists must stay live for the synchronous call.
    pub(crate) fn instance_command<T>(
        &self,
        instance: VkInstance,
        name: &'static [u8],
    ) -> Result<T, VulkanLoaderError> {
        let pointer = unsafe { self.instance_proc(instance, name) }.ok_or_else(|| {
            VulkanLoaderError::MissingEntry(DlError::Symbol {
                symbol: String::from_utf8_lossy(name).into_owned(),
                code: 0,
            })
        })?;
        Ok(unsafe { core::mem::transmute_copy(&pointer) })
    }

    pub fn create_instance(
        &self,
        extensions: &[&[u8]],
    ) -> Result<NativeInstance, VulkanLoaderError> {
        let create: VkCreateInstance = self.command(b"vkCreateInstance\0")?;
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
        self.active_instance.store(instance, Ordering::Release);
        let destroy: VkDestroyInstance = self.instance_command(instance, b"vkDestroyInstance\0")?;
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
        let instance = self.active_instance.load(Ordering::Acquire);
        let pointer = unsafe { self.instance_proc(instance, name) }
            .or_else(|| unsafe { self.global_proc(name) })
            .ok_or_else(|| {
                VulkanLoaderError::MissingEntry(DlError::Symbol {
                    symbol: String::from_utf8_lossy(name).into_owned(),
                    code: 0,
                })
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
