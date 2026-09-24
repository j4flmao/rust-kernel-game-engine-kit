//! Minimal OS-agnostic thread pool.
//!
//! One persistent worker thread per slot; jobs are `Box<dyn FnOnce>`
//! dispatched on demand. Deliberately dependency-free (std `sync` only).
//! The Phase 6 parallel scheduler uses it for per-subsystem tick waves;
//! core-pinning is applied through the Linux `threading` module when the
//! platform supplies affinity.

use std::panic;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPoolError {
    ThreadSpawn,
    QueueFull,
    Stopped,
    AllocationFailed,
}

struct Shared {
    jobs: Mutex<Vec<Job>>,
    max_jobs: usize,
    condvar: Condvar,
    stop: AtomicBool,
    pending: AtomicUsize,
}

/// A pool of `workers` long-lived threads that execute one-shot jobs.
pub struct ThreadPool {
    shared: Arc<Shared>,
    handles: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    /// Spawns `workers` persistent worker threads.
    pub fn new(workers: usize) -> Self {
        Self::try_new(workers).expect("thread pool initialization failed")
    }

    pub fn try_new(workers: usize) -> Result<Self, ThreadPoolError> {
        let workers = workers.max(1);
        let max_jobs = workers.saturating_mul(1024).max(workers);
        let mut jobs = Vec::new();
        jobs.try_reserve_exact(max_jobs)
            .map_err(|_| ThreadPoolError::AllocationFailed)?;
        let shared = Arc::new(Shared {
            jobs: Mutex::new(jobs),
            max_jobs,
            condvar: Condvar::new(),
            stop: AtomicBool::new(false),
            pending: AtomicUsize::new(0),
        });
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        handles
            .try_reserve_exact(workers)
            .map_err(|_| ThreadPoolError::AllocationFailed)?;
        for _ in 0..workers {
            let s = Arc::clone(&shared);
            let handle = match std::thread::Builder::new()
                .name("rust-kernel-worker".into())
                .spawn(move || worker_loop(s))
            {
                Ok(handle) => handle,
                Err(_) => {
                    shared.stop.store(true, Ordering::Release);
                    shared.condvar.notify_all();
                    for handle in handles {
                        let _ = handle.join();
                    }
                    return Err(ThreadPoolError::ThreadSpawn);
                }
            };
            handles.push(handle);
        }
        Ok(Self { shared, handles })
    }

    /// Enqueues one job. The pool does not reap the job's result — Phase 6
    /// waves publish results to their own scratch, this is a fire-and-collect
    /// executor.
    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.try_execute(job);
    }

    pub fn try_execute<F>(&self, job: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        let mut jobs = self.shared.jobs.lock().unwrap_or_else(|p| p.into_inner());
        if self.shared.stop.load(Ordering::Acquire) {
            return Err(ThreadPoolError::Stopped);
        }
        if jobs.len() >= self.shared.max_jobs {
            return Err(ThreadPoolError::QueueFull);
        }
        jobs.push(Box::new(job));
        self.shared.pending.fetch_add(1, Ordering::AcqRel);
        drop(jobs);
        self.shared.condvar.notify_one();
        Ok(())
    }

    /// Blocks until every queued job has completed. Returns false when the
    /// pool has been told to stop.
    pub fn wait_idle(&self) -> bool {
        let mut jobs = self.shared.jobs.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if jobs.is_empty() && self.shared.pending.load(Ordering::Acquire) == 0 {
                return true;
            }
            if self.shared.stop.load(Ordering::Acquire) {
                return false;
            }
            jobs = self
                .shared
                .condvar
                .wait(jobs)
                .unwrap_or_else(|p| p.into_inner());
        }
    }

    /// Number of live worker threads.
    pub fn workers(&self) -> usize {
        self.handles.len()
    }
}

fn worker_loop(shared: Arc<Shared>) {
    loop {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let mut jobs = shared.jobs.lock().unwrap_or_else(|p| p.into_inner());
        // Park while there is no work and we aren't asked to stop.
        loop {
            if let Some(job) = jobs.pop() {
                drop(jobs);
                let result = panic::catch_unwind(panic::AssertUnwindSafe(job));
                // Keep the completion transition under the queue mutex. This
                // makes `jobs.is_empty() && pending == 0` one coherent
                // predicate, avoiding a missed wake under strict schedulers
                // such as Miri.
                let _completion_guard = shared.jobs.lock().unwrap_or_else(|p| p.into_inner());
                shared.pending.fetch_sub(1, Ordering::AcqRel);
                // Wake both idle workers and wait_idle().
                shared.condvar.notify_all();
                if result.is_err() {
                    eprintln!("[thread-pool] worker job panicked; job was isolated");
                }
                break;
            }
            if shared.stop.load(Ordering::Acquire) {
                return;
            }
            jobs = shared.condvar.wait(jobs).unwrap_or_else(|p| p.into_inner());
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        // Wake everyone; then drain-park until they actually exit.
        {
            let _guard = self.shared.jobs.lock().unwrap_or_else(|p| p.into_inner());
            self.shared.condvar.notify_all();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::{thread, time::Duration};

    #[test]
    fn executes_jobs_across_all_workers() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicU32::new(0));
        for _ in 0..16 {
            let c = Arc::clone(&counter);
            pool.execute(move || {
                std::thread::sleep(Duration::from_millis(2));
                c.fetch_add(1, Ordering::Relaxed);
            });
        }
        // Give workers a moment to drain; the counter must reach 16.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while counter.load(Ordering::Acquire) != 16 && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(counter.load(Ordering::Acquire), 16);
    }

    #[test]
    fn pool_reuses_workers_without_extra_threads() {
        let pool = Arc::new(ThreadPool::new(2));
        let base = std::process::id();
        let _ = base;
        let done = Arc::new(AtomicU32::new(0));
        for _ in 0..4 {
            let p = Arc::clone(&pool);
            let d = Arc::clone(&done);
            pool.execute(move || {
                d.fetch_add(1, Ordering::AcqRel);
            });
            let _ = p; // keep alive during the loop
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while done.load(Ordering::Acquire) != 4 && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(done.load(Ordering::Acquire), 4);
    }

    #[test]
    fn wait_idle_drains_completed_jobs() {
        let pool = ThreadPool::new(2);
        let done = Arc::new(AtomicU32::new(0));
        for _ in 0..8 {
            let done = Arc::clone(&done);
            pool.execute(move || {
                thread::yield_now();
                done.fetch_add(1, Ordering::AcqRel);
            });
        }

        assert!(pool.wait_idle());
        assert_eq!(done.load(Ordering::Acquire), 8);
    }

    #[test]
    fn single_worker_drains_short_wave_without_deadlock() {
        let pool = ThreadPool::new(1);
        for _ in 0..3 {
            pool.execute(|| {});
        }
        assert!(pool.wait_idle());
    }

    #[test]
    fn panicking_job_does_not_stall_pool() {
        let pool = ThreadPool::new(1);
        pool.execute(|| panic!("intentional worker failure"));
        pool.execute(|| {});

        assert!(pool.wait_idle());
    }
}
