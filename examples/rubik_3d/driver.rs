//! Rubik application driver: move queue, fixed-step animation, and commands.

use std::collections::VecDeque;

use super::state::{Axis, CubeState, RotationCommand};
use rust_kernel_game_engine_kit::kernel::{KernelContext, Subsystem, SystemAccess};
use rust_kernel_game_engine_kit::subsystems::messages::InputFrame;

const MOVE_DURATION_NS: u64 = 250_000_000;
const MAX_QUEUED_MOVES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveFace {
    U,
    D,
    L,
    R,
    F,
    B,
    M,
    E,
    S,
}

impl MoveFace {
    pub const fn command(self, inverse: bool) -> RotationCommand {
        let (axis, layer, direction) = match self {
            Self::R => (Axis::X, 1, 1),
            Self::L => (Axis::X, -1, -1),
            Self::M => (Axis::X, 0, -1),
            Self::U => (Axis::Y, 1, 1),
            Self::D => (Axis::Y, -1, -1),
            Self::E => (Axis::Y, 0, -1),
            Self::F => (Axis::Z, 1, 1),
            Self::B => (Axis::Z, -1, -1),
            Self::S => (Axis::Z, 0, 1),
        };
        let direction = if inverse { -direction } else { direction };
        RotationCommand::new(axis, layer, direction).expect("static move must be valid")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveRequest {
    pub face: MoveFace,
    pub inverse: bool,
    pub double: bool,
}

impl MoveRequest {
    pub const fn normal(face: MoveFace) -> Self {
        Self {
            face,
            inverse: false,
            double: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveRotation {
    pub command: RotationCommand,
    pub progress: f32,
}

pub struct RubikDriver {
    state: CubeState,
    queue: VecDeque<RotationCommand>,
    active: Option<(RotationCommand, u64)>,
    rng: XorShift32,
}

impl RubikDriver {
    pub fn new() -> Self {
        Self {
            state: CubeState::new(),
            queue: VecDeque::with_capacity(MAX_QUEUED_MOVES),
            active: None,
            rng: XorShift32::new(0x9e37_79b9),
        }
    }

    pub fn state(&self) -> &CubeState {
        &self.state
    }

    pub fn active_rotation(&self) -> Option<ActiveRotation> {
        self.active.map(|(command, elapsed)| ActiveRotation {
            command,
            progress: (elapsed as f32 / MOVE_DURATION_NS as f32).clamp(0.0, 1.0),
        })
    }

    pub fn is_idle(&self) -> bool {
        self.active.is_none() && self.queue.is_empty()
    }

    pub fn enqueue(&mut self, request: MoveRequest) -> bool {
        let count = if request.double { 2 } else { 1 };
        if self.queue.len().saturating_add(count) > MAX_QUEUED_MOVES {
            return false;
        }
        let command = request.face.command(request.inverse);
        for _ in 0..count {
            self.queue.push_back(command);
        }
        true
    }

    pub fn enqueue_command(&mut self, command: RotationCommand) -> bool {
        if self.queue.len() >= MAX_QUEUED_MOVES {
            return false;
        }
        self.queue.push_back(command);
        true
    }

    /// Advances one fixed simulation tick.  A move is committed to the
    /// canonical state only after its animation reaches 100%.
    pub fn advance(&mut self, dt_ns: u64) {
        let mut remaining = dt_ns.min(MOVE_DURATION_NS);
        while remaining > 0 || self.active.is_none() && !self.queue.is_empty() {
            if self.active.is_none() {
                let Some(command) = self.queue.pop_front() else {
                    break;
                };
                self.active = Some((command, 0));
            }
            let Some((command, elapsed)) = self.active else {
                break;
            };
            let needed = MOVE_DURATION_NS.saturating_sub(elapsed);
            let step = remaining.min(needed);
            let next_elapsed = elapsed.saturating_add(step);
            self.active = Some((command, next_elapsed));
            remaining = remaining.saturating_sub(step);
            if next_elapsed >= MOVE_DURATION_NS {
                let _ = self.state.apply_move(command);
                self.active = None;
            } else {
                break;
            }
        }
    }

    pub fn undo(&mut self) -> bool {
        if !self.is_idle() {
            return false;
        }
        self.state.undo()
    }

    pub fn redo(&mut self) -> bool {
        if !self.is_idle() {
            return false;
        }
        self.state.redo()
    }

    pub fn reset(&mut self) {
        self.queue.clear();
        self.active = None;
        self.state.reset();
    }

    pub fn shuffle(&mut self, count: usize) {
        self.queue.clear();
        self.active = None;
        let mut previous: Option<RotationCommand> = None;
        for _ in 0..count.min(MAX_QUEUED_MOVES) {
            let command = loop {
                let face = match self.rng.next() % 9 {
                    0 => MoveFace::U,
                    1 => MoveFace::D,
                    2 => MoveFace::L,
                    3 => MoveFace::R,
                    4 => MoveFace::F,
                    5 => MoveFace::B,
                    6 => MoveFace::M,
                    7 => MoveFace::E,
                    _ => MoveFace::S,
                };
                let command = face.command(self.rng.next() & 1 != 0);
                if previous
                    .map(|last| last.axis != command.axis || last.layer != command.layer)
                    .unwrap_or(true)
                {
                    break command;
                }
            };
            previous = Some(command);
            let _ = self.enqueue_command(command);
        }
    }

    pub fn solve_history(&mut self) {
        if !self.is_idle() {
            return;
        }
        let history = self.state.history().to_vec();
        self.queue.clear();
        for command in history.into_iter().rev() {
            let _ = self.enqueue_command(command.inverse());
        }
    }
}

impl Default for RubikDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for RubikDriver {
    fn name(&self) -> &'static str {
        "rubik-driver"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["input"]
    }

    fn access(&self) -> SystemAccess {
        SystemAccess::read_write(&["input"], &["rubik-state", "rubik-render-command"])
    }

    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}

    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64) {
        for envelope in ctx.receive() {
            if envelope.topic != 2 {
                continue;
            }
            let Ok(input) = envelope.downcast::<InputFrame>() else {
                continue;
            };
            for key in input.pressed_keys[..usize::from(input.pressed_key_count).min(32)].iter() {
                match *key {
                    0x48 => self.shuffle(24),
                    0x5a => {
                        let _ = self.undo();
                    }
                    0x59 => {
                        let _ = self.redo();
                    }
                    0x54 => self.solve_history(),
                    0x4e => self.reset(),
                    _ => {
                        let request = match *key {
                            0x55 => Some(MoveRequest::normal(MoveFace::U)),
                            0x44 => Some(MoveRequest::normal(MoveFace::D)),
                            0x4c => Some(MoveRequest::normal(MoveFace::L)),
                            0x52 => Some(MoveRequest::normal(MoveFace::R)),
                            0x46 => Some(MoveRequest::normal(MoveFace::F)),
                            0x42 => Some(MoveRequest::normal(MoveFace::B)),
                            0x4d => Some(MoveRequest::normal(MoveFace::M)),
                            0x45 => Some(MoveRequest::normal(MoveFace::E)),
                            0x53 => Some(MoveRequest::normal(MoveFace::S)),
                            _ => None,
                        };
                        if let Some(request) = request {
                            let _ = self.enqueue(request);
                        }
                    }
                }
            }
        }
        self.advance(dt_ns);
    }

    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[derive(Clone, Copy)]
struct XorShift32(u32);

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        if seed == 0 {
            Self(1)
        } else {
            Self(seed)
        }
    }

    fn next(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.0 = value;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_commits_only_after_duration() {
        let mut driver = RubikDriver::new();
        assert!(driver.enqueue(MoveRequest::normal(MoveFace::R)));
        driver.advance(100_000_000);
        assert!(!driver.is_idle());
        assert_eq!(driver.state().history_len(), 0);
        driver.advance(150_000_000);
        assert!(driver.is_idle());
        assert_eq!(driver.state().history_len(), 1);
    }

    #[test]
    fn shuffle_never_repeats_the_same_layer_immediately() {
        let mut driver = RubikDriver::new();
        driver.shuffle(64);
        let mut previous: Option<RotationCommand> = None;
        while let Some(command) = driver.queue.pop_front() {
            if let Some(last) = previous {
                assert!(last.axis != command.axis || last.layer != command.layer);
            }
            previous = Some(command);
        }
    }
}
