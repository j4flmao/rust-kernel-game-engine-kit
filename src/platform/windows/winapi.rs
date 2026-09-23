//! Minimal Win32 file I/O PAL. No external Windows crate is required.
#![allow(unsafe_code)]

use std::ffi::CString;

type Handle = *mut core::ffi::c_void;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
const GENERIC_READ: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

extern "system" {
    fn CreateFileA(
        path: *const u8,
        access: u32,
        share: u32,
        security: *const core::ffi::c_void,
        creation: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn ReadFile(
        handle: Handle,
        buffer: *mut u8,
        count: u32,
        read: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
    fn GetFileSizeEx(handle: Handle, size: *mut i64) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn GetLastError() -> u32;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinApiError {
    pub op: &'static str,
    pub code: u32,
    pub path: Option<String>,
}

impl core::fmt::Display for WinApiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.path {
            Some(path) => write!(
                f,
                "{} failed for '{}' (Win32 error {})",
                self.op, path, self.code
            ),
            None => write!(f, "{} failed (Win32 error {})", self.op, self.code),
        }
    }
}
impl core::error::Error for WinApiError {}

pub struct FileHandle {
    raw: Handle,
    path: String,
}

impl FileHandle {
    pub fn open_read(path: &str) -> Result<Self, WinApiError> {
        let cpath = CString::new(path).map_err(|_| WinApiError {
            op: "CreateFileA",
            code: 87,
            path: Some(path.to_owned()),
        })?;
        // SAFETY: cpath is a valid NUL-terminated path for the duration of the call.
        let raw = unsafe {
            CreateFileA(
                cpath.as_ptr().cast(),
                GENERIC_READ,
                FILE_SHARE_READ,
                core::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                core::ptr::null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(WinApiError {
                op: "CreateFileA",
                code: unsafe { GetLastError() },
                path: Some(path.to_owned()),
            });
        }
        Ok(Self {
            raw,
            path: path.to_owned(),
        })
    }

    pub fn size_bytes(&self) -> Result<u64, WinApiError> {
        let mut size = 0;
        // SAFETY: size is writable and raw is an owned valid file handle.
        if unsafe { GetFileSizeEx(self.raw, &mut size) } == 0 || size < 0 {
            return Err(self.error("GetFileSizeEx"));
        }
        Ok(size as u64)
    }

    pub fn read_to_end(&self) -> Result<Vec<u8>, WinApiError> {
        let size = self.size_bytes()?;
        let capacity = usize::try_from(size).map_err(|_| self.error("read size overflow"))?;
        let mut output = Vec::with_capacity(capacity);
        let mut buffer = [0u8; 8192];
        loop {
            let mut read = 0;
            // SAFETY: buffer is writable and the handle is synchronous.
            if unsafe {
                ReadFile(
                    self.raw,
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    &mut read,
                    core::ptr::null_mut(),
                )
            } == 0
            {
                return Err(self.error("ReadFile"));
            }
            if read == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..read as usize]);
        }
        Ok(output)
    }

    fn error(&self, op: &'static str) -> WinApiError {
        WinApiError {
            op,
            code: unsafe { GetLastError() },
            path: Some(self.path.clone()),
        }
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // SAFETY: raw is owned and closed exactly once.
        let _ = unsafe { CloseHandle(self.raw) };
    }
}

pub fn read_file(path: &str) -> Result<Vec<u8>, WinApiError> {
    FileHandle::open_read(path)?.read_to_end()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_binary_file() {
        let path = std::env::temp_dir().join(format!("rust-kernel-win-{}.bin", std::process::id()));
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"a\0b\xff")
            .unwrap();
        let file = FileHandle::open_read(path.to_str().unwrap()).unwrap();
        assert_eq!(file.size_bytes().unwrap(), 4);
        assert_eq!(file.read_to_end().unwrap(), b"a\0b\xff");
        let _ = std::fs::remove_file(path);
    }
}
