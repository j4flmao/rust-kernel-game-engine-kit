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
type XSelectInput = unsafe extern "C" fn(*mut Display, Window, i64) -> i32;
type XLookupKeysym = unsafe extern "C" fn(*mut XEvent, i32) -> u64;
type XInternAtom = unsafe extern "C" fn(*mut Display, *const i8, i32) -> Atom;
type XSetWMProtocols = unsafe extern "C" fn(*mut Display, Window, *mut Atom, i32) -> i32;
type XStoreName = unsafe extern "C" fn(*mut Display, Window, *const i8) -> i32;

#[repr(C, align(8))]
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
    _library: DynamicLibrary,
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
    select_input: XSelectInput,
    lookup_keysym: XLookupKeysym,
    intern_atom: XInternAtom,
    set_protocols: XSetWMProtocols,
    store_name: XStoreName,
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
            select_input: symbol!("XSelectInput", XSelectInput),
            lookup_keysym: symbol!("XLookupKeysym", XLookupKeysym),
            intern_atom: symbol!("XInternAtom", XInternAtom),
            set_protocols: symbol!("XSetWMProtocols", XSetWMProtocols),
            store_name: symbol!("XStoreName", XStoreName),
            _library: library,
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
        let mut delete_atom =
            unsafe { (self.intern_atom)(display, c"WM_DELETE_WINDOW".as_ptr(), 0) };
        let protocols_atom = unsafe { (self.intern_atom)(display, c"WM_PROTOCOLS".as_ptr(), 0) };
        unsafe {
            (self.select_input)(display, window, 1 | 2 | 4 | 8 | 64 | (1 << 17) | (1 << 21));
            (self.set_protocols)(display, window, &mut delete_atom, 1);
            (self.map_window)(display, window);
            (self.flush)(display);
        }
        Ok(X11Display {
            api: self,
            display,
            window,
            delete_atom,
            protocols_atom,
            size: WindowSize {
                width: 640,
                height: 480,
            },
        })
    }
}

pub struct X11Display<'a> {
    api: &'a X11Library,
    display: *mut Display,
    window: Window,
    delete_atom: Atom,
    protocols_atom: Atom,
    size: WindowSize,
}
impl X11Display<'_> {
    pub fn set_title(&self, title: &str) {
        if let Ok(title) = std::ffi::CString::new(title) {
            // SAFETY: owned display/window and NUL-terminated text.
            unsafe {
                (self.api.store_name)(self.display, self.window, title.as_ptr());
            }
        }
    }
    pub fn client_size(&self) -> Option<WindowSize> {
        Some(self.size)
    }
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
                    let symbol = unsafe { (self.api.lookup_keysym)(&mut event, 0) };
                    let key = match symbol {
                        0xff1b => 0x1b,
                        0x61..=0x7a => symbol as u32 - 32,
                        _ => symbol as u32,
                    };
                    input_events.push(InputEvent::Key {
                        key,
                        pressed: event_type == KEY_PRESS,
                    });
                }
                MOTION_NOTIFY => {
                    let x = i32::from_ne_bytes(event.bytes[64..68].try_into().unwrap());
                    let y = i32::from_ne_bytes(event.bytes[68..72].try_into().unwrap());
                    input_events.push(InputEvent::MouseMoved { x, y });
                }
                CONFIGURE_NOTIFY => {
                    let width =
                        i32::from_ne_bytes(event.bytes[56..60].try_into().unwrap()).max(1) as u32;
                    let height =
                        i32::from_ne_bytes(event.bytes[60..64].try_into().unwrap()).max(1) as u32;
                    self.size = WindowSize { width, height };
                    window_events.push(WindowEvent::Resized(WindowSize { width, height }));
                }
                4 | 5 => {
                    let button = u32::from_ne_bytes(event.bytes[84..88].try_into().unwrap());
                    if (1..=3).contains(&button) {
                        input_events.push(InputEvent::MouseButton {
                            button: (button - 1) as u8,
                            pressed: event_type == 4,
                        });
                    }
                }
                9 | 10 => window_events.push(WindowEvent::Focused(event_type == 9)),
                CLIENT_MESSAGE => {
                    let message = u64::from_ne_bytes(event.bytes[40..48].try_into().unwrap());
                    let atom = u64::from_ne_bytes(event.bytes[56..64].try_into().unwrap());
                    if message == self.protocols_atom && atom == self.delete_atom {
                        window_events.push(WindowEvent::CloseRequested);
                    }
                }
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
