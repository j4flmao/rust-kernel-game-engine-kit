//! Per-site lock/atomic contention tracing.
//!
//! Every hand-rolled spinlock or failed CAS in the bus bumps a [`Site`]
//! counter on the slow path. With the `race_trace` feature the last few
//! contending thread ids are also recorded, so hot spots surface in the
//! frame tracer's report without pulling in a general-purpose sanitizer.

use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "race_trace")]
use std::sync::atomic::AtomicUsize;

/// A named contention accounting point.
#[allow(dead_code)] // metadata consumed starting Phase 6 contention report
pub struct Site {
    pub(crate) module: &'static str,
    pub(crate) label: &'static str,
    pub(crate) contentions: AtomicU64,
    /// Ring of last few contender thread ids (feature `race_trace`).
    #[cfg(feature = "race_trace")]
    contenders: [AtomicU64; 8],
    #[cfg(feature = "race_trace")]
    next: AtomicUsize,
}

impl Site {
    pub const fn new(module: &'static str, label: &'static str) -> Self {
        Self {
            module,
            label,
            contentions: AtomicU64::new(0),
            #[cfg(feature = "race_trace")]
            contenders: [const { AtomicU64::new(0) }; 8],
            #[cfg(feature = "race_trace")]
            next: AtomicUsize::new(0),
        }
    }

    /// Slow-path hit: increment tally; with `race_trace`, remember the
    /// contending thread id.
    pub(crate) fn bump(&self, _thread_id: u64) {
        self.contentions.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "race_trace")]
        {
            let slot = self.next.fetch_add(1, Ordering::Relaxed) % self.contenders.len();
            self.contenders[slot].store(_thread_id, Ordering::Relaxed);
        }
    }
}

/// Records a contention event at `site`.
///
/// Uses a stable hash of the OS-provided thread id so hot sites are
/// attributable without a backtrace capture on the hot path (`Thread::id()`
/// has no stable numeric form yet, only a `Hash` impl).
pub fn record(site: &'static Site) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    site.bump(hasher.finish());
}
