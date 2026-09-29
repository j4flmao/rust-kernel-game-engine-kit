//! Backend-neutral GPU pass contract.
//!
//! Native Linux/Vulkan code maps this validated sequence to command buffers.
//! Keeping the ordering here prevents a backend from issuing an indirect draw
//! before compute visibility writes are made visible to the graphics queue.

use super::renderer::{GpuCullingPlan, GpuPreparationError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPass {
    UploadInstances,
    CullFrustum,
    BuildIndirect,
    DrawIndirect,
    Present,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuAccess {
    HostWrite,
    ComputeRead,
    ComputeWrite,
    IndirectRead,
    ColorWrite,
    PresentRead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuBarrier {
    pub before: GpuAccess,
    pub after: GpuAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuPipelinePlan {
    pub culling: GpuCullingPlan,
    pub passes: [GpuPass; 5],
    pub barriers: [GpuBarrier; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBudget {
    pub target_fps: u32,
    pub frame_ms: f32,
}

impl FrameBudget {
    pub const fn for_fps(target_fps: u32) -> Option<Self> {
        if target_fps == 0 {
            None
        } else {
            Some(Self {
                target_fps,
                frame_ms: 1000.0 / target_fps as f32,
            })
        }
    }

    pub fn accepts(self, measured_frame_ms: f32) -> bool {
        measured_frame_ms.is_finite() && measured_frame_ms <= self.frame_ms
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTiming {
    pub cpu_prepare_ms: f32,
    pub gpu_compute_ms: f32,
    pub gpu_draw_ms: f32,
    pub present_ms: f32,
}

impl FrameTiming {
    pub fn end_to_end_ms(self) -> f32 {
        self.cpu_prepare_ms + self.gpu_compute_ms + self.gpu_draw_ms + self.present_ms
    }

    pub fn fps(self) -> Option<f32> {
        let frame_ms = self.end_to_end_ms();
        if frame_ms.is_finite() && frame_ms > 0.0 {
            Some(1000.0 / frame_ms)
        } else {
            None
        }
    }
}

impl GpuPipelinePlan {
    pub fn try_for_instances(
        instance_count: u32,
        workgroup_size: u32,
        max_instances: u32,
    ) -> Result<Self, GpuPreparationError> {
        let culling = GpuCullingPlan::try_new(instance_count, workgroup_size, max_instances)?;
        Ok(Self {
            culling,
            passes: [
                GpuPass::UploadInstances,
                GpuPass::CullFrustum,
                GpuPass::BuildIndirect,
                GpuPass::DrawIndirect,
                GpuPass::Present,
            ],
            barriers: [
                GpuBarrier {
                    before: GpuAccess::HostWrite,
                    after: GpuAccess::ComputeRead,
                },
                GpuBarrier {
                    before: GpuAccess::ComputeWrite,
                    after: GpuAccess::ComputeRead,
                },
                GpuBarrier {
                    before: GpuAccess::ComputeWrite,
                    after: GpuAccess::IndirectRead,
                },
                GpuBarrier {
                    before: GpuAccess::ColorWrite,
                    after: GpuAccess::PresentRead,
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameBudget, FrameTiming, GpuAccess, GpuPass, GpuPipelinePlan};

    #[test]
    fn plan_has_safe_compute_to_indirect_ordering() {
        let plan = GpuPipelinePlan::try_for_instances(18_000_000, 256, 18_000_000).unwrap();
        assert_eq!(plan.passes[1], GpuPass::CullFrustum);
        assert_eq!(plan.passes[2], GpuPass::BuildIndirect);
        assert_eq!(plan.passes[3], GpuPass::DrawIndirect);
        assert_eq!(plan.barriers[2].before, GpuAccess::ComputeWrite);
        assert_eq!(plan.barriers[2].after, GpuAccess::IndirectRead);
        assert_eq!(plan.culling.workgroup_count, 70_313);
    }

    #[test]
    fn plan_rejects_zero_workgroup_or_over_capacity() {
        assert!(GpuPipelinePlan::try_for_instances(1, 0, 1).is_err());
        assert!(GpuPipelinePlan::try_for_instances(2, 256, 1).is_err());
    }

    #[test]
    fn frame_budget_reports_60_and_120_fps_limits() {
        let sixty = FrameBudget::for_fps(60).unwrap();
        let one_twenty = FrameBudget::for_fps(120).unwrap();
        assert!((sixty.frame_ms - 16.666666).abs() < 0.001);
        assert!((one_twenty.frame_ms - 8.333333).abs() < 0.001);
        assert!(sixty.accepts(16.0));
        assert!(!one_twenty.accepts(9.0));
    }

    #[test]
    fn frame_timing_exposes_end_to_end_fps() {
        let timing = FrameTiming {
            cpu_prepare_ms: 2.0,
            gpu_compute_ms: 3.0,
            gpu_draw_ms: 5.0,
            present_ms: 0.0,
        };
        assert_eq!(timing.end_to_end_ms(), 10.0);
        assert_eq!(timing.fps(), Some(100.0));
    }
}
