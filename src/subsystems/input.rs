//! Deterministic input snapshot and headless event collector.

use std::collections::VecDeque;

use crate::kernel::{KernelContext, Subsystem};
use crate::subsystems::messages::InputFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key { key: u32, pressed: bool },
    MouseButton { button: u8, pressed: bool },
    MouseMoved { x: i32, y: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    AllocationFailed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputSnapshot {
    keys: Vec<u32>,
    mouse_buttons: u16,
    mouse_position: (i32, i32),
}

impl InputSnapshot {
    pub fn key_down(&self, key: u32) -> bool {
        self.keys.binary_search(&key).is_ok()
    }
    pub fn mouse_button_down(&self, button: u8) -> bool {
        button < 16 && self.mouse_buttons & (1u16 << button) != 0
    }
    pub fn mouse_position(&self) -> (i32, i32) {
        self.mouse_position
    }

    fn try_apply(&mut self, event: InputEvent) -> Result<(), InputError> {
        match event {
            InputEvent::Key { key, pressed } => match (pressed, self.keys.binary_search(&key)) {
                (true, Err(index)) => {
                    self.keys
                        .try_reserve(1)
                        .map_err(|_| InputError::AllocationFailed)?;
                    self.keys.insert(index, key)
                }
                (false, Ok(index)) => {
                    self.keys.remove(index);
                }
                _ => {}
            },
            InputEvent::MouseButton { button, pressed } if button < 16 => {
                let mask = 1u16 << button;
                if pressed {
                    self.mouse_buttons |= mask;
                } else {
                    self.mouse_buttons &= !mask;
                }
            }
            InputEvent::MouseButton { .. } => {}
            InputEvent::MouseMoved { x, y } => self.mouse_position = (x, y),
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct HeadlessInput {
    pending: VecDeque<InputEvent>,
    current: InputSnapshot,
}

impl HeadlessInput {
    pub fn push(&mut self, event: InputEvent) {
        self.try_push(event)
            .expect("input event queue allocation failed");
    }

    pub fn try_push(&mut self, event: InputEvent) -> Result<(), InputError> {
        self.pending
            .try_reserve(1)
            .map_err(|_| InputError::AllocationFailed)?;
        self.pending.push_back(event);
        Ok(())
    }

    /// Applies all events at the fixed-step boundary and returns the stable snapshot.
    pub fn advance(&mut self) -> &InputSnapshot {
        self.try_advance()
            .expect("input snapshot allocation failed")
    }

    pub fn try_advance(&mut self) -> Result<&InputSnapshot, InputError> {
        while let Some(&event) = self.pending.front() {
            self.current.try_apply(event)?;
            self.pending.pop_front();
        }
        Ok(&self.current)
    }

    pub fn snapshot(&self) -> &InputSnapshot {
        &self.current
    }
}

pub struct InputSubsystem {
    input: HeadlessInput,
}

impl Default for InputSubsystem {
    fn default() -> Self {
        let mut input = HeadlessInput::default();
        input.current.keys.reserve(32);
        Self { input }
    }
}

impl InputSubsystem {
    pub fn input(&self) -> &HeadlessInput {
        &self.input
    }
    pub fn input_mut(&mut self) -> &mut HeadlessInput {
        &mut self.input
    }
}

impl Subsystem for InputSubsystem {
    fn name(&self) -> &'static str {
        "input"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["window"]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        let _ = self.input.advance();
        if let Some(physics) = ctx.resolve("physics") {
            let snapshot = self.input.snapshot();
            let (mouse_x, mouse_y) = snapshot.mouse_position();
            let mut buttons = 0u16;
            for button in 0..16 {
                if snapshot.mouse_button_down(button) {
                    buttons |= 1u16 << button;
                }
            }
            let _ = ctx.publish(
                physics,
                2,
                InputFrame {
                    mouse_x,
                    mouse_y,
                    mouse_buttons: buttons,
                },
            );
        }
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_applies_events_deterministically() {
        let mut input = HeadlessInput::default();
        input.push(InputEvent::Key {
            key: 32,
            pressed: true,
        });
        input.push(InputEvent::MouseMoved { x: 10, y: -4 });
        input.push(InputEvent::MouseButton {
            button: 1,
            pressed: true,
        });

        let snapshot = input.advance();
        assert!(snapshot.key_down(32));
        assert!(snapshot.mouse_button_down(1));
        assert_eq!(snapshot.mouse_position(), (10, -4));
    }

    #[test]
    fn release_and_invalid_button_are_safe() {
        let mut input = HeadlessInput::default();
        input.push(InputEvent::Key {
            key: 7,
            pressed: true,
        });
        input.push(InputEvent::Key {
            key: 7,
            pressed: false,
        });
        input.push(InputEvent::MouseButton {
            button: 16,
            pressed: true,
        });
        let snapshot = input.advance();
        assert!(!snapshot.key_down(7));
        assert!(!snapshot.mouse_button_down(16));
    }
}
