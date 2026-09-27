//! Example-only native presentation bridge.
//!
//! The engine UI packet remains backend-neutral. On Windows this example uses
//! a tiny GDI presenter so the reference game is playable even when Vulkan is
//! unavailable. It consumes a bounded snapshot published by the game plugin;
//! it never owns or mutates Sudoku rules.
#![allow(unsafe_code)]

use super::{CellState, GameSession, Screen, SessionResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeSnapshot {
    pub(crate) screen: Screen,
    pub(crate) values: [[u8; 9]; 9],
    pub(crate) states: [[u8; 9]; 9],
    pub(crate) notes: [[[bool; 9]; 9]; 9],
    pub(crate) selected: Option<(usize, usize)>,
    pub(crate) lives: u8,
    pub(crate) max_lives: u8,
    pub(crate) hints: u8,
    pub(crate) elapsed: u32,
    pub(crate) max_time: u32,
    pub(crate) difficulty: &'static str,
    pub(crate) note_mode: bool,
    pub(crate) paused: bool,
    pub(crate) shake_offset: i32,
    /// 0 ongoing, 1 won, 2 lost lives, 3 lost time.
    pub(crate) result: u8,
}

impl NativeSnapshot {
    pub(crate) fn from_session(
        screen: Screen,
        session: &GameSession,
        selected: Option<(usize, usize)>,
        note_mode: bool,
        shake_offset: i32,
    ) -> Self {
        let mut states = [[0u8; 9]; 9];
        for (state_row, session_row) in states.iter_mut().zip(session.states.iter()) {
            for (state, session_state) in state_row.iter_mut().zip(session_row.iter()) {
                *state = match session_state {
                    CellState::Empty => 0,
                    CellState::Given => 1,
                    CellState::Player => 2,
                    CellState::Error => 3,
                    CellState::Hint => 4,
                };
            }
        }
        Self {
            screen,
            values: session.values,
            states,
            notes: session.notes,
            selected,
            lives: session.lives,
            max_lives: session.config.max_lives,
            hints: session.hints_remaining(),
            elapsed: session.elapsed,
            max_time: session.config.max_time_sec,
            difficulty: session.config.name,
            note_mode,
            paused: session.paused,
            shake_offset,
            result: match session.result {
                SessionResult::Ongoing => 0,
                SessionResult::Win => 1,
                SessionResult::LoseLives => 2,
                SessionResult::LoseTime => 3,
            },
        }
    }
}

#[cfg(windows)]
mod win32 {
    use super::{NativeSnapshot, Screen};
    use crate::layout::{PlayingLayout, DESIGN_HEIGHT, DESIGN_WIDTH};
    use std::ffi::c_void;

    type Handle = *mut c_void;
    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {}
    extern "system" {
        fn GetDC(window: Handle) -> Handle;
        fn ReleaseDC(window: Handle, dc: Handle) -> i32;
        fn CreateSolidBrush(color: u32) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateCompatibleBitmap(dc: Handle, width: i32, height: i32) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn DeleteDC(dc: Handle) -> i32;
        fn BitBlt(
            destination: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            source: Handle,
            source_x: i32,
            source_y: i32,
            operation: u32,
        ) -> i32;
        fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
        fn FrameRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
        fn SetBkMode(dc: Handle, mode: i32) -> i32;
        fn SetTextColor(dc: Handle, color: u32) -> u32;
        fn TextOutA(dc: Handle, x: i32, y: i32, text: *const u8, length: i32) -> i32;
    }

    pub(crate) struct Presenter {
        handle: Handle,
    }
    unsafe impl Send for Presenter {}

    impl Presenter {
        pub(crate) fn new(handle: Handle) -> Self {
            Self { handle }
        }

        pub(crate) fn paint(&self, frame: &NativeSnapshot) {
            let dc = unsafe { GetDC(self.handle) };
            if dc.is_null() {
                return;
            }
            unsafe {
                let buffer = CreateCompatibleDC(dc);
                if buffer.is_null() {
                    paint_scene(dc, frame);
                } else {
                    let bitmap = CreateCompatibleBitmap(dc, DESIGN_WIDTH, DESIGN_HEIGHT);
                    if bitmap.is_null() {
                        let _ = DeleteDC(buffer);
                        paint_scene(dc, frame);
                    } else {
                        let previous = SelectObject(buffer, bitmap);
                        paint_scene(buffer, frame);
                        let _ = BitBlt(
                            dc,
                            0,
                            0,
                            DESIGN_WIDTH,
                            DESIGN_HEIGHT,
                            buffer,
                            0,
                            0,
                            0x00CC0020,
                        );
                        let _ = SelectObject(buffer, previous);
                        let _ = DeleteObject(bitmap);
                        let _ = DeleteDC(buffer);
                    }
                }
                let _ = ReleaseDC(self.handle, dc);
            }
        }
    }

    unsafe fn paint_scene(dc: Handle, snapshot: &NativeSnapshot) {
        SetBkMode(dc, 1);
        fill(
            dc,
            Rect {
                left: 0,
                top: 0,
                right: DESIGN_WIDTH,
                bottom: DESIGN_HEIGHT,
            },
            0x00f8f9fc,
        );
        match snapshot.screen {
            Screen::MainMenu => paint_menu(dc),
            Screen::Difficulty => paint_difficulty(dc),
            Screen::Playing => paint_playing(dc, snapshot),
        }
    }

    unsafe fn fill(dc: Handle, rect: Rect, color: u32) {
        let brush = CreateSolidBrush(color);
        if !brush.is_null() {
            let _ = FillRect(dc, &rect, brush);
            let _ = DeleteObject(brush);
        }
    }

    unsafe fn frame(dc: Handle, rect: Rect, color: u32) {
        let brush = CreateSolidBrush(color);
        if !brush.is_null() {
            let _ = FrameRect(dc, &rect, brush);
            let _ = DeleteObject(brush);
        }
    }

    unsafe fn text(dc: Handle, x: i32, y: i32, value: &str, color: u32) {
        let _ = SetTextColor(dc, color);
        let _ = TextOutA(dc, x, y, value.as_ptr(), value.len() as i32);
    }

    unsafe fn button(
        dc: Handle,
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        color: u32,
        label: &str,
    ) {
        fill(
            dc,
            Rect {
                left,
                top,
                right: left + width,
                bottom: top + height,
            },
            color,
        );
        text(dc, left + 24, top + (height - 16) / 2, label, 0x00232d42);
    }

    unsafe fn paint_menu(dc: Handle) {
        text(dc, 390, 175, "SUDOKU", 0x00232d42);
        text(dc, 380, 220, "Classic logic puzzle", 0x00677b9d);
        button(dc, 310, 280, 280, 56, 0x003f83f1, "Continue");
        button(dc, 310, 350, 280, 56, 0x00f5f8ff, "New Game");
        button(dc, 310, 420, 280, 56, 0x00f5f8ff, "Leaderboard");
        button(dc, 310, 490, 280, 56, 0x00ef4145, "Quit");
        text(
            dc,
            265,
            805,
            "Arrow keys / mouse to select   |   1-9 to enter   |   ESC to pause",
            0x00677b9d,
        );
    }

    unsafe fn paint_difficulty(dc: Handle) {
        text(dc, 48, 60, "Sudoku", 0x00232d42);
        text(dc, 48, 96, "Choose your difficulty", 0x00677b9d);
        for (index, label) in ["EASY", "MEDIUM", "HARD"].into_iter().enumerate() {
            let left = 96 + index as i32 * 244;
            button(dc, left, 280, 220, 280, 0x00f5f8ff, label);
        }
        text(dc, 358, 790, "Choose a mode to start", 0x00677b9d);
    }

    unsafe fn paint_playing(dc: Handle, snapshot: &NativeSnapshot) {
        let layout = PlayingLayout::for_window(DESIGN_WIDTH, DESIGN_HEIGHT);
        let board_x = layout.board_x + snapshot.shake_offset;
        let board_y = layout.board_y + snapshot.shake_offset;
        text(dc, 48, 40, "DIFFICULTY", 0x00677b9d);
        text(dc, 48, 66, snapshot.difficulty, 0x003f83f1);
        text(dc, 218, 40, "TIME", 0x00677b9d);
        let remaining = snapshot.max_time.saturating_sub(snapshot.elapsed);
        let time = format!("{:02}:{:02}", remaining / 60, remaining % 60);
        text(
            dc,
            218,
            66,
            &time,
            if remaining < 60 {
                0x00d93035
            } else {
                0x00232d42
            },
        );
        text(dc, 368, 40, "MISTAKES", 0x00677b9d);
        let mistakes = format!(
            "{} / {}",
            snapshot.max_lives.saturating_sub(snapshot.lives),
            snapshot.max_lives
        );
        text(dc, 368, 66, &mistakes, 0x00232d42);
        text(dc, 518, 40, "LIVES", 0x00677b9d);
        let hearts = (0..snapshot.max_lives)
            .map(|index| if index < snapshot.lives { '*' } else { '-' })
            .collect::<String>();
        text(dc, 518, 66, &hearts, 0x00ef4145);
        text(dc, 760, 40, "HINTS", 0x00677b9d);
        text(dc, 760, 66, &snapshot.hints.to_string(), 0x00232d42);

        for row in 0..9 {
            for column in 0..9 {
                let left = board_x + column as i32 * layout.cell_size;
                let top = board_y + row as i32 * layout.cell_size;
                let selected = snapshot.selected == Some((row, column));
                let color = if selected {
                    0x00cfe3ff
                } else if snapshot.states[row][column] == 1 {
                    0x00e2e9f2
                } else {
                    0x00fdfdfd
                };
                fill(
                    dc,
                    Rect {
                        left,
                        top,
                        right: left + layout.cell_size - 1,
                        bottom: top + layout.cell_size - 1,
                    },
                    color,
                );
                frame(
                    dc,
                    Rect {
                        left,
                        top,
                        right: left + layout.cell_size,
                        bottom: top + layout.cell_size,
                    },
                    if row % 3 == 0 || column % 3 == 0 {
                        0x004b5b73
                    } else {
                        0x00c5d0dd
                    },
                );
                let value = snapshot.values[row][column];
                if value != 0 {
                    text(
                        dc,
                        left + 28,
                        top + 22,
                        &value.to_string(),
                        if snapshot.states[row][column] == 3 {
                            0x00d93035
                        } else {
                            0x00232d42
                        },
                    );
                } else {
                    for note in 0..9 {
                        if snapshot.notes[row][column][note] {
                            let nx = left + 7 + (note as i32 % 3) * 18;
                            let ny = top + 7 + (note as i32 / 3) * 18;
                            text(dc, nx, ny, &(note + 1).to_string(), 0x00677b9d);
                        }
                    }
                }
            }
        }
        for index in 0..9 {
            let row = index / 3;
            let column = index % 3;
            let left = layout.control_x + column * (layout.button_size + layout.gap);
            let top = layout.control_y + row * (layout.button_size + layout.gap);
            button(
                dc,
                left,
                top,
                layout.button_size,
                layout.button_size,
                0x003f83f1,
                &(index + 1).to_string(),
            );
        }
        for (index, label) in ["Erase", "Note", "Hint"].into_iter().enumerate() {
            let left = layout.control_x + index as i32 * (layout.button_size + layout.gap);
            button(
                dc,
                left,
                layout.action_y,
                layout.button_size,
                72,
                if label == "Note" && snapshot.note_mode {
                    0x003f83f1
                } else {
                    0x00f5f8ff
                },
                label,
            );
        }
        let top_width = (layout.button_size * 3 + layout.gap - layout.gap) / 2;
        button(
            dc,
            layout.control_x,
            layout.top_y,
            top_width,
            40,
            0x00f5f8ff,
            if snapshot.result != 0 {
                "Result"
            } else if snapshot.paused {
                "Resume"
            } else {
                "Pause"
            },
        );
        button(
            dc,
            layout.control_x + top_width + layout.gap,
            layout.top_y,
            top_width,
            40,
            0x00f5f8ff,
            "Menu",
        );
        if snapshot.result != 0 {
            fill(
                dc,
                Rect {
                    left: 180,
                    top: 260,
                    right: 720,
                    bottom: 500,
                },
                0x00f8f9fc,
            );
            let title = match snapshot.result {
                1 => "PUZZLE COMPLETE",
                2 => "GAME OVER",
                _ => "TIME UP",
            };
            let message = match snapshot.result {
                1 => "Great work",
                2 => "All lives used",
                _ => "Time limit reached",
            };
            text(dc, 390, 330, title, 0x00232d42);
            text(dc, 390, 370, message, 0x00677b9d);
            button(dc, 350, 420, 200, 48, 0x003f83f1, "Menu");
        }
    }
}

#[cfg(windows)]
pub(crate) use win32::Presenter;
