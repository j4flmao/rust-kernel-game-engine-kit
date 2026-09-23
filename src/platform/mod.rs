//! Platform Abstraction Layer (PAL).
//!
//! The ONLY place `#[cfg(target_os = ...)]` branches are allowed. Every other
//! crate module stays OS-agnostic. Phase 3 delivers the shared trait shapes;
//! Linux is implemented first, Windows lands later behind the same traits.

#[cfg(target_os = "linux")]
pub mod linux;
pub mod thread_pool;
pub mod vulkan_policy;
#[cfg(target_os = "linux")]
pub use linux::present;
#[cfg(target_os = "windows")]
pub use windows::present;
#[cfg(target_os = "windows")]
pub mod windows;

use std::time::Instant;

/// Monotonic-clock contract every backend implements.
///
/// Implementations must be monotonic and never jump backwards; wall-clock
/// time must never drive simulation.
pub trait Clock {
    /// Nanoseconds since an arbitrary fixed epoch (monotonic).
    fn now_ns(&self) -> u64;

    /// Nominal tick period in nanoseconds of the underlying clock source.
    fn resolution_ns(&self) -> u64;
}

/// Portable fallback built on `std::time` — used off-Linux until the Windows
/// backend lands; on Linux the raw-syscall clock wins.
pub struct StdClock {
    epoch: Instant,
}

impl StdClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Clock for StdClock {
    fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos().max(1) as u64
    }

    fn resolution_ns(&self) -> u64 {
        1
    }
}

impl Default for StdClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Selects the platform clock; composition roots call this once at startup.
pub fn system_clock() -> &'static dyn Clock {
    #[cfg(target_os = "linux")]
    {
        &linux::time::LINUX_CLOCK
    }
    #[cfg(target_os = "windows")]
    {
        &windows::time::WINDOWS_CLOCK
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        use std::sync::OnceLock;
        static CLOCK: OnceLock<StdClock> = OnceLock::new();
        CLOCK.get_or_init(StdClock::new)
    }
}
