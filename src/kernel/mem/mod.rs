//! Memory management: arena + pool allocators on raw `std::alloc`.
//!
//! No crate, no global-allocator replacement: these are stacked on the
//! platform allocator (glibc malloc on Linux) at *allocation sites we own*,
//! so hot-path data never goes through `Box`/`Vec` and every byte the
//! engine allocates is auditable.

pub mod arena;
pub mod pool;
pub mod ring_buffer;

pub use arena::BumpArena;
pub use pool::{Pool, PoolError, PoolHandle};
pub use ring_buffer::SpscRing;
