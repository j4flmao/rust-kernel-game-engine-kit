//! IOCP completion port and overlapped file streaming.
#![allow(unsafe_code)]

use core::ffi::c_void;
use std::{
    collections::HashMap,
    ptr,
    sync::{Arc, Mutex},
    vec::Vec,
};

type Handle = *mut c_void;
type Dword = u32;
type Bool = i32;
const INVALID_HANDLE_VALUE: Handle = (-1isize) as Handle;
const GENERIC_READ: Dword = 0x8000_0000;
const FILE_SHARE_READ: Dword = 1;
const OPEN_EXISTING: Dword = 3;
const FILE_FLAG_OVERLAPPED: Dword = 0x4000_0000;
const ERROR_IO_PENDING: Dword = 997;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub bytes: u32,
    pub key: usize,
    pub overlapped: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateIoCompletionPort(file: Handle, existing: Handle, key: usize, threads: Dword)
        -> Handle;
    fn CloseHandle(handle: Handle) -> Bool;
    fn PostQueuedCompletionStatus(
        port: Handle,
        bytes: Dword,
        key: usize,
        overlapped: *mut c_void,
    ) -> Bool;
    fn GetQueuedCompletionStatus(
        port: Handle,
        bytes: *mut Dword,
        key: *mut usize,
        overlapped: *mut *mut c_void,
        timeout: Dword,
    ) -> Bool;
    fn CreateFileW(
        name: *const u16,
        access: Dword,
        share: Dword,
        security: Handle,
        disposition: Dword,
        flags: Dword,
        template: Handle,
    ) -> Handle;
    fn ReadFile(
        file: Handle,
        buffer: *mut u8,
        length: Dword,
        read: *mut Dword,
        overlapped: *mut c_void,
    ) -> Bool;
    fn GetOverlappedResult(
        file: Handle,
        overlapped: *mut Overlapped,
        transferred: *mut Dword,
        wait: Bool,
    ) -> Bool;
    fn GetLastError() -> Dword;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IocpError {
    CreateFailed,
    PostFailed,
    OpenFailed(Dword),
    ReadFailed(Dword),
    CompletionFailed(Dword),
}
impl core::fmt::Display for IocpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CreateFailed => f.write_str("CreateIoCompletionPort failed"),
            Self::PostFailed => f.write_str("PostQueuedCompletionStatus failed"),
            Self::OpenFailed(c) => write!(f, "CreateFileW failed ({c})"),
            Self::ReadFailed(c) => write!(f, "ReadFile failed ({c})"),
            Self::CompletionFailed(c) => write!(f, "GetOverlappedResult failed ({c})"),
        }
    }
}
impl core::error::Error for IocpError {}

pub struct CompletionPort {
    handle: Handle,
    pending: Mutex<HashMap<usize, Box<PendingRead>>>,
}
impl CompletionPort {
    pub fn new(worker_threads: u32) -> Result<Self, IocpError> {
        // SAFETY: INVALID_HANDLE_VALUE requests a new port.
        let handle = unsafe {
            CreateIoCompletionPort(INVALID_HANDLE_VALUE, ptr::null_mut(), 0, worker_threads)
        };
        if handle.is_null() {
            Err(IocpError::CreateFailed)
        } else {
            Ok(Self {
                handle,
                pending: Mutex::new(HashMap::new()),
            })
        }
    }
    pub fn post(&self, completion: Completion) -> Result<(), IocpError> {
        // SAFETY: overlapped is opaque user data.
        let ok = unsafe {
            PostQueuedCompletionStatus(
                self.handle,
                completion.bytes,
                completion.key,
                completion.overlapped as *mut c_void,
            )
        };
        if ok == 0 {
            Err(IocpError::PostFailed)
        } else {
            Ok(())
        }
    }
    pub fn poll(&self, timeout_ms: u32) -> Option<Completion> {
        let (mut bytes, mut key, mut overlapped) = (0, 0, ptr::null_mut()); // SAFETY: output locals are valid.
        let ok = unsafe {
            GetQueuedCompletionStatus(
                self.handle,
                &mut bytes,
                &mut key,
                &mut overlapped,
                timeout_ms,
            )
        };
        (ok != 0 && !overlapped.is_null()).then_some(Completion {
            bytes,
            key,
            overlapped: overlapped as usize,
        })
    }
    pub fn associate(&self, file: &AsyncFile, key: usize) -> Result<(), IocpError> {
        // SAFETY: both handles are live.
        let result = unsafe { CreateIoCompletionPort(file.handle(), self.handle, key, 0) };
        if result.is_null() {
            Err(IocpError::CreateFailed)
        } else {
            Ok(())
        }
    }

    /// Submit a read whose OVERLAPPED and byte buffer remain owned by the port
    /// until the matching completion is consumed.
    pub fn read_at(
        &self,
        file: &AsyncFile,
        offset: u64,
        length: usize,
        key: usize,
    ) -> Result<usize, IocpError> {
        let mut pending = file.prepare_read(offset, length, key)?;
        let overlapped = (&mut pending.overlapped as *mut Overlapped) as usize;
        let mut immediate = 0;
        let ok = unsafe {
            ReadFile(
                file.handle(),
                pending.buffer.as_mut_ptr(),
                length as Dword,
                &mut immediate,
                (overlapped as *mut Overlapped).cast(),
            )
        };
        if ok == 0 {
            let code = unsafe { GetLastError() };
            if code != ERROR_IO_PENDING {
                return Err(IocpError::ReadFailed(code));
            }
        }
        self.pending
            .lock()
            .map_err(|_| IocpError::CompletionFailed(6))?
            .insert(overlapped, pending);
        Ok(overlapped)
    }

    /// Consume a completed read and return exactly the bytes reported by
    /// Windows. The request cannot be dropped early by the caller.
    pub fn complete_read(&self, overlapped: usize) -> Result<Vec<u8>, IocpError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| IocpError::CompletionFailed(6))?
            .remove(&overlapped)
            .ok_or(IocpError::CompletionFailed(6))?;
        pending.finish(0)
    }
}
impl Drop for CompletionPort {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.pending.lock() {
            for (_, request) in pending.drain() {
                let _ = request.finish(1);
            }
        }
        // SAFETY: owned kernel handle.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub struct AsyncFile {
    inner: Arc<FileHandle>,
}
impl AsyncFile {
    pub fn open(path: &str) -> Result<Self, IocpError> {
        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0); // SAFETY: NUL-terminated UTF-16 lives through call.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(IocpError::OpenFailed(unsafe { GetLastError() }))
        } else {
            Ok(Self {
                inner: Arc::new(FileHandle { handle }),
            })
        }
    }

    fn handle(&self) -> Handle {
        self.inner.handle
    }

    fn prepare_read(
        &self,
        offset: u64,
        length: usize,
        key: usize,
    ) -> Result<Box<PendingRead>, IocpError> {
        if length > Dword::MAX as usize {
            return Err(IocpError::ReadFailed(87));
        }
        let pending = Box::new(PendingRead {
            file: Arc::clone(&self.inner),
            key,
            buffer: vec![0; length],
            overlapped: Overlapped {
                offset: offset as u32,
                offset_high: (offset >> 32) as u32,
                ..Default::default()
            },
        });
        Ok(pending)
    }
}

struct FileHandle {
    handle: Handle,
}
// Windows kernel HANDLE values are process-wide references. The handle is
// closed only when the last Arc owner is dropped, so sharing this wrapper
// across the completion worker and caller is safe.
unsafe impl Send for FileHandle {}
unsafe impl Sync for FileHandle {}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // SAFETY: owned handle.
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub struct PendingRead {
    file: Arc<FileHandle>,
    key: usize,
    buffer: Vec<u8>,
    overlapped: Overlapped,
}
impl PendingRead {
    pub fn key(&self) -> usize {
        self.key
    }
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
    #[allow(clippy::boxed_local)]
    fn finish(mut self: Box<Self>, wait: Bool) -> Result<Vec<u8>, IocpError> {
        let mut transferred = 0; // SAFETY: request owns its OVERLAPPED until consumed.
        let ok = unsafe {
            GetOverlappedResult(
                self.file.handle,
                &mut self.overlapped,
                &mut transferred,
                wait,
            )
        };
        if ok == 0 {
            return Err(IocpError::CompletionFailed(unsafe { GetLastError() }));
        }
        self.buffer.truncate(transferred as usize);
        Ok(self.buffer)
    }

    #[deprecated(
        note = "use CompletionPort::read_at and complete_read to retain the OVERLAPPED safely"
    )]
    #[allow(clippy::boxed_local)]
    pub fn complete(self: Box<Self>) -> Result<Vec<u8>, IocpError> {
        self.finish(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn post_and_poll_roundtrip_preserves_completion() {
        let port = CompletionPort::new(1).unwrap();
        let expected = Completion {
            bytes: 4096,
            key: 7,
            overlapped: 0x1234,
        };
        port.post(expected).unwrap();
        assert_eq!(port.poll(100), Some(expected));
    }

    #[test]
    fn overlapped_file_read_roundtrip() {
        use std::fs;
        let path =
            std::env::temp_dir().join(format!("rust-kernel-iocp-{}.bin", std::process::id()));
        fs::write(&path, b"native async read").unwrap();
        let file = AsyncFile::open(path.to_str().unwrap()).unwrap();
        let port = CompletionPort::new(1).unwrap();
        port.associate(&file, 42).unwrap();
        let token = port.read_at(&file, 0, 17, 42).unwrap();
        let completion = port.poll(5_000).expect("ReadFile completion");
        assert_eq!(completion.key, 42);
        assert_eq!(completion.overlapped, token);
        assert_eq!(port.complete_read(token).unwrap(), b"native async read");
        drop(file);
        fs::remove_file(path).unwrap();
    }
}
