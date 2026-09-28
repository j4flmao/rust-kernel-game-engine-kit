//! The single trait boundary of the engine.
//!
//! Every subsystem (renderer, physics, audio, input, scripting bridge) is a
//! "driver" behind this trait — the engine's kernel owns only lifecycle and
//! scheduling, never subsystem internals. The trait shape is frozen at
//! Phase 0; changing it later is a migration, not a tweak.

use crate::kernel::KernelContext;

/// Drivers plugged into the kernel.
///
/// Implementors must not reach into another subsystem's state directly:
/// communication happens only through [`KernelContext`] (message bus + clock).
pub trait Subsystem {
    /// Stable, unique name. Also the key used by `dependencies()`.
    fn name(&self) -> &'static str;

    /// Names of subsystems that must `tick()` before this one.
    /// An unmet or cyclic set is a hard startup failure.
    fn dependencies(&self) -> &'static [&'static str];

    /// Declares the logical resources touched by the subsystem.
    ///
    /// The compatibility default is exclusive. This keeps existing subsystem
    /// implementations correct while allowing newer systems to opt into
    /// read/read schedule waves once their context is safe to parallelize.
    fn access(&self) -> SystemAccess {
        SystemAccess::exclusive()
    }

    /// One-time setup. Called in topological order. Allocation allowed here.
    fn init(&mut self, ctx: &mut KernelContext<'_>);

    /// Per-frame work. MUST be allocation-free and deterministic given `dt_ns`.
    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64);

    /// Teardown, called in reverse topological order.
    fn shutdown(&mut self, ctx: &mut KernelContext<'_>);
}

/// Static access declaration used by the schedule planner.
///
/// Keys are logical names, not memory addresses. A write conflicts with a
/// read or write of the same key. Exclusive systems conflict with every other
/// system and are the safe default for legacy subsystem callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemAccess {
    pub reads: &'static [&'static str],
    pub writes: &'static [&'static str],
    pub exclusive: bool,
}

impl SystemAccess {
    pub const fn exclusive() -> Self {
        Self {
            reads: &[],
            writes: &[],
            exclusive: true,
        }
    }

    pub const fn read_only(reads: &'static [&'static str]) -> Self {
        Self {
            reads,
            writes: &[],
            exclusive: false,
        }
    }

    pub const fn read_write(
        reads: &'static [&'static str],
        writes: &'static [&'static str],
    ) -> Self {
        Self {
            reads,
            writes,
            exclusive: false,
        }
    }

    pub(crate) fn conflicts(self, other: Self) -> bool {
        if self.exclusive || other.exclusive {
            return true;
        }
        self.writes
            .iter()
            .any(|key| other.reads.contains(key) || other.writes.contains(key))
            || other.writes.iter().any(|key| self.reads.contains(key))
    }
}
