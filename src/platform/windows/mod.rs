//! Windows PAL backend. Raw Win32 timing bindings, no external crates.

pub mod cpu;
pub mod dl;
pub mod gpu;
pub mod iocp;
pub mod present;
pub mod surface;
pub mod swapchain;
pub mod threading;
pub mod time;
pub mod vulkan;
pub mod winapi;
pub mod window;
