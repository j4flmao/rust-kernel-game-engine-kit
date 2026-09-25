//! Owned Linux io_uring boundary for asynchronous asset reads.
#![allow(unsafe_code)]

use std::{ffi::c_void, io, ptr};

const SYS_SETUP: usize = 425;
const SYS_ENTER: usize = 426;
const OFF_SQ_RING: i64 = 0;
const OFF_CQ_RING: i64 = 0x8000_0000;
const OFF_SQES: i64 = 0x1_0000_0000;
const ENTER_GETEVENTS: u32 = 1;
const OP_READ: u8 = 22;
const FEAT_SINGLE_MMAP: u32 = 1 << 0;
const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Offset {
    head: u32,
    tail: u32,
    ring_mask: u32,
    ring_entries: u32,
    flags: u32,
    dropped: u32,
    array: u32,
    resv1: u32,
    resv2: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Params {
    sq_entries: u32,
    cq_entries: u32,
    flags: u32,
    sq_thread_cpu: u32,
    sq_thread_idle: u32,
    features: u32,
    wq_fd: u32,
    resv: [u32; 3],
    sq_off: Offset,
    cq_off: Offset,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SubmissionEntry {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub rw_flags: u32,
    pub user_data: u64,
    pub extra: [u64; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompletionEntry {
    pub user_data: u64,
    pub result: i32,
    pub flags: u32,
}

/// Owns the read buffer until the kernel reports its CQE. Dropping a caller's
/// original slice while an io_uring operation is in flight is otherwise a
/// kernel-write-after-free hazard.
pub struct ReadRequest {
    user_data: u64,
}

impl ReadRequest {
    pub fn user_data(&self) -> u64 {
        self.user_data
    }
}

struct PendingRead {
    user_data: u64,
    buffer: Vec<u8>,
}

unsafe extern "C" {
    fn syscall(number: i64, ...) -> i64;
    fn close(fd: i32) -> i32;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
}

#[derive(Debug)]
pub enum IoUringError {
    InvalidEntries,
    InvalidRead,
    ReadTooLarge,
    Setup(io::Error),
    Mmap(io::Error),
    QueueFull,
    CompletionUnknown,
    Enter(io::Error),
}
impl core::fmt::Display for IoUringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidEntries => f.write_str("io_uring entries must be nonzero"),
            Self::InvalidRead => {
                f.write_str("io_uring read requires a valid fd and non-empty buffer")
            }
            Self::ReadTooLarge => f.write_str("io_uring read buffer exceeds the u32 length field"),
            Self::Setup(e) => write!(f, "io_uring_setup failed: {e}"),
            Self::Mmap(e) => write!(f, "io_uring mmap failed: {e}"),
            Self::QueueFull => f.write_str("io_uring submission queue is full"),
            Self::CompletionUnknown => f.write_str("io_uring completion has no owned request"),
            Self::Enter(e) => write!(f, "io_uring_enter failed: {e}"),
        }
    }
}
impl core::error::Error for IoUringError {}

pub struct IoUring {
    fd: i32,
    params: Params,
    sq_ring: *mut u8,
    sq_ring_len: usize,
    cq_ring: *mut u8,
    cq_ring_len: usize,
    single_mmap: bool,
    sqes: *mut SubmissionEntry,
    sqes_len: usize,
    pending: Vec<PendingRead>,
    completed: Vec<(CompletionEntry, Vec<u8>)>,
}

impl IoUring {
    pub fn new(entries: u32) -> Result<Self, IoUringError> {
        if entries == 0 {
            return Err(IoUringError::InvalidEntries);
        }
        let mut params = Params::default();
        // SAFETY: arguments match the Linux io_uring_setup ABI.
        let raw = unsafe { syscall(SYS_SETUP as i64, entries, &mut params) };
        if raw < 0 {
            return Err(IoUringError::Setup(io::Error::from_raw_os_error(
                (-raw) as i32,
            )));
        }
        let fd = raw as i32;
        let sq_ring_len = params.sq_off.array as usize + params.sq_entries as usize * 4;
        // In the CQ layout the field occupying `dropped`'s slot is `cqes`.
        let cq_ring_len = params.cq_off.dropped as usize
            + params.cq_entries as usize * core::mem::size_of::<CompletionEntry>();
        let sqes_len = params.sq_entries as usize * core::mem::size_of::<SubmissionEntry>();
        // IORING_FEAT_SINGLE_MMAP means SQ and CQ share one mapping. The
        // kernel still exposes separate offsets inside that mapping, so the
        // ring pointers intentionally remain the same base address here.
        let single_mmap = params.features & FEAT_SINGLE_MMAP != 0;
        let shared_ring_len = sq_ring_len.max(cq_ring_len);
        // SAFETY: kernel-provided offsets/sizes describe the shared mappings.
        let sq_ring = unsafe {
            mmap(
                ptr::null_mut(),
                if single_mmap {
                    shared_ring_len
                } else {
                    sq_ring_len
                },
                3,
                1,
                fd,
                OFF_SQ_RING,
            )
        };
        let cq_ring = if single_mmap {
            sq_ring
        } else {
            // SAFETY: the CQ mapping is a distinct kernel-provided region.
            unsafe { mmap(ptr::null_mut(), cq_ring_len, 3, 1, fd, OFF_CQ_RING) }
        };
        let sqes = unsafe { mmap(ptr::null_mut(), sqes_len, 3, 1, fd, OFF_SQES) };
        if [sq_ring, cq_ring, sqes].contains(&MAP_FAILED) {
            if sq_ring != MAP_FAILED {
                unsafe {
                    munmap(
                        sq_ring,
                        if single_mmap {
                            shared_ring_len
                        } else {
                            sq_ring_len
                        },
                    );
                }
            }
            if !single_mmap && cq_ring != MAP_FAILED {
                unsafe {
                    munmap(cq_ring, cq_ring_len);
                }
            }
            if sqes != MAP_FAILED {
                unsafe {
                    munmap(sqes, sqes_len);
                }
            }
            unsafe {
                close(fd);
            }
            return Err(IoUringError::Mmap(io::Error::from_raw_os_error(12)));
        }
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(params.sq_entries as usize)
            .map_err(|_| IoUringError::Mmap(io::Error::from_raw_os_error(12)))?;
        let mut completed = Vec::new();
        completed
            .try_reserve_exact(params.cq_entries as usize)
            .map_err(|_| IoUringError::Mmap(io::Error::from_raw_os_error(12)))?;
        Ok(Self {
            fd,
            params,
            sq_ring: sq_ring.cast(),
            sq_ring_len: if single_mmap {
                shared_ring_len
            } else {
                sq_ring_len
            },
            cq_ring: cq_ring.cast(),
            cq_ring_len: if single_mmap { 0 } else { cq_ring_len },
            single_mmap,
            sqes: sqes.cast(),
            sqes_len,
            pending,
            completed,
        })
    }
    pub const fn raw_fd(&self) -> i32 {
        self.fd
    }
    pub const fn entries(&self) -> u32 {
        self.params.sq_entries
    }
    pub fn submit_read(
        &mut self,
        fd: i32,
        mut buffer: Vec<u8>,
        offset: u64,
        user_data: u64,
    ) -> Result<ReadRequest, IoUringError> {
        if fd < 0 || buffer.is_empty() {
            return Err(IoUringError::InvalidRead);
        }
        let length = u32::try_from(buffer.len()).map_err(|_| IoUringError::ReadTooLarge)?;
        if self.pending.len() + self.completed.len() >= self.params.cq_entries as usize {
            return Err(IoUringError::QueueFull);
        }
        // SAFETY: ring pointers are valid shared mappings owned by self.
        unsafe {
            let head = self.sq_u32(self.params.sq_off.head).read_volatile();
            let tail_ptr = self.sq_u32(self.params.sq_off.tail);
            let tail = tail_ptr.read_volatile();
            if tail.wrapping_sub(head) >= self.params.sq_entries {
                return Err(IoUringError::QueueFull);
            }
            let index = tail & self.sq_u32(self.params.sq_off.ring_mask).read_volatile();
            *self.sqes.add(index as usize) = SubmissionEntry {
                opcode: OP_READ,
                fd,
                off: offset,
                addr: buffer.as_mut_ptr() as u64,
                len: length,
                user_data,
                ..Default::default()
            };
            self.sq_u32(self.params.sq_off.array)
                .add(index as usize)
                .write_volatile(index);
            tail_ptr.write_volatile(tail.wrapping_add(1));
        }
        self.pending.push(PendingRead { user_data, buffer });
        Ok(ReadRequest { user_data })
    }
    pub fn enter(&self, submit: u32, wait_for: u32) -> Result<u32, IoUringError> {
        let flags = if wait_for != 0 { ENTER_GETEVENTS } else { 0 }; // SAFETY: descriptor and syscall arguments are owned/validated.
        let raw = unsafe {
            syscall(
                SYS_ENTER as i64,
                self.fd,
                submit,
                wait_for,
                flags,
                ptr::null::<u8>(),
                0,
            )
        };
        if raw < 0 {
            Err(IoUringError::Enter(io::Error::from_raw_os_error(
                (-raw) as i32,
            )))
        } else {
            Ok(raw as u32)
        }
    }
    pub fn try_complete(&mut self) -> Option<CompletionEntry> {
        // SAFETY: volatile accesses follow the CQ ring protocol.
        unsafe {
            let head_ptr = self.cq_u32(self.params.cq_off.head);
            let head = head_ptr.read_volatile();
            let tail = self.cq_u32(self.params.cq_off.tail).read_volatile();
            if head == tail {
                return None;
            }
            let index = head & self.cq_u32(self.params.cq_off.ring_mask).read_volatile();
            let entry = self.cqes().add(index as usize).read_volatile();
            head_ptr.write_volatile(head.wrapping_add(1));
            if let Some(position) = self
                .pending
                .iter()
                .position(|request| request.user_data == entry.user_data)
            {
                let mut request = self.pending.swap_remove(position);
                if entry.result >= 0 {
                    request.buffer.truncate(entry.result as usize);
                } else {
                    request.buffer.clear();
                }
                self.completed.push((entry, request.buffer));
            }
            Some(entry)
        }
    }

    /// Takes the buffer retained for a completed CQE. The ring owns the
    /// allocation until this call, even if the original request token was
    /// dropped immediately after submission.
    pub fn take_completed(&mut self, user_data: u64) -> Result<Vec<u8>, IoUringError> {
        let position = self
            .completed
            .iter()
            .position(|(entry, _)| entry.user_data == user_data)
            .ok_or(IoUringError::CompletionUnknown)?;
        Ok(self.completed.swap_remove(position).1)
    }
    unsafe fn sq_u32(&self, offset: u32) -> *mut u32 {
        unsafe { self.sq_ring.add(offset as usize).cast() }
    }
    unsafe fn cq_u32(&self, offset: u32) -> *mut u32 {
        unsafe { self.cq_ring.add(offset as usize).cast() }
    }
    unsafe fn cqes(&self) -> *mut CompletionEntry {
        unsafe { self.cq_ring.add(self.params.cq_off.dropped as usize).cast() }
    }
}
impl Drop for IoUring {
    fn drop(&mut self) {
        // SAFETY: mappings and descriptor are exclusively owned.
        unsafe {
            munmap(self.sq_ring.cast(), self.sq_ring_len);
            if !self.single_mmap {
                munmap(self.cq_ring.cast(), self.cq_ring_len);
            }
            munmap(self.sqes.cast(), self.sqes_len);
            close(self.fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_entries_rejected_before_syscall() {
        assert!(matches!(IoUring::new(0), Err(IoUringError::InvalidEntries)));
    }

    #[test]
    fn invalid_read_inputs_are_rejected_before_queue_access() {
        let mut ring = core::mem::ManuallyDrop::new(IoUring {
            fd: -1,
            params: Params::default(),
            sq_ring: core::ptr::null_mut(),
            sq_ring_len: 0,
            cq_ring: core::ptr::null_mut(),
            cq_ring_len: 0,
            single_mmap: false,
            sqes: core::ptr::null_mut(),
            sqes_len: 0,
            pending: Vec::new(),
            completed: Vec::new(),
        });
        assert!(matches!(
            ring.submit_read(-1, Vec::new(), 0, 1),
            Err(IoUringError::InvalidRead)
        ));
    }
}
