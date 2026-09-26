//! Minimal Win32 window lifecycle and message polling.
#![allow(unsafe_code)]

#[link(name = "user32")]
unsafe extern "system" {}
#[link(name = "gdi32")]
unsafe extern "system" {}

use std::ffi::CString;

use crate::subsystems::input::InputEvent;
use crate::subsystems::window::{WindowEvent, WindowSize};

type Handle = *mut core::ffi::c_void;
type WindowProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;
const INVALID_DIMENSION: u32 = 87;
const CS_HREDRAW: u32 = 0x0002;
const CS_VREDRAW: u32 = 0x0001;
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const SW_SHOW: i32 = 5;
const PM_REMOVE: u32 = 0x0001;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_NCCREATE: u32 = 0x0081;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_SIZE: u32 = 0x0005;
const WM_CLOSE: u32 = 0x0010;
const IDC_ARROW: *const u8 = 32512usize as *const u8;

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
    fn LoadCursorA(instance: Handle, name: *const u8) -> Handle;
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
    fn GetDC(window: Handle) -> Handle;
    fn ReleaseDC(window: Handle, dc: Handle) -> i32;
    fn CreateSolidBrush(color: u32) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
    fn SetBkMode(dc: Handle, mode: i32) -> i32;
    fn SetTextColor(dc: Handle, color: u32) -> u32;
    fn TextOutA(dc: Handle, x: i32, y: i32, text: *const u8, length: i32) -> i32;
    fn DefWindowProcA(window: Handle, message: u32, w_param: usize, l_param: isize) -> isize;
}

unsafe extern "system" fn window_proc(
    window: Handle,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    // Win32 requires a non-zero result for WM_NCCREATE or
    // CreateWindowExA aborts the window construction.
    if message == WM_NCCREATE {
        1
    } else {
        // Preserve default hit-testing, cursor selection, activation and
        // non-client behavior. Returning zero for every message makes the
        // window look hung and can leave the pointer in the wait state.
        unsafe { DefWindowProcA(window, message, w_param, l_param) }
    }
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

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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
            cursor: unsafe { LoadCursorA(core::ptr::null_mut(), IDC_ARROW) },
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
                100,
                100,
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

    /// Paints the example UI through the Win32 fallback backend. This keeps
    /// the example visibly interactive even when a machine has no usable
    /// Vulkan device/driver.
    pub fn paint_sudoku(&self, board: bool) {
        let dc = unsafe { GetDC(self.handle) };
        if dc.is_null() {
            return;
        }
        unsafe {
            SetBkMode(dc, 1);
            let background = CreateSolidBrush(0x00f8f9fc);
            let full = Rect {
                left: 0,
                top: 0,
                right: 900,
                bottom: 840,
            };
            FillRect(dc, &full, background);
            DeleteObject(background);
            SetTextColor(dc, 0x00232d42);
            draw_text(dc, 48, 36, "Sudoku");
            SetTextColor(dc, 0x00677b9d);
            draw_text(
                dc,
                48,
                70,
                if board {
                    "Choose a cell and enter a digit"
                } else {
                    "Choose your difficulty"
                },
            );
            if board {
                draw_board(dc);
            } else {
                draw_button(dc, 72, 160, 312, 300, 0x00f5f8ff, "EASY");
                draw_button(dc, 330, 160, 570, 300, 0x00f5f8ff, "MEDIUM");
                draw_button(dc, 588, 160, 828, 300, 0x00f5f8ff, "HARD");
            }
            let _ = ReleaseDC(self.handle, dc);
        }
    }

    /// Paints the C++ reference application's landing screen.
    pub fn paint_main_menu(&self) {
        let dc = unsafe { GetDC(self.handle) };
        if dc.is_null() {
            return;
        }
        unsafe {
            SetBkMode(dc, 1);
            let background = CreateSolidBrush(0x00f8f9fc);
            let full = Rect {
                left: 0,
                top: 0,
                right: 900,
                bottom: 840,
            };
            FillRect(dc, &full, background);
            DeleteObject(background);
            SetTextColor(dc, 0x00232d42);
            draw_text(dc, 390, 175, "SUDOKU");
            SetTextColor(dc, 0x00677b9d);
            draw_text(dc, 380, 220, "Classic logic puzzle");
            draw_button(dc, 310, 280, 590, 336, 0x003f83f1, "Continue");
            draw_button(dc, 310, 350, 590, 406, 0x00f5f8ff, "New Game");
            draw_button(dc, 310, 420, 590, 476, 0x00f5f8ff, "Leaderboard");
            draw_button(dc, 310, 490, 590, 546, 0x00ef4145, "Quit");
            SetTextColor(dc, 0x00677b9d);
            draw_text(
                dc,
                265,
                805,
                "Arrow keys / mouse to select   |   1-9 to enter   |   ESC to pause",
            );
            let _ = ReleaseDC(self.handle, dc);
        }
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
                WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP => {
                    let x = (message.l_param as u32 & 0xffff) as i16 as i32;
                    let y = ((message.l_param as u32 >> 16) & 0xffff) as i16 as i32;
                    input_events.push(InputEvent::MouseMoved { x, y });
                    if message.message != WM_MOUSEMOVE {
                        input_events.push(InputEvent::MouseButton {
                            button: 0,
                            pressed: message.message == WM_LBUTTONDOWN,
                        });
                    }
                }
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

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_text(dc: Handle, x: i32, y: i32, text: &str) {
    let bytes = text.as_bytes();
    let _ = TextOutA(dc, x, y, bytes.as_ptr(), bytes.len() as i32);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_button(
    dc: Handle,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u32,
    label: &str,
) {
    let brush = CreateSolidBrush(color);
    let rect = Rect {
        left,
        top,
        right,
        bottom,
    };
    FillRect(dc, &rect, brush);
    DeleteObject(brush);
    SetTextColor(dc, 0x00232d42);
    draw_text(dc, left + 24, top + 24, label);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn draw_board(dc: Handle) {
    let origin_x = 72;
    let origin_y = 140;
    let cell = 72;
    let brush = CreateSolidBrush(0x00e7edf6);
    for row in 0..9 {
        for column in 0..9 {
            let rect = Rect {
                left: origin_x + column * cell,
                top: origin_y + row * cell,
                right: origin_x + (column + 1) * cell - 2,
                bottom: origin_y + (row + 1) * cell - 2,
            };
            FillRect(dc, &rect, brush);
        }
    }
    DeleteObject(brush);
    SetTextColor(dc, 0x00232d42);
    // A stable visual seed keeps the fallback renderer useful while the
    // gameplay state continues to live in the kernel/UI path.
    let givens = [
        [0, 0, 0, 0, 5, 0, 0, 0, 0],
        [7, 0, 0, 0, 0, 0, 9, 0, 4],
        [9, 0, 0, 3, 0, 6, 0, 5, 0],
        [0, 0, 0, 0, 3, 0, 1, 0, 7],
        [0, 1, 7, 9, 6, 4, 0, 0, 0],
        [0, 0, 9, 0, 0, 0, 4, 0, 8],
        [4, 5, 0, 0, 9, 1, 8, 0, 0],
        [3, 0, 0, 0, 0, 5, 0, 0, 2],
        [0, 7, 8, 6, 4, 3, 0, 0, 0],
    ];
    for (row, values) in givens.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            if *value != 0 {
                draw_text(
                    dc,
                    origin_x + column as i32 * cell + 28,
                    origin_y + row as i32 * cell + 22,
                    &value.to_string(),
                );
            }
        }
    }
    for row in 0..3 {
        for column in 0..3 {
            let left = 655 + column * 65;
            let top = 390 + row * 65;
            draw_button(
                dc,
                left,
                top,
                left + 55,
                top + 55,
                0x003f83f1,
                &((row * 3 + column + 1).to_string()),
            );
        }
    }
    SetTextColor(dc, 0x00677b9d);
    draw_text(
        dc,
        72,
        820,
        "Mouse: select   1-9: fill   N: notes   H: hint   P: pause   Esc: close",
    );
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
