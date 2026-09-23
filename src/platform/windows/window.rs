//! Minimal Win32 window lifecycle and message polling.
#![allow(unsafe_code)]

use std::ffi::CString;

use crate::subsystems::input::InputEvent;
use crate::subsystems::window::{WindowEvent, WindowSize};

type Handle = *mut core::ffi::c_void;
type WindowProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const INVALID_DIMENSION: u32 = 87;
const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const CW_USEDEFAULT: i32 = -0x8000_0000i32;
const SW_SHOW: i32 = 5;
const PM_REMOVE: u32 = 0x0001;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_SIZE: u32 = 0x0005;
const WM_CLOSE: u32 = 0x0010;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
struct Message {
    hwnd: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: Point,
}
#[repr(C)]
struct WindowClass {
    style: u32,
    window_proc: WindowProc,
    class_extra: i32,
    window_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu_name: *const u8,
    class_name: *const u8,
}

extern "system" {
    fn GetModuleHandleA(name: *const u8) -> Handle;
    fn RegisterClassA(class: *const WindowClass) -> u16;
    fn CreateWindowExA(
        ex_style: u32,
        class: *const u8,
        title: *const u8,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *const core::ffi::c_void,
    ) -> Handle;
    fn ShowWindow(window: Handle, command: i32) -> i32;
    fn DestroyWindow(window: Handle) -> i32;
    fn PeekMessageA(message: *mut Message, window: Handle, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(message: *const Message) -> i32;
    fn DispatchMessageA(message: *const Message) -> isize;
    fn GetLastError() -> u32;
}

unsafe extern "system" fn window_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    let _ = (window, message, w_param, l_param);
    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowError {
    pub op: &'static str,
    pub code: u32,
}
impl core::fmt::Display for WindowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} failed (Win32 error {})", self.op, self.code)
    }
}
impl core::error::Error for WindowError {}

pub struct Win32Window {
    handle: Handle,
    instance: Handle,
}

impl Win32Window {
    pub fn create(title: &str, width: i32, height: i32) -> Result<Self, WindowError> {
        if width <= 0 || height <= 0 {
            return Err(WindowError {
                op: "CreateWindowExA dimensions",
                code: INVALID_DIMENSION,
            });
        }
        let class_name = CString::new("RustKernelWindowClass").map_err(|_| WindowError {
            op: "CString",
            code: INVALID_DIMENSION,
        })?;
        let title = CString::new(title).map_err(|_| WindowError {
            op: "CString",
            code: INVALID_DIMENSION,
        })?;
        // SAFETY: null module name requests the current executable module.
        let instance = unsafe { GetModuleHandleA(core::ptr::null()) };
        if instance.is_null() {
            return Err(WindowError {
                op: "GetModuleHandleA",
                code: unsafe { GetLastError() },
            });
        }
        let class = WindowClass {
            style: CS_HREDRAW | CS_VREDRAW,
            window_proc,
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: core::ptr::null_mut(),
            cursor: core::ptr::null_mut(),
            background: core::ptr::null_mut(),
            menu_name: core::ptr::null(),
            class_name: class_name.as_ptr().cast(),
        };
        // SAFETY: class points to valid strings and a live module handle.
        let registered = unsafe { RegisterClassA(&class) };
        if registered == 0 && unsafe { GetLastError() } != 1410 {
            return Err(WindowError {
                op: "RegisterClassA",
                code: unsafe { GetLastError() },
            });
        }
        // SAFETY: all pointers are valid for the duration of the call.
        let handle = unsafe {
            CreateWindowExA(
                0,
                class_name.as_ptr().cast(),
                title.as_ptr().cast(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                instance,
                core::ptr::null(),
            )
        };
        if handle.is_null() {
            return Err(WindowError {
                op: "CreateWindowExA",
                code: unsafe { GetLastError() },
            });
        }
        Ok(Self { handle, instance })
    }
    pub fn raw_handle(&self) -> *mut core::ffi::c_void {
        self.handle
    }
    pub fn instance_handle(&self) -> *mut core::ffi::c_void {
        self.instance
    }
    pub fn show(&self) {
        // SAFETY: handle is live and owned by self.
        let _ = unsafe { ShowWindow(self.handle, SW_SHOW) };
    }
    pub fn poll_messages(&self, out: &mut Vec<u32>) {
        out.clear();
        let mut message = Message {
            hwnd: core::ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            point: Point { x: 0, y: 0 },
        };
        while unsafe { PeekMessageA(&mut message, self.handle, 0, 0, PM_REMOVE) } != 0 {
            out.push(message.message);
            // SAFETY: message was populated by PeekMessageA.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageA(&message);
            }
        }
    }

    pub fn poll_events(
        &self,
        window_events: &mut Vec<WindowEvent>,
        input_events: &mut Vec<InputEvent>,
    ) {
        window_events.clear();
        input_events.clear();
        let mut message = Message {
            hwnd: core::ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            point: Point { x: 0, y: 0 },
        };
        while unsafe { PeekMessageA(&mut message, self.handle, 0, 0, PM_REMOVE) } != 0 {
            match message.message {
                WM_KEYDOWN | WM_KEYUP => input_events.push(InputEvent::Key {
                    key: message.w_param as u32,
                    pressed: message.message == WM_KEYDOWN,
                }),
                WM_MOUSEMOVE => input_events.push(InputEvent::MouseMoved {
                    x: (message.l_param as u32 & 0xffff) as i16 as i32,
                    y: ((message.l_param as u32 >> 16) & 0xffff) as i16 as i32,
                }),
                WM_SIZE => window_events.push(WindowEvent::Resized(WindowSize {
                    width: (message.l_param as u32 & 0xffff).max(1),
                    height: ((message.l_param as u32 >> 16) & 0xffff).max(1),
                })),
                WM_CLOSE => window_events.push(WindowEvent::CloseRequested),
                _ => {}
            }
            // SAFETY: message was populated by PeekMessageA.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageA(&message);
            }
        }
    }
}

impl Drop for Win32Window {
    fn drop(&mut self) {
        // SAFETY: handle is owned.
        let _ = unsafe { DestroyWindow(self.handle) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_dimensions_without_ffi() {
        let result = Win32Window::create("invalid", 0, 720);
        assert!(matches!(
            result,
            Err(WindowError {
                op: "CreateWindowExA dimensions",
                code: INVALID_DIMENSION
            })
        ));
    }
}
