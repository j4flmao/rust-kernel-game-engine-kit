//! Monotonic time via `clock_gettime(CLOCK_MONOTONIC_RAW)`.
//!
//! `CLOCK_MONOTONIC_RAW` is unaffected by NTP adjustments and clock
//! stepping, so it is the right source for simulation delta time. The
//! `extern "C"` declaration is hand-written per `man 3 clock_gettime`.
#![allow(unsafe_code)] // justified: PAL FFI boundary

use core::fmt;

use crate::platform::Clock;

/// Linux `CLOCK_MONOTONIC_RAW`.
pub const CLOCK_MONOTONIC_RAW: i32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

extern "C" {
    #[link_name = "clock_gettime"]
    fn clock_gettime(clock_id: i32, tp: *mut Timespec) -> i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeError {
    pub retcode: i32,
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clock_gettime failed with retcode {}", self.retcode)
    }
}

impl core::error::Error for TimeError {}

/// Raw monotonic read. Returns `Err` only when the syscall itself fails,
/// which is a platform fault, not a transient condition.
pub fn now_ns() -> Result<u64, TimeError> {
    let mut ts = Timespec::default();
    // SAFETY: `ts` is a valid writable Timespec; the syscall writes in place
    // and returns 0 on success.
    let rc = unsafe { clock_gettime(CLOCK_MONOTONIC_RAW, &mut ts) };
    if rc != 0 {
        return Err(TimeError { retcode: rc });
    }
    // tv_sec is positive for any realistic epoch; saturating math keeps the
    // arithmetic total-loss-free on overflow instead of wrapping negative.
    Ok((ts.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(ts.tv_nsec as u64))
}

/// The Linux monotonic clock, usable as a [`Clock`].
pub struct LinuxClock;

pub static LINUX_CLOCK: LinuxClock = LinuxClock;

impl Clock for LinuxClock {
    fn now_ns(&self) -> u64 {
        // The clock is inarguably present post-boot; a failure here is a
        // hardware/kernel fault we refuse to paper over (fail-fast).
        match now_ns() {
            Ok(v) => v,
            Err(e) => panic!("CLOCK_MONOTONIC_RAW syscall failed: {e}"),
        }
    }

    fn resolution_ns(&self) -> u64 {
        1
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    #[cfg(not(miri))]
    use super::*;

    // Miri does not model the Linux `clock_gettime` FFI; native Linux CI
    // covers the real monotonic clock path.
    #[cfg(not(miri))]
    #[test]
    fn monotonic_increases() {
        let t0 = LinuxClock.now_ns();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t1 = LinuxClock.now_ns();
        assert!(t1 >= t0, "monotonic clock went backwards");
    }

    #[cfg(not(miri))]
    #[test]
    fn spans_sensible_epoch() {
        let now = LinuxClock.now_ns();
        // Must be within a plausible range: epoch-relative nanoseconds,
        // ~seconds-to-hours, not zero and not u64::MAX overflow.
        assert!(now > 0);
        assert!(now < 60 * 60 * 1_000_000_000, "epoch is unreasonably large");
    }
}
