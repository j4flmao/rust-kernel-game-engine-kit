//! Optional Windows CPU/NUMA topology probes.

#![allow(unsafe_code)]

use core::ffi::c_void;

type Word = u16;
type Dword = u32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetActiveProcessorCount(group: Word) -> Dword;
    fn GetNumaHighestNodeNumber(highest_node_number: *mut u8) -> i32;
    fn GetCurrentProcessorNumber() -> Dword;
    fn SetThreadAffinityMask(thread: *mut c_void, mask: usize) -> usize;
    fn GetCurrentThread() -> *mut c_void;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuTopology {
    pub logical_processors: u32,
    pub highest_numa_node: u8,
}

impl CpuTopology {
    /// Reads topology without changing process or thread state.
    pub fn query() -> Self {
        let mut highest = 0;
        // SAFETY: Windows writes one byte to a valid local output pointer.
        let numa_ok = unsafe { GetNumaHighestNodeNumber(&mut highest) != 0 };
        // GROUP_ALL_PROCESSOR_GROUPS is 0xffff.
        // SAFETY: this query has no mutable state and uses the documented group.
        let processors = unsafe { GetActiveProcessorCount(u16::MAX) };
        Self {
            logical_processors: processors,
            highest_numa_node: if numa_ok { highest } else { 0 },
        }
    }

    /// Returns the current processor index for diagnostics.
    pub fn current_processor() -> u32 {
        // SAFETY: direct read-only Win32 query.
        unsafe { GetCurrentProcessorNumber() }
    }

    /// Applies a single-word affinity mask to the current thread.
    pub fn pin_current_thread(mask: usize) -> bool {
        if mask == 0 {
            return false;
        }
        // SAFETY: current-thread pseudo handle and caller-provided nonzero mask
        // are valid for SetThreadAffinityMask.
        unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) != 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_query_returns_a_nonnegative_shape() {
        let topology = CpuTopology::query();
        assert!(topology.logical_processors > 0);
    }

    #[test]
    fn zero_affinity_mask_is_rejected_before_ffi() {
        assert!(!CpuTopology::pin_current_thread(0));
    }
}
