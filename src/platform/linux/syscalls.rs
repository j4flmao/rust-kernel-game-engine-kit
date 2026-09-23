//! Raw syscall FFI: `open`/`read`/`write`/`lseek`/`mmap`/`munmap`/`close`.
//!
//! Hand-written `extern "C"` bindings (no `libc` crate), signatures matched
//! against `man 2`. Safe wrappers keep the unsafe surface small and audited.
#![allow(unsafe_code)] // justified: PAL FFI boundary

use std::ffi::CString;
use std::io;

pub const O_RDONLY: i32 = 0;
pub const O_CLOEXEC: i32 = 0o2000000;

pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const MAP_POPULATE: i32 = 0x008000;
pub const MAP_HUGETLB: i32 = 0x40000;

pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;
pub const SEEK_SET: i32 = 0;

extern "C" {
    fn open(path: *const i8, flags: i32, mode: u32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
    fn close(fd: i32) -> i32;
    fn mmap(
        addr: *mut core::ffi::c_void,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut core::ffi::c_void;
    fn munmap(addr: *mut core::ffi::c_void, length: usize) -> i32;

    /// glibc errno accessor used to attribute -1 returns.
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut i32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysErrno {
    /// Retry (signal interrupted the call).
    Interrupted,
    /// Resource temporarily unavailable, retry later.
    WouldBlock,
    Other(i32),
}

#[derive(Debug)]
pub struct SysError {
    pub errno: SysErrno,
    pub op: &'static str,
    pub path: Option<String>,
}

impl SysError {
    fn errno() -> i32 {
        // SAFETY: glibc guarantees a valid thread-local errno pointer.
        unsafe { *errno_location() }
    }

    fn new(op: &'static str, path: Option<&str>) -> Self {
        let raw = Self::errno();
        let errno = match raw {
            4 => SysErrno::Interrupted,
            11 => SysErrno::WouldBlock,
            other => SysErrno::Other(other),
        };
        Self {
            errno,
            op,
            path: path.map(str::to_owned),
        }
    }
}

impl core::fmt::Display for SysError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{} failed on '{}' (errno {:?})", self.op, p, self.errno),
            None => write!(f, "{} failed (errno {:?})", self.op, self.errno),
        }
    }
}

impl core::error::Error for SysError {}

impl From<SysError> for io::Error {
    fn from(err: SysError) -> Self {
        match err.errno {
            SysErrno::Interrupted => io::Error::new(io::ErrorKind::Interrupted, err.op),
            SysErrno::WouldBlock => io::Error::new(io::ErrorKind::WouldBlock, err.op),
            SysErrno::Other(code) => io::Error::from_raw_os_error(code.max(1)),
        }
    }
}

/// A raw POSIX file descriptor, closed on drop (`close` is idempotent-safe
/// here: the wrapper owns the fd and hands it to `close` exactly once).
pub struct Fd {
    raw: i32,
}

impl Fd {
    pub fn open(path: &str, flags: i32, mode: u32) -> Result<Self, SysError> {
        let cpath = CString::new(path).map_err(|_| SysError {
            errno: SysErrno::Other(0),
            op: "open (CString)",
            path: Some(path.to_owned()),
        })?;
        // SAFETY: `cpath` is a valid NUL-terminated C string for the
        // duration of the call.
        let raw = unsafe { open(cpath.as_ptr(), flags, mode) };
        if raw < 0 {
            return Err(SysError::new("open", Some(path)));
        }
        Ok(Self { raw })
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, SysError> {
        // SAFETY: `buf` is writable for `buf.len()` bytes.
        let n = unsafe { read(self.raw, buf.as_mut_ptr(), buf.len()) };
        if n < 0 {
            return Err(SysError::new("read", None));
        }
        Ok(n as usize)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, SysError> {
        // SAFETY: `buf` is readable for `buf.len()` bytes.
        let n = unsafe { write(self.raw, buf.as_ptr(), buf.len()) };
        if n < 0 {
            return Err(SysError::new("write", None));
        }
        Ok(n as usize)
    }

    pub fn size_bytes(&self) -> Result<u64, SysError> {
        // SAFETY: lseek with SEEK_CUR queries the owned descriptor position.
        let current = unsafe { lseek(self.raw, 0, SEEK_CUR) };
        if current < 0 {
            return Err(SysError::new("lseek (position)", None));
        }
        // SAFETY: the descriptor is owned by this wrapper and SEEK_END is a
        // valid operation for regular files.
        let size = unsafe { lseek(self.raw, 0, SEEK_END) };
        if size < 0 {
            let _ = unsafe { lseek(self.raw, current, SEEK_SET) };
            return Err(SysError::new("lseek (size)", None));
        }
        // Restore the cursor even when the caller is reading incrementally.
        // If restoration fails, report it instead of silently corrupting the
        // caller's subsequent I/O position.
        let restored = unsafe { lseek(self.raw, current, SEEK_SET) };
        if restored < 0 {
            return Err(SysError::new("lseek (restore position)", None));
        }
        Ok(size as u64)
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        // SAFETY: `self.raw` is an owned live fd; close errors (e.g. EBADF
        // after a fork) are not recoverable here.
        unsafe { close(self.raw) };
    }
}

/// Reads a whole file through raw `open`/`read`. Uses std allocation for the
/// result buffer only — this is an init/asset path, never a tick path.
pub fn read_file(path: &str) -> Result<Vec<u8>, SysError> {
    let fd = Fd::open(path, O_RDONLY | O_CLOEXEC, 0)?;
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match fd.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(e) if e.errno == SysErrno::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// A read-only memory mapping (length fixed at construction; `munmap` on
/// drop). Intended for asset files the kernel page cache handles lazily.
pub struct MappedFile {
    ptr: *mut u8,
    len: usize,
}

// The mapping is read-only plain bytes; moving across threads is sound.
unsafe impl Send for MappedFile {}
unsafe impl Sync for MappedFile {}

impl MappedFile {
    /// Maps the first `len` bytes of `fd` read-only (page-cache backed).
    pub fn map_readonly(fd: &Fd, len: u64, path: &str) -> Result<Self, SysError> {
        let len = usize::try_from(len).map_err(|_| SysError {
            errno: SysErrno::Other(0),
            op: "mmap (len overflow)",
            path: Some(path.to_owned()),
        })?;
        if len == 0 {
            return Err(SysError {
                errno: SysErrno::Other(0),
                op: "mmap (zero length)",
                path: Some(path.to_owned()),
            });
        }
        // SAFETY: mmap with MAP_PRIVATE|MAP_POPULATE from an open readable
        // fd; returns MAP_FAILED (-1) on error.
        let ptr = unsafe {
            mmap(
                core::ptr::null_mut(),
                len,
                PROT_READ,
                MAP_PRIVATE | MAP_POPULATE,
                fd.raw,
                0,
            )
        };
        if ptr == usize::MAX as *mut core::ffi::c_void || ptr.is_null() {
            return Err(SysError::new("mmap", Some(path)));
        }
        Ok(Self {
            ptr: ptr as *mut u8,
            len,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: mapping is readable for `len` bytes for the lifetime of
        // `self` (munmap happens only in drop).
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        // SAFETY: `ptr`/`len` are exactly the values returned by mmap above.
        unsafe { munmap(self.ptr as *mut core::ffi::c_void, self.len) };
    }
}

/// Anonymous huge-page mapping (used by `cpu.rs` placement). Returns the
/// base pointer or `MAP_FAILED`/null — the caller decides how to treat the
/// outcome (likely falling back to a normal mapping).
///
/// # Safety
/// Same contract as the `mmap` syscall: caller passes valid constant flags
/// and a length that the kernel can back with huge pages.
#[allow(clippy::missing_safety_doc)]
pub unsafe fn syscall_mmap_huge(
    len: usize,
    prot: i32,
    flags: i32,
) -> Option<*mut core::ffi::c_void> {
    // SAFETY: same contract as the `mmap` syscall — caller passes valid
    // constant flags and a length the kernel can back with huge pages.
    let ptr = unsafe { mmap(core::ptr::null_mut(), len, prot, flags, -1, 0) };
    if ptr.is_null() || ptr == usize::MAX as *mut core::ffi::c_void {
        None
    } else {
        Some(ptr)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(tag: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!(
            "{}/rust_kernel_{tag}_{}_{}.tmp",
            std::env::temp_dir().display(),
            std::process::id(),
            nanos
        )
    }

    #[test]
    fn read_file_roundtrip() {
        let path = temp_path("read");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"hello kernel\x00bytes")
            .unwrap();

        let got = read_file(&path).unwrap();
        assert_eq!(got, b"hello kernel\x00bytes");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fd_size_and_mmap_match_content() {
        let path = temp_path("mmap");
        let content = b"0123456789abcdef";
        std::fs::File::create(&path)
            .unwrap()
            .write_all(content)
            .unwrap();

        let fd = Fd::open(&path, O_RDONLY | O_CLOEXEC, 0).unwrap();
        assert_eq!(fd.size_bytes().unwrap(), content.len() as u64);

        let map = MappedFile::map_readonly(&fd, content.len() as u64, &path).unwrap();
        assert_eq!(map.as_slice(), content);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn size_query_preserves_read_cursor() {
        let path = temp_path("cursor");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"abcdef")
            .unwrap();

        let fd = Fd::open(&path, O_RDONLY | O_CLOEXEC, 0).unwrap();
        let mut prefix = [0u8; 2];
        assert_eq!(fd.read(&mut prefix).unwrap(), 2);
        assert_eq!(fd.size_bytes().unwrap(), 6);
        let mut suffix = [0u8; 4];
        assert_eq!(fd.read(&mut suffix).unwrap(), 4);
        assert_eq!(&prefix, b"ab");
        assert_eq!(&suffix, b"cdef");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_errors() {
        let err = read_file("/nonexistent/definitely-not-here").unwrap_err();
        assert_eq!(err.op, "open");
        assert!(matches!(err.errno, SysErrno::Other(_)));
    }
}
