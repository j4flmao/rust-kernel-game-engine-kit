//! The world: owner of the entity table, all component storages, and the
//! component-type registry.
//!
//! Component *types* resolve to a dense [`ComponentId`] at registration;
//! storages live in `slots: Vec<Option<Box<dyn Any + Send>>>`, so borrowing two
//! different components at once is a split-index access — no interior
//! unsafety. Query iteration is closure-based (no self-referential
//! iterators); the candidate entity list is caller-owned scratch so the hot
//! tick path never allocates.

use std::any::Any;
use std::collections::HashMap;

use crate::kernel::ecs::bitset::Bitset;
use crate::kernel::ecs::entity::{Entities, Entity, EntityError};
use crate::kernel::ecs::query::{WorldPair, WorldSingle};
use crate::kernel::ecs::storage::{ComponentStorage, StorageError};

/// Dense identifier for a registered component type. World-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldError {
    Entity(EntityError),
    Storage(StorageError),
}

impl ComponentId {
    const fn new(raw: u32) -> Self {
        Self(raw)
    }
}

pub struct World {
    entities: Entities,
    /// Dense slot per component id. Mirrored by `by_type`.
    slots: Vec<Option<Box<dyn Any + Send>>>,
    by_type: HashMap<std::any::TypeId, usize>,
    /// Which slots are currently alive (for entity iteration).
    alive: Bitset,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: Entities::new(),
            slots: Vec::new(),
            by_type: HashMap::new(),
            alive: Bitset::new(0),
        }
    }

    // ---- Entity table -------------------------------------------------

    pub fn spawn(&mut self) -> Entity {
        self.try_spawn().expect("world entity allocation failed")
    }

    pub fn try_spawn(&mut self) -> Result<Entity, WorldError> {
        self.alive
            .try_reserve(self.entities.watermark().saturating_add(1))
            .map_err(|error| match error {
                crate::kernel::ecs::bitset::BitsetError::InvalidIndex => {
                    WorldError::Entity(EntityError::IndexOverflow)
                }
                crate::kernel::ecs::bitset::BitsetError::AllocationFailed => {
                    WorldError::Entity(EntityError::AllocationFailed)
                }
            })?;
        let entity = self.entities.try_create().map_err(WorldError::Entity)?;
        self.alive.set(entity.index as usize);
        Ok(entity)
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.try_despawn(entity).unwrap_or(false)
    }

    pub fn try_despawn(&mut self, entity: Entity) -> Result<bool, WorldError> {
        if self
            .entities
            .try_destroy(entity)
            .map_err(WorldError::Entity)?
        {
            self.alive.clear(entity.index as usize);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.live()
    }

    /// Iterates live entities (allocation-free; uses the spawn/despawn
    /// bitset, not a snapshot list).
    pub fn for_each_entity(&self, mut f: impl FnMut(Entity)) {
        let words = self.alive.words();
        for (w, &word) in words.iter().enumerate() {
            let mut mask = word;
            while mask != 0 {
                let bit = mask.trailing_zeros() as usize;
                mask &= mask - 1;
                let index = w * 64 + bit;
                if let Some(gen) = self.entities.generation_at(index) {
                    f(Entity::new(index as u32, gen));
                }
            }
        }
    }

    // ---- Component registry -------------------------------------------

    fn slot_index<T: 'static>(&self) -> Option<usize> {
        self.by_type.get(&std::any::TypeId::of::<T>()).copied()
    }

    /// Registers (or re-serves) a component type, returning its id.
    pub fn register_component<T: Send + 'static>(&mut self) -> ComponentId {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(&i) = self.by_type.get(&type_id) {
            return ComponentId::new(i as u32);
        }
        let i = self.slots.len();
        self.slots
            .push(Some(Box::new(ComponentStorage::<T>::new())));
        self.by_type.insert(type_id, i);
        ComponentId::new(i as u32)
    }

    pub fn component_id<T: Send + 'static>(&self) -> Option<ComponentId> {
        self.slot_index::<T>().map(|i| ComponentId::new(i as u32))
    }

    // ---- Accessors ------------------------------------------------------

    /// Inserts (or replaces) a component; registers the type on first use.
    pub fn insert<T: Send + 'static>(&mut self, entity: Entity, value: T) {
        self.try_insert(entity, value)
            .expect("world component allocation failed");
    }

    pub fn try_insert<T: Send + 'static>(
        &mut self,
        entity: Entity,
        value: T,
    ) -> Result<(), WorldError> {
        let id = self.register_component::<T>();
        self.try_insert_typed(id, entity, value)
    }

    /// Inserts a component using a cached component id (avoid re-registry).
    pub fn insert_typed<T: Send + 'static>(&mut self, _id: ComponentId, entity: Entity, value: T) {
        self.try_insert_typed(_id, entity, value)
            .expect("world component allocation failed");
    }

    pub fn try_insert_typed<T: Send + 'static>(
        &mut self,
        _id: ComponentId,
        entity: Entity,
        value: T,
    ) -> Result<(), WorldError> {
        // The id is only a hint; the type-keyed slot is authoritative.
        let idx = self.slot_index::<T>();
        if let Some(i) = idx {
            if let Some(boxed) = self.slots[i].as_mut() {
                if let Some(storage) = boxed.downcast_mut::<ComponentStorage<T>>() {
                    storage
                        .try_insert(entity, value)
                        .map_err(WorldError::Storage)?;
                }
            }
        }
        Ok(())
    }

    pub fn get<T: Send + 'static>(&self, entity: Entity) -> Option<&T> {
        let i = self.slot_index::<T>()?;
        self.slots[i]
            .as_ref()
            .and_then(|boxed| boxed.downcast_ref::<ComponentStorage<T>>())
            .and_then(|storage| storage.get(entity))
    }

    pub fn get_mut<T: Send + 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let i = self.slot_index::<T>()?;
        self.slots[i]
            .as_mut()
            .and_then(|boxed| boxed.downcast_mut::<ComponentStorage<T>>())
            .and_then(|storage| storage.get_mut(entity))
    }

    pub fn remove<T: Send + 'static>(&mut self, entity: Entity) -> Option<T> {
        let i = self.slot_index::<T>()?;
        self.slots[i]
            .as_mut()
            .and_then(|boxed| boxed.downcast_mut::<ComponentStorage<T>>())
            .and_then(|storage| storage.remove(entity))
    }

    /// Number of entities currently holding component `T`.
    pub fn count<T: Send + 'static>(&self) -> usize {
        let i = match self.slot_index::<T>() {
            Some(i) => i,
            None => return 0,
        };
        self.slots[i]
            .as_ref()
            .and_then(|boxed| boxed.downcast_ref::<ComponentStorage<T>>())
            .map_or(0, |storage| storage.len())
    }

    // ---- Queries ------------------------------------------------------

    /// Collects the entity indices that hold *both* A and B into `out`
    /// (cleared first). Allocation-free after `out` has warmed its
    /// capacity; `out` is caller-owned scratch.
    pub fn collect_intersection<A: Send + 'static, B: Send + 'static>(&self, out: &mut Vec<u32>) {
        out.clear();
        let (Some(ia), Some(ib)) = (self.slot_index::<A>(), self.slot_index::<B>()) else {
            return;
        };
        let (Some(a), Some(b)) = (
            self.slots[ia]
                .as_ref()
                .and_then(|s| s.downcast_ref::<ComponentStorage<A>>()),
            self.slots[ib]
                .as_ref()
                .and_then(|s| s.downcast_ref::<ComponentStorage<B>>()),
        ) else {
            return;
        };
        let words_a = a.presence().words();
        let words_b = b.presence().words();
        let n = words_a.len().min(words_b.len());
        for w in 0..n {
            let mut mask = words_a[w] & words_b[w];
            while mask != 0 {
                let bit = mask.trailing_zeros() as usize;
                mask &= mask - 1;
                out.push((w * 64 + bit) as u32);
            }
        }
        // Ordering is not guaranteed stable; sort for reproducibility.
        out.sort_unstable();
    }

    /// Splits the two component storages apart mutably (distinct slot
    /// indices) for the query driver.
    pub fn pair<A: Send + 'static, B: Send + 'static>(&mut self) -> Option<WorldPair<'_, A, B>> {
        let ia = self.slot_index::<A>()?;
        let ib = self.slot_index::<B>()?;
        if ia == ib {
            return None;
        }
        get2_mut(&mut self.slots, ia, ib)
    }

    /// Single-component query driver.
    pub fn single<T: Send + 'static>(&mut self) -> Option<WorldSingle<'_, T>> {
        let i = self.slot_index::<T>()?;
        self.slots[i]
            .as_mut()
            .and_then(|boxed| boxed.downcast_mut::<ComponentStorage<T>>())
            .map(|storage| WorldSingle { storage })
    }
}

/// Disjoint `&mut` to two slots of a Vec<Option<Box<dyn Any + Send>>> via
/// `split_at_mut` — pure safe code, distinct indices.
fn get2_mut<A: 'static, B: 'static>(
    slots: &mut Vec<Option<Box<dyn Any + Send>>>,
    ia: usize,
    ib: usize,
) -> Option<WorldPair<'_, A, B>> {
    if ia == ib {
        return None;
    }
    let (a, b) = if ia < ib {
        let (left, right) = slots.split_at_mut(ib);
        let a = left.get_mut(ia);
        let b = right.first_mut();
        (a, b)
    } else {
        let (left, right) = slots.split_at_mut(ia);
        let b = left.get_mut(ib);
        let a = right.first_mut();
        (a, b)
    };
    let a = a
        .and_then(|slot| slot.as_mut())
        .and_then(|boxed| boxed.downcast_mut::<ComponentStorage<A>>());
    let b = b
        .and_then(|slot| slot.as_mut())
        .and_then(|boxed| boxed.downcast_mut::<ComponentStorage<B>>());
    match (a, b) {
        (Some(a), Some(b)) => Some(WorldPair {
            storage_a: a,
            storage_b: b,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position(f32, f32);
    #[derive(Debug, PartialEq)]
    struct Velocity(f32, f32);

    #[test]
    fn spawn_get_insert_remove_despawn() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
        world.insert(e, 5u32);
        assert_eq!(*world.get::<u32>(e).unwrap(), 5);
        {
            let v = world.get_mut::<u32>(e).unwrap();
            *v = 6;
        }
        assert_eq!(*world.get::<u32>(e).unwrap(), 6);
        assert_eq!(world.remove::<u32>(e), Some(6));
        assert!(world.get::<u32>(e).is_none());
        assert!(world.despawn(e));
        assert!(!world.is_alive(e));
    }

    #[test]
    fn recycled_slot_hides_old_component() {
        let mut world = World::new();
        let e0 = world.spawn();
        world.insert(e0, 9u32);
        world.despawn(e0);
        let e1 = world.spawn();
        assert_eq!(e1.index, e0.index);
        assert_ne!(e1.generation, e0.generation);
        assert!(world.get::<u32>(e1).is_none(), "stale gen hides component");
    }

    #[test]
    fn pair_query_visits_intersection() {
        let mut world = World::new();
        let mut entities = Vec::new();
        for _ in 0..5 {
            entities.push(world.spawn());
        }
        for (i, &e) in entities.iter().enumerate() {
            world.insert(e, Position(i as f32, 0.0));
            if i < 3 {
                world.insert(e, Velocity(1.0, 0.0));
            }
        }

        // Only the first three entities hold Velocity too.
        let mut seen = Vec::new();
        {
            let mut candidates = Vec::new();
            world.collect_intersection::<Position, Velocity>(&mut candidates);
            assert_eq!(candidates, vec![0, 1, 2]);
            let pair = world.pair::<Position, Velocity>().unwrap();
            pair.for_each(&candidates, |e, pos, vel| {
                seen.push((e.index, pos.0, vel.0));
            });
        }
        assert_eq!(seen, vec![(0, 0.0, 1.0), (1, 1.0, 1.0), (2, 2.0, 1.0)]);
    }

    #[test]
    fn single_query_visits_all() {
        let mut world = World::new();
        for i in 0..4u32 {
            let e = world.spawn();
            world.insert(e, i);
        }
        let mut seen = Vec::new();
        {
            let mut out = Vec::new();
            world.collect_intersection::<u32, u32>(&mut out);
            let q = world.single::<u32>().unwrap();
            q.for_each(&out, |_e, v| seen.push(*v));
        }
        assert_eq!(seen, vec![0, 1, 2, 3]);
    }

    #[test]
    fn mutator_pair_updates_through_driver() {
        let mut world = World::new();
        let a = world.spawn();
        let b = world.spawn();
        world.insert(a, Position(0.0, 0.0));
        world.insert(b, Position(10.0, 10.0));
        world.insert(a, Velocity(2.0, 0.0));
        world.insert(b, Velocity(1.0, 1.0));

        let mut candidates = Vec::new();
        world.collect_intersection::<Position, Velocity>(&mut candidates);
        let pair = world.pair::<Position, Velocity>().unwrap();
        pair.for_each(&candidates, |_e, pos, vel| {
            pos.0 += vel.0;
            pos.1 += vel.1;
        });
        assert_eq!(*world.get::<Position>(a).unwrap(), Position(2.0, 0.0));
        assert_eq!(*world.get::<Position>(b).unwrap(), Position(11.0, 11.0));
    }
}
