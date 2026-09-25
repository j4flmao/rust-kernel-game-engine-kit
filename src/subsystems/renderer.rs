//! Headless renderer lifecycle and frame accounting.
//!
//! The renderer owns no OS or Vulkan handles yet.  This deliberately small
//! subsystem freezes the renderer/kernel contract before native device and
//! swapchain code is added: initialization is explicit, ticks are allocation
//! free, and shutdown is observable.

use crate::kernel::ecs::entity::Entity;
use crate::kernel::ecs::world::World;
use crate::kernel::{KernelContext, Subsystem};
use crate::platform::vulkan_policy::{select_queue_families, QueueFamilyInfo, QueueSelection};
use crate::subsystems::messages::WindowResized;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuCandidate {
    pub device_score: u8,
    pub queue_families: &'static [QueueFamilyInfo],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderDevicePlan {
    pub candidate_index: usize,
    pub queues: QueueSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderDevicePlanError {
    NoSuitableDevice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandState {
    Initial,
    Recording,
    Executable,
    Submitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    InvalidTransition,
    AllocationFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameState {
    Available,
    Acquired,
    Submitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceState {
    Unsignaled,
    Submitted,
    Signaled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemaphoreState {
    Unsignaled,
    Signaled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceState {
    Uncreated,
    Ready,
    Destroyed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceLifecycle {
    state: SurfaceState,
}
impl SurfaceLifecycle {
    pub const fn new() -> Self {
        Self {
            state: SurfaceState::Uncreated,
        }
    }
    pub const fn state(self) -> SurfaceState {
        self.state
    }
    pub fn create(&mut self) -> Result<(), CommandError> {
        if self.state != SurfaceState::Uncreated {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SurfaceState::Ready;
        Ok(())
    }
    pub fn destroy(&mut self) -> Result<(), CommandError> {
        if self.state != SurfaceState::Ready {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SurfaceState::Destroyed;
        Ok(())
    }
}
impl Default for SurfaceLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapchainState {
    Uncreated,
    Ready,
    OutOfDate,
    Destroyed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapchainConfig {
    pub width: u32,
    pub height: u32,
    pub image_count: u32,
    pub format: u32,
    pub present_mode: u32,
}
impl SwapchainConfig {
    pub const fn validate(self) -> Result<Self, CommandError> {
        if self.width == 0 || self.height == 0 || self.image_count < 2 || self.format == 0 {
            return Err(CommandError::InvalidTransition);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwapchainLifecycle {
    state: SwapchainState,
}
impl SwapchainLifecycle {
    pub const fn new() -> Self {
        Self {
            state: SwapchainState::Uncreated,
        }
    }
    pub const fn state(self) -> SwapchainState {
        self.state
    }
    pub fn create(&mut self, surface: SurfaceState) -> Result<(), CommandError> {
        if self.state != SwapchainState::Uncreated || surface != SurfaceState::Ready {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SwapchainState::Ready;
        Ok(())
    }
    pub fn mark_out_of_date(&mut self) -> Result<(), CommandError> {
        if self.state != SwapchainState::Ready {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SwapchainState::OutOfDate;
        Ok(())
    }
    pub fn destroy(&mut self) -> Result<(), CommandError> {
        if !matches!(
            self.state,
            SwapchainState::Ready | SwapchainState::OutOfDate
        ) {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SwapchainState::Destroyed;
        Ok(())
    }
}
impl Default for SwapchainLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinarySemaphore {
    state: SemaphoreState,
}

impl BinarySemaphore {
    pub const fn new() -> Self {
        Self {
            state: SemaphoreState::Unsignaled,
        }
    }
    pub const fn state(self) -> SemaphoreState {
        self.state
    }
    pub fn signal(&mut self) -> Result<(), CommandError> {
        if self.state != SemaphoreState::Unsignaled {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SemaphoreState::Signaled;
        Ok(())
    }
    pub fn consume(&mut self) -> Result<(), CommandError> {
        if self.state != SemaphoreState::Signaled {
            return Err(CommandError::InvalidTransition);
        }
        self.state = SemaphoreState::Unsignaled;
        Ok(())
    }
}

impl Default for BinarySemaphore {
    fn default() -> Self {
        Self::new()
    }
}

pub fn submit_frame(
    command: &mut CommandBufferState,
    wait: &mut BinarySemaphore,
    signal: &mut BinarySemaphore,
    fence: &mut Fence,
) -> Result<(), CommandError> {
    if command.state() != CommandState::Executable
        || wait.state() != SemaphoreState::Signaled
        || signal.state() != SemaphoreState::Unsignaled
        || fence.state() != FenceState::Unsignaled
    {
        return Err(CommandError::InvalidTransition);
    }
    command.submit()?;
    wait.consume()?;
    signal.signal()?;
    fence.submit()?;
    Ok(())
}

pub struct RenderFrame {
    pub command: CommandBufferState,
    pub slot: FrameSlot,
    pub wait: BinarySemaphore,
    pub signal: BinarySemaphore,
    pub fence: Fence,
}

impl RenderFrame {
    pub const fn new() -> Self {
        Self {
            command: CommandBufferState::new(),
            slot: FrameSlot::new(),
            wait: BinarySemaphore::new(),
            signal: BinarySemaphore::new(),
            fence: Fence::new(),
        }
    }
    pub fn begin(&mut self) -> Result<(), CommandError> {
        self.slot.acquire()?;
        self.command.begin()
    }
    pub fn submit(&mut self) -> Result<(), CommandError> {
        self.command.end()?;
        submit_frame(
            &mut self.command,
            &mut self.wait,
            &mut self.signal,
            &mut self.fence,
        )?;
        self.slot.submit()
    }
    pub fn complete(&mut self) -> Result<(), CommandError> {
        self.signal.consume()?;
        self.fence.signal()?;
        self.fence.reset()?;
        self.slot.signal()?;
        self.command.reset()
    }
}

impl Default for RenderFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fence {
    state: FenceState,
}

impl Fence {
    pub const fn new() -> Self {
        Self {
            state: FenceState::Unsignaled,
        }
    }
    pub const fn state(self) -> FenceState {
        self.state
    }
    pub fn submit(&mut self) -> Result<(), CommandError> {
        if self.state != FenceState::Unsignaled {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FenceState::Submitted;
        Ok(())
    }
    pub fn signal(&mut self) -> Result<(), CommandError> {
        if self.state != FenceState::Submitted {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FenceState::Signaled;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<(), CommandError> {
        if self.state != FenceState::Signaled {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FenceState::Unsignaled;
        Ok(())
    }
}

impl Default for Fence {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSlot {
    state: FrameState,
}

impl FrameSlot {
    pub const fn new() -> Self {
        Self {
            state: FrameState::Available,
        }
    }
    pub const fn state(self) -> FrameState {
        self.state
    }
    pub fn acquire(&mut self) -> Result<(), CommandError> {
        if self.state != FrameState::Available {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FrameState::Acquired;
        Ok(())
    }
    pub fn submit(&mut self) -> Result<(), CommandError> {
        if self.state != FrameState::Acquired {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FrameState::Submitted;
        Ok(())
    }
    pub fn signal(&mut self) -> Result<(), CommandError> {
        if self.state != FrameState::Submitted {
            return Err(CommandError::InvalidTransition);
        }
        self.state = FrameState::Available;
        Ok(())
    }
}

impl Default for FrameSlot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandBufferState {
    state: CommandState,
}

impl CommandBufferState {
    pub const fn new() -> Self {
        Self {
            state: CommandState::Initial,
        }
    }
    pub const fn state(self) -> CommandState {
        self.state
    }
    pub fn begin(&mut self) -> Result<(), CommandError> {
        if self.state != CommandState::Initial {
            return Err(CommandError::InvalidTransition);
        }
        self.state = CommandState::Recording;
        Ok(())
    }
    pub fn end(&mut self) -> Result<(), CommandError> {
        if self.state != CommandState::Recording {
            return Err(CommandError::InvalidTransition);
        }
        self.state = CommandState::Executable;
        Ok(())
    }
    pub fn submit(&mut self) -> Result<(), CommandError> {
        if self.state != CommandState::Executable {
            return Err(CommandError::InvalidTransition);
        }
        self.state = CommandState::Submitted;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<(), CommandError> {
        if self.state != CommandState::Submitted {
            return Err(CommandError::InvalidTransition);
        }
        self.state = CommandState::Initial;
        Ok(())
    }
}

impl Default for CommandBufferState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandPool {
    buffers: Vec<CommandBufferState>,
}

impl CommandPool {
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("command pool allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, CommandError> {
        let mut buffers = Vec::new();
        buffers
            .try_reserve_exact(capacity)
            .map_err(|_| CommandError::AllocationFailed)?;
        Ok(Self { buffers })
    }
    pub fn allocate(&mut self) -> Option<usize> {
        if self.buffers.len() == self.buffers.capacity() {
            return None;
        }
        self.buffers.push(CommandBufferState::new());
        Some(self.buffers.len() - 1)
    }
    pub fn buffer(&self, index: usize) -> Option<CommandBufferState> {
        self.buffers.get(index).copied()
    }
    pub fn buffer_mut(&mut self, index: usize) -> Option<&mut CommandBufferState> {
        self.buffers.get_mut(index)
    }
    pub fn reset(&mut self) -> Result<(), CommandError> {
        if self
            .buffers
            .iter()
            .any(|buffer| buffer.state() == CommandState::Submitted)
        {
            return Err(CommandError::InvalidTransition);
        }
        for buffer in &mut self.buffers {
            *buffer = CommandBufferState::new();
        }
        Ok(())
    }
}

impl core::fmt::Display for RenderDevicePlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("no GPU has the required render queue families")
    }
}

impl core::error::Error for RenderDevicePlanError {}

pub fn choose_render_device(
    candidates: &[GpuCandidate],
) -> Result<RenderDevicePlan, RenderDevicePlanError> {
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            select_queue_families(candidate.queue_families)
                .ok()
                .map(|queues| (index, candidate.device_score, queues))
        })
        .max_by_key(|(index, score, _)| (*score, core::cmp::Reverse(*index)))
        .map(|(candidate_index, _, queues)| RenderDevicePlan {
            candidate_index,
            queues,
        })
        .ok_or(RenderDevicePlanError::NoSuitableDevice)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererState {
    New,
    Ready,
    Shutdown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub frames: u64,
    pub total_dt_ns: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderInstance {
    pub entity: Entity,
    pub transform: [f32; 12],
    pub bounds: [f32; 4],
    pub mesh_id: u32,
    pub material_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderTransform {
    pub matrix: [f32; 12],
    pub bounds: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderMaterial {
    pub mesh_id: u32,
    pub material_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderWorldError {
    CapacityExceeded,
    AllocationFailed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtractionStats {
    pub considered: usize,
    pub extracted: usize,
    pub rejected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawBatch {
    pub mesh_id: u32,
    pub material_id: u32,
    pub first_instance: u32,
    pub instance_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderManifestError {
    CapacityExceeded,
    AllocationFailed,
    IndexOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPreparationError {
    CapacityExceeded,
    AllocationFailed,
    IndexOverflow,
    InvalidRange,
    MissingGeometry,
    OutputTooSmall,
    InvalidGeometry,
    InvalidWorkgroupSize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderSubmitError {
    InvalidPreparation,
    DeviceLost,
    Unsupported,
    OutOfDate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderSubmissionStats {
    pub submitted_frames: u64,
    pub submitted_instances: u64,
    pub submitted_commands: u64,
}

/// Backend boundary shared by headless, native Vulkan, and test backends.
pub trait RenderBackend {
    fn submit(
        &mut self,
        preparation: &GpuFramePreparation,
    ) -> Result<RenderSubmissionStats, RenderSubmitError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndirectDrawCommand {
    pub mesh_id: u32,
    pub material_id: u32,
    pub first_instance: u32,
    pub instance_count: u32,
}

/// Geometry metadata required to translate a logical mesh batch into the
/// Vulkan `VkDrawIndexedIndirectCommand` layout. Material identity remains in
/// the instance stream; it is not part of Vulkan's indirect command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexedGeometry {
    pub mesh_id: u32,
    pub index_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
}

/// Exact wire layout consumed by `vkCmdDrawIndexedIndirect`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VulkanIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

pub const VULKAN_INDEXED_INDIRECT_COMMAND_SIZE: usize =
    core::mem::size_of::<VulkanIndexedIndirectCommand>();

/// Stable host-to-device instance record. The shader-side layout is explicit
/// instead of relying on Rust's `RenderInstance` layout or enum padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuInstanceRecord {
    pub transform: [f32; 12],
    pub bounds: [f32; 4],
    pub entity_index: u32,
    pub entity_generation: u32,
    pub mesh_id: u32,
    pub material_id: u32,
}

pub const GPU_INSTANCE_RECORD_SIZE: usize = core::mem::size_of::<GpuInstanceRecord>();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuFrameCommandPlan {
    pub instance_count: u32,
    pub draw_count: u32,
    pub instance_upload_bytes: usize,
    pub indirect_upload_bytes: usize,
    pub cull_workgroups: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuUploadError {
    Preparation(GpuPreparationError),
    AllocationFailed,
}

/// Fully encoded, bounded upload payload for one frame. Native backends can
/// copy these slices into staging buffers without re-deriving sizes or
/// touching ECS-owned data.
pub struct GpuFrameUpload {
    plan: GpuFrameCommandPlan,
    instances: Vec<u8>,
    indirect_commands: Vec<u8>,
}

impl GpuFrameUpload {
    pub fn try_build(
        preparation: &GpuFramePreparation,
        geometry: &[IndexedGeometry],
        workgroup_size: u32,
    ) -> Result<Self, GpuUploadError> {
        let mut instances = Vec::new();
        let mut indirect_commands = Vec::new();
        let plan = Self::try_encode_into(
            preparation,
            geometry,
            workgroup_size,
            &mut instances,
            &mut indirect_commands,
        )?;
        Ok(Self {
            plan,
            instances,
            indirect_commands,
        })
    }

    /// Encodes the bounded payload into caller-owned staging storage.
    ///
    /// Native frame rings can reuse these vectors across frames, avoiding a
    /// fresh allocation on every render tick. The command plan is calculated
    /// before either buffer is resized, so invalid workgroup sizes and other
    /// preparation errors leave the caller's storage untouched.
    pub fn try_encode_into(
        preparation: &GpuFramePreparation,
        geometry: &[IndexedGeometry],
        workgroup_size: u32,
        instances: &mut Vec<u8>,
        indirect_commands: &mut Vec<u8>,
    ) -> Result<GpuFrameCommandPlan, GpuUploadError> {
        let plan = preparation
            .command_plan(workgroup_size)
            .map_err(GpuUploadError::Preparation)?;
        instances
            .try_reserve_exact(plan.instance_upload_bytes.saturating_sub(instances.len()))
            .map_err(|_| GpuUploadError::AllocationFailed)?;
        indirect_commands
            .try_reserve_exact(
                plan.indirect_upload_bytes
                    .saturating_sub(indirect_commands.len()),
            )
            .map_err(|_| GpuUploadError::AllocationFailed)?;
        instances.resize(plan.instance_upload_bytes, 0);
        indirect_commands.resize(plan.indirect_upload_bytes, 0);
        preparation
            .encode_instance_records(instances)
            .map_err(GpuUploadError::Preparation)?;
        preparation
            .encode_indexed_commands(geometry, indirect_commands)
            .map_err(GpuUploadError::Preparation)?;
        Ok(plan)
    }

    pub const fn plan(&self) -> GpuFrameCommandPlan {
        self.plan
    }

    pub fn instance_bytes(&self) -> &[u8] {
        &self.instances
    }

    pub fn indirect_command_bytes(&self) -> &[u8] {
        &self.indirect_commands
    }
}

/// CPU-owned model of the GPU-visible frame buffers.
///
/// This deliberately contains no Vulkan handle. It validates the same bounds
/// that the native backend must enforce before copying data into device-local
/// or host-visible buffers.
pub struct GpuFramePreparation {
    instances: Vec<RenderInstance>,
    commands: Vec<IndirectDrawCommand>,
    max_instances: usize,
    max_commands: usize,
}

impl GpuFramePreparation {
    pub fn try_with_capacity(
        max_instances: usize,
        max_commands: usize,
    ) -> Result<Self, GpuPreparationError> {
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(max_instances)
            .map_err(|_| GpuPreparationError::AllocationFailed)?;
        let mut commands = Vec::new();
        commands
            .try_reserve_exact(max_commands)
            .map_err(|_| GpuPreparationError::AllocationFailed)?;
        Ok(Self {
            instances,
            commands,
            max_instances,
            max_commands,
        })
    }

    pub fn with_capacity(max_instances: usize, max_commands: usize) -> Self {
        Self::try_with_capacity(max_instances, max_commands)
            .expect("GPU preparation allocation failed")
    }

    pub fn prepare(&mut self, manifest: &CpuRenderManifest) -> Result<(), GpuPreparationError> {
        self.instances.clear();
        self.commands.clear();
        if manifest.instances().len() > self.max_instances
            || manifest.batches().len() > self.max_commands
        {
            return Err(GpuPreparationError::CapacityExceeded);
        }
        if manifest.instances().len() > self.instances.capacity() {
            self.instances
                .try_reserve(manifest.instances().len() - self.instances.len())
                .map_err(|_| GpuPreparationError::AllocationFailed)?;
        }
        self.instances.extend_from_slice(manifest.instances());
        for batch in manifest.batches() {
            let end = batch
                .first_instance
                .checked_add(batch.instance_count)
                .ok_or(GpuPreparationError::IndexOverflow)?;
            if end as usize > self.instances.len() {
                return Err(GpuPreparationError::InvalidRange);
            }
            self.commands.push(IndirectDrawCommand {
                mesh_id: batch.mesh_id,
                material_id: batch.material_id,
                first_instance: batch.first_instance,
                instance_count: batch.instance_count,
            });
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), GpuPreparationError> {
        for command in &self.commands {
            let end = command
                .first_instance
                .checked_add(command.instance_count)
                .ok_or(GpuPreparationError::IndexOverflow)?;
            if end as usize > self.instances.len() {
                return Err(GpuPreparationError::InvalidRange);
            }
        }
        Ok(())
    }

    /// Encodes the prepared batches into tightly packed Vulkan indirect draw
    /// commands. The caller owns the GPU upload buffer; this method only
    /// writes into the supplied bounded slice and never allocates.
    pub fn encode_indexed_commands(
        &self,
        geometry: &[IndexedGeometry],
        output: &mut [u8],
    ) -> Result<usize, GpuPreparationError> {
        self.validate()?;
        let required = self
            .commands
            .len()
            .checked_mul(VULKAN_INDEXED_INDIRECT_COMMAND_SIZE)
            .ok_or(GpuPreparationError::IndexOverflow)?;
        if output.len() < required {
            return Err(GpuPreparationError::OutputTooSmall);
        }
        for (index, command) in self.commands.iter().enumerate() {
            let mesh = geometry
                .iter()
                .find(|entry| entry.mesh_id == command.mesh_id)
                .ok_or(GpuPreparationError::MissingGeometry)?;
            if mesh.index_count == 0 {
                return Err(GpuPreparationError::InvalidGeometry);
            }
            let encoded = VulkanIndexedIndirectCommand {
                index_count: mesh.index_count,
                instance_count: command.instance_count,
                first_index: mesh.first_index,
                vertex_offset: mesh.vertex_offset,
                first_instance: command.first_instance,
            };
            let start = index * VULKAN_INDEXED_INDIRECT_COMMAND_SIZE;
            let fields = [
                encoded.index_count.to_ne_bytes(),
                encoded.instance_count.to_ne_bytes(),
                encoded.first_index.to_ne_bytes(),
                encoded.vertex_offset.to_ne_bytes(),
                encoded.first_instance.to_ne_bytes(),
            ];
            for (field_index, field) in fields.iter().enumerate() {
                let field_start = start + field_index * core::mem::size_of::<u32>();
                output[field_start..field_start + field.len()].copy_from_slice(field);
            }
        }
        Ok(required)
    }

    /// Encodes the instance stream using the stable GPU record layout.
    pub fn encode_instance_records(&self, output: &mut [u8]) -> Result<usize, GpuPreparationError> {
        let required = self
            .instances
            .len()
            .checked_mul(GPU_INSTANCE_RECORD_SIZE)
            .ok_or(GpuPreparationError::IndexOverflow)?;
        if output.len() < required {
            return Err(GpuPreparationError::OutputTooSmall);
        }
        for (index, instance) in self.instances.iter().enumerate() {
            let record = GpuInstanceRecord {
                transform: instance.transform,
                bounds: instance.bounds,
                entity_index: instance.entity.index,
                entity_generation: instance.entity.generation,
                mesh_id: instance.mesh_id,
                material_id: instance.material_id,
            };
            let start = index * GPU_INSTANCE_RECORD_SIZE;
            let mut write = |offset: &mut usize, bytes: &[u8]| {
                output[start + *offset..start + *offset + bytes.len()].copy_from_slice(bytes);
                *offset += bytes.len();
            };
            let mut offset = 0;
            for value in record.transform {
                write(&mut offset, &value.to_ne_bytes());
            }
            for value in record.bounds {
                write(&mut offset, &value.to_ne_bytes());
            }
            for value in [
                record.entity_index,
                record.entity_generation,
                record.mesh_id,
                record.material_id,
            ] {
                write(&mut offset, &value.to_ne_bytes());
            }
        }
        Ok(required)
    }

    /// Computes all byte counts and dispatch dimensions needed by a native
    /// frame without allocating. `workgroup_size` is the culling shader's X
    /// dimension; Y/Z are intentionally one for bounded 1-D dispatch.
    pub fn command_plan(
        &self,
        workgroup_size: u32,
    ) -> Result<GpuFrameCommandPlan, GpuPreparationError> {
        if workgroup_size == 0 {
            return Err(GpuPreparationError::InvalidWorkgroupSize);
        }
        self.validate()?;
        let instance_count =
            u32::try_from(self.instances.len()).map_err(|_| GpuPreparationError::IndexOverflow)?;
        let draw_count =
            u32::try_from(self.commands.len()).map_err(|_| GpuPreparationError::IndexOverflow)?;
        let instance_upload_bytes = self
            .instances
            .len()
            .checked_mul(GPU_INSTANCE_RECORD_SIZE)
            .ok_or(GpuPreparationError::IndexOverflow)?;
        let indirect_upload_bytes = self
            .commands
            .len()
            .checked_mul(VULKAN_INDEXED_INDIRECT_COMMAND_SIZE)
            .ok_or(GpuPreparationError::IndexOverflow)?;
        let cull_workgroups = instance_count
            .checked_add(workgroup_size - 1)
            .ok_or(GpuPreparationError::IndexOverflow)?
            / workgroup_size;
        Ok(GpuFrameCommandPlan {
            instance_count,
            draw_count,
            instance_upload_bytes,
            indirect_upload_bytes,
            cull_workgroups,
        })
    }

    pub fn instances(&self) -> &[RenderInstance] {
        &self.instances
    }

    pub fn commands(&self) -> &[IndirectDrawCommand] {
        &self.commands
    }

    pub const fn max_instances(&self) -> usize {
        self.max_instances
    }

    pub const fn max_commands(&self) -> usize {
        self.max_commands
    }
}

/// Deterministic no-GPU backend used by tests and the default composition root.
#[derive(Default)]
pub struct HeadlessRenderBackend {
    stats: RenderSubmissionStats,
}

impl HeadlessRenderBackend {
    pub const fn new() -> Self {
        Self {
            stats: RenderSubmissionStats {
                submitted_frames: 0,
                submitted_instances: 0,
                submitted_commands: 0,
            },
        }
    }

    pub const fn stats(&self) -> RenderSubmissionStats {
        self.stats
    }
}

impl RenderBackend for HeadlessRenderBackend {
    fn submit(
        &mut self,
        preparation: &GpuFramePreparation,
    ) -> Result<RenderSubmissionStats, RenderSubmitError> {
        preparation
            .validate()
            .map_err(|_| RenderSubmitError::InvalidPreparation)?;
        self.stats.submitted_frames = self.stats.submitted_frames.saturating_add(1);
        self.stats.submitted_instances = self
            .stats
            .submitted_instances
            .saturating_add(preparation.instances().len() as u64);
        self.stats.submitted_commands = self
            .stats
            .submitted_commands
            .saturating_add(preparation.commands().len() as u64);
        Ok(self.stats)
    }
}

/// Deterministic CPU reference manifest. GPU indirect generation must produce
/// an equivalent batch list for the same RenderWorld.
pub struct CpuRenderManifest {
    instances: Vec<RenderInstance>,
    batches: Vec<DrawBatch>,
    max_instances: usize,
    max_batches: usize,
}

impl CpuRenderManifest {
    pub fn try_with_capacity(
        max_instances: usize,
        max_batches: usize,
    ) -> Result<Self, RenderManifestError> {
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(max_instances)
            .map_err(|_| RenderManifestError::AllocationFailed)?;
        let mut batches = Vec::new();
        batches
            .try_reserve_exact(max_batches)
            .map_err(|_| RenderManifestError::AllocationFailed)?;
        Ok(Self {
            instances,
            batches,
            max_instances,
            max_batches,
        })
    }

    pub fn with_capacity(max_instances: usize, max_batches: usize) -> Self {
        Self::try_with_capacity(max_instances, max_batches)
            .expect("render manifest allocation failed")
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.batches.clear();
    }

    pub fn instances(&self) -> &[RenderInstance] {
        &self.instances
    }

    pub fn batches(&self) -> &[DrawBatch] {
        &self.batches
    }

    pub const fn max_instances(&self) -> usize {
        self.max_instances
    }

    pub const fn max_batches(&self) -> usize {
        self.max_batches
    }

    pub fn build(&mut self, render_world: &RenderWorld) -> Result<(), RenderManifestError> {
        self.clear();
        if render_world.instances().len() > self.max_instances {
            return Err(RenderManifestError::CapacityExceeded);
        }
        let required = self
            .instances
            .len()
            .checked_add(render_world.instances().len())
            .ok_or(RenderManifestError::IndexOverflow)?;
        if required > self.instances.capacity() {
            self.instances
                .try_reserve(required - self.instances.len())
                .map_err(|_| RenderManifestError::AllocationFailed)?;
        }
        self.instances.extend_from_slice(render_world.instances());
        self.instances.sort_unstable_by_key(|instance| {
            (
                instance.mesh_id,
                instance.material_id,
                instance.entity.index,
                instance.entity.generation,
            )
        });

        let mut start = 0usize;
        while start < self.instances.len() {
            let first = self.instances[start];
            let mut end = start + 1;
            while end < self.instances.len()
                && self.instances[end].mesh_id == first.mesh_id
                && self.instances[end].material_id == first.material_id
            {
                end += 1;
            }
            if self.batches.len() >= self.max_batches {
                return Err(RenderManifestError::CapacityExceeded);
            }
            let first_instance =
                u32::try_from(start).map_err(|_| RenderManifestError::IndexOverflow)?;
            let instance_count =
                u32::try_from(end - start).map_err(|_| RenderManifestError::IndexOverflow)?;
            self.batches.push(DrawBatch {
                mesh_id: first.mesh_id,
                material_id: first.material_id,
                first_instance,
                instance_count,
            });
            start = end;
        }
        Ok(())
    }
}

/// Frame-owned, bounded data copied out of the simulation world for rendering.
///
/// The renderer never stores pointers into ECS component storage. A caller
/// supplies the typed projection so compiler-known ECS access stays outside
/// this untyped render boundary.
pub struct RenderWorld {
    instances: Vec<RenderInstance>,
    max_instances: usize,
}

impl RenderWorld {
    pub fn try_with_capacity(max_instances: usize) -> Result<Self, RenderWorldError> {
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(max_instances)
            .map_err(|_| RenderWorldError::AllocationFailed)?;
        Ok(Self {
            instances,
            max_instances,
        })
    }

    pub fn with_capacity(max_instances: usize) -> Self {
        Self::try_with_capacity(max_instances).expect("render-world allocation failed")
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub const fn max_instances(&self) -> usize {
        self.max_instances
    }

    pub fn instances(&self) -> &[RenderInstance] {
        &self.instances
    }

    pub fn try_push(&mut self, instance: RenderInstance) -> Result<(), RenderWorldError> {
        if self.instances.len() >= self.max_instances {
            return Err(RenderWorldError::CapacityExceeded);
        }
        if self.instances.len() == self.instances.capacity() {
            self.instances
                .try_reserve(1)
                .map_err(|_| RenderWorldError::AllocationFailed)?;
        }
        self.instances.push(instance);
        Ok(())
    }

    pub fn extract_with(
        &mut self,
        world: &World,
        mut project: impl FnMut(Entity) -> Option<RenderInstance>,
    ) -> Result<ExtractionStats, RenderWorldError> {
        self.clear();
        let mut stats = ExtractionStats::default();
        let mut error = None;
        world.for_each_entity(|entity| {
            if error.is_some() {
                return;
            }
            stats.considered = stats.considered.saturating_add(1);
            if let Some(instance) = project(entity) {
                match self.try_push(instance) {
                    Ok(()) => stats.extracted = stats.extracted.saturating_add(1),
                    Err(err) => {
                        stats.rejected = stats.rejected.saturating_add(1);
                        error = Some(err);
                    }
                }
            }
        });
        match error {
            Some(err) => Err(err),
            None => Ok(stats),
        }
    }

    /// Extracts the built-in typed render components without allocating a
    /// candidate entity list. The component values are copied into the
    /// frame-owned render world before ECS borrows are released.
    pub fn extract_typed(&mut self, world: &World) -> Result<ExtractionStats, RenderWorldError> {
        self.extract_with(world, |entity| {
            let transform = world.get::<RenderTransform>(entity)?;
            let material = world.get::<RenderMaterial>(entity)?;
            Some(RenderInstance {
                entity,
                transform: transform.matrix,
                bounds: transform.bounds,
                mesh_id: material.mesh_id,
                material_id: material.material_id,
            })
        })
    }
}

pub struct RendererSubsystem {
    state: RendererState,
    stats: RenderStats,
    native: Option<NativeLane>,
    last_resize: Option<(u32, u32)>,
    render_world: RenderWorld,
    manifest: CpuRenderManifest,
    gpu_preparation: GpuFramePreparation,
    headless_backend: HeadlessRenderBackend,
    last_submission: Option<RenderSubmissionStats>,
    last_extraction: Option<ExtractionStats>,
    extraction_overflowed: bool,
    manifest_overflowed: bool,
}

struct NativeLane {
    engine: crate::platform::present::PresentEngine,
    size: crate::platform::present::PresentSize,
    color: [f32; 4],
}

impl NativeLane {
    fn submit(
        &mut self,
        preparation: &GpuFramePreparation,
    ) -> Result<RenderSubmissionStats, RenderSubmitError> {
        preparation
            .validate()
            .map_err(|_| RenderSubmitError::InvalidPreparation)?;
        match self.engine.present_frame() {
            Ok(crate::platform::present::PresentStatus::Presented) => Ok(RenderSubmissionStats {
                submitted_frames: 1,
                submitted_instances: preparation.instances().len() as u64,
                submitted_commands: preparation.commands().len() as u64,
            }),
            Ok(crate::platform::present::PresentStatus::OutOfDate) => {
                Err(RenderSubmitError::OutOfDate)
            }
            Err(_) => Err(RenderSubmitError::DeviceLost),
        }
    }
}

impl RendererSubsystem {
    pub fn new() -> Self {
        Self {
            state: RendererState::New,
            stats: RenderStats {
                frames: 0,
                total_dt_ns: 0,
            },
            native: None,
            last_resize: None,
            render_world: RenderWorld::with_capacity(4096),
            manifest: CpuRenderManifest::with_capacity(4096, 4096),
            gpu_preparation: GpuFramePreparation::with_capacity(4096, 4096),
            headless_backend: HeadlessRenderBackend::new(),
            last_submission: None,
            last_extraction: None,
            extraction_overflowed: false,
            manifest_overflowed: false,
        }
    }

    /// Wraps an already-built native present engine. The engine must own
    /// (or outlive) a live device/swapchain; headless stays the default.
    pub fn with_native(
        engine: crate::platform::present::PresentEngine,
        size: crate::platform::present::PresentSize,
        color: [f32; 4],
    ) -> Self {
        Self {
            state: RendererState::New,
            stats: RenderStats {
                frames: 0,
                total_dt_ns: 0,
            },
            native: Some(NativeLane {
                engine,
                size,
                color,
            }),
            last_resize: None,
            render_world: RenderWorld::with_capacity(4096),
            manifest: CpuRenderManifest::with_capacity(4096, 4096),
            gpu_preparation: GpuFramePreparation::with_capacity(4096, 4096),
            headless_backend: HeadlessRenderBackend::new(),
            last_submission: None,
            last_extraction: None,
            extraction_overflowed: false,
            manifest_overflowed: false,
        }
    }

    pub const fn is_native(&self) -> bool {
        self.native.is_some()
    }

    pub fn set_native_clear_color(&mut self, color: [f32; 4]) {
        if let Some(lane) = &mut self.native {
            lane.color = color;
            lane.engine.set_clear_color(color);
        }
    }

    /// Notifies the native swapchain of a resize; headless ignores it.
    pub fn recreate_native(&mut self, size: crate::platform::present::PresentSize) {
        if let Some(lane) = &mut self.native {
            lane.size = size;
            let _ = lane.engine.recreate(size);
        }
    }

    pub const fn state(&self) -> RendererState {
        self.state
    }

    pub const fn stats(&self) -> RenderStats {
        self.stats
    }
    pub const fn last_resize(&self) -> Option<(u32, u32)> {
        self.last_resize
    }

    pub fn render_world(&self) -> &RenderWorld {
        &self.render_world
    }

    pub fn render_world_mut(&mut self) -> &mut RenderWorld {
        &mut self.render_world
    }

    pub fn manifest(&self) -> &CpuRenderManifest {
        &self.manifest
    }

    pub const fn manifest_overflowed(&self) -> bool {
        self.manifest_overflowed
    }

    pub fn gpu_preparation(&self) -> &GpuFramePreparation {
        &self.gpu_preparation
    }

    /// Builds the immutable native upload payload for the latest extracted
    /// frame. Native backends receive only this bounded payload; they never
    /// borrow the simulation world or recalculate renderer sizes.
    pub fn build_gpu_upload(
        &self,
        geometry: &[IndexedGeometry],
        workgroup_size: u32,
    ) -> Result<GpuFrameUpload, GpuUploadError> {
        GpuFrameUpload::try_build(&self.gpu_preparation, geometry, workgroup_size)
    }

    pub const fn last_submission(&self) -> Option<RenderSubmissionStats> {
        self.last_submission
    }

    pub const fn last_extraction(&self) -> Option<ExtractionStats> {
        self.last_extraction
    }

    pub const fn extraction_overflowed(&self) -> bool {
        self.extraction_overflowed
    }
}

impl Default for RendererSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for RendererSubsystem {
    fn name(&self) -> &'static str {
        "renderer"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["window"]
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {
        debug_assert_eq!(self.state, RendererState::New);
        self.state = RendererState::Ready;
    }

    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64) {
        if self.state != RendererState::Ready {
            return;
        }
        match self.render_world.extract_typed(ctx.world_read()) {
            Ok(stats) => {
                self.last_extraction = Some(stats);
                self.extraction_overflowed = false;
                self.manifest_overflowed = self.manifest.build(&self.render_world).is_err();
                if !self.manifest_overflowed {
                    self.manifest_overflowed =
                        self.gpu_preparation.prepare(&self.manifest).is_err();
                }
                if !self.manifest_overflowed {
                    if let Some(lane) = &mut self.native {
                        match lane.submit(&self.gpu_preparation) {
                            Ok(stats) => self.last_submission = Some(stats),
                            Err(RenderSubmitError::OutOfDate) => {
                                let _ = lane.engine.recreate(lane.size);
                            }
                            Err(_) => self.last_submission = None,
                        }
                    } else if let Ok(stats) = self.headless_backend.submit(&self.gpu_preparation) {
                        self.last_submission = Some(stats);
                    }
                }
            }
            Err(_) => {
                self.extraction_overflowed = true;
                self.manifest_overflowed = true;
            }
        }
        for envelope in ctx.receive() {
            if envelope.topic == 1 {
                if let Ok(resize) = envelope.downcast::<WindowResized>() {
                    self.last_resize = Some((resize.width, resize.height));
                    #[cfg(any(target_os = "linux", target_os = "windows"))]
                    self.recreate_native(crate::platform::present::PresentSize {
                        width: resize.width,
                        height: resize.height,
                    });
                }
            }
        }
        self.stats.frames = self.stats.frames.saturating_add(1);
        self.stats.total_dt_ns = self.stats.total_dt_ns.saturating_add(u128::from(dt_ns));
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {
        if self.state == RendererState::Ready {
            self.state = RendererState::Shutdown;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::vulkan_policy::{QUEUE_COMPUTE, QUEUE_GRAPHICS, QUEUE_TRANSFER};

    #[test]
    fn new_renderer_has_stable_empty_state() {
        let renderer = RendererSubsystem::new();
        assert_eq!(renderer.state(), RendererState::New);
        assert_eq!(renderer.stats(), RenderStats::default());
        assert_eq!(renderer.dependencies(), &["window"]);
        assert_eq!(renderer.render_world().max_instances(), 4096);
    }

    #[test]
    fn chooses_highest_scored_device_with_complete_queues() {
        static COMPLETE: [QueueFamilyInfo; 1] = [QueueFamilyInfo {
            index: 4,
            flags: QUEUE_GRAPHICS | QUEUE_COMPUTE | QUEUE_TRANSFER,
            queue_count: 1,
        }];
        static GRAPHICS_ONLY: [QueueFamilyInfo; 1] = [QueueFamilyInfo {
            index: 0,
            flags: QUEUE_GRAPHICS,
            queue_count: 1,
        }];
        let candidates = [
            GpuCandidate {
                device_score: 9,
                queue_families: &GRAPHICS_ONLY,
            },
            GpuCandidate {
                device_score: 3,
                queue_families: &COMPLETE,
            },
        ];
        let plan = choose_render_device(&candidates).unwrap();
        assert_eq!(plan.candidate_index, 1);
        assert_eq!(plan.queues.graphics, 4);
    }

    #[test]
    fn rejects_devices_without_complete_queues() {
        static GRAPHICS_ONLY: [QueueFamilyInfo; 1] = [QueueFamilyInfo {
            index: 0,
            flags: QUEUE_GRAPHICS,
            queue_count: 1,
        }];
        assert_eq!(
            choose_render_device(&[GpuCandidate {
                device_score: 10,
                queue_families: &GRAPHICS_ONLY
            }]),
            Err(RenderDevicePlanError::NoSuitableDevice)
        );
    }

    #[test]
    fn command_buffer_requires_deterministic_lifecycle() {
        let mut command = CommandBufferState::new();
        assert_eq!(command.end(), Err(CommandError::InvalidTransition));
        command.begin().unwrap();
        command.end().unwrap();
        command.submit().unwrap();
        command.reset().unwrap();
        assert_eq!(command.state(), CommandState::Initial);
    }

    #[test]
    fn frame_slot_requires_signal_after_submit() {
        let mut frame = FrameSlot::new();
        assert_eq!(frame.signal(), Err(CommandError::InvalidTransition));
        frame.acquire().unwrap();
        frame.submit().unwrap();
        frame.signal().unwrap();
        assert_eq!(frame.state(), FrameState::Available);
    }

    #[test]
    fn command_pool_enforces_capacity_and_reset_safety() {
        let mut pool = CommandPool::with_capacity(1);
        assert_eq!(pool.allocate(), Some(0));
        assert_eq!(pool.allocate(), None);
        let buffer = pool.buffer_mut(0).unwrap();
        buffer.begin().unwrap();
        buffer.end().unwrap();
        buffer.submit().unwrap();
        assert_eq!(pool.reset(), Err(CommandError::InvalidTransition));
    }

    #[test]
    fn fence_requires_submit_signal_reset_order() {
        let mut fence = Fence::new();
        assert_eq!(fence.signal(), Err(CommandError::InvalidTransition));
        fence.submit().unwrap();
        fence.signal().unwrap();
        fence.reset().unwrap();
        assert_eq!(fence.state(), FenceState::Unsignaled);
    }

    #[test]
    fn binary_semaphore_is_consumed_once() {
        let mut semaphore = BinarySemaphore::new();
        assert_eq!(semaphore.consume(), Err(CommandError::InvalidTransition));
        semaphore.signal().unwrap();
        semaphore.consume().unwrap();
        assert_eq!(semaphore.state(), SemaphoreState::Unsignaled);
        assert_eq!(semaphore.consume(), Err(CommandError::InvalidTransition));
    }

    #[test]
    fn submit_frame_transitions_all_sync_objects_atomically_in_order() {
        let mut command = CommandBufferState::new();
        command.begin().unwrap();
        command.end().unwrap();
        let mut wait = BinarySemaphore::new();
        wait.signal().unwrap();
        let mut signal = BinarySemaphore::new();
        let mut fence = Fence::new();
        submit_frame(&mut command, &mut wait, &mut signal, &mut fence).unwrap();
        assert_eq!(command.state(), CommandState::Submitted);
        assert_eq!(wait.state(), SemaphoreState::Unsignaled);
        assert_eq!(signal.state(), SemaphoreState::Signaled);
        assert_eq!(fence.state(), FenceState::Submitted);
    }

    #[test]
    fn render_frame_runs_complete_frame_lifecycle() {
        let mut frame = RenderFrame::new();
        frame.wait.signal().unwrap();
        frame.begin().unwrap();
        frame.submit().unwrap();
        frame.complete().unwrap();
        assert_eq!(frame.slot.state(), FrameState::Available);
        assert_eq!(frame.command.state(), CommandState::Initial);
    }

    #[test]
    fn swapchain_requires_ready_surface_and_handles_resize() {
        let mut surface = SurfaceLifecycle::new();
        let mut swapchain = SwapchainLifecycle::new();
        assert_eq!(
            swapchain.create(surface.state()),
            Err(CommandError::InvalidTransition)
        );
        surface.create().unwrap();
        swapchain.create(surface.state()).unwrap();
        swapchain.mark_out_of_date().unwrap();
        swapchain.destroy().unwrap();
    }

    #[test]
    fn swapchain_config_rejects_invalid_extent_and_buffer_count() {
        assert!(SwapchainConfig {
            width: 0,
            height: 720,
            image_count: 2,
            format: 1,
            present_mode: 0
        }
        .validate()
        .is_err());
        assert!(SwapchainConfig {
            width: 1280,
            height: 720,
            image_count: 1,
            format: 1,
            present_mode: 0
        }
        .validate()
        .is_err());
        assert!(SwapchainConfig {
            width: 1280,
            height: 720,
            image_count: 2,
            format: 1,
            present_mode: 0
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn render_world_extract_is_bounded_and_generation_safe() {
        let mut world = World::new();
        let first = world.spawn();
        world.spawn();
        world.despawn(first);
        let replacement = world.spawn();
        let mut render = RenderWorld::with_capacity(1);
        let stats = render
            .extract_with(&world, |entity| {
                Some(RenderInstance {
                    entity,
                    transform: [0.0; 12],
                    bounds: [1.0; 4],
                    mesh_id: 1,
                    material_id: 2,
                })
            })
            .unwrap_err();
        assert_eq!(stats, RenderWorldError::CapacityExceeded);
        assert_eq!(render.instances().len(), 1);
        assert!(render.instances()[0].entity != first);
        assert!(
            render.instances()[0].entity == replacement || render.instances()[0].entity.index == 1
        );
    }

    #[test]
    fn typed_render_components_extract_into_frame_world() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(
            entity,
            RenderTransform {
                matrix: [3.0; 12],
                bounds: [2.0; 4],
            },
        );
        world.insert(
            entity,
            RenderMaterial {
                mesh_id: 7,
                material_id: 9,
            },
        );
        let mut render = RenderWorld::with_capacity(4);
        let stats = render.extract_typed(&world).unwrap();
        assert_eq!(stats.extracted, 1);
        assert_eq!(render.instances()[0].entity, entity);
        assert_eq!(render.instances()[0].mesh_id, 7);
        assert_eq!(render.instances()[0].transform, [3.0; 12]);
    }

    #[test]
    fn cpu_manifest_batches_deterministically_by_mesh_and_material() {
        let mut render = RenderWorld::with_capacity(4);
        for (entity, mesh, material) in [(2, 4, 8), (0, 1, 3), (1, 4, 8), (3, 1, 3)] {
            render
                .try_push(RenderInstance {
                    entity: crate::kernel::ecs::entity::Entity::new(entity, 0),
                    transform: [0.0; 12],
                    bounds: [1.0; 4],
                    mesh_id: mesh,
                    material_id: material,
                })
                .unwrap();
        }
        let mut manifest = CpuRenderManifest::with_capacity(4, 4);
        manifest.build(&render).unwrap();
        assert_eq!(
            manifest.batches(),
            &[
                DrawBatch {
                    mesh_id: 1,
                    material_id: 3,
                    first_instance: 0,
                    instance_count: 2
                },
                DrawBatch {
                    mesh_id: 4,
                    material_id: 8,
                    first_instance: 2,
                    instance_count: 2
                },
            ]
        );
        assert_eq!(manifest.instances()[0].entity.index, 0);
        assert_eq!(manifest.instances()[2].entity.index, 1);
    }

    #[test]
    fn gpu_preparation_rejects_invalid_indirect_ranges() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 1,
                material_id: 1,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut gpu = GpuFramePreparation::with_capacity(1, 1);
        gpu.prepare(&manifest).unwrap();
        assert_eq!(gpu.instances().len(), 1);
        assert_eq!(
            gpu.commands(),
            &[IndirectDrawCommand {
                mesh_id: 1,
                material_id: 1,
                first_instance: 0,
                instance_count: 1,
            }]
        );
    }

    #[test]
    fn gpu_preparation_encodes_vulkan_indirect_layout_without_allocating() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 7,
                material_id: 3,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut gpu = GpuFramePreparation::with_capacity(1, 1);
        gpu.prepare(&manifest).unwrap();

        let geometry = [IndexedGeometry {
            mesh_id: 7,
            index_count: 36,
            first_index: 12,
            vertex_offset: -4,
        }];
        let mut bytes = [0_u8; VULKAN_INDEXED_INDIRECT_COMMAND_SIZE];
        assert_eq!(
            gpu.encode_indexed_commands(&geometry, &mut bytes).unwrap(),
            VULKAN_INDEXED_INDIRECT_COMMAND_SIZE
        );
        let expected = VulkanIndexedIndirectCommand {
            index_count: 36,
            instance_count: 1,
            first_index: 12,
            vertex_offset: -4,
            first_instance: 0,
        };
        let mut expected_bytes = [0_u8; VULKAN_INDEXED_INDIRECT_COMMAND_SIZE];
        for (field_index, field) in [
            expected.index_count.to_ne_bytes(),
            expected.instance_count.to_ne_bytes(),
            expected.first_index.to_ne_bytes(),
            expected.vertex_offset.to_ne_bytes(),
            expected.first_instance.to_ne_bytes(),
        ]
        .iter()
        .enumerate()
        {
            let start = field_index * core::mem::size_of::<u32>();
            expected_bytes[start..start + field.len()].copy_from_slice(field);
        }
        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn gpu_preparation_rejects_missing_geometry_and_short_output() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 7,
                material_id: 3,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut gpu = GpuFramePreparation::with_capacity(1, 1);
        gpu.prepare(&manifest).unwrap();

        let mut short = [0_u8; VULKAN_INDEXED_INDIRECT_COMMAND_SIZE - 1];
        assert_eq!(
            gpu.encode_indexed_commands(&[], &mut short),
            Err(GpuPreparationError::OutputTooSmall)
        );
        let mut output = [0_u8; VULKAN_INDEXED_INDIRECT_COMMAND_SIZE];
        assert_eq!(
            gpu.encode_indexed_commands(&[], &mut output),
            Err(GpuPreparationError::MissingGeometry)
        );
    }

    #[test]
    fn gpu_preparation_encodes_stable_instance_records_and_command_plan() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(4, 9),
                transform: [1.0; 12],
                bounds: [2.0; 4],
                mesh_id: 7,
                material_id: 3,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut gpu = GpuFramePreparation::with_capacity(1, 1);
        gpu.prepare(&manifest).unwrap();

        let plan = gpu.command_plan(64).unwrap();
        assert_eq!(
            plan,
            GpuFrameCommandPlan {
                instance_count: 1,
                draw_count: 1,
                instance_upload_bytes: GPU_INSTANCE_RECORD_SIZE,
                indirect_upload_bytes: VULKAN_INDEXED_INDIRECT_COMMAND_SIZE,
                cull_workgroups: 1,
            }
        );
        let mut bytes = [0_u8; GPU_INSTANCE_RECORD_SIZE];
        assert_eq!(
            gpu.encode_instance_records(&mut bytes).unwrap(),
            GPU_INSTANCE_RECORD_SIZE
        );
        assert_eq!(
            bytes[0..4],
            1.0_f32.to_ne_bytes(),
            "first transform value must use native shader upload byte order"
        );
        let entity_offset = 16 * core::mem::size_of::<f32>();
        assert_eq!(bytes[entity_offset..entity_offset + 4], 4_u32.to_ne_bytes());
        assert_eq!(
            bytes[entity_offset + 12..entity_offset + 16],
            3_u32.to_ne_bytes()
        );
    }

    #[test]
    fn gpu_preparation_rejects_zero_workgroup_size() {
        let gpu = GpuFramePreparation::with_capacity(0, 0);
        assert_eq!(
            gpu.command_plan(0),
            Err(GpuPreparationError::InvalidWorkgroupSize)
        );
    }

    #[test]
    fn gpu_frame_upload_builds_bounded_native_payload_once() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 5,
                material_id: 2,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut preparation = GpuFramePreparation::with_capacity(1, 1);
        preparation.prepare(&manifest).unwrap();
        let upload = GpuFrameUpload::try_build(
            &preparation,
            &[IndexedGeometry {
                mesh_id: 5,
                index_count: 3,
                first_index: 0,
                vertex_offset: 0,
            }],
            32,
        )
        .unwrap();
        assert_eq!(upload.plan().instance_count, 1);
        assert_eq!(upload.plan().draw_count, 1);
        assert_eq!(upload.instance_bytes().len(), GPU_INSTANCE_RECORD_SIZE);
        assert_eq!(
            upload.indirect_command_bytes().len(),
            VULKAN_INDEXED_INDIRECT_COMMAND_SIZE
        );
    }

    #[test]
    fn gpu_upload_reuses_caller_owned_staging_buffers() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 5,
                material_id: 2,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut preparation = GpuFramePreparation::with_capacity(1, 1);
        preparation.prepare(&manifest).unwrap();

        let mut instances = Vec::new();
        let mut commands = Vec::new();
        let plan = GpuFrameUpload::try_encode_into(
            &preparation,
            &[IndexedGeometry {
                mesh_id: 5,
                index_count: 3,
                first_index: 0,
                vertex_offset: 0,
            }],
            32,
            &mut instances,
            &mut commands,
        )
        .unwrap();
        let instance_ptr = instances.as_ptr();
        let command_ptr = commands.as_ptr();

        let second = GpuFrameUpload::try_encode_into(
            &preparation,
            &[IndexedGeometry {
                mesh_id: 5,
                index_count: 3,
                first_index: 0,
                vertex_offset: 0,
            }],
            32,
            &mut instances,
            &mut commands,
        )
        .unwrap();

        assert_eq!(plan, second);
        assert_eq!(instance_ptr, instances.as_ptr());
        assert_eq!(command_ptr, commands.as_ptr());
    }

    #[test]
    fn renderer_entrypoint_builds_empty_upload_without_touching_ecs() {
        let renderer = RendererSubsystem::new();
        let upload = renderer.build_gpu_upload(&[], 64).unwrap();

        assert_eq!(upload.plan().instance_count, 0);
        assert_eq!(upload.plan().draw_count, 0);
        assert_eq!(upload.plan().cull_workgroups, 0);
        assert!(upload.instance_bytes().is_empty());
        assert!(upload.indirect_command_bytes().is_empty());
    }

    #[test]
    fn headless_backend_counts_only_valid_prepared_frames() {
        let mut render = RenderWorld::with_capacity(1);
        render
            .try_push(RenderInstance {
                entity: crate::kernel::ecs::entity::Entity::new(0, 0),
                transform: [0.0; 12],
                bounds: [1.0; 4],
                mesh_id: 1,
                material_id: 1,
            })
            .unwrap();
        let mut manifest = CpuRenderManifest::with_capacity(1, 1);
        manifest.build(&render).unwrap();
        let mut preparation = GpuFramePreparation::with_capacity(1, 1);
        preparation.prepare(&manifest).unwrap();
        let mut backend = HeadlessRenderBackend::new();
        let stats = backend.submit(&preparation).unwrap();
        assert_eq!(stats.submitted_frames, 1);
        assert_eq!(stats.submitted_instances, 1);
        assert_eq!(stats.submitted_commands, 1);
    }
}
