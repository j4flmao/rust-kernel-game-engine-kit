//! Sparse-set component storage.
//!
//! Layout: `dense: Vec<T>` holds the actual component values (compact,
//! cache-friendly for iteration), `sparse: Vec<Option<u32>>` maps an entity
//! *index* to its position in `dense`. A per-entity *generation* is stored
//! alongside so that a recycled entity slot never aliases the previous
//! owner's component — mirroring the entity table's guarantees.
//!
//! The doc (`kit/common/03-ecs-design.md`) suggests a `Vec<Option<T>>` first
//! cut; generation-safety forces the sparse-set shape here, since index
//! reuse otherwise aliases stale components.

use std::marker::PhantomData;

use crate::kernel::ecs::bitset::{Bitset, BitsetError};
use crate::kernel::ecs::entity::Entity;

/// Sparse-set storage for one concrete component type `T`.
pub struct ComponentStorage<T> {
    /// Entity index -> position in `dense` (None = absent).
    sparse: Vec<Option<u32>>,
    /// Entity index -> generation at insertion (guards stale handles).
    generations: Vec<u32>,
    dense: Vec<T>,
    /// Which entity indices currently have this component.
    presence: Bitset,
    _marker: PhantomData<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    IndexOverflow,
    AllocationFailed,
}

impl<T> Default for ComponentStorage<T> {
    fn default() -> Self {
        Self {
            sparse: Vec::new(),
            generations: Vec::new(),
            dense: Vec::new(),
            presence: Bitset::new(0),
            _marker: PhantomData,
        }
    }
}

impl<T> ComponentStorage<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_index(&mut self, entity: &Entity) -> Result<(), StorageError> {
        let need = (entity.index as usize)
            .checked_add(1)
            .ok_or(StorageError::IndexOverflow)?;
        if self.sparse.len() < need {
            let additional = need - self.sparse.len();
            self.sparse
                .try_reserve_exact(additional)
                .map_err(|_| StorageError::AllocationFailed)?;
            self.generations
                .try_reserve_exact(additional)
                .map_err(|_| StorageError::AllocationFailed)?;
            self.sparse.resize(need, None);
            self.generations.resize(need, 0);
        }
        Ok(())
    }

    /// Inserts (replaces) a component for `entity`.
    pub fn insert(&mut self, entity: Entity, value: T) {
        self.try_insert(entity, value)
            .expect("component storage allocation failed");
    }

    pub fn try_insert(&mut self, entity: Entity, value: T) -> Result<(), StorageError> {
        self.ensure_index(&entity)?;
        let idx = entity.index as usize;
        self.presence
            .try_reserve(idx)
            .map_err(|error| match error {
                BitsetError::InvalidIndex => StorageError::IndexOverflow,
                BitsetError::AllocationFailed => StorageError::AllocationFailed,
            })?;
        self.generations[idx] = entity.generation;
        match self.sparse[idx] {
            Some(pos) => self.dense[pos as usize] = value,
            None => {
                self.dense
                    .try_reserve(1)
                    .map_err(|_| StorageError::AllocationFailed)?;
                let pos = self.dense.len();
                self.dense.push(value);
                self.sparse[idx] = Some(pos as u32);
            }
        }
        self.presence.set(idx);
        Ok(())
    }

    /// Removes the component, if present and generation-current.
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() || self.generations[idx] != entity.generation {
            return None;
        }
        let pos = self.sparse[idx]? as usize;

        // Swap-remove keeps dense compact: the *last* dense element moves
        // into `pos`; its owner's sparse pointer must follow.
        let len_before = self.dense.len();
        let removed = self.dense.swap_remove(pos);
        if pos != len_before - 1 {
            let moved_from = (len_before - 1) as u32;
            if let Some(moved) = self.sparse.iter().position(|&p| p == Some(moved_from)) {
                self.sparse[moved] = Some(pos as u32);
            }
        }

        self.sparse[idx] = None;
        self.presence.clear(idx);
        Some(removed)
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() || self.generations[idx] != entity.generation {
            return None;
        }
        self.sparse[idx].map(|pos| &self.dense[pos as usize])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let idx = entity.index as usize;
        if idx >= self.sparse.len() || self.generations[idx] != entity.generation {
            return None;
        }
        let pos = self.sparse[idx]? as usize;
        self.dense.get_mut(pos)
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        let idx = entity.index as usize;
        idx < self.sparse.len()
            && self.generations[idx] == entity.generation
            && self.sparse[idx].is_some()
    }

    /// Generation recorded at a table index, if it has one. Used by query
    /// drivers to rebuild a valid `Entity` handle from a candidate index.
    #[inline]
    pub(crate) fn generation_at(&self, idx: usize) -> Option<u32> {
        if idx < self.sparse.len() && self.sparse[idx].is_some() {
            Some(self.generations[idx])
        } else {
            None
        }
    }

    /// The bitset of entity indices holding this component (query fuel).
    pub fn presence(&self) -> &Bitset {
        &self.presence
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.dense.iter()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::kernel::ecs::entity::Entities;

    #[test]
    fn insert_get_remove_with_generations() {
        let mut ents = Entities::new();
        let e0 = ents.create();
        let mut st = ComponentStorage::new();
        assert!(!st.contains(e0));

        st.insert(e0, 42u32);
        assert!(st.contains(e0));
        assert_eq!(*st.get(e0).unwrap(), 42);

        // Recycle slot, fresh generation must not see the old component.
        assert!(ents.destroy(e0));
        let e0b = ents.create();
        assert_ne!(e0b.generation, e0.generation);
        assert!(!st.contains(e0b), "stale generation must hide component");

        st.insert(e0b, 7u32);
        assert_eq!(*st.get(e0b).unwrap(), 7);
        assert!(st.get(e0).is_none());

        assert_eq!(st.remove(e0b), Some(7));
        assert!(!st.contains(e0b));
    }

    #[test]
    fn swap_remove_keeps_dense_pointers_consistent() {
        let mut ents = Entities::new();
        let a = ents.create();
        let b = ents.create();
        let c = ents.create();
        let mut st = ComponentStorage::new();
        st.insert(a, 10u32);
        st.insert(b, 20u32);
        st.insert(c, 30u32);

        assert_eq!(st.remove(a), Some(10));
        // b and c must still be reachable after the swap-remove.
        assert_eq!(*st.get(b).unwrap(), 20);
        assert_eq!(*st.get(c).unwrap(), 30);
        assert_eq!(st.len(), 2);
    }

    #[test]
    fn presence_bitset_tracks_membership() {
        let mut ents = Entities::new();
        let mut st = ComponentStorage::new();
        for _ in 0..5 {
            let e = ents.create();
            st.insert(e, e.index);
        }
        let gathered: Vec<u32> = ents
            .iter()
            .filter(|e| st.contains(*e))
            .map(|e| *st.get(e).unwrap())
            .collect();
        assert_eq!(gathered, vec![0, 1, 2, 3, 4]);
    }
}
