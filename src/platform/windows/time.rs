//! Monotonic time through `QueryPerformanceCounter`.
#![allow(unsafe_code)]

use core::fmt;
use std::sync::OnceLock;

use crate::platform::Clock;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeError(pub u32);

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueryPerformanceCounter failed with Win32 error {}",
            self.0
        )
    }
}

impl core::error::Error for TimeError {}

extern "system" {
    fn QueryPerformanceCounter(value: *mut i64) -> i32;
    fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    fn GetLastError() -> u32;
}

fn frequency() -> Result<i64, TimeError> {
    let mut value = 0;
    // SAFETY: `value` is a valid writable pointer for the Win32 API.
    let ok = unsafe { QueryPerformanceFrequency(&mut value) };
    if ok == 0 || value <= 0 {
        return Err(TimeError(unsafe { GetLastError() }));
    }
    Ok(value)
}

/// Returns monotonic nanoseconds from the performance-counter epoch.
pub fn now_ns() -> Result<u64, TimeError> {
    static FREQ: OnceLock<Result<i64, TimeError>> = OnceLock::new();
    let freq = FREQ
        .get_or_init(frequency)
        .as_ref()
        .map(|value| *value)
        .map_err(|error| *error)?;
    let mut counter = 0;
    // SAFETY: `counter` is a valid writable pointer for the Win32 API.
    let ok = unsafe { QueryPerformanceCounter(&mut counter) };
    if ok == 0 || counter < 0 {
        return Err(TimeError(unsafe { GetLastError() }));
    }
    Ok((counter as u128 * 1_000_000_000 / freq as u128) as u64)
}

pub struct WindowsClock;

pub static WINDOWS_CLOCK: WindowsClock = WindowsClock;

impl Clock for WindowsClock {
    fn now_ns(&self) -> u64 {
        now_ns().unwrap_or_else(|error| panic!("Windows monotonic clock failed: {error}"))
    }

    fn resolution_ns(&self) -> u64 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn monotonic_does_not_move_backwards() {
        let before = WINDOWS_CLOCK.now_ns();
        std::thread::sleep(Duration::from_millis(2));
        let after = WINDOWS_CLOCK.now_ns();
        assert!(after >= before);
    }

    #[test]
    fn raw_and_trait_reads_are_close() {
        let direct = now_ns().expect("QPC must be available on Windows");
        let via_trait = WINDOWS_CLOCK.now_ns();
        assert!(via_trait >= direct);
        assert!(via_trait - direct < 100_000_000);
    }
}
