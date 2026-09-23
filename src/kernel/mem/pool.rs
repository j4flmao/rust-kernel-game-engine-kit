//! Fixed-capacity pool allocator over a single raw block.
//!
//! The `slots` block is one `std::alloc::alloc` region sized to
//! `capacity * size_of::<T>()`; a free-slot index list recycles slots in
//! O(1). This is the home for long-lived, churn-heavy objects (particles,
//! pooled state) that would otherwise fragment the global allocator.
//!
//! The memory block itself uses the raw allocator; the free list is a plain
//! `Vec<u32>` of arity metadata that lives alongside the pool (not on the
//! per-object hot path).
#![allow(unsafe_code)] // the raw block is the only side-channel here

use std::alloc::{alloc, dealloc, Layout};
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolError {
    /// All slots are currently in use.
    Exhausted,
    /// `idx` is out of bounds or was never the slot the caller thinks.
    InvalidSlot(u32),
    InvalidCapacity,
    AllocationFailed,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => write!(f, "pool exhausted"),
            Self::InvalidSlot(idx) => write!(f, "invalid pool slot {idx}"),
            Self::InvalidCapacity => write!(f, "invalid pool capacity"),
            Self::AllocationFailed => write!(f, "pool allocation failed"),
        }
    }
}

impl core::error::Error for PoolError {}

/// A fixed-capacity, freelist-driven object pool.
pub struct Pool<T> {
    base: NonNull<MaybeUninit<T>>,
    capacity: usize,
    /// Slot indices available for reuse (free list; LIFO order).
    free: Vec<u32>,
    /// Per-slot generation; guards against use-after-free of stale handles.
    generations: Vec<u32>,
}

/// A live handle into a [`Pool`]. Carries the slot's generation so a stale
/// handle (freed slot reused by someone else) is detected on deref.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolHandle {
    pub(crate) idx: u32,
    pub(crate) generation: u32,
}

impl PoolHandle {
    const INVALID: Self = Self {
        idx: u32::MAX,
        generation: u32::MAX,
    };

    pub const fn invalid() -> Self {
        Self::INVALID
    }

    pub fn is_valid(self) -> bool {
        self != Self::INVALID
    }
}

impl<T> Pool<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("pool allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, PoolError> {
        let capacity = capacity.max(1);
        let layout =
            Layout::array::<MaybeUninit<T>>(capacity).map_err(|_| PoolError::InvalidCapacity)?;
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr).ok_or(PoolError::AllocationFailed)?;
        let base = ptr.cast::<MaybeUninit<T>>();
        let mut free = Vec::new();
        free.try_reserve_exact(capacity).map_err(|_| {
            unsafe { dealloc(base.cast::<u8>().as_ptr(), layout) };
            PoolError::AllocationFailed
        })?;
        for i in 0..capacity {
            free.push(i as u32);
        }
        let mut generations = Vec::new();
        if generations.try_reserve_exact(capacity).is_err() {
            unsafe { dealloc(base.cast::<u8>().as_ptr(), layout) };
            return Err(PoolError::AllocationFailed);
        }
        generations.resize(capacity, 0);
        Ok(Self {
            base,
            capacity,
            free,
            generations,
        })
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn available(&self) -> usize {
        self.free.len()
    }

    /// Occupies a slot with `value`, returning a generation-stamped handle.
    pub fn alloc(&mut self, value: T) -> Result<PoolHandle, PoolError> {
        let slot = self.free.pop().ok_or(PoolError::Exhausted)? as usize;
        // SAFETY: `slot` came from the free list, hence `slot < capacity`
        // and the slot is uninitialized for our use.
        unsafe {
            (*self.base.as_ptr().add(slot)).write(value);
        }
        let generation = self.generations[slot];
        Ok(PoolHandle {
            idx: slot as u32,
            generation,
        })
    }

    /// Releases a slot; the handle is invalid afterwards.
    pub fn free(&mut self, handle: PoolHandle) -> Result<(), PoolError> {
        let slot = handle.idx as usize;
        if slot >= self.capacity {
            return Err(PoolError::InvalidSlot(handle.idx));
        }
        if self.generations[slot] != handle.generation {
            return Err(PoolError::InvalidSlot(handle.idx));
        }
        // SAFETY: slot holds a live value (generation matched).
        unsafe {
            (*self.base.as_ptr().add(slot)).assume_init_drop();
        }
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        self.free.push(handle.idx);
        Ok(())
    }

    pub fn get(&self, handle: PoolHandle) -> Option<&T> {
        let slot = handle.idx as usize;
        if slot >= self.capacity || self.generations[slot] != handle.generation {
            return None;
        }
        // SAFETY: generation matched, slot is live and initialized.
        Some(unsafe { (*self.base.as_ptr().add(slot)).assume_init_ref() })
    }

    pub fn get_mut(&mut self, handle: PoolHandle) -> Option<&mut T> {
        let slot = handle.idx as usize;
        if slot >= self.capacity || self.generations[slot] != handle.generation {
            return None;
        }
        // SAFETY: exclusive &mut access, slot is live.
        Some(unsafe { (*self.base.as_ptr().add(slot)).assume_init_mut() })
    }

    /// Iterates over every currently-stored object.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        (0..self.capacity).filter_map(move |i| {
            if self.free.contains(&(i as u32)) {
                None
            } else {
                // SAFETY: not on the free list ⇒ live and initialized.
                let v = unsafe { (*self.base.as_ptr().add(i)).assume_init_ref() };
                Some((i as u32, v))
            }
        })
    }
}

impl<T> Drop for Pool<T> {
    fn drop(&mut self) {
        // Drop every live slot (no `iter` here — we must not hold shared
        // borrows into the block while also dropping from raw pointers).
        let freelist = self.free.clone();
        for i in 0..self.capacity {
            if freelist.contains(&(i as u32)) {
                continue;
            }
            // SAFETY: not on the free list ⇒ live and initialized.
            unsafe {
                (*self.base.as_ptr().add(i)).assume_init_drop();
            }
        }
        // SAFETY: layout matches with_capacity.
        let layout = Layout::array::<MaybeUninit<T>>(self.capacity).unwrap();
        unsafe { dealloc(self.base.as_ptr().cast::<u8>(), layout) };
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn pool_alloc_free_recycles_with_generation_guard() {
        let mut pool = Pool::<u64>::with_capacity(4);
        let a = pool.alloc(1).unwrap();
        let b = pool.alloc(2).unwrap();
        assert_eq!(*pool.get(a).unwrap(), 1);
        assert_eq!(*pool.get(b).unwrap(), 2);

        pool.free(a).unwrap();
        // Stale handle now fails (generation bumped).
        assert!(pool.get(a).is_none());
        // Recycling hands back a *new* handle.
        let c = pool.alloc(3).unwrap();
        assert_ne!(c, a);
        assert_eq!(*pool.get(c).unwrap(), 3);
    }

    #[test]
    fn pool_exhaustion_errors_instead_of_corrupting() {
        let mut pool = Pool::<u8>::with_capacity(2);
        let a = pool.alloc(1).unwrap();
        let b = pool.alloc(2).unwrap();
        assert_eq!(pool.alloc(3).unwrap_err(), PoolError::Exhausted);
        pool.free(b).unwrap();
        let c = pool.alloc(4).unwrap();
        assert_ne!(c, b);
        let _ = a;
    }

    #[test]
    fn double_free_is_rejected() {
        let mut pool = Pool::<u8>::with_capacity(2);
        let a = pool.alloc(1).unwrap();
        pool.free(a).unwrap();
        assert_eq!(pool.free(a).unwrap_err(), PoolError::InvalidSlot(a.idx));
    }
}
