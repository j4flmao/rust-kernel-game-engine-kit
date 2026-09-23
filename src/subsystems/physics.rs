//! Deterministic fixed-point physics kernel suitable for the first subsystem pass.

use crate::kernel::{KernelContext, Subsystem};
use crate::subsystems::messages::InputFrame;

const NANOS_PER_SECOND: i64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub position: [i64; 2],
    pub velocity: [i64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsError {
    AllocationFailed,
}

pub struct PhysicsSubsystem {
    bodies: Vec<Body>,
    last_input: Option<InputFrame>,
}

impl PhysicsSubsystem {
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("physics allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, PhysicsError> {
        let mut bodies = Vec::new();
        bodies
            .try_reserve_exact(capacity)
            .map_err(|_| PhysicsError::AllocationFailed)?;
        Ok(Self {
            bodies,
            last_input: None,
        })
    }

    pub fn add_body(&mut self, body: Body) -> Option<usize> {
        if self.bodies.len() == self.bodies.capacity() {
            return None;
        }
        self.bodies.push(body);
        Some(self.bodies.len() - 1)
    }

    pub fn bodies(&self) -> &[Body] {
        &self.bodies
    }

    pub fn last_input(&self) -> Option<InputFrame> {
        self.last_input
    }

    fn integrate(&mut self, dt_ns: u64) {
        let dt = i64::try_from(dt_ns).unwrap_or(i64::MAX);
        for body in &mut self.bodies {
            for axis in 0..2 {
                body.position[axis] = body.position[axis]
                    .saturating_add(body.velocity[axis].saturating_mul(dt) / NANOS_PER_SECOND);
            }
        }
    }
}

impl Default for PhysicsSubsystem {
    fn default() -> Self {
        Self::with_capacity(128)
    }
}

impl Subsystem for PhysicsSubsystem {
    fn name(&self) -> &'static str {
        "physics"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["input"]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, dt_ns: u64) {
        for envelope in ctx.receive() {
            if envelope.topic == 2 {
                if let Ok(input) = envelope.downcast::<InputFrame>() {
                    self.last_input = Some(*input);
                }
            }
        }
        self.integrate(dt_ns);
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_in_fixed_point_without_allocating() {
        let mut physics = PhysicsSubsystem::with_capacity(1);
        assert_eq!(
            physics.add_body(Body {
                position: [0, 10],
                velocity: [2, -4]
            }),
            Some(0)
        );
        assert_eq!(
            physics.add_body(Body {
                position: [0, 0],
                velocity: [0, 0]
            }),
            None
        );
        physics.integrate(500_000_000);
        assert_eq!(physics.bodies()[0].position, [1, 8]);
    }
}
