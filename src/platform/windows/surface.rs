//! Win32 Vulkan surface creation boundary.

#![allow(unsafe_code)]

use super::vulkan::{
    VkInstance, VkResult, VkSurfaceKHR, VulkanLoader, VulkanLoaderError, VK_SUCCESS,
};
use core::ffi::c_void;

type CreateWin32Surface = unsafe extern "system" fn(
    VkInstance,
    *const Win32SurfaceCreateInfo,
    *const c_void,
    *mut VkSurfaceKHR,
) -> VkResult;
type DestroySurface = unsafe extern "system" fn(VkInstance, VkSurfaceKHR, *const c_void);

#[repr(C)]
struct Win32SurfaceCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    hinstance: *mut c_void,
    hwnd: *mut c_void,
}

pub struct Win32Surface {
    instance: VkInstance,
    handle: VkSurfaceKHR,
    destroy: DestroySurface,
}

impl Win32Surface {
    pub const fn raw(&self) -> VkSurfaceKHR {
        self.handle
    }
}
impl Drop for Win32Surface {
    fn drop(&mut self) {
        // SAFETY: surface and instance remain owned and valid until drop.
        unsafe { (self.destroy)(self.instance, self.handle, core::ptr::null()) };
    }
}

/// # Safety
/// `instance` must be live; `hinstance` and `hwnd` must be valid Win32 handles.
pub unsafe fn create(
    loader: &VulkanLoader,
    instance: VkInstance,
    hinstance: *mut c_void,
    hwnd: *mut c_void,
) -> Result<Win32Surface, VulkanLoaderError> {
    if instance.is_null() || hinstance.is_null() || hwnd.is_null() {
        return Err(VulkanLoaderError::InvalidSurfaceHandle);
    }
    let create: CreateWin32Surface = unsafe { resolve(loader, b"vkCreateWin32SurfaceKHR\0") }?;
    let destroy: DestroySurface = unsafe { resolve(loader, b"vkDestroySurfaceKHR\0") }?;
    let info = Win32SurfaceCreateInfo {
        s_type: 1000009000,
        next: core::ptr::null(),
        flags: 0,
        hinstance,
        hwnd,
    };
    let mut handle = 0;
    // SAFETY: handles and create info are valid for this synchronous Vulkan call.
    let result = unsafe { create(instance, &info, core::ptr::null(), &mut handle) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(Win32Surface {
        instance,
        handle,
        destroy,
    })
}

/// # Safety
/// `display` must be a valid Win32 module instance and `window` a valid
/// top-level `HWND`; both must outlive the returned surface.
pub unsafe fn create_with_handles(
    loader: &VulkanLoader,
    instance: VkInstance,
    display: *const c_void,
    window: *const c_void,
) -> Result<Win32Surface, VulkanLoaderError> {
    if instance.is_null() || display.is_null() || window.is_null() {
        return Err(VulkanLoaderError::InvalidSurfaceHandle);
    }
    let create: CreateWin32Surface = unsafe { resolve(loader, b"vkCreateWin32SurfaceKHR\0") }?;
    let destroy: DestroySurface = unsafe { resolve(loader, b"vkDestroySurfaceKHR\0") }?;
    let info = Win32SurfaceCreateInfo {
        s_type: 1000009000,
        next: core::ptr::null(),
        flags: 0,
        hinstance: display as *mut c_void,
        hwnd: window as *mut c_void,
    };
    let mut handle = 0;
    // SAFETY: handles and create info are valid for this synchronous Vulkan call.
    let result = unsafe { create(instance, &info, core::ptr::null(), &mut handle) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(Win32Surface {
        instance,
        handle,
        destroy,
    })
}

unsafe fn resolve<T>(loader: &VulkanLoader, name: &'static [u8]) -> Result<T, VulkanLoaderError> {
    // SAFETY: the static name is NUL terminated and T is selected to match the Vulkan ABI.
    let pointer = unsafe { loader.global_proc(name) }.ok_or_else(|| {
        VulkanLoaderError::MissingEntry(super::dl::DlError::Symbol {
            symbol: String::from_utf8_lossy(name).into_owned(),
            code: 0,
        })
    })?;
    // SAFETY: caller selects the exact function ABI for T.
    Ok(unsafe { core::mem::transmute_copy(&pointer) })
}
