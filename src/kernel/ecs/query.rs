//! Closure-based component queries.
//!
//! Iterators would need self-referential borrows into the world, so queries
//! here are *drivers*: they consume a caller-owned scratch index list (the
//! candidate entities, i.e. the intersected has-component bitsets) and call
//! a closure per match with the components already disjointly borrowed.

use crate::kernel::ecs::entity::Entity;
use crate::kernel::ecs::storage::ComponentStorage;

/// Mutable pair of disjoint component storages (index-level split apart at
/// construction, so access is safe without interior mutability).
pub struct WorldPair<'w, A, B> {
    pub(crate) storage_a: &'w mut ComponentStorage<A>,
    pub(crate) storage_b: &'w mut ComponentStorage<B>,
}

impl<'w, A, B> WorldPair<'w, A, B> {
    /// Runs `f` for every entity in `candidates` that still holds both
    /// components, handing it two disjoint mutable views `(&mut A, &mut B)`
    /// for the physics-style "read B to update A" pattern.
    pub fn for_each(self, candidates: &[u32], mut f: impl FnMut(Entity, &mut A, &mut B)) {
        for &idx in candidates {
            let Some(generation) = self.storage_a.generation_at(idx as usize) else {
                continue;
            };
            let entity = Entity::new(idx, generation);
            if self.storage_a.contains(entity) && self.storage_b.contains(entity) {
                // Both present: hand out disjoint views.
                let a = self.storage_a.get_mut(entity).expect("checked present");
                let b = self.storage_b.get_mut(entity).expect("checked present");
                f(entity, a, b);
            }
        }
    }
}

/// Pointer-arity specialization is avoided here — `for_each` re-checks the
/// immutable `contains` cheaply and the hot path is the closure itself.
///
/// Single-component driver (kept for parity with `WorldPair`).
pub struct WorldSingle<'w, T> {
    pub(crate) storage: &'w mut ComponentStorage<T>,
}

impl<'w, T> WorldSingle<'w, T> {
    pub fn for_each(self, candidates: &[u32], mut f: impl FnMut(Entity, &mut T)) {
        for &idx in candidates {
            let Some(generation) = self.storage.generation_at(idx as usize) else {
                continue;
            };
            let entity = Entity::new(idx, generation);
            if self.storage.contains(entity) {
                let value = self.storage.get_mut(entity).expect("checked present");
                f(entity, value);
            }
        }
    }
}

/// Type-shape of the two built-in query drivers.
pub type Query1<'w, T> = WorldSingle<'w, T>;
pub type Query2<'w, A, B> = WorldPair<'w, A, B>;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::WorldPair;
    use crate::kernel::ecs::entity::Entities;
    use crate::kernel::ecs::storage::ComponentStorage;

    #[test]
    fn world_pair_foreach_visits_intersection() {
        let mut ents = Entities::new();
        let a = ents.create();
        let b = ents.create();
        let c = ents.create();

        let mut xs = ComponentStorage::new();
        xs.insert(a, 1u32);
        xs.insert(c, 3u32);
        let mut ys = ComponentStorage::new();
        ys.insert(a, 10u32);
        ys.insert(b, 20u32);

        // Callers build the candidate list themselves (normally the bitset
        // intersection) — here: every entity scheduled at all.
        let mut seen = Vec::new();
        let pair: WorldPair<u32, u32> = WorldPair {
            storage_a: &mut xs,
            storage_b: &mut ys,
        };
        let candidates = [a.index, b.index, c.index];
        pair.for_each(&candidates, |e, x, y| {
            seen.push((e.index, *x, *y));
        });
        // Only `a` holds both components.
        assert_eq!(seen, vec![(0, 1, 10)]);
    }
}
