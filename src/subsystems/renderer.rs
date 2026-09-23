//! Headless renderer lifecycle and frame accounting.
//!
//! The renderer owns no OS or Vulkan handles yet.  This deliberately small
//! subsystem freezes the renderer/kernel contract before native device and
//! swapchain code is added: initialization is explicit, ticks are allocation
//! free, and shutdown is observable.

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

pub struct RendererSubsystem {
    state: RendererState,
    stats: RenderStats,
    native: Option<NativeLane>,
    last_resize: Option<(u32, u32)>,
}

struct NativeLane {
    engine: crate::platform::present::PresentEngine,
    size: crate::platform::present::PresentSize,
    color: [f32; 4],
}

impl RendererSubsystem {
    pub const fn new() -> Self {
        Self {
            state: RendererState::New,
            stats: RenderStats {
                frames: 0,
                total_dt_ns: 0,
            },
            native: None,
            last_resize: None,
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
        if let Some(lane) = &mut self.native {
            match lane.engine.present_frame() {
                Ok(crate::platform::present::PresentStatus::Presented) => {}
                Ok(crate::platform::present::PresentStatus::OutOfDate) => {
                    let _ = lane.engine.recreate(lane.size);
                }
                Err(_) => {}
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
}
