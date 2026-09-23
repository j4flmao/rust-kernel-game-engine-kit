//! Hand-rolled ECS — the engine's "process table".
//!
//! Follows `kit/common/03-ecs-design.md`: entities carry a generation
//! counter (kills stale handles exactly like kernel PID reuse), component
//! storage is a sparse-set keyed by entity index with per-slot generation,
//! and queries pre-intersect per-component bitsets before touching storage.

pub mod bitset;
pub mod entity;
pub mod query;
pub mod storage;
pub mod world;

pub use bitset::{Bitset, BitsetError};
pub use entity::{Entities, Entity, EntityError};
pub use storage::{ComponentStorage, StorageError};
pub use world::{ComponentId, World, WorldError};
