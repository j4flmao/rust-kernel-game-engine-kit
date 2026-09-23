//! X11 Vulkan surface creation boundary.

#![allow(unsafe_code)]

use super::vulkan::{
    VkInstance, VkResult, VkSurfaceKHR, VulkanLoader, VulkanLoaderError, VK_SUCCESS,
};
use super::x11::X11Display;
use core::ffi::c_void;

type CreateXlibSurface = unsafe extern "C" fn(
    VkInstance,
    *const XlibSurfaceCreateInfo,
    *const c_void,
    *mut VkSurfaceKHR,
) -> VkResult;
type DestroySurface = unsafe extern "C" fn(VkInstance, VkSurfaceKHR, *const c_void);
#[repr(C)]
struct XlibSurfaceCreateInfo {
    s_type: u32,
    next: *const c_void,
    flags: u32,
    dpy: *mut c_void,
    window: u64,
}

pub struct X11Surface {
    instance: VkInstance,
    handle: VkSurfaceKHR,
    destroy: DestroySurface,
}
impl X11Surface {
    pub const fn raw(&self) -> VkSurfaceKHR {
        self.handle
    }
}
impl Drop for X11Surface {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.instance, self.handle, core::ptr::null()) };
    }
}

/// # Safety
/// `instance` must be live and `display` must outlive the returned surface.
pub unsafe fn create(
    loader: &VulkanLoader,
    instance: VkInstance,
    display: &X11Display<'_>,
) -> Result<X11Surface, VulkanLoaderError> {
    if instance.is_null() || display.display_handle().is_null() {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let create: CreateXlibSurface = unsafe { resolve(loader, b"vkCreateXlibSurfaceKHR\0")? };
    let destroy: DestroySurface = unsafe { resolve(loader, b"vkDestroySurfaceKHR\0")? };
    let info = XlibSurfaceCreateInfo {
        s_type: 1000004000,
        next: core::ptr::null(),
        flags: 0,
        dpy: display.display_handle(),
        window: display.window(),
    };
    let mut handle = 0;
    let result = unsafe { create(instance, &info, core::ptr::null(), &mut handle) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(X11Surface {
        instance,
        handle,
        destroy,
    })
}

/// # Safety
/// `display` must be a live X11 `Display*` and `window` a live mapped window
/// id; both must outlive the returned surface.
pub unsafe fn create_with_handles(
    loader: &VulkanLoader,
    instance: VkInstance,
    display: *const c_void,
    window: u64,
) -> Result<X11Surface, VulkanLoaderError> {
    if instance.is_null() || display.is_null() {
        return Err(VulkanLoaderError::InvalidQueuePlan);
    }
    let create: CreateXlibSurface = unsafe { resolve(loader, b"vkCreateXlibSurfaceKHR\0")? };
    let destroy: DestroySurface = unsafe { resolve(loader, b"vkDestroySurfaceKHR\0")? };
    let info = XlibSurfaceCreateInfo {
        s_type: 1000004000,
        next: core::ptr::null(),
        flags: 0,
        dpy: display as *mut c_void,
        window,
    };
    let mut handle = 0;
    // SAFETY: handles and create info are valid for the synchronous call.
    let result = unsafe { create(instance, &info, core::ptr::null(), &mut handle) };
    if result != VK_SUCCESS {
        return Err(VulkanLoaderError::Api(result));
    }
    Ok(X11Surface {
        instance,
        handle,
        destroy,
    })
}

unsafe fn resolve<T>(loader: &VulkanLoader, name: &'static [u8]) -> Result<T, VulkanLoaderError> {
    let pointer = unsafe { loader.global_proc(name) }.ok_or_else(|| {
        VulkanLoaderError::MissingCommand(String::from_utf8_lossy(name).into_owned())
    })?;
    Ok(unsafe { core::mem::transmute_copy(&pointer) })
}
