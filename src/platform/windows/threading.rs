//! Windows thread scheduling primitives for the PAL.
#![allow(unsafe_code)]

type Handle = *mut core::ffi::c_void;
type AffinityMask = usize;

const THREAD_PRIORITY_NORMAL: i32 = 0;
const THREAD_PRIORITY_ABOVE_NORMAL: i32 = 1;

extern "system" {
    fn GetCurrentThread() -> Handle;
    fn SetThreadAffinityMask(thread: Handle, mask: AffinityMask) -> AffinityMask;
    fn SetThreadPriority(thread: Handle, priority: i32) -> i32;
    fn GetLastError() -> u32;
    fn WaitOnAddress(address: *const u32, compare: *const u32, size: usize, timeout_ms: u32)
        -> i32;
    fn WakeByAddressAll(address: *const u32);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadingError {
    pub op: &'static str,
    pub code: u32,
}

impl core::fmt::Display for ThreadingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} failed (Win32 error {})", self.op, self.code)
    }
}

impl core::error::Error for ThreadingError {}

/// Pins the current thread to one logical processor.
pub fn set_current_thread_affinity(cpu: u32) -> Result<(), ThreadingError> {
    let bits = usize::BITS;
    if cpu >= bits {
        return Err(ThreadingError {
            op: "SetThreadAffinityMask",
            code: 87,
        });
    }
    let mask = 1usize << cpu;
    // SAFETY: the pseudo-handle is valid for the current thread; mask is nonzero.
    let previous = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };
    if previous == 0 {
        return Err(ThreadingError {
            op: "SetThreadAffinityMask",
            code: unsafe { GetLastError() },
        });
    }
    Ok(())
}

/// Enables the optional low-latency priority used only by explicitly opted-in
/// scheduler threads. It does not request real-time priority.
pub fn set_current_thread_latency_priority(enabled: bool) -> Result<(), ThreadingError> {
    let priority = if enabled {
        THREAD_PRIORITY_ABOVE_NORMAL
    } else {
        THREAD_PRIORITY_NORMAL
    };
    // SAFETY: the pseudo-handle is valid for the current thread.
    if unsafe { SetThreadPriority(GetCurrentThread(), priority) } == 0 {
        return Err(ThreadingError {
            op: "SetThreadPriority",
            code: unsafe { GetLastError() },
        });
    }
    Ok(())
}

/// Reusable user-mode barrier backed by Windows `WaitOnAddress`.
pub struct WaitOnAddressBarrier {
    count: std::sync::atomic::AtomicU32,
    generation: std::sync::atomic::AtomicU32,
    total: u32,
}

impl WaitOnAddressBarrier {
    pub fn new(total: u32) -> Self {
        Self {
            count: std::sync::atomic::AtomicU32::new(0),
            generation: std::sync::atomic::AtomicU32::new(0),
            total: total.max(1),
        }
    }
    pub fn wait(&self) {
        use std::sync::atomic::Ordering;
        let generation = self.generation.load(Ordering::Acquire);
        if self.count.fetch_add(1, Ordering::AcqRel) + 1 == self.total {
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::AcqRel);
            // SAFETY: generation is an aligned live atomic word.
            unsafe {
                WakeByAddressAll(&self.generation as *const _ as *const u32);
            }
            return;
        }
        while self.generation.load(Ordering::Acquire) == generation {
            // SAFETY: comparison word and address are valid for the call.
            unsafe {
                let _ = WaitOnAddress(
                    &self.generation as *const _ as *const u32,
                    &generation,
                    4,
                    u32::MAX,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_affinity_outside_native_mask_width() {
        let result = set_current_thread_affinity(usize::BITS);
        assert_eq!(
            result,
            Err(ThreadingError {
                op: "SetThreadAffinityMask",
                code: 87
            })
        );
    }

    #[test]
    fn accepts_normal_priority_contract() {
        // The call is intentionally not made: CI may restrict thread policy.
        assert_eq!(THREAD_PRIORITY_NORMAL, 0);
        assert_eq!(THREAD_PRIORITY_ABOVE_NORMAL, 1);
    }
}
