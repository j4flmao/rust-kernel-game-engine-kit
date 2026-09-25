//! The scheduler and the kernel driver.
//!
//! Scheduling is a manual topological sort over [`Subsystem::dependencies`]:
//! an unmet or cyclic dependency is a hard startup failure, never a silent
//! skip. Ticks run in the sorted order each frame; parallel ticking is a
//! later-phase optimization.

use std::time::{Duration, Instant};

use crate::kernel::bus::{MessageBus, SubscriberId};
use crate::kernel::context::KernelContext;
use crate::kernel::ecs::world::World;
use crate::kernel::error::KernelError;
use crate::kernel::mem::arena::BumpArena;
use crate::kernel::registry::Registry;
use crate::kernel::subsystem::{Subsystem, SystemAccess};
use crate::kernel::trace::frame_tracer::{FrameTracer, TraceSample};
use crate::kernel::{
    DeferredCommands, DEFAULT_ARENA_CAPACITY, DEFAULT_DEFERRED_COMMAND_CAPACITY,
    DEFAULT_FRAME_BUDGET_NS, FIXED_DT_NS,
};

/// DFS colors for cycle detection.
const WHITE: u8 = 0; // unvisited
const GRAY: u8 = 1; // on the current DFS path
const BLACK: u8 = 2; // fully processed

fn visit(
    node: usize,
    deps: &[Vec<usize>],
    names: &[String],
    state: &mut [u8],
    stack: &mut Vec<usize>,
    order: &mut Vec<usize>,
) -> Result<(), KernelError> {
    state[node] = GRAY;
    stack.push(node);
    for &dep in &deps[node] {
        match state[dep] {
            WHITE => visit(dep, deps, names, state, stack, order)?,
            GRAY => {
                let pos = stack
                    .iter()
                    .position(|&n| n == dep)
                    .expect("gray node must be on the current DFS stack");
                let mut path: Vec<String> =
                    stack[pos..].iter().map(|&n| names[n].clone()).collect();
                // Close the loop for a readable report: a -> b -> a.
                path.push(names[dep].clone());
                stack.pop();
                return Err(KernelError::cycle(path));
            }
            BLACK => {}
            _ => unreachable!("state is only ever WHITE/GRAY/BLACK"),
        }
    }
    state[node] = BLACK;
    stack.pop();
    order.push(node);
    Ok(())
}

/// Topological order with dependencies listed first.
///
/// Building the list by pushing each node after all its dependencies have
/// finished already yields deps-first order — no reversal needed (a node can
/// only finish after every dependency on its edge has finished).
fn toposort(deps: &[Vec<usize>], names: &[String]) -> Result<Vec<usize>, KernelError> {
    let n = deps.len();
    let mut order = Vec::with_capacity(n);
    let mut state = vec![WHITE; n];
    let mut stack = Vec::with_capacity(n);
    for start in 0..n {
        if state[start] == WHITE {
            visit(start, deps, names, &mut state, &mut stack, &mut order)?;
        }
    }
    Ok(order)
}

/// Groups a dependency-resolved order into deterministic execution waves.
///
/// Nodes in one wave have no dependency on another node in that same wave.
/// The kernel does not execute these waves concurrently yet; exposing the
/// validated grouping first keeps the eventual parallel executor separate
/// from lifecycle and routing correctness.
pub fn dependency_waves(order: &[usize], deps: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut level = vec![0usize; deps.len()];
    for &node in order {
        level[node] = deps[node]
            .iter()
            .map(|&dependency| level[dependency].saturating_add(1))
            .max()
            .unwrap_or(0);
    }
    let wave_count = level.iter().copied().max().map_or(0, |max| max + 1);
    let mut waves = vec![Vec::new(); wave_count];
    for &node in order {
        waves[level[node]].push(node);
    }
    waves
}

/// Builds deterministic waves from dependency order and declared access sets.
///
/// The topological order is the tie-breaker. A conflict is placed in a later
/// wave, so independent read/read systems can share a wave while legacy or
/// write-conflicting systems remain ordered. This function only plans work;
/// it does not bypass the borrow-safe serial KernelContext execution path.
pub fn access_waves(
    order: &[usize],
    deps: &[Vec<usize>],
    accesses: &[SystemAccess],
) -> Vec<Vec<usize>> {
    assert_eq!(
        deps.len(),
        accesses.len(),
        "dependency/access length mismatch"
    );
    let mut level = vec![0usize; deps.len()];
    for &node in order {
        let mut node_level = deps[node]
            .iter()
            .map(|&dependency| level[dependency].saturating_add(1))
            .max()
            .unwrap_or(0);
        for &prior in order.iter().take_while(|&&candidate| candidate != node) {
            if accesses[node].conflicts(accesses[prior]) {
                node_level = node_level.max(level[prior].saturating_add(1));
            }
        }
        level[node] = node_level;
    }
    let wave_count = level.iter().copied().max().map_or(0, |max| max + 1);
    let mut waves = vec![Vec::new(); wave_count];
    for &node in order {
        waves[level[node]].push(node);
    }
    waves
}

/// Validated, immutable execution plan for the kernel schedule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulePlan {
    order: Vec<usize>,
    waves: Vec<Vec<usize>>,
}

impl SchedulePlan {
    pub(crate) fn build(
        deps: &[Vec<usize>],
        names: &[String],
        accesses: &[SystemAccess],
    ) -> Result<Self, KernelError> {
        let order = toposort(deps, names)?;
        let waves = access_waves(&order, deps, accesses);
        Ok(Self { order, waves })
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn waves(&self) -> &[Vec<usize>] {
        &self.waves
    }
}

/// Executes independent jobs in one validated dependency wave concurrently.
/// Subsystems remain isolated from one another; callers exchange results via
/// the message bus after `run` returns. This is the safe parallel primitive
/// used by higher-level schedulers instead of sharing `&mut Kernel` across
/// worker threads.
pub struct ParallelWaveExecutor {
    pool: crate::platform::thread_pool::ThreadPool,
}

impl ParallelWaveExecutor {
    pub fn new(workers: usize) -> Self {
        Self::try_new(workers).expect("parallel executor initialization failed")
    }
    pub fn try_new(workers: usize) -> Result<Self, crate::platform::thread_pool::ThreadPoolError> {
        Ok(Self {
            pool: crate::platform::thread_pool::ThreadPool::try_new(workers)?,
        })
    }
    pub fn run(&self, jobs: Vec<Box<dyn FnOnce() + Send + 'static>>) -> bool {
        self.try_run(jobs).unwrap_or(false)
    }
    pub fn try_run(
        &self,
        jobs: Vec<Box<dyn FnOnce() + Send + 'static>>,
    ) -> Result<bool, crate::platform::thread_pool::ThreadPoolError> {
        for job in jobs {
            self.pool.try_execute(job)?;
        }
        Ok(self.pool.wait_idle())
    }
    pub fn workers(&self) -> usize {
        self.pool.workers()
    }
}

/// Paces the frame loop to a target frequency with sleep + spin fallback.
pub struct FrameLimiter {
    period: Duration,
    last: Instant,
}

impl FrameLimiter {
    pub fn at(hz: f64) -> Self {
        let secs = 1.0 / hz.max(1.0);
        Self {
            period: Duration::from_secs_f64(secs),
            last: Instant::now(),
        }
    }

    /// Blocks until the next frame boundary, then resets the anchor.
    pub fn wait(&mut self) {
        let deadline = self.last + self.period;
        let now = Instant::now();
        if deadline > now {
            let lead = deadline - now;
            // Sleep the bulk, spin the tail for precision.
            if lead > Duration::from_millis(2) {
                std::thread::sleep(lead - Duration::from_millis(2));
            }
            while Instant::now() < deadline {
                std::hint::spin_loop();
            }
        }
        self.last = Instant::now();
    }
}

/// The engine kernel: registry + message bus + scheduler + tracer + world.
pub struct Kernel {
    registry: Registry,
    bus: MessageBus,
    names: Vec<String>,
    order: Vec<usize>,
    waves: Vec<Vec<usize>>,
    tracer: FrameTracer,
    /// Phase 2: the shared ECS world all subsystems read/write.
    world: World,
    /// Per-frame scratch arena, rewound at the start of every tick.
    frame_arena: BumpArena,
    deferred: DeferredCommands,
    initialized: bool,
    frame_index: u64,
    parallel_executor: ParallelWaveExecutor,
    sync: crate::kernel::sync::Replicator,
}

impl Kernel {
    /// Fresh kernel with default tracer config.
    pub fn new() -> Self {
        Self::with_tracer(FrameTracer::default_config())
    }

    pub fn with_tracer(tracer: FrameTracer) -> Self {
        Self {
            registry: Registry::new(),
            bus: MessageBus::new(),
            names: Vec::new(),
            order: Vec::new(),
            waves: Vec::new(),
            tracer,
            world: World::new(),
            frame_arena: BumpArena::new(DEFAULT_ARENA_CAPACITY),
            deferred: DeferredCommands::with_capacity(DEFAULT_DEFERRED_COMMAND_CAPACITY),
            initialized: false,
            frame_index: 0,
            parallel_executor: ParallelWaveExecutor::new(
                std::thread::available_parallelism().map_or(1, |count| count.get()),
            ),
            sync: crate::kernel::sync::Replicator::default(),
        }
    }

    /// Registers a subsystem. Duplicate names are a hard startup error.
    pub fn register(&mut self, subsystem: Box<dyn Subsystem>) -> Result<(), KernelError> {
        self.registry.register(subsystem).map(|_| ())
    }

    /// Re-resolves names so `KernelContext::resolve` stays in sync.
    fn refresh_names(&mut self) {
        self.names = self
            .registry
            .entries
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
    }

    /// Freezes scheduling order and initializes subsystems in topo order.
    pub fn init(&mut self) -> Result<(), KernelError> {
        if self.initialized {
            return Ok(());
        }
        let deps = self.registry.resolve_dependencies()?;
        self.refresh_names();
        let accesses: Vec<SystemAccess> = self
            .registry
            .entries
            .iter()
            .map(|(_, subsystem)| subsystem.access())
            .collect();
        let plan = SchedulePlan::build(&deps, &self.names, &accesses)?;

        // Register exactly one inbox per subsystem, index == registry index.
        for _ in 0..self.registry.entries.len() {
            self.bus.add_subscriber();
        }
        self.bus.set_debug_edges(&deps);

        self.order = plan.order;
        self.waves = plan.waves;
        self.frame_arena.reset();
        for &idx in &self.order {
            let id = SubscriberId::new(idx as u32);
            let mut ctx = KernelContext::new(
                &mut self.bus,
                &self.names,
                id,
                0,
                &mut self.world,
                &mut self.deferred,
                &mut self.frame_arena,
            );
            self.registry.entries[idx].1.init(&mut ctx);
        }
        self.initialized = true;
        Ok(())
    }

    /// Runs `frames` simulation frames at a fixed 1/60 step.
    pub fn run(&mut self, frames: u64) -> Result<(), KernelError> {
        if !self.initialized {
            return Err(KernelError::NotInitialized);
        }
        if frames == 0 {
            return Ok(());
        }
        let exec = self.order.clone();
        let mut limiter = FrameLimiter::at(60.0);

        for _ in 0..frames {
            self.bus.flush();
            // Rewind the per-frame arena before any subsystem touches it.
            self.frame_arena.reset();
            let frame = self.frame_index;

            for &idx in &exec {
                let id = SubscriberId::new(idx as u32);
                let t0 = Instant::now();
                {
                    let mut ctx = KernelContext::new(
                        &mut self.bus,
                        &self.names,
                        id,
                        FIXED_DT_NS,
                        &mut self.world,
                        &mut self.deferred,
                        &mut self.frame_arena,
                    );
                    self.registry.entries[idx].1.tick(&mut ctx, FIXED_DT_NS);
                }
                let dur = t0.elapsed().as_nanos().max(1) as u64;
                self.tracer.record(TraceSample {
                    subsystem_idx: idx as u32,
                    frame_index: frame,
                    duration_ns: dur,
                    budget_ns: DEFAULT_FRAME_BUDGET_NS,
                    over_budget: dur > DEFAULT_FRAME_BUDGET_NS,
                });
            }

            // Structural ECS changes become visible only after every system
            // in the frame has finished its current world view.
            self.deferred.apply(&mut self.world);

            self.frame_index += 1;
            self.tracer.frame(Instant::now());
            limiter.wait();
        }
        Ok(())
    }

    /// Current frame counter (post-run inspection / programmatic stepping).
    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }

    /// Shared view of the engine world (for inspection / test assertions).
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Mutable view of the engine world (for bootstrapping / external tools).
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Registered subsystem count.
    pub fn subsystem_count(&self) -> usize {
        self.registry.entries.len()
    }

    /// Dependency-safe deterministic execution waves reserved for the
    /// parallel scheduler. The current executor still runs each wave serially.
    pub fn dependency_waves(&self) -> &[Vec<usize>] {
        &self.waves
    }

    /// Returns the validated schedule waves. The current tick callback path
    /// remains serial until deferred commands and isolated contexts are wired.
    pub fn schedule_waves(&self) -> &[Vec<usize>] {
        &self.waves
    }

    /// Runs an explicitly isolated wave of work on the kernel-owned executor.
    ///
    /// Subsystem ticks continue to use the serialized `KernelContext` path:
    /// it owns `&mut World` and `&mut MessageBus`, so parallelizing those
    /// callbacks would violate Rust's aliasing contract. This API is the
    /// scheduler integration point for jobs that operate on disjoint ECS
    /// snapshots or per-subsystem scratch and publish only after the wave.
    pub fn execute_parallel_wave(&self, jobs: Vec<Box<dyn FnOnce() + Send + 'static>>) -> bool {
        self.parallel_executor.run(jobs)
    }

    pub fn sync(&self) -> &crate::kernel::sync::Replicator {
        &self.sync
    }
    pub fn sync_mut(&mut self) -> &mut crate::kernel::sync::Replicator {
        &mut self.sync
    }

    /// Pumps the replication transport with bounded memory and retries.
    pub fn pump_sync<T: crate::kernel::sync::SyncTransport>(
        &mut self,
        transport: &mut T,
        frame: u64,
    ) -> crate::kernel::sync::PumpReport {
        self.sync.pump(transport, frame)
    }

    /// Tears subsystems down in reverse topological order.
    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        for &idx in self.order.iter().rev() {
            let id = SubscriberId::new(idx as u32);
            let mut ctx = KernelContext::new(
                &mut self.bus,
                &self.names,
                id,
                0,
                &mut self.world,
                &mut self.deferred,
                &mut self.frame_arena,
            );
            self.registry.entries[idx].1.shutdown(&mut ctx);
        }
        self.initialized = false;
        self.order.clear();
        self.waves.clear();
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::kernel::KernelContext;

    type Log = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;

    struct Dummy {
        name: &'static str,
        deps: &'static [&'static str],
        order_log: Log,
    }

    impl Subsystem for Dummy {
        fn name(&self) -> &'static str {
            self.name
        }
        fn dependencies(&self) -> &'static [&'static str] {
            self.deps
        }
        fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
        fn tick(&mut self, _ctx: &mut KernelContext<'_>, _dt_ns: u64) {
            self.order_log.borrow_mut().push(self.name);
        }
        fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
    }

    fn kernel_with(subs: &[(&'static str, &'static [&'static str])]) -> (Kernel, Log) {
        let log = Log::default();
        let mut kernel = Kernel::new();
        for &(name, deps) in subs {
            kernel
                .register(Box::new(Dummy {
                    name,
                    deps,
                    order_log: log.clone(),
                }))
                .unwrap();
        }
        (kernel, log)
    }

    #[test]
    fn toposort_respects_dependencies() {
        let (mut kernel, log) = kernel_with(&[("b", &["a"]), ("a", &[]), ("c", &["b"])]);
        kernel.init().unwrap();
        kernel.run(1).unwrap();
        let seen = log.borrow().clone();
        assert_eq!(seen, vec!["a", "b", "c"]);
    }

    #[test]
    fn dependency_waves_are_deterministic_and_dependency_safe() {
        let deps = vec![vec![], vec![0], vec![0], vec![1, 2]];
        let order = vec![0, 1, 2, 3];
        assert_eq!(
            dependency_waves(&order, &deps),
            vec![vec![0], vec![1, 2], vec![3]]
        );
    }

    #[test]
    fn access_waves_keep_independent_reads_together() {
        static READS: [&str; 1] = ["position"];
        let deps = vec![vec![], vec![], vec![]];
        let order = vec![0, 1, 2];
        let accesses = vec![
            SystemAccess::read_only(&READS),
            SystemAccess::read_only(&READS),
            SystemAccess::read_write(&READS, &READS),
        ];
        assert_eq!(
            access_waves(&order, &deps, &accesses),
            vec![vec![0, 1], vec![2]]
        );
    }

    #[test]
    fn toposort_cycles_are_hard_errors() {
        let (mut kernel, _) = kernel_with(&[("a", &["b"]), ("b", &["a"])]);
        let err = kernel.init().unwrap_err();
        assert!(matches!(err, KernelError::DependencyCycle(_)));
        assert!(!kernel.initialized);
    }

    #[test]
    fn toposort_self_dependency_is_a_cycle() {
        let (mut kernel, _) = kernel_with(&[("a", &["a"])]);
        let err = kernel.init().unwrap_err();
        assert!(matches!(err, KernelError::DependencyCycle(_)));
    }

    #[test]
    fn unknown_dependency_fails_init() {
        let (mut kernel, _) = kernel_with(&[("a", &["ghost"])]);
        let err = kernel.init().unwrap_err();
        assert!(matches!(
            err,
            KernelError::UnknownDependency {
                subsystem,
                dependency
            } if subsystem == "a" && dependency == "ghost"
        ));
    }

    #[test]
    fn duplicate_name_rejected() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut kernel = Kernel::new();
        // First register must succeed; the second under the same name is the
        // case under test and must come back as an error, not a silent dup.
        kernel
            .register(Box::new(Dummy {
                name: "a",
                deps: &[],
                order_log: log.clone(),
            }))
            .unwrap();
        let err = kernel
            .register(Box::new(Dummy {
                name: "a",
                deps: &[],
                order_log: log,
            }))
            .unwrap_err();
        assert!(matches!(err, KernelError::DuplicateName(_)));
    }

    #[test]
    fn run_before_init_is_an_error() {
        let (mut kernel, _) = kernel_with(&[("a", &[])]);
        let err = kernel.run(1).unwrap_err();
        assert!(matches!(err, KernelError::NotInitialized));
    }

    #[test]
    fn zero_frames_ok() {
        let (mut kernel, _) = kernel_with(&[("a", &[])]);
        kernel.init().unwrap();
        kernel.run(0).unwrap();
        assert_eq!(kernel.frame_index(), 0);
    }

    #[test]
    fn shutdown_runs_reverse_order() {
        let log = Log::new(std::cell::RefCell::new(Vec::new()));
        struct ShutdownSub {
            name: &'static str,
            log: Log,
        }
        impl Subsystem for ShutdownSub {
            fn name(&self) -> &'static str {
                self.name
            }
            fn dependencies(&self) -> &'static [&'static str] {
                &[]
            }
            fn init(&mut self, _: &mut KernelContext<'_>) {}
            fn tick(&mut self, _: &mut KernelContext<'_>, _: u64) {}
            fn shutdown(&mut self, _: &mut KernelContext<'_>) {
                self.log.borrow_mut().push(self.name);
            }
        }
        let mut kernel = Kernel::new();
        kernel
            .register(Box::new(ShutdownSub {
                name: "x",
                log: log.clone(),
            }))
            .unwrap();
        kernel.init().unwrap();
        kernel.shutdown();
        assert_eq!(log.borrow().clone(), vec!["x"]);
    }
}
