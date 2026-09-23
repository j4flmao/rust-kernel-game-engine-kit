//! Entities and the entity/process table.
//!
//! An [`Entity`] is `{ index, generation }`: the index is a slot into the
//! table, the generation invalidates stale handles when the slot is
//! recycled. Freed slots go on an internal freelist and are handed back out
//! with a bumped generation — the same mechanism a kernel uses so a stale
//! PID never refers to the wrong process.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityError {
    IndexOverflow,
    AllocationFailed,
}

impl Entity {
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: u32::MAX,
    };

    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn invalid() -> Self {
        Self::INVALID
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }
}

/// The entity table proper: per-slot generations plus a free-list.
#[derive(Default)]
pub struct Entities {
    generations: Vec<u32>,
    free: Vec<u32>,
    live: usize,
}

impl Entities {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns an entity. Reuses a freed slot when available, bumping its
    /// generation so any stale handle that referenced it is now invalid.
    pub fn create(&mut self) -> Entity {
        self.try_create().expect("entity allocation failed")
    }

    pub fn try_create(&mut self) -> Result<Entity, EntityError> {
        match self.free.pop() {
            Some(slot) => {
                let generation = self.generations[slot as usize].checked_add(1).unwrap_or(0); // wrap-after-u32-max keeps validity forever
                self.generations[slot as usize] = generation;
                self.live += 1;
                Ok(Entity::new(slot, generation))
            }
            None => {
                if self.generations.len() >= u32::MAX as usize {
                    return Err(EntityError::IndexOverflow);
                }
                let index = self.generations.len() as u32;
                self.generations
                    .try_reserve(1)
                    .map_err(|_| EntityError::AllocationFailed)?;
                self.generations.push(0);
                self.live += 1;
                Ok(Entity::new(index, 0))
            }
        }
    }

    /// Destroys an entity (generation-neutral: just marks the slot free).
    /// Returns `false` when the entity was already dead.
    pub fn destroy(&mut self, entity: Entity) -> bool {
        self.try_destroy(entity).unwrap_or(false)
    }

    pub fn try_destroy(&mut self, entity: Entity) -> Result<bool, EntityError> {
        let slot = entity.index as usize;
        if slot >= self.generations.len() || !self.is_alive(entity) {
            return Ok(false);
        }
        self.free
            .try_reserve(1)
            .map_err(|_| EntityError::AllocationFailed)?;
        self.free.push(slot as u32);
        self.live -= 1;
        Ok(true)
    }

    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        (entity.index as usize) < self.generations.len()
            && self.generations[entity.index as usize] == entity.generation
            && !self.free.contains(&entity.index)
    }

    /// Number of currently-alive entities.
    pub fn live(&self) -> usize {
        self.live
    }

    /// Highest slot index ever allocated (capacity estimate).
    pub fn watermark(&self) -> usize {
        self.generations.len()
    }

    /// Snapshot-backed iteration over live entities: allocates the index
    /// list once per call (init-path usage; a no-allocation iteration is
    /// the `World`-level query's job).
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        let free = self.free.clone();
        self.generations
            .iter()
            .enumerate()
            .filter_map(move |(i, &gen)| {
                if free.contains(&(i as u32)) {
                    None
                } else {
                    Some(Entity::new(i as u32, gen))
                }
            })
    }

    /// Generation of the entity currently occupying a slot, if any.
    pub(crate) fn generation_at(&self, index: usize) -> Option<u32> {
        self.generations.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn generation_bumps_on_recycle() {
        let mut table = Entities::new();
        let a = table.create();
        assert_eq!(a, Entity::new(0, 0));
        let b = table.create();
        assert_eq!(b, Entity::new(1, 0));

        assert!(table.destroy(a));
        let a2 = table.create();
        assert_eq!(a2.index, 0);
        assert_eq!(a2.generation, 1, "generation must bump on reuse");
        assert!(!table.is_alive(a), "stale handle invalid");
        assert!(table.is_alive(a2));
    }

    #[test]
    fn double_destroy_is_noop() {
        let mut table = Entities::new();
        let e = table.create();
        assert!(table.destroy(e));
        assert!(!table.destroy(e));
        assert_eq!(table.live(), 0);
    }

    #[test]
    fn watermark_tracks_allocations() {
        let mut table = Entities::new();
        for _ in 0..10 {
            table.create();
        }
        assert_eq!(table.watermark(), 10);
        assert_eq!(table.live(), 10);
    }
}
