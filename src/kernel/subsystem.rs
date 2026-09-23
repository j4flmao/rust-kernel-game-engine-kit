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

    /// One-time setup. Called in topological order. Allocation allowed here.
    fn init(&mut self, ctx: &mut KernelContext<'_>);

    /// Per-frame work. MUST be allocation-free and deterministic given `dt_ns`.
    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64);

    /// Teardown, called in reverse topological order.
    fn shutdown(&mut self, ctx: &mut KernelContext<'_>);
}
