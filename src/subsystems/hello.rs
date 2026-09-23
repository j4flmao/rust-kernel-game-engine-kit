//! Demo driver: publishes tick pulses and echoes a self-message.
//!
//! This exercises the message bus both addressed (`hello -> tock`) and
//! self-published (allowed: `from == to`), proving the single syscall
//! boundary. Purely a Phase-0/1 proof; real subsystems replace it.
//!
//! Counters live behind `Arc`s so the composition root can read them after
//! the subsystems are moved into the kernel and destroyed there.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::kernel::bus::SubscriberId;
use crate::kernel::{KernelContext, Subsystem};

pub const TOPIC_TICK_PULSE: u16 = 0;
pub const TOPIC_ECHO: u16 = 1;

#[derive(Default)]
pub struct HelloCounters {
    pub ticks: AtomicU64,
    pub pulses_sent: AtomicU64,
    pub echos_received: AtomicU64,
}

impl HelloCounters {
    pub fn summary(&self) -> (u64, u64, u64) {
        (
            self.ticks.load(Ordering::Relaxed),
            self.pulses_sent.load(Ordering::Relaxed),
            self.echos_received.load(Ordering::Relaxed),
        )
    }
}

pub struct Hello {
    counters: Arc<HelloCounters>,
    tock: Option<SubscriberId>,
}

impl Hello {
    pub fn new(counters: Arc<HelloCounters>) -> Self {
        Self {
            counters,
            tock: None,
        }
    }
}

impl Subsystem for Hello {
    fn name(&self) -> &'static str {
        "hello"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn init(&mut self, ctx: &mut KernelContext<'_>) {
        // Resolve the downstream subsystem once, at init (never per tick).
        self.tock = ctx.resolve("tock");
    }

    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        let tick = self.counters.ticks.fetch_add(1, Ordering::Relaxed) + 1;

        // Addressed message: hello -> tock (tock declares hello as a
        // dependency, so the dependency edge legitimizes this send).
        if tick.is_multiple_of(30) {
            if let Some(tock) = self.tock {
                if ctx.publish(tock, TOPIC_TICK_PULSE, tick).is_ok() {
                    self.counters.pulses_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Self-echo demonstrates `from == to` routing without a lock.
        if tick.is_multiple_of(10) && ctx.publish(ctx.self_id(), TOPIC_ECHO, tick).is_ok() {
            for msg in ctx.receive() {
                if msg.topic == TOPIC_ECHO {
                    self.counters.echos_received.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {
        eprintln!(
            "[hello] shutdown after {} ticks",
            self.counters.ticks.load(Ordering::Relaxed)
        );
    }
}
