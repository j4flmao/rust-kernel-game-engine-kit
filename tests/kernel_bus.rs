//! Message bus contract exercised through the public kernel API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(not(debug_assertions))]
use rust_kernel_game_engine_kit::kernel::bus::BusError;
use rust_kernel_game_engine_kit::kernel::bus::{Envelope, MessageBus, SubscriberId};

#[test]
fn addressed_roundtrip_preserves_order() {
    let mut bus = MessageBus::new();
    bus.add_subscriber(); // id 0
    bus.add_subscriber(); // id 1
    let sender = SubscriberId::new(0);
    let receiver = SubscriberId::new(1);
    bus.set_debug_edges(&[vec![], vec![0]]); // receiver depends on sender

    // Capacity is 64; keep N under the inbox cap so nothing is dropped.
    const N: u64 = 16;
    for i in 0..N {
        let env = bus.envelope(sender, receiver, 7, Box::new(i));
        bus.publish(env).unwrap();
    }

    for i in 0..N {
        let env: Envelope = bus.pop_inbox(receiver).unwrap();
        assert_eq!(env.from, sender, "envelope keeps its sender address");
        let value: u64 = *env.downcast().unwrap();
        assert_eq!(value, i, "strict send order is preserved");
    }
    assert!(bus.pop_inbox(receiver).is_none());
}

#[test]
#[cfg(not(debug_assertions))]
fn unknown_recipient_is_an_error_in_release() {
    let mut bus = MessageBus::new();
    bus.add_subscriber();
    let sender = SubscriberId::new(0);
    let ghost = SubscriberId::new(77);
    let env = bus.envelope(sender, ghost, 0, Box::new(1u32));
    let err = bus.publish(env).unwrap_err();
    assert!(matches!(err, BusError::UnknownRecipient(_)));
}

#[test]
#[cfg(debug_assertions)]
fn unregistered_target_is_a_routing_violation_in_debug() {
    let mut bus = MessageBus::new();
    bus.add_subscriber();
    let sender = SubscriberId::new(0);
    let ghost = SubscriberId::new(77);
    let env = bus.envelope(sender, ghost, 0, Box::new(1u32));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = bus.publish(env);
    }));
    assert!(outcome.is_err(), "debug builds panic on undeclared sends");
}

#[test]
fn full_inbox_rejects_without_loss() {
    let mut bus = MessageBus::new();
    bus.add_subscriber(); // id 0
    bus.add_subscriber(); // id 1
    let a = SubscriberId::new(0);
    let b = SubscriberId::new(1);
    bus.set_debug_edges(&[vec![], vec![0]]);

    // Small capacity: fill it, then confirm overflow is rejected, not dropped.
    let mut published = 0;
    for _ in 0..200 {
        let env = bus.envelope(a, b, 0, Box::new(published));
        if bus.publish(env).is_ok() {
            published += 1;
        }
    }
    let drained: usize = std::iter::from_fn(|| bus.pop_inbox(b)).count();
    assert_eq!(drained, published);
}
