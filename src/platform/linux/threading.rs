//! Linux threading primitives: futex wait/wake plus a futex-backed barrier,
//! and `sched_setaffinity` core pinning.
//!
//! These exist as *primitives* here (Phase 3) so the parallel scheduler
//! (Phase 6) can consume them; the scheduler wires them in next phase.
#![allow(unsafe_code)] // justified: PAL FFI boundary

use std::sync::atomic::{AtomicU32, Ordering};

/// x86_64 syscall numbers (the primary desktop/target arch for this kit).
#[cfg(target_arch = "x86_64")]
mod sy {
    pub const FUTEX: i64 = 202;
    pub const SCHED_SETAFFINITY: i64 = 203;
    pub const SCHED_SETSCHEDULER: i64 = 144;
}

#[cfg(not(target_arch = "x86_64"))]
mod sy {
    // The syscall number is architecture-dependent outside x86_64. Keep the
    // module compilable for unsupported targets, but fail at the FFI boundary
    // rather than claiming this number is portable.
    pub const FUTEX: i64 = 202;
    pub const SCHED_SETAFFINITY: i64 = 203;
    pub const SCHED_SETSCHEDULER: i64 = 144;
}

extern "C" {
    /// Raw `syscall()` dispatcher; variadic per the Linux kernel ABI.
    fn syscall(number: i64, ...) -> i64;
}

const FUTEX_WAIT: i32 = 0;
const FUTEX_WAKE: i32 = 1;
const FUTEX_PRIVATE_FLAG: i32 = 128;

/// Atomically waits on a futex word (returns on wake; a spurious
/// EWOULDBLOCK simply makes us re-check the word in the caller's loop).
fn futex_wait(word: &AtomicU32, expected: u32) {
    let ptr = word as *const AtomicU32 as *const u32;
    // SAFETY: `ptr` points at a valid u32 aligned per AtomicU32; the kernel
    // only reads it during the wait.
    unsafe {
        syscall(
            sy::FUTEX,
            ptr,
            FUTEX_WAIT | FUTEX_PRIVATE_FLAG,
            expected as i32,
            core::ptr::null::<u8>(), // no timeout: wake only on FUTEX_WAKE
            0i64,
            0i64,
        )
    };
}

/// Wakes up to `n` waiters on a futex word. The kernel reads the count as a
/// signed `int`, so the caller must never pass a value whose low 32 bits are
/// negative (negative counts wake nothing and would deadlock a barrier).
fn futex_wake(word: &AtomicU32, n: u32) {
    let ptr = word as *const AtomicU32 as *const u32;
    // SAFETY: same shape as `futex_wait`.
    unsafe {
        syscall(
            sy::FUTEX,
            ptr,
            FUTEX_WAKE | FUTEX_PRIVATE_FLAG,
            n as i32,
            0i64,
            0i64,
            0i64,
        )
    };
}

/// A futex-backed barrier: low-latency group rendezvous without spinning.
///
/// Classic sense-flip design: arrivals count on `count`, and every release
/// flips a monotonically increasing `sense` word, which is what the waiters
/// actually block on. Because the sense word never rewinds, the barrier is
/// safe to reuse across generations — a slow waiter from generation *n* can
/// never be mistaken for one from generation *n+1* (the earlier rewind-based
/// version deadlocked exactly on that race).
pub struct FutexBarrier {
    /// Arrivals in the current generation (reset to 0 on release).
    count: AtomicU32,
    /// Generation counter; released by flipping (fetch_add 1).
    sense: AtomicU32,
    total: u32,
}

impl FutexBarrier {
    pub fn new(total: u32) -> Self {
        Self {
            count: AtomicU32::new(0),
            sense: AtomicU32::new(0),
            total: total.max(1),
        }
    }

    /// Blocks until `total` threads have called `wait`. All callers return
    /// together; the barrier is reusable (generations auto-advance).
    pub fn wait(&self) {
        let my_sense = self.sense.load(Ordering::Acquire);
        let arrived = self.count.fetch_add(1, Ordering::AcqRel) + 1;
        if arrived == self.total {
            // Last one: rewind the counter, flip the generation, release
            // everyone (wake-count stays a positive int).
            self.count.store(0, Ordering::Release);
            self.sense.fetch_add(1, Ordering::AcqRel);
            futex_wake(&self.sense, i32::MAX as u32);
            return;
        }
        // Wait until the generation has passed mine.
        loop {
            let cur = self.sense.load(Ordering::Acquire);
            if cur != my_sense {
                return;
            }
            futex_wait(&self.sense, my_sense);
        }
    }
}

/// Sizes a Linux CPU set (1024 potential CPUs = 128 bytes).
const CPU_SET_SIZE: usize = 1024 / 8;

/// Pins the calling thread to exactly `cpus`.
pub fn set_thread_affinity(cpus: &[usize]) -> Result<(), i32> {
    let mut mask = [0u8; CPU_SET_SIZE];
    for &cpu in cpus {
        if cpu < CPU_SET_SIZE * 8 {
            mask[cpu / 8] |= 1 << (cpu % 8);
        }
    }
    // SAFETY: mask is a valid CPU_SET layout of the declared size.
    let r = unsafe {
        syscall(
            sy::SCHED_SETAFFINITY,
            0i64, // pid 0 == calling thread
            CPU_SET_SIZE as i64,
            mask.as_ptr(),
            0i64,
            0i64,
            0i64,
        )
    };
    if r < 0 {
        Err(-r as i32)
    } else {
        Ok(())
    }
}

/// Optional real-time priority (`SCHED_FIFO`, requires privileges).
pub fn set_realtime_priority(priority: i32) -> Result<(), i32> {
    const SCHED_FIFO: i32 = 1;
    // SAFETY: sched_param is a single i32 on Linux; nulled rest.
    let param: i32 = priority;
    let r = unsafe {
        syscall(
            sy::SCHED_SETSCHEDULER,
            0i64, // current thread
            SCHED_FIFO as i64,
            &param as *const i32,
            0i64,
            0i64,
            0i64,
        )
    };
    if r < 0 {
        Err(-r as i32)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn futex_barrier_syncs_threads() {
        let barrier = FutexBarrier::new(4);
        let results: Vec<u64> = std::thread::scope(|s| {
            let mut handles = Vec::new();
            for i in 0..4u64 {
                let b = &barrier;
                handles.push(s.spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(i * 3));
                    b.wait();
                    i
                }));
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert_eq!(results.len(), 4);
        // Everyone returned → the barrier released them all together.
        assert_eq!(barrier.count.load(Ordering::Acquire), 0);
    }

    #[test]
    fn barrier_reusable() {
        let barrier = FutexBarrier::new(2);
        std::thread::scope(|s| {
            let b1 = &barrier;
            let t = s.spawn(move || {
                b1.wait();
                b1.wait();
            });
            barrier.wait();
            barrier.wait();
            t.join().unwrap();
        });
    }

    // Miri does not implement the Linux `sched_setaffinity` syscall; native
    // Linux CI covers this platform operation.
    #[cfg(not(miri))]
    #[test]
    fn affinity_pins_to_first_cpu() {
        // Pinning to whatever CPU 0 is always legal for the current thread.
        set_thread_affinity(&[0]).unwrap();
    }
}
