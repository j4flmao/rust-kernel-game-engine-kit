//! Small copyable messages shared by subsystem drivers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowResized {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputFrame {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u16,
}
