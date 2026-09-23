//! Kernel core: subsystem lifecycle, scheduling, message bus, tracing.
//!
//! Microkernel-pattern: this module owns ONLY registry + scheduler + bus.
//! No gameplay, rendering, or platform logic lives here.

mod context;
mod error;
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
pub use self::ecs::world::{ComponentId, World};
pub use self::error::KernelError;
pub use self::mem::{BumpArena, Pool, PoolError, PoolHandle};
pub use self::scheduler::{dependency_waves, FrameLimiter, Kernel, ParallelWaveExecutor};
pub use self::subsystem::Subsystem;
pub use self::sync::{
    HmacSha256Authenticator, Replicator, SyncAuthenticator, SyncConfig, SyncError, SyncKind,
    SyncMode, SyncPacket, SyncStats, SyncTransport, SyncWal, WalError,
};

pub const FIXED_DT_NS: u64 = 16_666_667; // 1/60 s fixed simulation step
pub const DEFAULT_FRAME_BUDGET_NS: u64 = 1_000_000; // 1 ms per-subsystem debug budget
pub const DEFAULT_ARENA_CAPACITY: usize = 1 << 20; // 1 MiB per-frame scratch arena
