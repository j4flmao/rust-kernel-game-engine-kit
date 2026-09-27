//! Small copyable messages shared by subsystem drivers.

use crate::subsystems::input::InputEvent;
use crate::subsystems::ui::UiCommand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowResized {
    pub width: u32,
    pub height: u32,
}

/// Bounded asynchronous UI asset readiness notification.
/// `kind` is 0 for texture and 1 for font; `state` follows the UI asset state
/// ABI: 0 missing, 1 loading, 2 ready, 3 failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAssetReady {
    pub kind: u8,
    pub index: u32,
    pub generation: u32,
    pub state: u8,
}

/// One bounded deferred UI mutation sent by an application/game subsystem.
///
/// The receiver puts it into the UI command buffer and applies it at the
/// normal frame boundary; game logic never reaches into the retained tree.
#[derive(Clone, Debug, PartialEq)]
pub struct UiCommandMessage(pub UiCommand);

/// Native/window drivers forward bounded input events to the input subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEventMessage(pub InputEvent);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputFrame {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u16,
    pub pressed_buttons: u16,
    pub released_buttons: u16,
    pub key_count: u8,
    pub keys: [u32; 32],
    pub pressed_key_count: u8,
    pub pressed_keys: [u32; 32],
    pub released_key_count: u8,
    pub released_keys: [u32; 32],
    pub text_count: u8,
    pub text: [u8; 64],
}
