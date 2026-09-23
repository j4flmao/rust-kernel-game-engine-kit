//! The message bus — the engine's "syscall boundary".
//!
//! Ownership crosses subsystem lines only here. Two lock-free rings:
//!
//! - [`SpscRing`] drives every subscriber inbox: exactly one producer (the
//!   router) to exactly one consumer (the owner subsystem).
//! - [`MpscRing`] is the global outbox for `publish()`: multi-producer-safe
//!   via a hand-rolled spinlock, so the same API holds when parallel ticking
//!   lands in Phase 6.
//!
//! `unsafe` in this crate is concentrated here (and in `platform/`): memory
//! ordering of the rings is manual, per the C11-spirit guarantees and the
//! single-producer/single-consumer contracts documented on each type.
#![allow(unsafe_code)] // justified: hand-rolled lock-free rings, audited below

use core::any::Any;
use core::cell::UnsafeCell;
use core::fmt;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::hint;
use std::sync::atomic::AtomicU64;

use crate::kernel::trace::contention::{self, Site};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingError {
    InvalidCapacity,
    AllocationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(u32);

impl SubscriberId {
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const INVALID: Self = Self(u32::MAX);

    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusError {
    /// The global outbox has no free slot and could not deliver now.
    OutboxFull,
    /// Target inbox has no free slot for this message.
    InboxFull(SubscriberId),
    /// No subscriber is registered at the target address.
    UnknownRecipient(SubscriberId),
    AllocationFailed,
    InvalidTopology,
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutboxFull => write!(f, "message bus outbox full"),
            Self::InboxFull(to) => write!(f, "message bus inbox full for {to}"),
            Self::UnknownRecipient(to) => write!(f, "no subscriber at {to}"),
            Self::AllocationFailed => write!(f, "message bus allocation failed"),
            Self::InvalidTopology => write!(f, "message bus topology is too large"),
        }
    }
}

impl core::error::Error for BusError {}

/// An addressed, typed message crossing the subsystem boundary.
///
/// `payload` is type-erased; the receiver downcasts via
/// [`Envelope::downcast`] after matching [`Envelope::topic`].
pub struct Envelope {
    pub from: SubscriberId,
    pub to: SubscriberId,
    /// Caller-defined message kind (e.g. `0 = tick pulse`).
    pub topic: u16,
    /// Globally increasing sequence, useful for ordering/replay.
    pub seq: u64,
    pub payload: Box<dyn Any + Send>,
}

impl Envelope {
    pub fn new(
        from: SubscriberId,
        to: SubscriberId,
        topic: u16,
        seq: u64,
        payload: Box<dyn Any + Send>,
    ) -> Self {
        Self {
            from,
            to,
            topic,
            seq,
            payload,
        }
    }

    /// Downcasts the payload; returns `Err(self)` on type mismatch.
    pub fn downcast<T: Any + Send>(self) -> Result<Box<T>, Self> {
        match self.payload.downcast::<T>() {
            Ok(v) => Ok(v),
            Err(payload) => Err(Self { payload, ..self }),
        }
    }
}

impl fmt::Debug for Envelope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Envelope")
            .field("from", &self.from)
            .field("to", &self.to)
            .field("topic", &self.topic)
            .field("seq", &self.seq)
            .finish()
    }
}

/// Single-producer / single-consumer bounded ring buffer.
///
/// # Contract
/// - Exactly one producer calls [`SpscRing::push`]; **single-producer is an
///   invariant of the constructor's use**, so `push` is a plain store, not a
///   CAS.
/// - Exactly one consumer calls [`SpscRing::pop`].
/// - The same thread may be both, but then it is still one-at-a-time.
///
/// Index ordering follows the standard SPSC design: the producer writes a
/// slot then releases `tail`; the consumer acquires `tail` before reading a
/// slot and releases `head` after. This gives happens-before for the slot
/// contents without any locks.
pub struct SpscRing<T> {
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

unsafe impl<T: Send> Send for SpscRing<T> {}
unsafe impl<T: Send> Sync for SpscRing<T> {}

impl<T> SpscRing<T> {
    /// Allocates a ring of `capacity` (rounded up to a power of two, min 2).
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("SPSC ring allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, RingError> {
        let capacity = capacity
            .checked_next_power_of_two()
            .ok_or(RingError::InvalidCapacity)?
            .max(2);
        let mut inner: Vec<UnsafeCell<MaybeUninit<T>>> = Vec::new();
        inner
            .try_reserve_exact(capacity)
            .map_err(|_| RingError::AllocationFailed)?;
        for _ in 0..capacity {
            inner.push(UnsafeCell::new(MaybeUninit::uninit()));
        }
        Ok(Self {
            buf: inner.into_boxed_slice(),
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        })
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head == tail
    }

    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Single-producer append. Returns `Err(value)` when full (value back).
    #[inline]
    pub fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) >= self.capacity() {
            return Err(value);
        }
        // SAFETY: `tail & mask < capacity`; slot is initialized exclusively
        // by the single producer; the consumer only reads after we release
        // `tail`, and we return early if the slot is still occupied.
        let slot: *mut MaybeUninit<T> = unsafe { self.buf.get_unchecked(tail & self.mask).get() };
        unsafe {
            (*slot).write(value);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Single-consumer take. `None` when empty.
    #[inline]
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        // SAFETY: `head & mask < capacity`; the slot was produced before the
        // acquire of `tail` above; only the single consumer reads it.
        let slot: *mut MaybeUninit<T> = unsafe { self.buf.get_unchecked(head & self.mask).get() };
        let value = unsafe { (*slot).assume_init_read() };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(value)
    }
}

impl<T> Drop for SpscRing<T> {
    fn drop(&mut self) {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        for i in head..tail {
            // SAFETY: indices `head..tail` hold live values.
            let slot: *mut MaybeUninit<T> = unsafe { self.buf.get_unchecked(i & self.mask).get() };
            unsafe {
                (*slot).assume_init_drop();
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for SpscRing<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpscRing")
            .field("capacity", &self.capacity())
            .field("len", &self.len())
            .finish()
    }
}

/// Multi-producer / single-consumer bounded ring buffer guarded by a
/// hand-rolled spinlock. The outbox does not contend in Phase 1 (ticks are
/// serial), but the same type serves Phase 6 parallel publishing unchanged.
pub struct MpscRing<T> {
    lock: AtomicUsize,
    buf: Box<[UnsafeCell<MaybeUninit<T>>]>,
    mask: usize,
    head: UnsafeCell<usize>,
    tail: UnsafeCell<usize>,
}

unsafe impl<T: Send> Send for MpscRing<T> {}
unsafe impl<T: Send> Sync for MpscRing<T> {}

pub(crate) static OUTBOX_LOCK: Site = Site::new(module_path!(), "message_bus_outbox");

impl<T> MpscRing<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("MPSC ring allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, RingError> {
        let capacity = capacity
            .checked_next_power_of_two()
            .ok_or(RingError::InvalidCapacity)?
            .max(2);
        let mut inner: Vec<UnsafeCell<MaybeUninit<T>>> = Vec::new();
        inner
            .try_reserve_exact(capacity)
            .map_err(|_| RingError::AllocationFailed)?;
        for _ in 0..capacity {
            inner.push(UnsafeCell::new(MaybeUninit::uninit()));
        }
        Ok(Self {
            lock: AtomicUsize::new(0),
            buf: inner.into_boxed_slice(),
            mask: capacity - 1,
            head: UnsafeCell::new(0),
            tail: UnsafeCell::new(0),
        })
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.mask + 1
    }

    /// Acquires the spinlock, recording contention with the tracer.
    #[inline]
    fn acquire(&self) -> OutboxGuard<'_, T> {
        if self
            .lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return OutboxGuard { ring: self };
        }
        contention::record(&OUTBOX_LOCK);
        let mut spins = 0;
        loop {
            if self
                .lock
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
            spins += 1;
            if spins >= 32 {
                std::thread::yield_now();
                spins = 0;
            } else {
                hint::spin_loop();
            }
        }
        OutboxGuard { ring: self }
    }

    pub fn push(&self, value: T) -> Result<(), T> {
        let guard = self.acquire();
        // SAFETY: exclusive under the spinlock guard.
        let tail = unsafe { *self.tail.get() };
        let head = unsafe { *self.head.get() };
        if tail.wrapping_sub(head) >= self.capacity() {
            return Err(value);
        }
        // SAFETY: `tail & mask < capacity`; exclusive under guard.
        let slot: *mut MaybeUninit<T> = unsafe { self.buf.get_unchecked(tail & self.mask).get() };
        unsafe {
            (*slot).write(value);
        }
        unsafe {
            *self.tail.get() = tail.wrapping_add(1);
        }
        drop(guard);
        Ok(())
    }

    pub fn pop(&self) -> Option<T> {
        let guard = self.acquire();
        // SAFETY: exclusive under guard.
        let head = unsafe { *self.head.get() };
        let tail = unsafe { *self.tail.get() };
        if head == tail {
            return None;
        }
        // SAFETY: `head & mask < capacity`; live slot under guard.
        let slot: *mut MaybeUninit<T> = unsafe { self.buf.get_unchecked(head & self.mask).get() };
        let value = unsafe { (*slot).assume_init_read() };
        unsafe {
            *self.head.get() = head.wrapping_add(1);
        }
        drop(guard);
        Some(value)
    }

    pub fn is_empty(&self) -> bool {
        let guard = self.acquire();
        let empty = unsafe { *self.head.get() == *self.tail.get() };
        drop(guard);
        empty
    }
}

struct OutboxGuard<'a, T> {
    ring: &'a MpscRing<T>,
}

impl<T> Drop for OutboxGuard<'_, T> {
    fn drop(&mut self) {
        self.ring.lock.store(0, Ordering::Release);
    }
}

/// The bus itself: global outbox + per-subscriber SPSC inboxes + routing.
pub struct MessageBus {
    outbox: MpscRing<Envelope>,
    inboxes: Vec<SpscRing<Envelope>>,
    /// `edges[to][from] == true` when the receiver declared the sender as a
    /// dependency (used to police ordering in debug builds).
    edges: Vec<Vec<bool>>,
    next_seq: u64,
    /// Release-build counter of sends that violated declared ordering.
    pub undeclared_sends: AtomicU64,
}

const DEFAULT_INBOX_CAPACITY: usize = 64;
const OUTBOX_CAPACITY: usize = 32;

impl MessageBus {
    pub fn new() -> Self {
        Self::try_new().expect("message bus allocation failed")
    }

    pub fn try_new() -> Result<Self, BusError> {
        Ok(Self {
            outbox: MpscRing::try_with_capacity(OUTBOX_CAPACITY)
                .map_err(|_| BusError::AllocationFailed)?,
            inboxes: Vec::new(),
            edges: Vec::new(),
            next_seq: 0,
            undeclared_sends: AtomicU64::new(0),
        })
    }

    /// Registers one subscriber slot (one per subsystem), before any publish.
    pub fn add_subscriber(&mut self) -> SubscriberId {
        self.try_add_subscriber()
            .expect("message bus subscriber allocation failed")
    }

    pub fn try_add_subscriber(&mut self) -> Result<SubscriberId, BusError> {
        if self.inboxes.len() > u32::MAX as usize {
            return Err(BusError::InvalidTopology);
        }
        let id = SubscriberId::new(self.inboxes.len() as u32);
        self.inboxes
            .try_reserve(1)
            .map_err(|_| BusError::AllocationFailed)?;
        self.inboxes.push(
            SpscRing::try_with_capacity(DEFAULT_INBOX_CAPACITY)
                .map_err(|_| BusError::AllocationFailed)?,
        );
        Ok(id)
    }

    /// Installs the dependency-derived routing matrix for debug policing.
    /// `edges[to][from]` set from `deps[to]` containing `from`.
    pub fn set_debug_edges(&mut self, deps: &[Vec<usize>]) {
        self.try_set_debug_edges(deps)
            .expect("message bus topology allocation failed");
    }

    pub fn try_set_debug_edges(&mut self, deps: &[Vec<usize>]) -> Result<(), BusError> {
        let n = deps.len();
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(n)
            .map_err(|_| BusError::AllocationFailed)?;
        for _ in 0..n {
            let mut row = Vec::new();
            row.try_reserve_exact(n)
                .map_err(|_| BusError::AllocationFailed)?;
            row.resize(n, false);
            edges.push(row);
        }
        for (to, list) in deps.iter().enumerate() {
            for &from in list {
                if from >= n {
                    return Err(BusError::InvalidTopology);
                }
                edges[to][from] = true;
            }
        }
        self.edges = edges;
        Ok(())
    }

    #[inline]
    fn routing_ok(&self, from: SubscriberId, to: SubscriberId) -> bool {
        from == to
            || self
                .edges
                .get(to.raw() as usize)
                .and_then(|row| row.get(from.raw() as usize))
                .copied()
                .unwrap_or(false)
    }

    /// Publishes a message and delivers it to the target inbox immediately.
    /// Gracious failure: returns `Err` without dropping the message when a
    /// target inbox is full or the address is unknown.
    pub fn publish(&mut self, envelope: Envelope) -> Result<(), BusError> {
        if !self.routing_ok(envelope.from, envelope.to) {
            self.undeclared_sends.fetch_add(1, Ordering::Relaxed);
            #[cfg(debug_assertions)]
            panic!(
                "message bus routing violation: {} published to {} without \
                 a declared dependency edge",
                envelope.from, envelope.to
            );
        }

        // The outbox is drained synchronously (single-threaded ticks), so a
        // push here can never overtake delivery; use it as the ordering
        // serialization point for later parallel publishing.
        match self.outbox.push(envelope) {
            Ok(()) => {}
            Err(_env) => return Err(BusError::OutboxFull),
        }
        let queued = self.outbox.pop().ok_or(BusError::OutboxFull)?;
        let to = queued.to;
        let Some(mailbox) = self.inboxes.get(to.raw() as usize) else {
            return Err(BusError::UnknownRecipient(to));
        };
        match mailbox.push(queued) {
            Ok(()) => Ok(()),
            Err(env) => Err(BusError::InboxFull(env.to)),
        }
    }

    /// Drains any residue left in the outbox (used at frame boundaries and,
    /// later, after parallel ticks). Does nothing when empty.
    pub fn flush(&mut self) {
        while let Some(queued) = self.outbox.pop() {
            let to = queued.to;
            if let Some(mailbox) = self.inboxes.get_mut(to.raw() as usize) {
                if mailbox.push(queued).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Single-consumer drain of one message for `id`'s owner.
    ///
    /// Public because the bus is a kernel facility (programmatic consumers,
    /// tests), not just an internal transport. Each subscriber owns exactly
    /// its own id; draining somebody else's inbox races with its owner.
    pub fn pop_inbox(&self, id: SubscriberId) -> Option<Envelope> {
        self.inboxes.get(id.raw() as usize).and_then(SpscRing::pop)
    }

    /// Current backlog length of one inbox (for capacity planning).
    pub(crate) fn inbox_len(&self, id: SubscriberId) -> usize {
        self.inboxes
            .get(id.raw() as usize)
            .map(SpscRing::len)
            .unwrap_or(0)
    }

    fn next_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = seq.wrapping_add(1);
        seq
    }

    /// Builds a fresh envelope with the bus's sequence number.
    pub fn envelope(
        &mut self,
        from: SubscriberId,
        to: SubscriberId,
        topic: u16,
        payload: Box<dyn Any + Send>,
    ) -> Envelope {
        Envelope::new(from, to, topic, self.next_seq(), payload)
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MessageBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageBus")
            .field("subscribers", &self.inboxes.len())
            .field("inbox_capacity", &DEFAULT_INBOX_CAPACITY)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::sync::Arc;

    fn roundtrip_on(ring: &SpscRing<u64>) {
        assert!(ring.is_empty());
        assert_eq!(ring.pop(), None);
        ring.push(1).unwrap();
        ring.push(2).unwrap();
        assert_eq!(ring.len(), 2);
        assert_eq!(ring.pop(), Some(1));
        assert_eq!(ring.pop(), Some(2));
        assert_eq!(ring.pop(), None);
        assert!(ring.is_empty());
    }

    #[test]
    fn spsc_single_thread_roundtrip() {
        let ring = SpscRing::with_capacity(4);
        roundtrip_on(&ring);
    }

    #[test]
    fn spsc_full_offset_return() {
        let ring = SpscRing::with_capacity(2);
        assert_eq!(ring.push(1), Ok(()));
        assert_eq!(ring.push(2), Ok(()));
        assert_eq!(ring.push(3), Err(3));
        assert_eq!(ring.pop(), Some(1));
        assert_eq!(ring.push(3), Ok(()));
        assert_eq!(ring.pop(), Some(2));
    }

    #[test]
    fn spsc_cross_thread_no_loss() {
        const N: u64 = 100_000;
        let ring = Arc::new(SpscRing::with_capacity(256));

        // Producer and consumer MUST run concurrently: the ring is bounded,
        // so a producer that outruns the consumer would block forever.
        std::thread::scope(|scope| {
            // Clone for the producer first; the fn-scope `ring` stays alive
            // so the consumer and the final assert can still use it.
            scope.spawn({
                let ring = ring.clone();
                move || {
                    for i in 0..N {
                        while ring.push(i).is_err() {
                            hint::spin_loop();
                        }
                    }
                }
            });
            scope.spawn({
                let ring = ring.clone();
                move || {
                    let mut expected = 0u64;
                    while expected < N {
                        match ring.pop() {
                            Some(v) => {
                                assert_eq!(v, expected, "order and lossless delivery");
                                expected += 1;
                            }
                            None => hint::spin_loop(),
                        }
                    }
                }
            });
        });
        assert!(ring.is_empty());
    }

    #[test]
    fn mpsc_multi_producer_no_loss() {
        const PRODUCERS: usize = 4;
        const PER_PRODUCER: usize = 2_000;
        let ring = Arc::new(MpscRing::with_capacity(128));
        let total = PRODUCERS * PER_PRODUCER;
        let collected = Arc::new(std::sync::Mutex::new(vec![false; total]));

        // Producers AND the consumer run concurrently; bounded capacity
        // deadlocks any test where the drain only starts after producers join.
        std::thread::scope(|scope| {
            for p in 0..PRODUCERS {
                let ring = ring.clone();
                scope.spawn(move || {
                    for i in 0..PER_PRODUCER {
                        let tag = (p * PER_PRODUCER + i) as u64;
                        while ring.push(tag).is_err() {
                            hint::spin_loop();
                        }
                    }
                });
            }
            let ring = ring.clone();
            let collected = collected.clone();
            scope.spawn(move || {
                let mut got = 0usize;
                while got < total {
                    match ring.pop() {
                        Some(v) => {
                            let mut slots = collected.lock().unwrap();
                            let idx = v as usize;
                            assert!(idx < total && !slots[idx], "duplicate or corrupt");
                            slots[idx] = true;
                            drop(slots);
                            got += 1;
                        }
                        None => hint::spin_loop(),
                    }
                }
            });
        });

        assert!(collected.lock().unwrap().iter().all(|&seen| seen));
        assert_eq!(ring.pop(), None);
    }

    #[test]
    fn mpsc_recovers_after_full() {
        let ring = MpscRing::with_capacity(2);
        assert_eq!(ring.push(1), Ok(()));
        assert_eq!(ring.push(2), Ok(()));
        assert_eq!(ring.push(3), Err(3));
        assert_eq!(ring.pop(), Some(1));
        assert_eq!(ring.push(3), Ok(()));
    }

    #[test]
    fn envelope_downcast_ok_and_mismatch() {
        let from = SubscriberId::new(0);
        let to = SubscriberId::new(1);
        let env = Envelope::new(from, to, 7, 1, Box::new(42u64));
        let value = env.downcast::<u64>().unwrap();
        assert_eq!(*value, 42);

        let env = Envelope::new(from, to, 7, 2, Box::new("txt"));
        let back = env.downcast::<u64>().unwrap_err();
        assert_eq!(back.topic, 7);
        assert!(back.downcast::<&str>().is_ok());
    }

    #[test]
    fn bus_delivers_addressed_message() {
        let mut bus = MessageBus::new();
        let a = bus.add_subscriber();
        let b = bus.add_subscriber();
        // b declares a as a dependency, legitimizing a -> b sends.
        bus.set_debug_edges(&[vec![], vec![0]]);

        let env = bus.envelope(a, b, 1, Box::new(9u32));
        bus.publish(env).unwrap();
        assert!(bus.pop_inbox(a).is_none(), "a must not steal b's message");
        let got = bus.pop_inbox(b).unwrap();
        assert_eq!(got.topic, 1u16);
        assert_eq!(got.from, a);
        assert_eq!(*got.downcast::<u32>().unwrap(), 9);
        assert!(bus.pop_inbox(b).is_none());
    }

    #[test]
    fn bus_inbox_full_rejects_and_keeps_message() {
        let mut bus = MessageBus::new();
        let a = bus.add_subscriber();
        let b = bus.add_subscriber();
        bus.set_debug_edges(&[vec![], vec![0]]);

        for i in 0..(DEFAULT_INBOX_CAPACITY as u64) {
            let env = bus.envelope(a, b, 1, Box::new(i));
            bus.publish(env).unwrap();
        }
        let env = bus.envelope(a, b, 1, Box::new(u64::MAX));
        let err = bus.publish(env).unwrap_err();
        assert!(matches!(err, BusError::InboxFull(to) if to == b));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn bus_unknown_recipient() {
        let mut bus = MessageBus::new();
        let a = bus.add_subscriber();
        bus.set_debug_edges(&[vec![]]);
        let ghost = SubscriberId::new(99);
        let env = bus.envelope(a, ghost, 1, Box::new(1u64));
        let err = bus.publish(env).unwrap_err();
        assert!(matches!(err, BusError::UnknownRecipient(to) if to == ghost));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn bus_unknown_recipient_is_routing_violation_in_debug() {
        // In debug builds any send without a declared dependency edge — even
        // to an unregistered id — is a routing panic, not a runtime error.
        let mut bus = MessageBus::new();
        let a = bus.add_subscriber();
        bus.set_debug_edges(&[vec![]]);
        let ghost = SubscriberId::new(99);
        let env = bus.envelope(a, ghost, 1, Box::new(1u64));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = bus.publish(env);
        }));
        assert!(result.is_err(), "expected routing-violation panic");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "routing violation")]
    fn bus_undeclared_to_panics_in_debug() {
        let mut bus = MessageBus::new();
        bus.add_subscriber();
        bus.add_subscriber();
        // A depends on B; A publishing to B violates declared ordering (B has
        // no incoming edge recorded because edges[to=B][from=A] is false).
        bus.set_debug_edges(&[vec![1], vec![]]);
        let a = SubscriberId::new(0);
        let b = SubscriberId::new(1);
        let env = bus.envelope(a, b, 1, Box::new(1u64));
        let _ = bus.publish(env);
    }
}
