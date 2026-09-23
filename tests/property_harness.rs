use rust_kernel_game_engine_kit::kernel::Kernel;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

#[test]
fn wave_executor_runs_every_generated_job_once() {
    let kernel = Kernel::new();
    let count = Arc::new(AtomicU32::new(0));
    let mut jobs = Vec::new();
    for _ in 0..257 {
        let c = Arc::clone(&count);
        jobs.push(Box::new(move || {
            c.fetch_add(1, Ordering::Relaxed);
        }) as Box<dyn FnOnce() + Send>);
    }
    assert!(kernel.execute_parallel_wave(jobs));
    assert_eq!(count.load(Ordering::Acquire), 257);
}
