#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::kernel::ParallelWaveExecutor;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

fuzz_target!(|bytes: &[u8]| {
    // Keep each fuzz case bounded and deterministic. The production executor
    // remains in this target; its queue/worker teardown is the behavior under
    // fuzz, while the workload is capped to keep case cost predictable.
    let executor = ParallelWaveExecutor::new(1);
    let count = Arc::new(AtomicU32::new(0));
    let jobs = bytes
        .iter()
        .skip(1)
        .take(8)
        .map(|_| {
            let count = Arc::clone(&count);
            Box::new(move || {
                count.fetch_add(1, Ordering::Relaxed);
            }) as Box<dyn FnOnce() + Send>
        })
        .collect();
    assert!(executor.run(jobs));
});
