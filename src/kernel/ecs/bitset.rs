//! A dense bitmask used to pre-filter "entity has component" before any
//! storage access. One bit per entity index; component storages own one
//! each, queries intersect them first.
//!
//! Indexed by *entity index* (not raw `Entity`), which is exactly what a
//! query walk needs to stay branch-light.

#[derive(Clone)]
pub struct Bitset {
    words: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitsetError {
    AllocationFailed,
    InvalidIndex,
}

impl Bitset {
    pub fn new(capacity: usize) -> Self {
        Self {
            words: vec![0u64; Bitset::word_count(capacity)],
        }
    }

    #[inline]
    fn word_count(capacity: usize) -> usize {
        capacity.div_ceil(64)
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.words.len() * 64
    }

    fn ensure(&mut self, index: usize) {
        let count = Bitset::word_count(index.saturating_add(1));
        if count > self.words.len() {
            self.words.resize(count, 0);
        }
    }

    pub fn try_reserve(&mut self, index: usize) -> Result<(), BitsetError> {
        let Some(next) = index.checked_add(1) else {
            return Err(BitsetError::InvalidIndex);
        };
        let count = Bitset::word_count(next);
        if count > self.words.len() {
            self.words
                .try_reserve_exact(count - self.words.len())
                .map_err(|_| BitsetError::AllocationFailed)?;
            self.words.resize(count, 0);
        }
        Ok(())
    }

    #[inline]
    pub fn set(&mut self, index: usize) -> bool {
        self.ensure(index);
        let (word, bit) = (index / 64, index % 64);
        let old = self.words[word] & (1 << bit) != 0;
        self.words[word] |= 1 << bit;
        !old
    }

    pub fn try_set(&mut self, index: usize) -> Result<bool, BitsetError> {
        self.try_reserve(index)?;
        Ok(self.set(index))
    }

    #[inline]
    pub fn clear(&mut self, index: usize) -> bool {
        let (word, bit) = (index / 64, index % 64);
        if word >= self.words.len() {
            return false;
        }
        let old = self.words[word] & (1 << bit) != 0;
        self.words[word] &= !(1 << bit);
        old
    }

    #[inline]
    pub fn test(&self, index: usize) -> bool {
        let (word, bit) = (index / 64, index % 64);
        word < self.words.len() && (self.words[word] & (1 << bit)) != 0
    }

    /// True when `self` has every bit `other` has (used by query
    /// pre-filters against the *smaller* side to keep it cheap).
    pub fn superset_of(&self, other: &Bitset) -> bool {
        let (a, b) = (self.words.as_slice(), other.words.as_slice());
        if a.len() < b.len() {
            return false;
        }
        a[..b.len()].iter().zip(b.iter()).all(|(x, y)| x & y == *y)
    }

    /// Mutably borrows two disjoint words by index handling (index
    /// arithmetic on the caller side).
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn set_clear_test_roundtrip() {
        let mut bits = Bitset::new(10);
        assert!(!bits.test(7));
        assert!(bits.set(7));
        assert!(!bits.set(7), "second set is a no-op");
        assert!(bits.test(7));
        assert!(bits.clear(7));
        assert!(!bits.test(7));
    }

    #[test]
    fn grows_beyond_initial_capacity() {
        let mut bits = Bitset::new(10);
        bits.set(400);
        assert!(bits.test(400));
    }

    #[test]
    fn superset_semantics() {
        let mut a = Bitset::new(10);
        let mut b = Bitset::new(10);
        a.set(3);
        a.set(5);
        b.set(3);
        b.set(5);
        assert!(a.superset_of(&b), "identical sets are supersets");
        b.clear(5);
        assert!(a.superset_of(&b), "a with 3,5 is a superset of b with 3");
        assert!(
            !b.superset_of(&a),
            "b with 3 is not a superset of a with 3,5"
        );
    }
}
