#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::kernel::ParallelWaveExecutor;
use std::sync::{atomic::{AtomicU32, Ordering}, Arc};

fuzz_target!(|bytes: &[u8]| {
    let executor = ParallelWaveExecutor::new((bytes.first().copied().unwrap_or(1) as usize % 4) + 1);
    let count = Arc::new(AtomicU32::new(0));
    let jobs = bytes.iter().take(64).map(|_| {
        let count = Arc::clone(&count);
        Box::new(move || { count.fetch_add(1, Ordering::Relaxed); })
            as Box<dyn FnOnce() + Send>
    }).collect();
    assert!(executor.run(jobs));
});
