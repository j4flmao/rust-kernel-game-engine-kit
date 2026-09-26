//! Sudoku domain model: puzzle generation, rules, timer, lives and hints.
//!
//! This module deliberately has no UI or platform dependencies. It is the
//! reusable part a different game front-end can call without inheriting the
//! example's window/layout code.

pub(crate) const SOLUTION: [[u8; 9]; 9] = [
    [5, 3, 4, 6, 7, 8, 9, 1, 2],
    [6, 7, 2, 1, 9, 5, 3, 4, 8],
    [1, 9, 8, 3, 4, 2, 5, 6, 7],
    [8, 5, 9, 7, 6, 1, 4, 2, 3],
    [4, 2, 6, 8, 5, 3, 7, 9, 1],
    [7, 1, 3, 9, 2, 4, 8, 5, 6],
    [9, 6, 1, 5, 3, 7, 2, 8, 4],
    [2, 8, 7, 4, 1, 9, 6, 3, 5],
    [3, 4, 5, 2, 8, 6, 1, 7, 9],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DifficultyConfig {
    pub(crate) name: &'static str,
    pub(crate) blanks_min: usize,
    pub(crate) blanks_max: usize,
    pub(crate) max_lives: u8,
    pub(crate) max_hints: u8,
    pub(crate) max_time_sec: u32,
    pub(crate) three_star_sec: u32,
    pub(crate) two_star_sec: u32,
    pub(crate) score_multiplier_x10: u32,
}

impl Difficulty {
    pub(crate) fn config(self) -> DifficultyConfig {
        match self {
            Self::Easy => DifficultyConfig {
                name: "Easy",
                blanks_min: 46,
                blanks_max: 50,
                max_lives: 3,
                max_hints: 3,
                max_time_sec: 1200,
                three_star_sec: 300,
                two_star_sec: 600,
                score_multiplier_x10: 10,
            },
            Self::Medium => DifficultyConfig {
                name: "Medium",
                blanks_min: 51,
                blanks_max: 55,
                max_lives: 3,
                max_hints: 1,
                max_time_sec: 1800,
                three_star_sec: 480,
                two_star_sec: 1080,
                score_multiplier_x10: 15,
            },
            Self::Hard => DifficultyConfig {
                name: "Hard",
                blanks_min: 56,
                blanks_max: 64,
                max_lives: 3,
                max_hints: 0,
                max_time_sec: 2700,
                three_star_sec: 720,
                two_star_sec: 1680,
                score_multiplier_x10: 20,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionResult {
    Ongoing,
    Win,
    LoseLives,
    LoseTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellState {
    Empty,
    Given,
    Player,
    Error,
    Hint,
}

struct PuzzleGenerator {
    state: u64,
}

impl PuzzleGenerator {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 7;
        self.state ^= self.state >> 9;
        self.state ^= self.state << 8;
        self.state
    }
    fn generate(&mut self, difficulty: Difficulty) -> [[u8; 9]; 9] {
        let config = difficulty.config();
        let target = config.blanks_min
            + (self.next() as usize % (config.blanks_max - config.blanks_min + 1));
        let mut puzzle = SOLUTION;
        let mut positions = [0usize; 81];
        for (index, position) in positions.iter_mut().enumerate() {
            *position = index;
        }
        for index in (1..positions.len()).rev() {
            let swap = self.next() as usize % (index + 1);
            positions.swap(index, swap);
        }
        let mut removed = 0usize;
        for position in positions {
            if removed == target {
                break;
            }
            let row = position / 9;
            let column = position % 9;
            let saved = puzzle[row][column];
            puzzle[row][column] = 0;
            let mut candidate = puzzle;
            if count_solutions(&mut candidate, 2) == 1 {
                removed += 1;
            } else {
                puzzle[row][column] = saved;
            }
        }
        puzzle
    }
}

fn count_solutions(grid: &mut [[u8; 9]; 9], limit: u8) -> u8 {
    let Some((row, column)) = first_empty(grid) else {
        return 1;
    };
    let mut found = 0u8;
    for value in 1..=9 {
        if valid_value(grid, row, column, value) {
            grid[row][column] = value;
            found = found.saturating_add(count_solutions(grid, limit));
            grid[row][column] = 0;
            if found >= limit {
                return found;
            }
        }
    }
    found
}

fn first_empty(grid: &[[u8; 9]; 9]) -> Option<(usize, usize)> {
    for row in 0..9 {
        for column in 0..9 {
            if grid[row][column] == 0 {
                return Some((row, column));
            }
        }
    }
    None
}

fn valid_value(grid: &[[u8; 9]; 9], row: usize, column: usize, value: u8) -> bool {
    (0..9).all(|other| grid[row][other] != value)
        && (0..9).all(|other| grid[other][column] != value)
        && {
            let row0 = row / 3 * 3;
            let col0 = column / 3 * 3;
            (row0..row0 + 3).all(|r| (col0..col0 + 3).all(|c| grid[r][c] != value))
        }
}

#[derive(Clone)]
pub(crate) struct GameSession {
    pub(crate) config: DifficultyConfig,
    pub(crate) puzzle: [[u8; 9]; 9],
    pub(crate) values: [[u8; 9]; 9],
    pub(crate) states: [[CellState; 9]; 9],
    pub(crate) notes: [[[bool; 9]; 9]; 9],
    pub(crate) elapsed: u32,
    pub(crate) lives: u8,
    pub(crate) mistakes: u32,
    pub(crate) hints_used: u32,
    pub(crate) result: SessionResult,
    pub(crate) paused: bool,
    fractional_ns: u64,
}

impl GameSession {
    pub(crate) fn new(difficulty: Difficulty) -> Self {
        let mut generator = PuzzleGenerator::new(0x5EED_2026);
        let puzzle = generator.generate(difficulty);
        let mut states = [[CellState::Empty; 9]; 9];
        for row in 0..9 {
            for column in 0..9 {
                if puzzle[row][column] != 0 {
                    states[row][column] = CellState::Given;
                }
            }
        }
        Self {
            config: difficulty.config(),
            puzzle,
            values: puzzle,
            states,
            notes: [[[false; 9]; 9]; 9],
            elapsed: 0,
            lives: difficulty.config().max_lives,
            mistakes: 0,
            hints_used: 0,
            result: SessionResult::Ongoing,
            paused: false,
            fractional_ns: 0,
        }
    }

    pub(crate) fn tick(&mut self, dt_ns: u64) {
        if self.paused || self.result != SessionResult::Ongoing {
            return;
        }
        self.fractional_ns = self.fractional_ns.saturating_add(dt_ns);
        let elapsed_seconds = self.fractional_ns / 1_000_000_000;
        self.fractional_ns %= 1_000_000_000;
        self.elapsed = self
            .elapsed
            .saturating_add(u32::try_from(elapsed_seconds).unwrap_or(u32::MAX));
        if self.elapsed >= self.config.max_time_sec {
            self.elapsed = self.config.max_time_sec;
            self.result = SessionResult::LoseTime;
            self.paused = true;
        }
    }

    pub(crate) fn make_move(&mut self, row: usize, column: usize, value: u8) -> (bool, bool) {
        if row >= 9
            || column >= 9
            || self.result != SessionResult::Ongoing
            || self.paused
            || self.states[row][column] == CellState::Given
            || value > 9
        {
            return (false, false);
        }
        if value == 0 {
            self.clear(row, column);
            return (true, true);
        }
        self.values[row][column] = value;
        self.notes[row][column] = [false; 9];
        if value == SOLUTION[row][column] {
            self.states[row][column] = CellState::Player;
            if self.values.iter().flatten().all(|v| *v != 0) && self.values == SOLUTION {
                self.result = SessionResult::Win;
                self.paused = true;
            }
            (true, true)
        } else {
            self.states[row][column] = CellState::Error;
            self.mistakes = self.mistakes.saturating_add(1);
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.result = SessionResult::LoseLives;
                self.paused = true;
            }
            (true, false)
        }
    }

    pub(crate) fn clear(&mut self, row: usize, column: usize) {
        if row < 9 && column < 9 && self.states[row][column] != CellState::Given {
            self.values[row][column] = 0;
            self.states[row][column] = CellState::Empty;
            self.notes[row][column] = [false; 9];
        }
    }

    pub(crate) fn toggle_note(&mut self, row: usize, column: usize, value: u8) {
        if row >= 9
            || column >= 9
            || value == 0
            || value > 9
            || self.states[row][column] == CellState::Given
            || self.values[row][column] != 0
        {
            return;
        }
        self.notes[row][column][usize::from(value - 1)] =
            !self.notes[row][column][usize::from(value - 1)];
    }

    pub(crate) fn hint(&mut self) -> Option<(usize, usize, u8)> {
        if self.hints_used >= u32::from(self.config.max_hints)
            || self.result != SessionResult::Ongoing
        {
            return None;
        }
        for row in 0..9 {
            for column in 0..9 {
                if self.values[row][column] == 0 {
                    return self.hint_at(row, column);
                }
            }
        }
        None
    }

    pub(crate) fn hint_at(&mut self, row: usize, column: usize) -> Option<(usize, usize, u8)> {
        if row >= 9
            || column >= 9
            || self.hints_used >= u32::from(self.config.max_hints)
            || self.result != SessionResult::Ongoing
            || self.values[row][column] != 0
            || self.states[row][column] == CellState::Given
        {
            return None;
        }
        let value = SOLUTION[row][column];
        self.values[row][column] = value;
        self.states[row][column] = CellState::Hint;
        self.notes[row][column] = [false; 9];
        self.hints_used = self.hints_used.saturating_add(1);
        if self.values.iter().flatten().all(|value| *value != 0) {
            self.result = SessionResult::Win;
            self.paused = true;
        }
        Some((row, column, value))
    }

    pub(crate) fn hints_remaining(&self) -> u8 {
        self.config
            .max_hints
            .saturating_sub(self.hints_used.min(u32::from(self.config.max_hints)) as u8)
    }

    pub(crate) fn score(&self) -> (u8, u32) {
        if self.result != SessionResult::Win {
            return (0, 0);
        }
        let stars = if self.elapsed <= self.config.three_star_sec {
            3
        } else if self.elapsed <= self.config.two_star_sec {
            2
        } else {
            1
        };
        let time_bonus = self
            .config
            .max_time_sec
            .saturating_sub(self.elapsed)
            .saturating_mul(10);
        let penalties = self
            .mistakes
            .saturating_mul(500)
            .saturating_add(self.hints_used.saturating_mul(200));
        let raw = 10_000u32
            .saturating_add(time_bonus)
            .saturating_sub(penalties);
        (
            stars,
            raw.saturating_mul(self.config.score_multiplier_x10) / 10,
        )
    }
}

pub(crate) fn difficulty_from_environment() -> Difficulty {
    match std::env::var("SUDOKU_DIFFICULTY")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "medium" => Difficulty::Medium,
        "hard" => Difficulty::Hard,
        _ => Difficulty::Easy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_accumulates_subsecond_deltas_without_drift() {
        let mut session = GameSession::new(Difficulty::Easy);
        session.tick(500_000_000);
        assert_eq!(session.elapsed, 0);
        session.tick(500_000_000);
        assert_eq!(session.elapsed, 1);
        session.paused = true;
        session.tick(10_000_000_000);
        assert_eq!(session.elapsed, 1);
    }

    #[test]
    fn wrong_move_consumes_one_life_and_given_cells_are_immutable() {
        let mut session = GameSession::new(Difficulty::Easy);
        let Some((given_row, given_column)) = (0..9)
            .flat_map(|row| (0..9).map(move |column| (row, column)))
            .find(|&(row, column)| session.states[row][column] == CellState::Given)
        else {
            panic!("generated puzzle must contain a given");
        };
        let given = session.values[given_row][given_column];
        assert!(!session.make_move(given_row, given_column, 1).0);
        assert_eq!(session.values[given_row][given_column], given);

        let (row, column) = (0..9)
            .flat_map(|row| (0..9).map(move |column| (row, column)))
            .find(|&(row, column)| session.states[row][column] == CellState::Empty)
            .expect("generated puzzle must contain an editable cell");
        let wrong = if SOLUTION[row][column] == 1 { 2 } else { 1 };
        let lives = session.lives;
        let (_, correct) = session.make_move(row, column, wrong);
        assert!(!correct);
        assert_eq!(session.lives, lives - 1);
        assert_eq!(session.mistakes, 1);
    }

    #[test]
    fn selected_hint_fills_that_cell_and_respects_hint_budget() {
        let mut session = GameSession::new(Difficulty::Easy);
        let (row, column) = (0..9)
            .flat_map(|row| (0..9).map(move |column| (row, column)))
            .find(|&(row, column)| session.states[row][column] == CellState::Empty)
            .expect("generated puzzle must contain an editable cell");
        assert_eq!(
            session.hint_at(row, column),
            Some((row, column, SOLUTION[row][column]))
        );
        assert_eq!(session.hints_remaining(), 2);
        assert!(session.hint_at(row, column).is_none());
    }
}
