//! Hand-rolled `dlopen`/`dlsym` loader.
//!
//! The engine loads system shared objects (X11, Vulkan's `libvulkan.so`) at
//! run time instead of linking against them, keeping the "zero external
//! dependency" rule intact while still reaching the full native API — the
//! same technique the official Vulkan loader uses. `Symbol` is a typed
//! pointer to a `extern "C"` function; the caller supplies the signature via
//! the type parameter and we only guarantee the *pointer fetch* is sound.
#![allow(unsafe_code)] // justified: process-level shared-object loading

use std::ffi::{CStr, CString};
use std::marker::PhantomData;

pub enum DlError {
    /// `dlopen` could not load the object (lib missing / deps unresolved).
    Open { lib: String },
    /// `dlsym` could not resolve the symbol name.
    Symbol { lib: String, name: String },
    /// The reason string reported by `dlerror`.
    Detail { reason: String },
}

impl core::fmt::Debug for DlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Open { lib } => write!(f, "dlopen('{lib}') failed"),
            Self::Symbol { lib, name } => write!(f, "dlsym('{name}') in '{lib}' failed"),
            Self::Detail { reason } => write!(f, "loader error: {reason}"),
        }
    }
}

impl core::fmt::Display for DlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl core::error::Error for DlError {}

const RTLD_LAZY: i32 = 0x1;
const RTLD_NOW: i32 = 0x2;

extern "C" {
    fn dlopen(filename: *const i8, flag: i32) -> *mut core::ffi::c_void;
    fn dlsym(handle: *mut core::ffi::c_void, symbol: *const i8) -> *mut core::ffi::c_void;
    fn dlclose(handle: *mut core::ffi::c_void) -> i32;
    fn dlerror() -> *const i8;
}

/// A loaded shared object; unloaded on drop.
pub struct DynamicLibrary {
    handle: *mut core::ffi::c_void,
    name: String,
}

impl core::fmt::Debug for DynamicLibrary {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicLibrary({:p}, '{}')", self.handle, self.name)
    }
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Loads a shared object by path or soname (e.g. `"libX11.so.1"`).
    /// `now` uses `RTLD_NOW` (resolve everything eagerly, surface errors
    /// early); `false` uses `RTLD_LAZY`.
    pub fn open(lib: &str, now: bool) -> Result<Self, DlError> {
        let cname = CString::new(lib).map_err(|_| DlError::Open {
            lib: lib.to_owned(),
        })?;
        // SAFETY: `cname` is NUL-terminated for the duration of the call;
        // the raw handle is kept alive by `Self`.
        let handle = unsafe { dlopen(cname.as_ptr(), if now { RTLD_NOW } else { RTLD_LAZY }) };
        if handle.is_null() {
            return Err(Self::describe(DlError::Open {
                lib: lib.to_owned(),
            }));
        }
        Ok(Self {
            handle,
            name: lib.to_owned(),
        })
    }

    /// Resolves a function pointer of the requested C ABI. Type safety for
    /// the *contents* of the function is the caller's job.
    pub fn symbol<T>(&self, name: &str) -> Result<Symbol<T>, DlError> {
        let cname = CString::new(name).map_err(|_| DlError::Symbol {
            lib: self.name.clone(),
            name: name.to_owned(),
        })?;
        // SAFETY: handle is a live dlopen handle; `cname` NUL-terminated.
        let ptr = unsafe { dlsym(self.handle, cname.as_ptr()) };
        if ptr.is_null() {
            return Err(Self::describe(DlError::Symbol {
                lib: self.name.clone(),
                name: name.to_owned(),
            }));
        }
        Ok(Symbol {
            ptr: ptr as *const (),
            _marker: PhantomData,
        })
    }

    /// Wraps an error with the current `dlerror` detail when available.
    fn describe(mut err: DlError) -> DlError {
        if let Some(reason) = last_error() {
            err = DlError::Detail { reason };
        }
        err
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: handle is ours and valid; dlclose returns 0 on success
        // and errors about nothing we can act on here.
        unsafe { dlclose(self.handle) };
    }
}

/// Reads (and clears) the current `dlerror` string, if any.
fn last_error() -> Option<String> {
    // SAFETY: dlerror returns a static lifetime string or NULL.
    let ptr = unsafe { dlerror() };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the pointer is the static error string, valid until the next
    // load call.
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// A resolved function pointer. Fetch the address, then cast it at the
/// call site with a *concrete* fn-pointer type (so the compiler statically
/// verifies the sizes — fn pointers are exactly pointer-sized):
///
/// ```
/// # fn demo(lib: &DynamicLibrary) -> Result<(), DlError> {
/// let sym = lib.symbol::<unsafe extern "C" fn(*const i8) -> usize>("strlen")?;
/// let strlen: unsafe extern "C" fn(*const i8) -> usize = unsafe { sym.as_fn() };
/// # Ok(())
/// # }
/// ```
pub struct Symbol<F> {
    ptr: *const (),
    _marker: PhantomData<F>,
}

impl<F> Symbol<F> {
    /// The raw symbol address.
    pub fn as_ptr(&self) -> *const () {
        self.ptr
    }

    /// Interprets the address as the concrete extern fn pointer `F`.
    ///
    /// # Safety
    /// The symbol must *actually* be the C ABI function `F` describes with
    /// its signature — the loader only guarantees the address exists.
    pub unsafe fn as_fn(self) -> F {
        // SAFETY: caller guaranteed the address is a valid C ABI function
        // of signature F.
        unsafe { core::mem::transmute_copy(&self.ptr) }
    }
}

impl<F> core::fmt::Debug for Symbol<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Symbol({:p})", self.ptr)
    }
}

// SAFETY: loading/symbol lookup is thread-safe in glibc; the payload is
// just an address.
unsafe impl<F: Send> Send for Symbol<F> {}
unsafe impl<F: Send> Sync for Symbol<F> {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    // libc is linked into every glibc binary; dlopen-able by soname on
    // common distros (best-effort, skipped when unavailable).
    #[test]
    fn can_load_libc_and_resolve_symbol() {
        let lib = match DynamicLibrary::open("libc.so.6", true) {
            Ok(l) => l,
            Err(_) => return, // not every CI image ships it by soname
        };
        let sym: Symbol<unsafe extern "C" fn(*const i8) -> usize> = lib.symbol("strlen").unwrap();
        // SAFETY: symbol is the real `strlen`; yield a concrete fn pointer.
        let strlen: unsafe extern "C" fn(*const i8) -> usize = unsafe { sym.as_fn() };
        let c = CString::new("engine").unwrap();
        // SAFETY: strlen is irrefutable on valid NUL-terminated input.
        let len = unsafe { strlen(c.as_ptr()) };
        assert_eq!(len, 6);
    }

    #[test]
    fn missing_library_is_an_error() {
        let err = DynamicLibrary::open("libno-such-lib-xyz.so.9", true).unwrap_err();
        let _ = format!("{err}");
    }

    #[test]
    fn missing_symbol_is_an_error() {
        let lib = DynamicLibrary::open("libc.so.6", true).unwrap();
        let err = lib
            .symbol::<unsafe extern "C" fn()>("no_such_symbol_zzz")
            .unwrap_err();
        let _ = format!("{err}");
    }
}
