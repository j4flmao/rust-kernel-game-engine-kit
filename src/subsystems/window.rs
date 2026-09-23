//! OS-neutral window contract and deterministic headless backend.

use std::collections::VecDeque;

use crate::kernel::{KernelContext, Subsystem};
use crate::subsystems::messages::WindowResized;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            None
        } else {
            Some(Self { width, height })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    Resized(WindowSize),
    CloseRequested,
    Focused(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowError {
    AllocationFailed,
}

/// Deterministic window backend for tests and headless/server execution.
pub struct HeadlessWindow {
    id: WindowId,
    size: WindowSize,
    open: bool,
    focused: bool,
    events: VecDeque<WindowEvent>,
}

impl HeadlessWindow {
    pub fn new(id: WindowId, size: WindowSize) -> Self {
        Self {
            id,
            size,
            open: true,
            focused: true,
            events: VecDeque::new(),
        }
    }

    pub fn id(&self) -> WindowId {
        self.id
    }
    pub fn size(&self) -> WindowSize {
        self.size
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn push_event(&mut self, event: WindowEvent) {
        self.try_push_event(event)
            .expect("window event queue allocation failed");
    }

    pub fn try_push_event(&mut self, event: WindowEvent) -> Result<(), WindowError> {
        self.events
            .try_reserve(1)
            .map_err(|_| WindowError::AllocationFailed)?;
        match event {
            WindowEvent::Resized(size) => self.size = size,
            WindowEvent::CloseRequested => self.open = false,
            WindowEvent::Focused(focused) => self.focused = focused,
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn drain_events(&mut self, out: &mut Vec<WindowEvent>) {
        out.clear();
        out.extend(self.events.drain(..));
    }
}

pub struct WindowSubsystem {
    window: HeadlessWindow,
    frame_events: Vec<WindowEvent>,
}

impl WindowSubsystem {
    pub fn new(id: WindowId, size: WindowSize) -> Self {
        Self::try_new(id, size).expect("window subsystem allocation failed")
    }

    pub fn try_new(id: WindowId, size: WindowSize) -> Result<Self, WindowError> {
        let mut frame_events = Vec::new();
        frame_events
            .try_reserve_exact(16)
            .map_err(|_| WindowError::AllocationFailed)?;
        Ok(Self {
            window: HeadlessWindow::new(id, size),
            frame_events,
        })
    }

    pub fn window(&self) -> &HeadlessWindow {
        &self.window
    }
    pub fn window_mut(&mut self) -> &mut HeadlessWindow {
        &mut self.window
    }
    pub fn frame_events(&self) -> &[WindowEvent] {
        &self.frame_events
    }
}

impl Subsystem for WindowSubsystem {
    fn name(&self) -> &'static str {
        "window"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        self.window.drain_events(&mut self.frame_events);
        if self
            .frame_events
            .iter()
            .any(|event| matches!(event, WindowEvent::Resized(_)))
        {
            if let Some(renderer) = ctx.resolve("renderer") {
                let size = self.window.size();
                let _ = ctx.publish(
                    renderer,
                    1,
                    WindowResized {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
        }
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_size_is_rejected() {
        assert_eq!(WindowSize::new(0, 720), None);
        assert_eq!(WindowSize::new(1280, 0), None);
    }

    #[test]
    fn events_update_state_and_drain_in_order() {
        let size = WindowSize::new(1280, 720).unwrap();
        let resized = WindowSize::new(1920, 1080).unwrap();
        let mut window = HeadlessWindow::new(WindowId(1), size);
        window.push_event(WindowEvent::Resized(resized));
        window.push_event(WindowEvent::Focused(false));
        window.push_event(WindowEvent::CloseRequested);

        let mut events = Vec::new();
        window.drain_events(&mut events);

        assert_eq!(window.size(), resized);
        assert!(!window.is_focused());
        assert!(!window.is_open());
        assert_eq!(events.len(), 3);
    }
}
