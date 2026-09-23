//! Basic frame tracer.
//!
//! The scheduler wraps every [`crate::kernel::Subsystem::tick`] with
//! [`std::time::Instant`] stamps and records [`TraceSample`]s into a
//! pre-allocated ring — zero allocation and zero locking on the tick path.
//! At `flush_interval` the ring is drained into a compact one-line summary.

use std::time::{Duration, Instant};

use crate::kernel::bus::SpscRing;

#[derive(Debug, Clone, Copy)]
pub struct TraceSample {
    /// Registry index of the subsystem that ran.
    pub subsystem_idx: u32,
    /// 0-based frame counter this sample belongs to.
    pub frame_index: u64,
    /// Actual tick duration in nanoseconds.
    pub duration_ns: u64,
    /// Declared budget for the subsystem in nanoseconds.
    pub budget_ns: u64,
    pub over_budget: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetReport {
    pub samples: u64,
    pub over_budget: u64,
    pub dropped: u64,
}

impl BudgetReport {
    pub const fn within_budget(self) -> bool {
        self.over_budget == 0 && self.dropped == 0
    }
}

pub struct FrameTracer {
    log: SpscRing<TraceSample>,
    dropped: u64,
    flush_interval: Duration,
    last_flush: Instant,
    last_report: BudgetReport,
}

impl FrameTracer {
    pub fn new(sample_capacity: usize, flush_interval: Duration) -> Self {
        Self {
            log: SpscRing::with_capacity(sample_capacity),
            dropped: 0,
            flush_interval,
            last_flush: Instant::now(),
            last_report: BudgetReport::default(),
        }
    }

    /// Default: 4096 samples, flush every second.
    pub fn default_config() -> Self {
        Self::new(4096, Duration::from_secs(1))
    }

    /// Appends one sample. A full ring drops the sample and tallies it —
    /// never spins, never allocates on the tick path.
    pub fn record(&mut self, sample: TraceSample) {
        match self.log.push(sample) {
            Ok(()) => {}
            Err(_) => self.dropped += 1,
        }
    }

    /// Called once per frame. Flushes when the interval has elapsed.
    pub fn frame(&mut self, now: Instant) {
        if now.duration_since(self.last_flush) >= self.flush_interval {
            self.flush(now);
        }
    }

    pub fn last_report(&self) -> BudgetReport {
        self.last_report
    }

    fn flush(&mut self, now: Instant) {
        let mut total = 0u64;
        let mut over = 0u64;
        let mut max_dur = 0u64;
        let mut max_idx = u32::MAX;
        while let Some(s) = self.log.pop() {
            total += 1;
            if s.over_budget {
                over += 1;
            }
            if s.duration_ns > max_dur {
                max_dur = s.duration_ns;
                max_idx = s.subsystem_idx;
            }
        }
        eprintln!(
            "[trace] {}ms window: {} samples, {} over budget, {} dropped, slowest = subsystem#{max_idx} @ {:.2}ms",
            self.flush_interval.as_millis(),
            total,
            over,
            self.dropped,
            max_dur as f64 / 1_000_000.0
        );
        self.last_report = BudgetReport {
            samples: total,
            over_budget: over,
            dropped: self.dropped,
        };
        self.dropped = 0;
        self.last_flush = now;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn frames_flush_at_interval() {
        let mut tracer = FrameTracer::new(16, Duration::from_millis(10));
        let mut now = Instant::now();
        for frame in 0..100 {
            tracer.record(TraceSample {
                subsystem_idx: 0,
                frame_index: frame,
                duration_ns: 1,
                budget_ns: 2,
                over_budget: false,
            });
            now += Duration::from_millis(1);
            tracer.frame(now);
        }
        // Every sample recorded without spinning.
        assert_eq!(tracer.log.len(), 0);
    }

    #[test]
    fn ring_overflow_drops_but_stays_alloc_free() {
        let mut tracer = FrameTracer::new(4, Duration::from_secs(3600));
        for frame in 0..16 {
            tracer.record(TraceSample {
                subsystem_idx: 0,
                frame_index: frame,
                duration_ns: 0,
                budget_ns: 0,
                over_budget: false,
            });
        }
        assert_eq!(tracer.log.len(), 4);
        // Dropped still counted; flush resets it.
        assert!(tracer.dropped > 0);
    }

    #[test]
    fn budget_report_exposes_gate_state_after_flush() {
        let mut tracer = FrameTracer::new(4, Duration::from_millis(1));
        tracer.record(TraceSample {
            subsystem_idx: 0,
            frame_index: 0,
            duration_ns: 5,
            budget_ns: 1,
            over_budget: true,
        });
        let now = Instant::now() + Duration::from_millis(2);
        tracer.frame(now);
        assert_eq!(tracer.last_report().samples, 1);
        assert_eq!(tracer.last_report().over_budget, 1);
        assert!(!tracer.last_report().within_budget());
    }
}
