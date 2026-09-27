#![allow(dead_code)]

//! Deterministic quality milestone harness for the Criterion benchmark.
//!
//! This is intentionally a CPU-side contract rather than a fake GPU claim:
//! it exercises bounded per-object update work, split across configurable
//! render batches, and reports wall-clock time separately. CI can therefore
//! verify exact coverage at 18M objects for both 60 and 120 FPS targets
//! without making a hosted runner's variable speed a normal PR failure.
//!
use std::hint::black_box;
use std::time::{Duration, Instant};

const TARGET_OBJECTS: u64 = 18_000_000;
const TARGET_FPS: u32 = 60;
const MAX_PARTITIONS: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub objects: u64,
    pub frames: u32,
    pub partitions: u32,
    pub target_fps: u32,
    pub warmup: u32,
    pub strict: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            objects: TARGET_OBJECTS,
            frames: 3,
            partitions: 1,
            target_fps: TARGET_FPS,
            warmup: 0,
            strict: false,
        }
    }
}

impl Config {
    fn validate(self) -> Result<(), String> {
        if self.objects == 0 || self.frames == 0 || self.partitions == 0 || self.target_fps == 0 {
            return Err("all numeric options must be positive".to_string());
        }
        if self.partitions > MAX_PARTITIONS {
            return Err(format!("--partitions cannot exceed {MAX_PARTITIONS}"));
        }
        if !matches!(self.target_fps, 60 | 120) {
            return Err("--target-fps currently supports 60 or 120".to_string());
        }
        if self.objects.checked_mul(u64::from(self.frames)).is_none() {
            return Err("--objects multiplied by --frames exceeds u64".to_string());
        }
        Ok(())
    }

    pub fn budget(self) -> Duration {
        let fps = u64::from(self.target_fps);
        Duration::from_nanos(1_000_000_000_u64.div_ceil(fps))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub config: Config,
    pub processed: u64,
    pub checksum: u64,
    pub min_frame: Duration,
    pub max_frame: Duration,
    pub total: Duration,
    pub min_partition_objects: u64,
    pub max_partition_objects: u64,
}

impl Report {
    pub fn average_frame(self) -> Duration {
        self.total / self.config.frames.max(1)
    }

    pub fn equivalent_fps(self) -> f64 {
        self.average_frame().as_secs_f64().recip()
    }

    pub fn budget(self) -> Duration {
        self.config.budget()
    }

    pub fn contract_passed(self) -> bool {
        self.processed
            == self
                .config
                .objects
                .checked_mul(u64::from(self.config.frames.max(1)))
                .unwrap_or(0)
            && self.checksum != 0
            && self.config.partitions > 0
            && self.min_partition_objects > 0
            && self.max_partition_objects >= self.min_partition_objects
    }

    pub fn realtime_passed(self) -> bool {
        self.max_frame <= self.budget()
    }
}

fn partition_range(objects: u64, partitions: u32, partition: u32) -> (u64, u64) {
    let partition_count = u64::from(partitions.max(1));
    let index = u64::from(partition).min(partition_count - 1);
    let base = objects / partition_count;
    let remainder = objects % partition_count;
    let start = base * index + remainder.min(index);
    let next = index + 1;
    let end = base * next + remainder.min(next);
    (start, end)
}

fn run_frame(config: Config, frame: u32, mut checksum: u64) -> (u64, u64, u64, u64) {
    let mut processed = 0_u64;
    let mut min_partition_objects = u64::MAX;
    let mut max_partition_objects = 0_u64;
    for partition in 0..config.partitions.max(1) {
        let (start, end) = partition_range(config.objects, config.partitions, partition);
        let count = end - start;
        min_partition_objects = min_partition_objects.min(count);
        max_partition_objects = max_partition_objects.max(count);
        checksum = checksum.wrapping_add(u64::from(partition).rotate_left(17));
        for object in start..end {
            // This mirrors bounded transform/instance preparation without
            // allocating 18M objects or touching a device.
            let object_id = object.wrapping_add(u64::from(frame) << 32);
            let transformed = object_id
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .rotate_left((frame % 31) + 1);
            checksum = checksum
                .rotate_left(5)
                .wrapping_add(transformed ^ object_id.rotate_right(7));
            if object & 0x0fff == 0 {
                checksum = black_box(checksum);
            }
        }
        processed += count;
    }
    (
        processed,
        checksum,
        min_partition_objects,
        max_partition_objects,
    )
}

pub fn run(config: Config) -> Report {
    let mut checksum = 0x9e37_79b9_7f4a_7c15_u64;
    let mut min_frame = Duration::MAX;
    let mut max_frame = Duration::ZERO;
    let mut total = Duration::ZERO;
    let mut processed = 0_u64;
    let mut min_partition_objects = u64::MAX;
    let mut max_partition_objects = 0_u64;

    for frame in 0..config.warmup {
        let (_, next_checksum, _, _) = run_frame(config, frame, checksum);
        checksum = black_box(next_checksum);
    }
    for frame in 0..config.frames.max(1) {
        let start = Instant::now();
        let (frame_processed, next_checksum, frame_min, frame_max) =
            run_frame(config, frame + config.warmup, checksum);
        checksum = black_box(next_checksum);
        processed = processed.saturating_add(frame_processed);
        min_partition_objects = min_partition_objects.min(frame_min);
        max_partition_objects = max_partition_objects.max(frame_max);
        let elapsed = start.elapsed();
        min_frame = min_frame.min(elapsed);
        max_frame = max_frame.max(elapsed);
        total = total.saturating_add(elapsed);
    }

    Report {
        config,
        processed,
        checksum,
        min_frame,
        max_frame,
        total,
        min_partition_objects,
        max_partition_objects,
    }
}
