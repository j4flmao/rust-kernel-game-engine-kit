//! Linux PAL clock tests through the public API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "linux")]
mod linux_only {
    use std::time::Duration;

    use rust_kernel_game_engine_kit::platform::linux::time::{now_ns, LinuxClock};
    use rust_kernel_game_engine_kit::platform::Clock;

    #[test]
    fn monotonic_does_not_move_backwards() {
        let t0 = LinuxClock.now_ns();
        std::thread::sleep(Duration::from_millis(2));
        assert!(LinuxClock.now_ns() >= t0);
    }

    #[test]
    fn raw_syscall_matches_trait_value() {
        let direct = now_ns().unwrap();
        let via_trait = LinuxClock.now_ns();
        assert!(direct <= via_trait && via_trait - direct < 100_000_000);
    }

    #[test]
    fn resolution_is_reported_not_assumed() {
        assert!(LinuxClock.resolution_ns() >= 1);
    }
}
