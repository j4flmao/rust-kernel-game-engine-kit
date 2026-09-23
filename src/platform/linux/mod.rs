//! Linux PAL backend. Raw FFI, no external crates.

pub mod cpu;
pub mod dl;
pub mod gpu;
pub mod io_uring;
pub mod present;
pub mod surface;
pub mod swapchain;
pub mod syscalls;
pub mod threading;
pub mod time;
pub mod vulkan;
pub mod x11;
