//! The only handle a subsystem receives.
//!
//! Exposes the message bus (publish/drain), the frame delta, and — Phase 2 —
//! the ECS [`World`] plus the per-frame scratch [`BumpArena`]. Never a raw
//! reference to another subsystem.

use core::marker::PhantomData;

use crate::kernel::bus::{BusError, Envelope, MessageBus, SubscriberId};
use crate::kernel::ecs::world::World;
use crate::kernel::mem::arena::BumpArena;

/// Opaque handle handed to [`crate::kernel::Subsystem`] callbacks.
pub struct KernelContext<'a> {
    bus: &'a mut MessageBus,
    /// Registry names, index == `SubscriberId` raw value.
    names: &'a [String],
    world: &'a mut World,
    /// Per-frame bump arena; the kernel rewinds it once per tick.
    arena: &'a mut BumpArena,
    id: SubscriberId,
    dt_ns: u64,
    _pinned: PhantomData<&'a mut MessageBus>,
}

impl<'a> KernelContext<'a> {
    pub(crate) fn new(
        bus: &'a mut MessageBus,
        names: &'a [String],
        id: SubscriberId,
        dt_ns: u64,
        world: &'a mut World,
        arena: &'a mut BumpArena,
    ) -> Self {
        Self {
            bus,
            names,
            world,
            arena,
            id,
            dt_ns,
            _pinned: PhantomData,
        }
    }

    /// Fixed simulation step for the current tick, in nanoseconds.
    pub fn dt_ns(&self) -> u64 {
        self.dt_ns
    }

    /// The caller's own bus address.
    pub fn self_id(&self) -> SubscriberId {
        self.id
    }

    /// Resolves a registered subsystem by name to its bus address.
    pub fn resolve(&self, name: &str) -> Option<SubscriberId> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| SubscriberId::new(i as u32))
    }

    // ---- ECS + scratch access (Phase 2) --------------------------------

    /// Shared read access to the engine world.
    pub fn world_read(&self) -> &World {
        &*self.world
    }

    /// Exclusive write access to the engine world.
    pub fn world_write(&mut self) -> &mut World {
        &mut *self.world
    }

    /// Per-frame scratch bump arena (rewound by the kernel at frame start).
    pub fn arena_mut(&mut self) -> &mut BumpArena {
        &mut *self.arena
    }

    /// Sends an addressed, typed message. Delivered to the target inbox
    /// immediately. Fail-fast: `Err` on a full inbox / unknown recipient.
    pub fn publish<T: Send + 'static>(
        &mut self,
        to: SubscriberId,
        topic: u16,
        payload: T,
    ) -> Result<(), BusError> {
        let env = self.bus.envelope(self.id, to, topic, Box::new(payload));
        self.bus.publish(env)
    }

    /// Drains the caller's own inbox (single-consumer).
    ///
    /// Iteration borrows the bus; do not call `publish` while an iterator from
    /// this method is still alive — drain, then send.
    pub fn receive(&self) -> InboxIter<'_> {
        InboxIter {
            bus: &*self.bus,
            id: self.id,
        }
    }
}

/// Streaming drain of a subscriber's inbox with zero allocation.
pub struct InboxIter<'a> {
    bus: &'a MessageBus,
    id: SubscriberId,
}

impl Iterator for InboxIter<'_> {
    type Item = Envelope;

    fn next(&mut self) -> Option<Envelope> {
        self.bus.pop_inbox(self.id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.bus.inbox_len(self.id)))
    }
}
