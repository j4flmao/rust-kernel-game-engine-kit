//! Deterministic audio voice scheduler.
//!
//! This owns simulation-time voice state only. Native ALSA/WASAPI output is a
//! later PAL concern; keeping the scheduler independent makes headless tests
//! and offline rendering deterministic.

use crate::kernel::{KernelContext, Subsystem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voice {
    pub phase: u64,
    pub phase_step: u64,
    pub gain_milli: u16,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioError {
    AllocationFailed,
}

pub struct AudioSubsystem {
    voices: Vec<Voice>,
}

impl AudioSubsystem {
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity).expect("audio allocation failed")
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, AudioError> {
        let mut voices = Vec::new();
        voices
            .try_reserve_exact(capacity)
            .map_err(|_| AudioError::AllocationFailed)?;
        Ok(Self { voices })
    }

    pub fn add_voice(&mut self, phase_step: u64, gain_milli: u16) -> Option<usize> {
        if self.voices.len() == self.voices.capacity() {
            return None;
        }
        self.voices.push(Voice {
            phase: 0,
            phase_step,
            gain_milli,
            active: true,
        });
        Some(self.voices.len() - 1)
    }

    pub fn stop_voice(&mut self, index: usize) -> bool {
        let Some(voice) = self.voices.get_mut(index) else {
            return false;
        };
        voice.active = false;
        true
    }

    pub fn voices(&self) -> &[Voice] {
        &self.voices
    }

    fn advance(&mut self, dt_ns: u64) {
        for voice in &mut self.voices {
            if voice.active {
                voice.phase = voice
                    .phase
                    .wrapping_add(voice.phase_step.saturating_mul(dt_ns));
            }
        }
    }
}

impl Default for AudioSubsystem {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

impl Subsystem for AudioSubsystem {
    fn name(&self) -> &'static str {
        "audio"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, _ctx: &mut KernelContext<'_>, dt_ns: u64) {
        self.advance(dt_ns);
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_active_voices_and_freezes_stopped_voices() {
        let mut audio = AudioSubsystem::with_capacity(2);
        assert_eq!(audio.add_voice(3, 500), Some(0));
        assert_eq!(audio.add_voice(7, 1000), Some(1));
        audio.advance(10);
        assert_eq!(audio.voices()[0].phase, 30);
        assert_eq!(audio.voices()[1].phase, 70);
        assert!(audio.stop_voice(1));
        audio.advance(10);
        assert_eq!(audio.voices()[0].phase, 60);
        assert_eq!(audio.voices()[1].phase, 70);
    }
}
