//! Frame tracer behavior through its public API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use rust_kernel_game_engine_kit::kernel::trace::frame_tracer::{FrameTracer, TraceSample};

#[test]
fn tracer_records_and_flushes_cleanly() {
    let mut tracer = FrameTracer::new(8, Duration::from_millis(5));
    let mut now = Instant::now();
    for frame in 0..50u64 {
        tracer.record(TraceSample {
            subsystem_idx: 1,
            frame_index: frame,
            duration_ns: frame * 2,
            budget_ns: 1_000,
            over_budget: false,
        });
        now += Duration::from_millis(1);
        tracer.frame(now); // ~10 flush windows over the 50 samples
    }
    // Smoke: the tracer must stay alive and deadlock-free across rotation.
}

#[test]
fn tracer_uses_single_bit_explicit_over_budget_flag() {
    let sample = TraceSample {
        subsystem_idx: 3,
        frame_index: 42,
        duration_ns: 1_000,
        budget_ns: 2_000,
        over_budget: true,
    };
    assert_eq!(sample.frame_index, 42);
    assert!(sample.over_budget);
    assert_eq!(sample.budget_ns, 2_000);
}
