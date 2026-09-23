//! End-to-end scheduler contract: topo order, cycle detection, shutdown.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rust_kernel_game_engine_kit::kernel::{Kernel, KernelContext, Subsystem};

#[derive(Default)]
struct Ticker {
    name: &'static str,
    deps: &'static [&'static str],
    log: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
}

impl Subsystem for Ticker {
    fn name(&self) -> &'static str {
        self.name
    }
    fn dependencies(&self) -> &'static [&'static str] {
        self.deps
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, _ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        self.log.borrow_mut().push(self.name);
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

fn build(subs: &[(&'static str, &'static [&'static str])]) -> Kernel {
    let mut kernel = Kernel::new();
    for &(name, deps) in subs {
        kernel
            .register(Box::new(Ticker {
                name,
                deps,
                ..Default::default()
            }))
            .unwrap();
    }
    kernel
}

#[test]
fn three_chain_ticks_in_dependency_order() {
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut kernel = Kernel::new();
    for &(name, deps) in &[("b", &["a"][..]), ("a", &[]), ("c", &["b"])] {
        kernel
            .register(Box::new(Ticker {
                name,
                deps,
                log: log.clone(),
            }))
            .unwrap();
    }
    kernel.init().unwrap();
    kernel.run(3).unwrap();
    let seen = log.borrow().clone();

    // Each of the 3 frames ticked in topo order, no interleaving drift.
    assert_eq!(seen, ["a", "b", "c", "a", "b", "c", "a", "b", "c"]);
    kernel.shutdown();
}

#[test]
fn shutdown_after_run_leaves_kernel_grounded() {
    let mut kernel = build(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
    kernel.init().unwrap();
    kernel.run(1).unwrap();
    kernel.shutdown();
    // Second shutdown is a no-op (guard), and run now fails as not init.
    kernel.shutdown();
    assert!(kernel.run(1).is_err());
}

#[test]
fn uninitialized_run_and_zero_frames() {
    let mut kernel = build(&[("a", &[])]);
    assert!(kernel.run(1).is_err());
    kernel.init().unwrap();
    assert_eq!(kernel.subsystem_count(), 1);
    kernel.run(0).unwrap();
    assert_eq!(kernel.frame_index(), 0);
}
