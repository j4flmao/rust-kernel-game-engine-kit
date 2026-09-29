//! Windows Vulkan command ordering for GPU voxel culling and indirect draws.
#![allow(unsafe_code)] // Raw Vulkan FFI boundary, matching the platform loader.

use core::ffi::c_void;

use super::vulkan::{
    cmd_bind_pipeline_device, cmd_buffer_barrier_device, cmd_dispatch_device,
    cmd_draw_indexed_indirect_device, VkBuffer, VkCommandBuffer, VkDevice, VkPipeline,
    VkPipelineLayout, VkShaderModule, VulkanLoader, VulkanLoaderError,
};

#[repr(C)]
struct ComputeStage {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    stage: u32,
    module: VkShaderModule,
    name: *const u8,
    specialization_info: *const c_void,
}
#[repr(C)]
struct ComputeInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    stage: ComputeStage,
    layout: VkPipelineLayout,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

pub unsafe fn create_compute_pipeline(
    loader: &VulkanLoader,
    device: VkDevice,
    shader: VkShaderModule,
    layout: VkPipelineLayout,
) -> Result<VkPipeline, VulkanLoaderError> {
    if device.is_null() || shader == 0 || layout == 0 {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    type Create = unsafe extern "system" fn(
        VkDevice,
        VkPipeline,
        u32,
        *const ComputeInfo,
        *const c_void,
        *mut VkPipeline,
    ) -> i32;
    let create: Create = loader.device_command(device, b"vkCreateComputePipelines\0")?;
    let entry = b"main\0";
    let info = ComputeInfo {
        s_type: 28,
        next: core::ptr::null(),
        flags: 0,
        stage: ComputeStage {
            s_type: 16,
            next: core::ptr::null(),
            flags: 0,
            stage: 0x20,
            module: shader,
            name: entry.as_ptr(),
            specialization_info: core::ptr::null(),
        },
        layout,
        base_pipeline_handle: 0,
        base_pipeline_index: -1,
    };
    let mut pipeline = 0;
    let result = unsafe { create(device, 0, 1, &info, core::ptr::null(), &mut pipeline) };
    if result != 0 || pipeline == 0 {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(pipeline)
}

pub const VK_PIPELINE_BIND_POINT_GRAPHICS: u32 = 0;
pub const VK_PIPELINE_BIND_POINT_COMPUTE: u32 = 1;
pub const VK_PIPELINE_STAGE_COMPUTE_SHADER: u32 = 0x800;
pub const VK_PIPELINE_STAGE_DRAW_INDIRECT: u32 = 0x2;
pub const VK_ACCESS_SHADER_WRITE: u32 = 0x40;
pub const VK_ACCESS_SHADER_READ: u32 = 0x20;
pub const VK_ACCESS_INDIRECT_COMMAND_READ: u32 = 0x2;

/// Records compute culling, indirect-command generation, synchronization, and
/// the final indexed indirect draw. Descriptor binding remains owned by the
/// caller because the voxel instance layout is application-specific.
#[allow(clippy::too_many_arguments)]
pub unsafe fn record_cull_and_indirect_draw(
    loader: &VulkanLoader,
    device: VkDevice,
    command: VkCommandBuffer,
    compute_pipeline: VkPipeline,
    graphics_pipeline: VkPipeline,
    visibility_buffer: VkBuffer,
    visibility_size: u64,
    indirect_buffer: VkBuffer,
    indirect_size: u64,
    workgroups: [u32; 3],
    draw_count: u32,
    indirect_stride: u32,
) -> Result<(), VulkanLoaderError> {
    if visibility_buffer == 0
        || indirect_buffer == 0
        || visibility_size == 0
        || indirect_size == 0
        || draw_count == 0
    {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    unsafe {
        cmd_bind_pipeline_device(
            loader,
            device,
            command,
            VK_PIPELINE_BIND_POINT_COMPUTE,
            compute_pipeline,
        )?;
        cmd_dispatch_device(loader, device, command, workgroups)?;
        cmd_buffer_barrier_device(
            loader,
            device,
            command,
            visibility_buffer,
            0,
            visibility_size,
            VK_PIPELINE_STAGE_COMPUTE_SHADER,
            VK_PIPELINE_STAGE_COMPUTE_SHADER,
            VK_ACCESS_SHADER_WRITE,
            VK_ACCESS_SHADER_READ,
        )?;
        cmd_buffer_barrier_device(
            loader,
            device,
            command,
            indirect_buffer,
            0,
            indirect_size,
            VK_PIPELINE_STAGE_COMPUTE_SHADER,
            VK_PIPELINE_STAGE_DRAW_INDIRECT,
            VK_ACCESS_SHADER_WRITE,
            VK_ACCESS_INDIRECT_COMMAND_READ,
        )?;
        cmd_bind_pipeline_device(
            loader,
            device,
            command,
            VK_PIPELINE_BIND_POINT_GRAPHICS,
            graphics_pipeline,
        )?;
        cmd_draw_indexed_indirect_device(
            loader,
            device,
            command,
            indirect_buffer,
            0,
            draw_count,
            indirect_stride,
        )?;
    }
    Ok(())
}
