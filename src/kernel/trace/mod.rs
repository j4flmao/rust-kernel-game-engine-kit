//! Hand-rolled tracing: frame tracer + contention counters. No external
//! profiling crates.

pub mod contention;
pub mod frame_tracer;

pub use frame_tracer::{BudgetReport, FrameTracer, TraceSample};
