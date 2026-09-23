//! Bump/arena allocator for per-frame scratch memory.
//!
//! One contiguous block from `std::alloc::alloc`. `alloc` only bumps a
//! cursor; `reset` rewinds it to zero and **does not drop** previous
//! contents. The arena therefore carries "trivially destructible"
//! [`Copy`]-like data (draw-call lists, contact pairs, event payloads) —
//! nothing that owns a heap allocation, unless the owner manually reads it
//! out before `reset` runs (callers register such drains explicitly).
#![allow(unsafe_code)] // the raw block is the only side-channel here

use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::mem::{align_of, size_of_val, MaybeUninit};
use std::ptr::NonNull;

const DEFAULT_ALIGN: usize = 16;

/// Bounded bump allocator. Not `Sync`: one arena per owner (per frame for
/// the kernel, per subsystem for long-lived scratch).
pub struct BumpArena {
    base: NonNull<u8>,
    capacity: usize,
    used: Cell<usize>,
    peak: Cell<usize>,
}

impl BumpArena {
    /// Allocates a fresh block; panics on allocation failure the same way
    /// the default allocator does.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        // SAFETY: allocation is fallible; `alloc` returns a dangling
        // NonNull on OOM, which we refuse to sweep under the rug.
        let layout = Layout::from_size_align(capacity, DEFAULT_ALIGN).unwrap();
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr).unwrap_or_else(|| {
            // Match std OOM behaviour: abort with a hard error.
            std::process::abort()
        });
        Self {
            base: ptr,
            capacity,
            used: Cell::new(0),
            peak: Cell::new(0),
        }
    }

    pub fn try_new(capacity: usize) -> Option<Self> {
        let capacity = capacity.max(1);
        let layout = Layout::from_size_align(capacity, DEFAULT_ALIGN).ok()?;
        let ptr = NonNull::new(unsafe { alloc(layout) })?;
        Some(Self {
            base: ptr,
            capacity,
            used: Cell::new(0),
            peak: Cell::new(0),
        })
    }

    #[inline]
    fn align_up(offset: usize, align: usize) -> usize {
        offset.saturating_add(align.saturating_sub(1)) & !(align.saturating_sub(1))
    }

    /// Bytes currently consumed (after resets this drops back down).
    pub fn used(&self) -> usize {
        self.used.get()
    }

    /// Highest watermark reached since construction.
    pub fn peak(&self) -> usize {
        self.peak.get()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Rewinds the cursor to zero. Does NOT run any destructors.
    #[inline]
    pub fn reset(&mut self) {
        self.used.set(0);
    }

    /// Reserves `size` bytes with `align` alignment; returns `None` when the
    /// arena cannot satisfy the request (caller falls back or drops work).
    #[inline]
    pub fn alloc_bytes(&self, size: usize, align: usize) -> Option<NonNull<u8>> {
        if align == 0 || !align.is_power_of_two() {
            return None;
        }
        let offset = Self::align_up(self.used.get(), align);
        let end = offset.checked_add(size)?;
        if end > self.capacity {
            return None;
        }
        self.used.set(end);
        self.peak.set(self.peak.get().max(end));
        // SAFETY: `offset..end` is inside `0..capacity` and aligned per the
        // caller's request.
        Some(unsafe { NonNull::new_unchecked(self.base.as_ptr().add(offset)) })
    }

    /// Writes a [`Copy`] value into the arena and returns a reference.
    pub fn put<T: Copy>(&mut self, value: T) -> Option<&mut T> {
        let ptr = self.alloc_bytes(size_of_val(&value), align_of::<T>())?;
        // SAFETY: `ptr` is `align_of::<T>()`-aligned and writable for
        // `size_of::<T>()` bytes; T is Copy and never dropped by reset.
        unsafe {
            ptr.as_ptr()
                .cast::<MaybeUninit<T>>()
                .write(MaybeUninit::new(value));
            Some(&mut *ptr.as_ptr().cast::<T>())
        }
    }

    /// Returns an uninitialized slot (the caller fills with
    /// [`std::ptr::write`]); `reset` still won't drop it.
    pub fn slot<T>(&mut self) -> Option<*mut T> {
        let ptr = self.alloc_bytes(size_of::<T>(), align_of::<T>())?;
        Some(ptr.as_ptr().cast::<T>())
    }
}

impl Drop for BumpArena {
    fn drop(&mut self) {
        // SAFETY: layout matches the original `new` allocation exactly.
        let layout = Layout::from_size_align(self.capacity, DEFAULT_ALIGN).unwrap();
        unsafe { dealloc(self.base.as_ptr(), layout) };
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn bump_rewinds_and_recycles() {
        let mut arena = BumpArena::new(1024);
        let a = arena.put(42u32).unwrap();
        assert_eq!(*a, 42);
        let a1 = arena.put(7u64).unwrap();
        assert_eq!(*a1, 7);
        arena.reset();
        assert_eq!(arena.used(), 0);
        // After reset the same bytes are reusable.
        let b = arena.put(99u8).unwrap();
        assert_eq!(*b, 99);
    }

    #[test]
    fn arena_never_grows_past_capacity() {
        let mut arena = BumpArena::new(64);
        let mut bytes = 0u64;
        let mut ok = 0;
        while arena.put(0xdeadbeefu64).is_some() {
            ok += 1;
            bytes += 8;
        }
        assert_eq!(bytes, 64);
        assert_eq!(ok, 8);
        assert!(arena.used() > 0);
    }

    #[test]
    fn alignment_is_enforced() {
        let mut arena = BumpArena::new(256);
        let _ = arena.put(1u8).unwrap();
        let slot = arena.slot::<u64>().unwrap() as usize;
        assert_eq!(slot % 8, 0, "u64 slot must be 8-aligned");
    }
}
