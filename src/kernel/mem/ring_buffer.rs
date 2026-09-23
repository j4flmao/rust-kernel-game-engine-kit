//! Shared bounded SPSC ring-buffer surface.
//!
//! The implementation remains owned by the kernel transport while this
//! public memory module gives tracing, streaming, and future async backends a
//! stable import path without duplicating ring code.

pub use crate::kernel::bus::SpscRing;
