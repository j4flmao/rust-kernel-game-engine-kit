use super::*;

pub(crate) struct SudokuGame {
    board: BoardUi,
    session: GameSession,
    saved: Option<SavedGame>,
    selected: Option<(usize, usize)>,
    note_mode: bool,
    screen: Screen,
    snapshot: std::sync::Arc<std::sync::Mutex<NativeSnapshot>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    shake_remaining_ns: u64,
    shake_phase: u8,
}

#[derive(Clone)]
struct SavedGame {
    session: GameSession,
    selected: Option<(usize, usize)>,
    note_mode: bool,
}

impl SudokuGame {
    pub(crate) fn new(
        board: BoardUi,
        session: GameSession,
        snapshot: std::sync::Arc<std::sync::Mutex<NativeSnapshot>>,
        running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            board,
            session,
            saved: None,
            selected: None,
            note_mode: false,
            screen: Screen::MainMenu,
            snapshot,
            running,
            shake_remaining_ns: 0,
            shake_phase: 0,
        }
    }

    fn save_current_session(&mut self) {
        if self.session.result == SessionResult::Ongoing {
            self.saved = Some(SavedGame::capture(
                &self.session,
                self.selected,
                self.note_mode,
            ));
        }
    }

    fn restore_saved_session(&mut self) -> bool {
        let Some(saved) = self.saved.take() else {
            return false;
        };
        let (session, selected, note_mode) = saved.restore();
        self.session = session;
        self.selected = selected;
        self.note_mode = note_mode;
        self.shake_remaining_ns = 0;
        self.shake_phase = 0;
        true
    }

    fn discard_saved_session(&mut self) {
        self.saved = None;
    }

    fn sync_snapshot(&self) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = NativeSnapshot::from_session(
                self.screen,
                &self.session,
                self.selected,
                self.note_mode,
                if self.shake_remaining_ns == 0 {
                    0
                } else {
                    match self.shake_phase % 6 {
                        0 => -6,
                        1 => 6,
                        2 => -4,
                        3 => 4,
                        4 => -2,
                        _ => 0,
                    }
                },
            );
        }
    }

    fn apply_action(
        &mut self,
        ctx: &mut KernelContext<'_>,
        ui: rust_kernel_game_engine_kit::kernel::SubscriberId,
        action: GameAction,
    ) {
        match action {
            GameAction::Move(row_delta, column_delta) => {
                let (row, column) = self.selected.unwrap_or((0, 0));
                let next_row = (row as i32 + row_delta).clamp(0, 8) as usize;
                let next_column = (column as i32 + column_delta).clamp(0, 8) as usize;
                self.selected = Some((next_row, next_column));
                publish_cell_style(
                    ctx,
                    ui,
                    self.board.cells[row][column],
                    &self.session,
                    row,
                    column,
                    false,
                );
                publish_cell_style(
                    ctx,
                    ui,
                    self.board.cells[next_row][next_column],
                    &self.session,
                    next_row,
                    next_column,
                    true,
                );
            }
            GameAction::ToggleNote => {
                self.note_mode = !self.note_mode;
                publish_status(
                    ctx,
                    ui,
                    self.board.status,
                    if self.note_mode {
                        "Note mode on"
                    } else {
                        "Note mode off"
                    },
                );
            }
            GameAction::Clear => {
                if let Some((row, column)) = self.selected {
                    self.session.clear(row, column);
                    publish_cell(ctx, ui, &self.board, &self.session, row, column, true);
                    publish_status(ctx, ui, self.board.status, "Cell cleared");
                }
            }
            GameAction::Hint => {
                let hinted = self.selected.and_then(|(row, column)| {
                    if self.session.values[row][column] == 0
                        && self.session.states[row][column] != CellState::Given
                    {
                        self.session.hint_at(row, column)
                    } else {
                        None
                    }
                });
                let hinted = hinted.or_else(|| self.session.hint());
                if let Some((row, column, _)) = hinted {
                    self.selected = Some((row, column));
                    publish_cell(ctx, ui, &self.board, &self.session, row, column, true);
                    publish_status(ctx, ui, self.board.status, "Hint used");
                } else {
                    publish_status(ctx, ui, self.board.status, "No hints remaining");
                }
            }
            GameAction::Pause => {
                if self.session.result == SessionResult::Ongoing {
                    self.session.paused = !self.session.paused;
                    if self.session.paused {
                        self.save_current_session();
                    }
                    publish_status(
                        ctx,
                        ui,
                        self.board.status,
                        if self.session.paused {
                            "Paused"
                        } else {
                            "Resumed"
                        },
                    );
                } else {
                    publish_status(ctx, ui, self.board.status, "Session finished | choose Menu");
                }
            }
            GameAction::Digit(value) => {
                if let Some((row, column)) = self.selected {
                    if self.note_mode {
                        self.session.toggle_note(row, column, value);
                        publish_cell(ctx, ui, &self.board, &self.session, row, column, true);
                    } else {
                        let (accepted, correct) = self.session.make_move(row, column, value);
                        if accepted && !correct {
                            self.shake_remaining_ns = 300_000_000;
                            self.shake_phase = 0;
                        }
                        publish_cell(ctx, ui, &self.board, &self.session, row, column, true);
                        if !correct {
                            publish_status(ctx, ui, self.board.status, "Wrong number: life lost");
                        } else if self.session.result == SessionResult::Win {
                            let (stars, score) = self.session.score();
                            let status = format!(
                                "Solved {} • {} stars • {} points",
                                self.session.config.name, stars, score
                            );
                            publish_status(ctx, ui, self.board.status, &status);
                        } else {
                            publish_status(ctx, ui, self.board.status, "Good move");
                        }
                    }
                }
            }
        }
    }
}

impl SavedGame {
    fn capture(session: &GameSession, selected: Option<(usize, usize)>, note_mode: bool) -> Self {
        let mut session = session.clone();
        // A saved game is always restored from a safe boundary. The timer
        // must not advance while the game is in the menu.
        session.paused = true;
        Self {
            session,
            selected,
            note_mode,
        }
    }

    fn restore(self) -> (GameSession, Option<(usize, usize)>, bool) {
        let mut session = self.session;
        session.paused = false;
        (session, self.selected, self.note_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_game_round_trip_preserves_progress_and_resumes_timer() {
        let mut session = GameSession::new(Difficulty::Medium);
        let (row, column) = (0..9)
            .flat_map(|row| (0..9).map(move |column| (row, column)))
            .find(|&(row, column)| session.states[row][column] == CellState::Empty)
            .expect("generated puzzle has an editable cell");
        let value = SOLUTION[row][column];
        assert!(session.make_move(row, column, value).0);
        session.tick(2_500_000_000);
        let saved = SavedGame::capture(&session, Some((row, column)), true);

        let (restored, selected, note_mode) = saved.restore();
        assert_eq!(restored.values, session.values);
        assert_eq!(restored.states, session.states);
        assert_eq!(restored.notes, session.notes);
        assert_eq!(restored.elapsed, 2);
        assert_eq!(restored.lives, session.lives);
        assert_eq!(restored.mistakes, session.mistakes);
        assert_eq!(restored.hints_used, session.hints_used);
        assert_eq!(selected, Some((row, column)));
        assert!(note_mode);
        assert!(!restored.paused);
        let elapsed = restored.elapsed;
        let mut resumed = restored;
        resumed.tick(1_000_000_000);
        assert_eq!(resumed.elapsed, elapsed + 1);
    }
}

impl GamePlugin for SudokuGame {
    fn name(&self) -> &'static str {
        "sudoku-game"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["input", "ui"]
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}

    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64) {
        self.session.tick(dt_ns);
        if self.shake_remaining_ns != 0 {
            self.shake_remaining_ns = self.shake_remaining_ns.saturating_sub(dt_ns);
            self.shake_phase = self.shake_phase.wrapping_add(1);
        }
        let mut target = None;
        let mut action = None;
        for envelope in ctx.receive() {
            if envelope.topic == 2 {
                if let Ok(frame) = envelope.downcast::<InputFrame>() {
                    if frame.pressed_buttons & 1 != 0 {
                        target = if self.screen == Screen::Playing
                            && self.session.result != SessionResult::Ongoing
                        {
                            layout::hit_test_result_menu(frame.mouse_x, frame.mouse_y)
                        } else {
                            layout::hit_test(self.screen, frame.mouse_x, frame.mouse_y)
                        };
                    }
                    action = frame
                        .pressed_keys
                        .iter()
                        .take(usize::from(frame.pressed_key_count))
                        .find_map(|key| action_from_key(*key));
                }
            }
        }
        let Some(ui) = ctx.resolve("ui") else { return };
        match self.screen {
            Screen::MainMenu => {
                match target {
                    Some(HitTarget::Menu(MenuAction::Continue)) => {
                        if self.restore_saved_session() {
                            self.screen = Screen::Playing;
                            publish_visibility(ctx, ui, self.board.main_menu_root, false);
                            publish_visibility(ctx, ui, self.board.menu_root, false);
                            publish_visibility(ctx, ui, self.board.board_root, true);
                            for row in 0..9 {
                                for column in 0..9 {
                                    publish_cell(
                                        ctx,
                                        ui,
                                        &self.board,
                                        &self.session,
                                        row,
                                        column,
                                        self.selected == Some((row, column)),
                                    );
                                }
                            }
                            publish_status(ctx, ui, self.board.status, "Game continued");
                        } else {
                            publish_status(ctx, ui, self.board.status, "No saved game");
                        }
                    }
                    Some(HitTarget::Menu(MenuAction::NewGame)) => {
                        self.screen = Screen::Difficulty;
                        publish_visibility(ctx, ui, self.board.main_menu_root, false);
                        publish_visibility(ctx, ui, self.board.menu_root, true);
                        publish_status(ctx, ui, self.board.status, "Choose a difficulty");
                    }
                    Some(HitTarget::Menu(MenuAction::Leaderboard)) => {
                        publish_status(
                            ctx,
                            ui,
                            self.board.status,
                            "Leaderboard is ready for your scores",
                        );
                    }
                    Some(HitTarget::Menu(MenuAction::Quit)) => {
                        self.running
                            .store(false, std::sync::atomic::Ordering::Release);
                    }
                    _ => {}
                }
                self.sync_snapshot();
                return;
            }
            Screen::Difficulty => {
                if let Some(HitTarget::Difficulty(choice)) = target {
                    let difficulty = match choice {
                        DifficultyChoice::Easy => Difficulty::Easy,
                        DifficultyChoice::Medium => Difficulty::Medium,
                        DifficultyChoice::Hard => Difficulty::Hard,
                    };
                    self.discard_saved_session();
                    self.session = GameSession::new(difficulty);
                    self.screen = Screen::Playing;
                    // A new session must never inherit transient state from
                    // the previous game (especially Note mode after a
                    // Game Over -> Menu -> New Game flow).
                    self.selected = None;
                    self.note_mode = false;
                    self.shake_remaining_ns = 0;
                    self.shake_phase = 0;
                    publish_visibility(ctx, ui, self.board.main_menu_root, false);
                    publish_visibility(ctx, ui, self.board.menu_root, false);
                    publish_visibility(ctx, ui, self.board.board_root, true);
                    for row in 0..9 {
                        for column in 0..9 {
                            publish_cell(ctx, ui, &self.board, &self.session, row, column, false);
                        }
                    }
                    let status = format!(
                        "Game started | 1-9 enter numbers | {} hints",
                        self.session.hints_remaining()
                    );
                    publish_status(ctx, ui, self.board.status, &status);
                }
                self.sync_snapshot();
                return;
            }
            Screen::Playing => {}
        }

        if let Some(HitTarget::Cell { row, column }) = target {
            let previous = self.selected.replace((row, column));
            if let Some((old_row, old_column)) = previous {
                publish_cell_style(
                    ctx,
                    ui,
                    self.board.cells[old_row][old_column],
                    &self.session,
                    old_row,
                    old_column,
                    false,
                );
            }
            publish_cell_style(
                ctx,
                ui,
                self.board.cells[row][column],
                &self.session,
                row,
                column,
                true,
            );
        }

        let target_action = match target {
            Some(HitTarget::ResultMenu) => {
                self.screen = Screen::MainMenu;
                self.selected = None;
                publish_visibility(ctx, ui, self.board.board_root, false);
                publish_visibility(ctx, ui, self.board.main_menu_root, true);
                publish_status(ctx, ui, self.board.status, "Choose an option");
                None
            }
            Some(HitTarget::Digit(value)) => Some(GameAction::Digit(value)),
            Some(HitTarget::Action(PlayingAction::Erase)) => Some(GameAction::Clear),
            Some(HitTarget::Action(PlayingAction::Note)) => Some(GameAction::ToggleNote),
            Some(HitTarget::Action(PlayingAction::Hint)) => Some(GameAction::Hint),
            Some(HitTarget::Top(TopAction::Pause)) => Some(GameAction::Pause),
            Some(HitTarget::Top(TopAction::Menu)) => {
                self.save_current_session();
                self.session.paused = true;
                self.screen = Screen::MainMenu;
                publish_visibility(ctx, ui, self.board.board_root, false);
                publish_visibility(ctx, ui, self.board.main_menu_root, true);
                publish_status(ctx, ui, self.board.status, "Choose an option");
                None
            }
            _ => None,
        };
        if let Some(action) = target_action.or(action) {
            self.apply_action(ctx, ui, action);
        }
        self.sync_snapshot();
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}
