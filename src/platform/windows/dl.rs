//! Minimal Windows dynamic-library loader for PAL/GPU function tables.
#![allow(unsafe_code)]

use std::ffi::{c_void, CString};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlError {
    InvalidName,
    Open { library: String, code: u32 },
    Symbol { symbol: String, code: u32 },
}

impl std::fmt::Display for DlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName => write!(f, "library or symbol name contains NUL"),
            Self::Open { library, code } => {
                write!(f, "LoadLibraryA failed for '{library}' (error {code})")
            }
            Self::Symbol { symbol, code } => {
                write!(f, "GetProcAddress failed for '{symbol}' (error {code})")
            }
        }
    }
}

impl std::error::Error for DlError {}

type Hmodule = *mut c_void;

extern "system" {
    fn LoadLibraryA(name: *const u8) -> Hmodule;
    fn GetProcAddress(module: Hmodule, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: Hmodule) -> i32;
    fn GetLastError() -> u32;
}

#[derive(Debug)]
pub struct DynamicLibrary {
    handle: Hmodule,
    name: String,
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    pub fn open(name: &str) -> Result<Self, DlError> {
        let cname = CString::new(name).map_err(|_| DlError::InvalidName)?;
        // SAFETY: CString is NUL-terminated and lives through the call.
        let handle = unsafe { LoadLibraryA(cname.as_ptr().cast()) };
        if handle.is_null() {
            return Err(DlError::Open {
                library: name.to_owned(),
                code: unsafe { GetLastError() },
            });
        }
        Ok(Self {
            handle,
            name: name.to_owned(),
        })
    }

    pub fn symbol<T>(&self, name: &str) -> Result<Symbol<T>, DlError> {
        let cname = CString::new(name).map_err(|_| DlError::InvalidName)?;
        // SAFETY: handle is live and name is a valid NUL-terminated string.
        let ptr = unsafe { GetProcAddress(self.handle, cname.as_ptr().cast()) };
        if ptr.is_null() {
            return Err(DlError::Symbol {
                symbol: name.to_owned(),
                code: unsafe { GetLastError() },
            });
        }
        Ok(Symbol {
            ptr,
            marker: core::marker::PhantomData,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: handle was returned by LoadLibraryA and is owned here.
        let _ = unsafe { FreeLibrary(self.handle) };
    }
}

pub struct Symbol<T> {
    ptr: *mut c_void,
    marker: core::marker::PhantomData<T>,
}

impl<T> Symbol<T> {
    /// Converts the address into the caller-declared function type.
    ///
    /// # Safety
    /// `T` must exactly match the exported function ABI and signature.
    pub unsafe fn as_fn(self) -> T {
        // SAFETY: required by this function's contract; function pointers are
        // represented by the same pointer-sized value on supported Windows.
        unsafe { core::mem::transmute_copy(&self.ptr) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_kernel32_export() {
        let library = DynamicLibrary::open("kernel32.dll").expect("kernel32 must exist");
        let symbol: Symbol<unsafe extern "system" fn() -> u32> = library
            .symbol("GetCurrentProcessId")
            .expect("export must exist");
        // SAFETY: signature matches the documented Win32 export.
        let get_pid = unsafe { symbol.as_fn() };
        // SAFETY: function pointer came from a validated system DLL export.
        let pid = unsafe { get_pid() };
        assert_ne!(pid, 0);
    }

    #[test]
    fn missing_library_reports_error() {
        let error = DynamicLibrary::open("rust_kernel_missing_library_9f8c.dll").unwrap_err();
        assert!(matches!(error, DlError::Open { .. }));
    }
}
