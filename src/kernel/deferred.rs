//! Bounded structural ECS commands applied at explicit kernel boundaries.

use core::fmt;

use crate::kernel::ecs::entity::Entity;
use crate::kernel::ecs::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredCommand {
    Spawn,
    Despawn(Entity),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredCommandError {
    CapacityExceeded,
    AllocationFailed,
}

impl fmt::Display for DeferredCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded => f.write_str("deferred command capacity exceeded"),
            Self::AllocationFailed => f.write_str("deferred command allocation failed"),
        }
    }
}

impl core::error::Error for DeferredCommandError {}

/// A frame-bounded command buffer for structural ECS changes.
pub struct DeferredCommands {
    commands: Vec<DeferredCommand>,
    capacity: usize,
}

impl DeferredCommands {
    pub fn try_with_capacity(capacity: usize) -> Result<Self, DeferredCommandError> {
        let mut commands = Vec::new();
        commands
            .try_reserve_exact(capacity)
            .map_err(|_| DeferredCommandError::AllocationFailed)?;
        Ok(Self { commands, capacity })
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("deferred command allocation failed")
    }

    pub fn try_spawn(&mut self) -> Result<(), DeferredCommandError> {
        self.push(DeferredCommand::Spawn)
    }

    pub fn try_despawn(&mut self, entity: Entity) -> Result<(), DeferredCommandError> {
        self.push(DeferredCommand::Despawn(entity))
    }

    fn push(&mut self, command: DeferredCommand) -> Result<(), DeferredCommandError> {
        if self.commands.len() >= self.capacity {
            return Err(DeferredCommandError::CapacityExceeded);
        }
        if self.commands.len() == self.commands.capacity() {
            self.commands
                .try_reserve(1)
                .map_err(|_| DeferredCommandError::AllocationFailed)?;
        }
        self.commands.push(command);
        Ok(())
    }

    pub const fn len(&self) -> usize {
        self.commands.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Applies commands in submission order and empties the buffer.
    pub fn apply(&mut self, world: &mut World) -> usize {
        let mut applied = 0usize;
        for command in self.commands.drain(..) {
            match command {
                DeferredCommand::Spawn => {
                    if world.try_spawn().is_ok() {
                        applied = applied.saturating_add(1);
                    }
                }
                DeferredCommand::Despawn(entity) => {
                    if world.despawn(entity) {
                        applied = applied.saturating_add(1);
                    }
                }
            }
        }
        applied
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_commands_apply_in_order() {
        let mut commands = DeferredCommands::with_capacity(2);
        commands.try_spawn().unwrap();
        commands.try_spawn().unwrap();
        assert_eq!(
            commands.try_spawn(),
            Err(DeferredCommandError::CapacityExceeded)
        );
        let mut world = World::new();
        assert_eq!(commands.apply(&mut world), 2);
        assert_eq!(world.entity_count(), 2);
        assert!(commands.is_empty());
    }

    #[test]
    fn stale_despawn_is_safe_and_does_not_affect_replacement() {
        let mut world = World::new();
        let old = world.spawn();
        world.despawn(old);
        let replacement = world.spawn();
        let mut commands = DeferredCommands::with_capacity(1);
        commands.try_despawn(old).unwrap();
        assert_eq!(commands.apply(&mut world), 0);
        assert!(world.is_alive(replacement));
    }
}
