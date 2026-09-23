//! Demo consumer: depends on `hello`, receives its tick pulses.
//!
//! Declared dependency on `hello` makes this subsystem tick *after* hello,
//! which is also exactly the edge the message bus uses to authorize the
//! `hello -> tock` send in debug builds.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::kernel::bus::Envelope;
use crate::kernel::{KernelContext, Subsystem};

use super::hello::TOPIC_TICK_PULSE;

#[derive(Default)]
pub struct TockCounters {
    pub pulses: AtomicU64,
    pub last_seq: AtomicU64,
}

impl TockCounters {
    pub fn summary(&self) -> (u64, u64) {
        (
            self.pulses.load(Ordering::Relaxed),
            self.last_seq.load(Ordering::Relaxed),
        )
    }
}

pub struct Tock {
    counters: Arc<TockCounters>,
}

impl Tock {
    pub fn new(counters: Arc<TockCounters>) -> Self {
        Self { counters }
    }
}

impl Subsystem for Tock {
    fn name(&self) -> &'static str {
        "tock"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["hello"]
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}

    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        for msg in ctx.receive() {
            self.consume(msg);
        }
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {
        eprintln!(
            "[tock] shutdown after {} pulses",
            self.counters.pulses.load(Ordering::Relaxed)
        );
    }
}

impl Tock {
    fn consume(&mut self, msg: Envelope) {
        if msg.topic != TOPIC_TICK_PULSE {
            return;
        }
        let seq = msg.seq;
        let prev = self.counters.last_seq.fetch_max(seq, Ordering::Relaxed);
        // Sequence numbers arrive strictly increasing from one producer.
        debug_assert!(seq > prev || prev == 0);
        self.counters.pulses.fetch_add(1, Ordering::Relaxed);
    }
}
