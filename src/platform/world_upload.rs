//! Shared validation and transfer bookkeeping for native world presenters.
use crate::subsystems::renderer::{WorldDrawCommand, GPU_INSTANCE_RECORD_SIZE};

/// Counts successful queue submissions, not GPU-completion timing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldUploadStats {
    pub transfers: u64,
    pub bytes: u64,
}

#[derive(Default)]
pub(crate) struct UploadState {
    dirty: bool,
    stats: WorldUploadStats,
}
impl UploadState {
    pub(crate) fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub(crate) fn pending(&self) -> bool {
        self.dirty
    }
    pub(crate) fn submitted(&mut self, bytes: usize) {
        if self.dirty {
            self.stats.transfers = self.stats.transfers.saturating_add(1);
            self.stats.bytes = self.stats.bytes.saturating_add(bytes as u64);
            self.dirty = false;
        }
    }
    pub(crate) fn stats(&self) -> WorldUploadStats {
        self.stats
    }
}

/// Reject malformed packets before a shader can address past its storage buffer.
pub fn valid_world_packet(
    bytes: &[u8],
    capacity: u64,
    camera: &[f32; 16],
    draws: &[WorldDrawCommand],
) -> bool {
    if bytes.is_empty()
        || bytes.len() as u64 > capacity
        || !bytes.len().is_multiple_of(GPU_INSTANCE_RECORD_SIZE)
        || !camera.iter().all(|v| v.is_finite())
        || draws.is_empty()
    {
        return false;
    }
    let instances = bytes.len() / GPU_INSTANCE_RECORD_SIZE;
    draws.iter().all(|draw| {
        draw.vertex_count != 0
            && draw.instance_count != 0
            && draw
                .first_vertex
                .checked_add(draw.vertex_count)
                .is_some_and(|end| end <= 36)
            && draw
                .first_instance
                .checked_add(draw.instance_count)
                .is_some_and(|end| end as usize <= instances)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_bounds_are_checked() {
        let bytes = [0; 160];
        let camera = [0.0; 16];
        let mut draw = WorldDrawCommand {
            first_vertex: 30,
            vertex_count: 6,
            first_instance: 1,
            instance_count: 1,
        };
        assert!(valid_world_packet(&bytes, 160, &camera, &[draw]));
        assert!(!valid_world_packet(&bytes, 159, &camera, &[draw]));
        assert!(!valid_world_packet(&bytes[..159], 160, &camera, &[draw]));
        draw.instance_count = 2;
        assert!(!valid_world_packet(&bytes, 160, &camera, &[draw]));
        draw.instance_count = u32::MAX;
        assert!(!valid_world_packet(&bytes, 160, &camera, &[draw]));
        draw.instance_count = 1;
        draw.vertex_count = 7;
        assert!(!valid_world_packet(&bytes, 160, &camera, &[draw]));
        draw.vertex_count = 6;
        assert!(!valid_world_packet(&bytes, 160, &[f32::NAN; 16], &[draw]));
    }
    #[test]
    fn pending_upload_survives_failed_or_out_of_date_frame() {
        let mut state = UploadState::default();
        assert!(!state.pending());
        state.invalidate();
        // No successful queue submission: do not call submitted, keep retry pending.
        assert!(state.pending());
        assert_eq!(state.stats().transfers, 0);
        state.submitted(160);
        assert!(!state.pending());
        for _ in 0..100 {
            state.submitted(160);
        }
        assert_eq!(
            state.stats(),
            WorldUploadStats {
                transfers: 1,
                bytes: 160
            }
        );
        state.invalidate();
        state.submitted(80);
        assert_eq!(
            state.stats(),
            WorldUploadStats {
                transfers: 2,
                bytes: 240
            }
        );
    }
}
