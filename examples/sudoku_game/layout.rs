//! Single source of truth for the Sudoku example's geometry and hit testing.
//!
//! The C++ reference computes the board and control rectangles once per
//! window size and uses those same rectangles for drawing and input. Keeping
//! that contract here prevents the old failure mode where the visible button
//! and the clickable button had different coordinates.

pub const DESIGN_WIDTH: i32 = 900;
pub const DESIGN_HEIGHT: i32 = 840;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    MainMenu,
    Difficulty,
    Playing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HitTarget {
    Menu(MenuAction),
    Difficulty(DifficultyChoice),
    Cell { row: usize, column: usize },
    Digit(u8),
    Action(PlayingAction),
    Top(TopAction),
    ResultMenu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Continue,
    NewGame,
    Leaderboard,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DifficultyChoice {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlayingAction {
    Erase,
    Note,
    Hint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TopAction {
    Pause,
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlayingLayout {
    pub board_x: i32,
    pub board_y: i32,
    pub board_size: i32,
    pub cell_size: i32,
    pub panel_x: i32,
    pub control_x: i32,
    pub control_y: i32,
    pub button_size: i32,
    pub gap: i32,
    pub action_y: i32,
    pub top_y: i32,
}

impl PlayingLayout {
    pub fn for_window(width: i32, height: i32) -> Self {
        let mut board_size = (width - 320).min(height - 200).max(360);
        board_size = (board_size / 9) * 9;
        let board_x = 40;
        let board_y = 110;
        let button_size = 56;
        let gap = 10;
        let grid_width = button_size * 3 + gap * 2;
        let panel_x = board_x + board_size + 30;
        let available_width = (width - panel_x - 30).max(grid_width);
        let control_x = panel_x + ((available_width - grid_width) / 2).max(0);
        let control_y =
            (board_y + board_size - (button_size * 3 + gap * 2 + 16 + 72)).max(board_y + 60);
        Self {
            board_x,
            board_y,
            board_size,
            cell_size: board_size / 9,
            panel_x,
            control_x,
            control_y,
            button_size,
            gap,
            action_y: control_y + button_size * 3 + gap * 3 + 16,
            top_y: board_y,
        }
    }

    fn contains(x: i32, y: i32, left: i32, top: i32, width: i32, height: i32) -> bool {
        x >= left && x < left + width && y >= top && y < top + height
    }

    pub fn hit_test(self, x: i32, y: i32) -> Option<HitTarget> {
        if Self::contains(
            x,
            y,
            self.board_x,
            self.board_y,
            self.board_size,
            self.board_size,
        ) {
            let column = ((x - self.board_x) / self.cell_size) as usize;
            let row = ((y - self.board_y) / self.cell_size) as usize;
            return Some(HitTarget::Cell { row, column });
        }

        for index in 0..9 {
            let row = index / 3;
            let column = index % 3;
            let left = self.control_x + column * (self.button_size + self.gap);
            let top = self.control_y + row * (self.button_size + self.gap);
            if Self::contains(x, y, left, top, self.button_size, self.button_size) {
                return Some(HitTarget::Digit((index + 1) as u8));
            }
        }

        for (index, action) in [
            PlayingAction::Erase,
            PlayingAction::Note,
            PlayingAction::Hint,
        ]
        .into_iter()
        .enumerate()
        {
            let left = self.control_x + index as i32 * (self.button_size + self.gap);
            if Self::contains(x, y, left, self.action_y, self.button_size, 72) {
                return Some(HitTarget::Action(action));
            }
        }

        let top_width = (self.button_size * 3 + self.gap * 2 - self.gap) / 2;
        if Self::contains(x, y, self.control_x, self.top_y, top_width, 40) {
            return Some(HitTarget::Top(TopAction::Pause));
        }
        if Self::contains(
            x,
            y,
            self.control_x + top_width + self.gap,
            self.top_y,
            top_width,
            40,
        ) {
            return Some(HitTarget::Top(TopAction::Menu));
        }
        None
    }
}

pub(crate) fn hit_test(screen: Screen, x: i32, y: i32) -> Option<HitTarget> {
    match screen {
        Screen::MainMenu => {
            let left = (DESIGN_WIDTH - 280) / 2;
            let top = 280;
            for (index, action) in [
                MenuAction::Continue,
                MenuAction::NewGame,
                MenuAction::Leaderboard,
                MenuAction::Quit,
            ]
            .into_iter()
            .enumerate()
            {
                if PlayingLayout::contains(x, y, left, top + index as i32 * 70, 280, 56) {
                    return Some(HitTarget::Menu(action));
                }
            }
            None
        }
        Screen::Difficulty => {
            let card_width = 220;
            let card_height = 280;
            let gap = 24;
            let left = (DESIGN_WIDTH - card_width * 3 - gap * 2) / 2;
            let top = (DESIGN_HEIGHT - card_height) / 2;
            for (index, choice) in [
                DifficultyChoice::Easy,
                DifficultyChoice::Medium,
                DifficultyChoice::Hard,
            ]
            .into_iter()
            .enumerate()
            {
                if PlayingLayout::contains(
                    x,
                    y,
                    left + index as i32 * (card_width + gap),
                    top,
                    card_width,
                    card_height,
                ) {
                    return Some(HitTarget::Difficulty(choice));
                }
            }
            None
        }
        Screen::Playing => PlayingLayout::for_window(DESIGN_WIDTH, DESIGN_HEIGHT).hit_test(x, y),
    }
}

pub(crate) fn hit_test_result_menu(x: i32, y: i32) -> Option<HitTarget> {
    if PlayingLayout::contains(x, y, 350, 420, 200, 48) {
        Some(HitTarget::ResultMenu)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playing_hit_test_matches_reference_layout() {
        let layout = PlayingLayout::for_window(DESIGN_WIDTH, DESIGN_HEIGHT);
        assert_eq!(layout.board_x, 40);
        assert_eq!(layout.board_y, 110);
        assert_eq!(layout.board_size, 576);
        assert_eq!(
            layout.hit_test(72, 142),
            Some(HitTarget::Cell { row: 0, column: 0 })
        );
        assert_eq!(
            layout.hit_test(layout.control_x + 5, layout.control_y + 5),
            Some(HitTarget::Digit(1))
        );
        assert_eq!(
            layout.hit_test(layout.control_x + 5, layout.action_y + 5),
            Some(HitTarget::Action(PlayingAction::Erase))
        );
        for value in 1u8..=9 {
            let index = value - 1;
            let row = index / 3;
            let column = index % 3;
            let x = layout.control_x + column as i32 * (layout.button_size + layout.gap) + 5;
            let y = layout.control_y + row as i32 * (layout.button_size + layout.gap) + 5;
            assert_eq!(layout.hit_test(x, y), Some(HitTarget::Digit(value)));
        }
        assert_eq!(
            layout.hit_test(
                layout.control_x + layout.button_size + layout.gap + 5,
                layout.action_y + 5
            ),
            Some(HitTarget::Action(PlayingAction::Note))
        );
        assert_eq!(
            layout.hit_test(
                layout.control_x + 2 * (layout.button_size + layout.gap) + 5,
                layout.action_y + 5
            ),
            Some(HitTarget::Action(PlayingAction::Hint))
        );
        assert_eq!(
            layout.hit_test(layout.control_x + 5, layout.top_y + 5),
            Some(HitTarget::Top(TopAction::Pause))
        );
    }

    #[test]
    fn menu_and_difficulty_rects_are_clickable() {
        assert_eq!(
            hit_test(Screen::MainMenu, 450, 300),
            Some(HitTarget::Menu(MenuAction::Continue))
        );
        assert_eq!(
            hit_test(Screen::Difficulty, 110, 300),
            Some(HitTarget::Difficulty(DifficultyChoice::Easy))
        );
        assert_eq!(hit_test_result_menu(450, 440), Some(HitTarget::ResultMenu));
    }
}
