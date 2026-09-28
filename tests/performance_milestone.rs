#[path = "../benches/quality_milestones/harness.rs"]
mod harness;

use harness::{run, Config};

#[test]
fn milestone_harness_covers_every_object_in_partitioned_frames() {
    let report = run(Config {
        objects: 1025,
        frames: 3,
        partitions: 17,
        target_fps: 60,
        warmup: 1,
        strict: false,
    });
    assert_eq!(report.processed, 3075);
    assert_ne!(report.checksum, 0);
    assert!(report.contract_passed());
}

#[test]
fn milestone_harness_supports_the_120_fps_budget() {
    let report = run(Config {
        objects: 4096,
        frames: 2,
        partitions: 64,
        target_fps: 120,
        warmup: 0,
        strict: false,
    });
    assert_eq!(report.budget().as_nanos(), 8_333_334);
    assert!(report.contract_passed());
}
