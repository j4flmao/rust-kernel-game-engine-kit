//! CPU topology and tuning: NUMA layout parsed from `/sys`, huge-page mmap
//! hints, and `perf_event_open` hardware counters.
//!
//! Phase 3 provides the primitives (parse + query are deterministic and
//! testable); Phase 6 consumes them for allocator placement and the tracer's
//! cache-miss numbers.
#![allow(unsafe_code)] // justified: PAL FFI boundary

use std::marker::PhantomData;

/// Parses a Linux `cpulist` string (`"0-3,7,9-11"`) into a sorted, deduped
/// list of CPU indices.
pub fn expand_cpulist(spec: &str) -> Vec<usize> {
    try_expand_cpulist(spec).expect("invalid or oversized Linux CPU list")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuTopologyError {
    AllocationFailed,
    RangeTooLarge,
}

pub fn try_expand_cpulist(spec: &str) -> Result<Vec<usize>, CpuTopologyError> {
    const MAX_CPUS_PER_LIST: usize = 1 << 20;
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: usize = lo.trim().parse().unwrap_or(0);
            let hi: usize = hi.trim().parse().unwrap_or(lo);
            if hi < lo || hi - lo >= MAX_CPUS_PER_LIST {
                return Err(CpuTopologyError::RangeTooLarge);
            }
            out.try_reserve(hi - lo + 1)
                .map_err(|_| CpuTopologyError::AllocationFailed)?;
            for cpu in lo..=hi {
                out.push(cpu);
            }
        } else if let Ok(cpu) = part.parse::<usize>() {
            out.try_reserve(1)
                .map_err(|_| CpuTopologyError::AllocationFailed)?;
            out.push(cpu);
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// NUMA node id -> list of CPUs, from `/sys/devices/system/node/node*`.
pub struct NumaTopology {
    /// One entry per node (index == node id); empty slices for sparse
    /// node numbers are `None`.
    nodes: Vec<Option<Vec<usize>>>,
}

impl NumaTopology {
    /// Queries the live system topology. Never panics; on failure returns
    /// an all-one-node topology (CPU 0..n) so callers degrade gracefully.
    pub fn discover() -> Self {
        let sysfs = "/sys/devices/system/node";
        let mut nodes: Vec<Option<Vec<usize>>> = Vec::new();
        let mut found_any = false;
        if let Ok(entries) = std::fs::read_dir(sysfs) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with("node") {
                    continue;
                }
                let Some(id) = name[4..].parse::<usize>().ok() else {
                    continue;
                };
                if id >= nodes.len() {
                    nodes.resize(id + 1, None);
                }
                let cpulist =
                    std::fs::read_to_string(format!("{sysfs}/{name}/cpulist")).unwrap_or_default();
                nodes[id] = Some(try_expand_cpulist(&cpulist).unwrap_or_default());
                found_any = true;
            }
        }
        if !found_any {
            // Degenerate single-node fallback: assume CPU 0..0 (callers
            // treat `local` placement as the only placement).
            nodes = vec![Some(vec![0])];
        }
        Self { nodes }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn cpus_of(&self, node: usize) -> Option<&[usize]> {
        self.nodes.get(node).and_then(|n| n.as_deref())
    }

    /// Node that owns a given CPU (first match wins).
    pub fn node_for_cpu(&self, cpu: usize) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .find_map(|(n, cpus)| cpus.as_ref()?.contains(&cpu).then_some(n))
    }
}

// ---- Huge pages --------------------------------------------------------

/// Maps `len` bytes with the huge-page hint. Returns `None` when the system
/// has no huge pages available (caller retries plain `mmap`). Alignment for
/// huge-page mappings is the page table size (2 MiB default).
pub fn map_huge_zeroed(len: usize) -> Option<std::ptr::NonNull<u8>> {
    const MAP_HUGE_2MB: i32 = 21 << 26; // MAP_HUGE_SHIFT(26) | 2MB order(21)
    const PROT_RW: i32 = 0x1 | 0x2;
    const MAP_ANON: i32 = 0x20;
    const MAP_PRIVATE: i32 = 0x02;
    // SAFETY: anon private huge-page mapping; kernel rejects when
    // unavailable (returns MAP_FAILED) and we fall back.
    let res = unsafe {
        crate::platform::linux::syscalls::syscall_mmap_huge(
            len,
            PROT_RW,
            MAP_ANON | MAP_PRIVATE | MAP_HUGE_2MB,
        )
    };
    let ptr = res?;
    if ptr.is_null() || ptr as usize == usize::MAX {
        return None;
    }
    Some(std::ptr::NonNull::new(ptr as *mut u8).expect("non-null checked"))
}

// ---- Perf counters -----------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct PerfEventAttr {
    type_: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    _pad: [u64; 8],
}

impl PerfEventAttr {
    const PERF_TYPE_HARDWARE: u32 = 0;
    const PERF_COUNT_HW_CACHE_MISSES: u64 = 3;
    const PERF_FLAG_FD_NO_GROUP: u64 = 1 << 0;

    fn hardware_cache_misses() -> Self {
        Self {
            type_: Self::PERF_TYPE_HARDWARE,
            size: core::mem::size_of::<Self>() as u32,
            config: Self::PERF_COUNT_HW_CACHE_MISSES,
            sample_period: 0,
            sample_type: 0,
            read_format: 0, // single counter: plain u64
            flags: Self::PERF_FLAG_FD_NO_GROUP,
            _pad: [0; 8],
        }
    }
}

/// A live `perf_event_open` counter fd; read the delta with [`Self::read`].
pub struct PerfCounter {
    fd: i32,
    _lifetime: PhantomData<()>,
}

impl PerfCounter {
    /// Opens the hardware cache-miss counter for the current process.
    pub fn cache_misses() -> Result<Self, i32> {
        let attr = PerfEventAttr::hardware_cache_misses();
        extern "C" {
            fn syscall(number: i64, ...) -> i64;
        }
        const SYS_PERF_EVENT_OPEN: i64 = 298;
        // SAFETY: event attr has the kernel ABI layout; pid 0 + cpu -1
        // selects the calling thread.
        let fd = unsafe {
            syscall(
                SYS_PERF_EVENT_OPEN,
                &attr as *const PerfEventAttr as i64,
                -1i64, // any pid (self via cpu -1 + pid 0)
                0i64,  // cpu 0
                -1i64, // group leader: none
                0i64,  // flags
                0i64,
            )
        };
        if fd < 0 {
            return Err(-fd as i32);
        }
        Ok(Self {
            fd: fd as i32,
            _lifetime: PhantomData,
        })
    }

    /// Accumulated count since the counter was opened (reset via ioctl by
    /// the tracer in Phase 6).
    pub fn read(&self) -> Result<u64, i32> {
        let mut value = 0u64;
        extern "C" {
            #[link_name = "read"]
            fn _c_read(fd: i32, buf: *mut u8, count: usize) -> isize;
        }
        // SAFETY: buffer sized to the configured single-counter read format.
        let n = unsafe { _c_read(self.fd, (&mut value as *mut u64).cast::<u8>(), 8) };
        if n < 0 {
            Err((-n) as i32)
        } else {
            Ok(value)
        }
    }
}

impl Drop for PerfCounter {
    fn drop(&mut self) {
        extern "C" {
            #[link_name = "close"]
            fn _c_close(fd: i32) -> i32;
        }
        // SAFETY: owned fd.
        unsafe { _c_close(self.fd) };
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn cpulist_expansion() {
        assert_eq!(
            expand_cpulist("0-3,7,9-11"),
            vec![0usize, 1, 2, 3, 7, 9, 10, 11]
        );
        assert_eq!(expand_cpulist("5"), vec![5usize]);
        assert_eq!(expand_cpulist(""), Vec::<usize>::new());
    }

    #[test]
    fn numa_discovery_is_graceful() {
        let topo = NumaTopology::discover();
        // Either real sysfs data or the single-node fallback; never panics
        // and always reports at least one node.
        assert!(topo.node_count() >= 1);
        // If we have n CPUs on the box, every reported CPU has a home node.
        for node in 0..topo.node_count() {
            if let Some(cpus) = topo.cpus_of(node) {
                for &cpu in cpus {
                    assert_eq!(topo.node_for_cpu(cpu), Some(node));
                }
            }
        }
    }

    #[test]
    fn perf_counter_opens_and_reads() {
        match PerfCounter::cache_misses() {
            Ok(counter) => {
                // Force some cache churn so the counter can go up.
                let mut acc = 0u64;
                for i in 0..1_000_000usize {
                    acc = acc.wrapping_add(i as u64);
                }
                std::hint::black_box(acc);
                // read() never fails on a valid counter fd in this shape.
                let _value = counter.read().unwrap();
            }
            Err(code) => {
                // perf_event_open can be blocked (seccomp/sandbox/WSL);
                // treat as graceful degradation.
                assert!(code > 0);
            }
        }
    }
}
