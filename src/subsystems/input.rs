//! Deterministic input snapshot and headless event collector.

use std::collections::VecDeque;

use crate::kernel::{KernelContext, Subsystem};
use crate::subsystems::messages::{InputEventMessage, InputFrame};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Key { key: u32, pressed: bool },
    MouseButton { button: u8, pressed: bool },
    MouseMoved { x: i32, y: i32 },
    TextByte { byte: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    AllocationFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputSnapshot {
    keys: Vec<u32>,
    mouse_buttons: u16,
    mouse_position: (i32, i32),
    text: [u8; 64],
    text_count: u8,
}

impl Default for InputSnapshot {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            mouse_buttons: 0,
            mouse_position: (0, 0),
            text: [0; 64],
            text_count: 0,
        }
    }
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
    pub fn keys(&self) -> &[u32] {
        &self.keys
    }
    pub fn text(&self) -> &[u8] {
        &self.text[..usize::from(self.text_count)]
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
            InputEvent::TextByte { byte } => {
                let index = usize::from(self.text_count);
                if index < self.text.len() {
                    self.text[index] = byte;
                    self.text_count = self.text_count.saturating_add(1);
                }
            }
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
        self.current.text_count = 0;
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
    previous_mouse_buttons: u16,
    previous_keys: Vec<u32>,
}

impl Default for InputSubsystem {
    fn default() -> Self {
        let mut input = HeadlessInput::default();
        input.current.keys.reserve(32);
        Self {
            input,
            previous_mouse_buttons: 0,
            previous_keys: Vec::with_capacity(32),
        }
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
        for envelope in ctx.receive() {
            if envelope.topic == 4 {
                if let Ok(message) = envelope.downcast::<InputEventMessage>() {
                    let _ = self.input.try_push(message.0);
                }
            }
        }
        let _ = self.input.advance();
        let snapshot = self.input.snapshot();
        let (mouse_x, mouse_y) = snapshot.mouse_position();
        let mut buttons = 0u16;
        for button in 0..16 {
            if snapshot.mouse_button_down(button) {
                buttons |= 1u16 << button;
            }
        }
        let pressed_buttons = buttons & !self.previous_mouse_buttons;
        let released_buttons = self.previous_mouse_buttons & !buttons;
        self.previous_mouse_buttons = buttons;
        let mut keys = [0u32; 32];
        let key_count = self.input.snapshot().keys().len().min(keys.len());
        keys[..key_count].copy_from_slice(&self.input.snapshot().keys()[..key_count]);
        let mut pressed_keys = [0u32; 32];
        let mut pressed_key_count = 0usize;
        for key in self.input.snapshot().keys() {
            if !self.previous_keys.contains(key) && pressed_key_count < pressed_keys.len() {
                pressed_keys[pressed_key_count] = *key;
                pressed_key_count += 1;
            }
        }
        let mut released_keys = [0u32; 32];
        let mut released_key_count = 0usize;
        for key in &self.previous_keys {
            if !self.input.snapshot().keys().contains(key)
                && released_key_count < released_keys.len()
            {
                released_keys[released_key_count] = *key;
                released_key_count += 1;
            }
        }
        self.previous_keys.clear();
        self.previous_keys
            .extend_from_slice(self.input.snapshot().keys());
        let frame = InputFrame {
            mouse_x,
            mouse_y,
            mouse_buttons: buttons,
            pressed_buttons,
            released_buttons,
            key_count: u8::try_from(key_count).unwrap_or(32),
            keys,
            pressed_key_count: u8::try_from(pressed_key_count).unwrap_or(32),
            pressed_keys,
            released_key_count: u8::try_from(released_key_count).unwrap_or(32),
            released_keys,
            text_count: u8::try_from(snapshot.text().len()).unwrap_or(64),
            text: {
                let mut text = [0u8; 64];
                let count = snapshot.text().len().min(text.len());
                text[..count].copy_from_slice(&snapshot.text()[..count]);
                text
            },
        };
        if let Some(ui) = ctx.resolve("ui") {
            let _ = ctx.publish(ui, 2, frame);
        }
        if let Some(physics) = ctx.resolve("physics") {
            let _ = ctx.publish(physics, 2, frame);
        }
        if let Some(game) = ctx.resolve("sudoku-game") {
            let _ = ctx.publish(game, 2, frame);
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

    #[test]
    fn text_input_is_bounded_to_one_frame() {
        let mut input = HeadlessInput::default();
        input.push(InputEvent::TextByte { byte: b'A' });
        input.push(InputEvent::TextByte { byte: b'B' });
        assert_eq!(input.advance().text(), b"AB");
        assert!(input.advance().text().is_empty());
    }
}
