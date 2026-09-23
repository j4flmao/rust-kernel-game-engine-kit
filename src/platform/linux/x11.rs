//! Runtime-loaded X11 boundary for the Linux window backend.
#![allow(unsafe_code)]

use core::ffi::c_void;

use crate::subsystems::input::InputEvent;
use crate::subsystems::window::{WindowEvent, WindowSize};

use super::dl::{DlError, DynamicLibrary};

type Display = c_void;
type Window = u64;
type Atom = u64;

type XOpenDisplay = unsafe extern "C" fn(*const i8) -> *mut Display;
type XCloseDisplay = unsafe extern "C" fn(*mut Display) -> i32;
type XDefaultScreen = unsafe extern "C" fn(*mut Display) -> i32;
type XRootWindow = unsafe extern "C" fn(*mut Display, i32) -> Window;
type XCreateSimpleWindow =
    unsafe extern "C" fn(*mut Display, Window, i32, i32, u32, u32, u32, u64, u64) -> Window;
type XMapWindow = unsafe extern "C" fn(*mut Display, Window) -> i32;
type XDestroyWindow = unsafe extern "C" fn(*mut Display, Window) -> i32;
type XFlush = unsafe extern "C" fn(*mut Display) -> i32;
type XPending = unsafe extern "C" fn(*mut Display) -> i32;
type XNextEvent = unsafe extern "C" fn(*mut Display, *mut XEvent);

#[repr(C)]
struct XEvent {
    bytes: [u8; 192],
}

const KEY_PRESS: i32 = 2;
const KEY_RELEASE: i32 = 3;
const MOTION_NOTIFY: i32 = 6;
const CONFIGURE_NOTIFY: i32 = 22;
const CLIENT_MESSAGE: i32 = 33;

#[derive(Debug)]
pub enum X11Error {
    Library(DlError),
    MissingSymbol(DlError),
    DisplayUnavailable,
}

impl core::fmt::Display for X11Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Library(error) => write!(f, "X11 library load failed: {error}"),
            Self::MissingSymbol(error) => write!(f, "X11 symbol missing: {error}"),
            Self::DisplayUnavailable => write!(f, "XOpenDisplay returned no display"),
        }
    }
}
impl core::error::Error for X11Error {}

pub struct X11Library {
    library: DynamicLibrary,
    open_display: XOpenDisplay,
    close_display: XCloseDisplay,
    default_screen: XDefaultScreen,
    root_window: XRootWindow,
    create_window: XCreateSimpleWindow,
    map_window: XMapWindow,
    destroy_window: XDestroyWindow,
    flush: XFlush,
    pending: XPending,
    next_event: XNextEvent,
}

impl X11Library {
    pub fn load() -> Result<Self, X11Error> {
        let library = DynamicLibrary::open("libX11.so.6", true).map_err(X11Error::Library)?;
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                library
                    .symbol::<$ty>($name)
                    .map_err(X11Error::MissingSymbol)
                    .and_then(|symbol| Ok(unsafe { symbol.as_fn() }))?
            };
        }
        Ok(Self {
            open_display: symbol!("XOpenDisplay", XOpenDisplay),
            close_display: symbol!("XCloseDisplay", XCloseDisplay),
            default_screen: symbol!("XDefaultScreen", XDefaultScreen),
            root_window: symbol!("XRootWindow", XRootWindow),
            create_window: symbol!("XCreateSimpleWindow", XCreateSimpleWindow),
            map_window: symbol!("XMapWindow", XMapWindow),
            destroy_window: symbol!("XDestroyWindow", XDestroyWindow),
            flush: symbol!("XFlush", XFlush),
            pending: symbol!("XPending", XPending),
            next_event: symbol!("XNextEvent", XNextEvent),
            library,
        })
    }

    pub fn open_display(&self) -> Result<X11Display<'_>, X11Error> {
        // SAFETY: null selects DISPLAY from the process environment.
        let display = unsafe { (self.open_display)(core::ptr::null()) };
        if display.is_null() {
            return Err(X11Error::DisplayUnavailable);
        }
        let screen = unsafe { (self.default_screen)(display) };
        let root = unsafe { (self.root_window)(display, screen) };
        let window = unsafe { (self.create_window)(display, root, 0, 0, 640, 480, 0, 0, 0) };
        unsafe {
            (self.map_window)(display, window);
            (self.flush)(display);
        }
        Ok(X11Display {
            api: self,
            display,
            window,
        })
    }
}

pub struct X11Display<'a> {
    api: &'a X11Library,
    display: *mut Display,
    window: Window,
}
impl X11Display<'_> {
    pub fn display_handle(&self) -> *mut c_void {
        self.display
    }
    pub fn window(&self) -> Window {
        self.window
    }

    pub fn poll_events(
        &mut self,
        window_events: &mut Vec<WindowEvent>,
        input_events: &mut Vec<InputEvent>,
    ) {
        window_events.clear();
        input_events.clear();
        while unsafe { (self.api.pending)(self.display) } > 0 {
            let mut event = XEvent { bytes: [0; 192] };
            unsafe { (self.api.next_event)(self.display, &mut event) };
            let event_type = i32::from_ne_bytes(event.bytes[0..4].try_into().unwrap());
            match event_type {
                KEY_PRESS | KEY_RELEASE => {
                    let key = u32::from_ne_bytes(event.bytes[84..88].try_into().unwrap());
                    input_events.push(InputEvent::Key {
                        key,
                        pressed: event_type == KEY_PRESS,
                    });
                }
                MOTION_NOTIFY => {
                    let x = i16::from_ne_bytes(event.bytes[72..74].try_into().unwrap()) as i32;
                    let y = i16::from_ne_bytes(event.bytes[74..76].try_into().unwrap()) as i32;
                    input_events.push(InputEvent::MouseMoved { x, y });
                }
                CONFIGURE_NOTIFY => {
                    let width =
                        u16::from_ne_bytes(event.bytes[56..58].try_into().unwrap()).max(1) as u32;
                    let height =
                        u16::from_ne_bytes(event.bytes[58..60].try_into().unwrap()).max(1) as u32;
                    window_events.push(WindowEvent::Resized(WindowSize { width, height }));
                }
                CLIENT_MESSAGE => window_events.push(WindowEvent::CloseRequested),
                _ => {}
            }
        }
    }
}
impl Drop for X11Display<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.api.destroy_window)(self.display, self.window);
            (self.api.close_display)(self.display);
        }
    }
}

#[allow(dead_code)]
fn _keep_atom_type(_: Atom) {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_error_is_actionable() {
        assert_eq!(
            X11Error::DisplayUnavailable.to_string(),
            "XOpenDisplay returned no display"
        );
    }
}
