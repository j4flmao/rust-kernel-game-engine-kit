//! Kernel core: subsystem lifecycle, scheduling, message bus, tracing.
//!
//! Microkernel-pattern: this module owns ONLY registry + scheduler + bus.
//! No gameplay, rendering, or platform logic lives here.

mod context;
mod deferred;
mod error;
mod game;
mod registry;
mod scheduler;
mod subsystem;
pub mod sync;

pub mod bus;
pub mod ecs;
pub mod mem;
pub mod trace;

pub use self::bus::{BusError, Envelope, RingError, SubscriberId};
pub use self::context::KernelContext;
pub use self::deferred::{DeferredCommand, DeferredCommandError, DeferredCommands};
pub use self::ecs::world::{ComponentId, World};
pub use self::error::KernelError;
pub use self::game::{GamePlugin, GamePluginHost};
pub use self::mem::{BumpArena, Pool, PoolError, PoolHandle};
pub use self::scheduler::{
    access_waves, dependency_waves, FrameLimiter, Kernel, ParallelWaveExecutor, SchedulePlan,
};
pub use self::subsystem::{Subsystem, SystemAccess};
pub use self::sync::{
    HmacSha256Authenticator, Replicator, SyncAuthenticator, SyncConfig, SyncError, SyncKind,
    SyncMode, SyncPacket, SyncStats, SyncTransport, SyncWal, WalError,
};

pub const FIXED_DT_NS: u64 = 16_666_667; // 1/60 s fixed simulation step
pub const DEFAULT_FRAME_BUDGET_NS: u64 = 1_000_000; // 1 ms per-subsystem debug budget
pub const DEFAULT_ARENA_CAPACITY: usize = 1 << 20; // 1 MiB per-frame scratch arena
pub const DEFAULT_DEFERRED_COMMAND_CAPACITY: usize = 4096;
