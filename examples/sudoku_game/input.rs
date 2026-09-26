//! Input actions owned by the Sudoku application, not by the engine.

#[derive(Clone, Copy)]
pub(crate) enum GameAction {
    Digit(u8),
    Clear,
    ToggleNote,
    Hint,
    Pause,
    Move(i32, i32),
}

pub(crate) fn action_from_key(key: u32) -> Option<GameAction> {
    match key {
        0x30 => Some(GameAction::Clear),
        0x31..=0x39 => Some(GameAction::Digit((key - 0x30) as u8)),
        0x08 | 0x2E => Some(GameAction::Clear),
        0x48 => Some(GameAction::Hint),
        0x4E => Some(GameAction::ToggleNote),
        0x50 | 0x20 | 0x1B => Some(GameAction::Pause),
        0x25 => Some(GameAction::Move(0, -1)),
        0x26 => Some(GameAction::Move(-1, 0)),
        0x27 => Some(GameAction::Move(0, 1)),
        0x28 => Some(GameAction::Move(1, 0)),
        _ => None,
    }
}
