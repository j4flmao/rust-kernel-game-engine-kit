//! Generic application/game plugin boundary.
//!
//! The engine owns scheduling, input, UI, rendering, ECS, and message routing.
//! A game owns its rules and presentation composition and is hosted through
//! this adapter. The kernel never needs a Sudoku-, chess-, or genre-specific
//! subsystem.

use crate::kernel::{KernelContext, Subsystem, SystemAccess};

/// Application-owned game logic hosted by the generic kernel.
pub trait GamePlugin: Send + 'static {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> &'static [&'static str];

    fn access(&self) -> SystemAccess {
        SystemAccess::exclusive()
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64);
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

/// Turns any application-owned [`GamePlugin`] into a normal kernel subsystem.
pub struct GamePluginHost<P> {
    plugin: P,
}

impl<P> GamePluginHost<P> {
    pub const fn new(plugin: P) -> Self {
        Self { plugin }
    }

    pub fn plugin(&self) -> &P {
        &self.plugin
    }

    pub fn plugin_mut(&mut self) -> &mut P {
        &mut self.plugin
    }
}

impl<P: GamePlugin> Subsystem for GamePluginHost<P> {
    fn name(&self) -> &'static str {
        self.plugin.name()
    }

    fn dependencies(&self) -> &'static [&'static str] {
        self.plugin.dependencies()
    }

    fn access(&self) -> SystemAccess {
        self.plugin.access()
    }

    fn init(&mut self, ctx: &mut KernelContext<'_>) {
        self.plugin.init(ctx);
    }

    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64) {
        self.plugin.tick(ctx, dt_ns);
    }

    fn shutdown(&mut self, ctx: &mut KernelContext<'_>) {
        self.plugin.shutdown(ctx);
    }
}
