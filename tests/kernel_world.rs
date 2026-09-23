//! End-to-end check that a subsystem can drive the ECS world through
//! `KernelContext` across real kernel ticks (spawn, mutate via query,
//! despawn, and recycle-slot generation safety).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use rust_kernel_game_engine_kit::kernel::ecs::entity::Entity;
use rust_kernel_game_engine_kit::kernel::{Kernel, KernelContext, Subsystem};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position(f32, f32);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity(f32, f32);

#[derive(Default)]
struct Physics {
    scratch: RefCell<Vec<u32>>,
    log: Rc<RefCell<Vec<String>>>,
}

impl Subsystem for Physics {
    fn name(&self) -> &'static str {
        "physics"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
    fn init(&mut self, ctx: &mut KernelContext<'_>) {
        let world = ctx.world_write();
        let e = world.spawn();
        world.insert(e, Position(0.0, 0.0));
        world.insert(e, Velocity(1.0, 2.0));
        self.log.borrow_mut().push(format!("spawned {}", e.index));
    }
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        let mut candidates = self.scratch.borrow_mut();
        ctx.world_read()
            .collect_intersection::<Position, Velocity>(&mut candidates);
        let pair = ctx.world_write().pair::<Position, Velocity>().unwrap();
        pair.for_each(&candidates, |_e, pos, vel| {
            pos.0 += vel.0;
            pos.1 += vel.1;
        });
    }
    fn shutdown(&mut self, ctx: &mut KernelContext<'_>) {
        let mut entity = None;
        ctx.world_read().for_each_entity(|e| {
            if entity.is_none() {
                entity = Some(e);
            }
        });
        let _ = ctx
            .world_write()
            .despawn(entity.expect("physics owns one entity"));
        self.log.borrow_mut().push("despawned".to_string());
    }
}

#[test]
fn world_is_driven_through_kernel_ticks() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let physics = Physics {
        scratch: RefCell::new(Vec::new()),
        log: log.clone(),
    };
    let mut kernel = Kernel::new();
    kernel.register(Box::new(physics)).unwrap();
    kernel.init().unwrap();
    kernel.run(3).unwrap();

    // Three ticks of velocity integration.
    let mut entity = None;
    kernel.world().for_each_entity(|e| entity = Some(e));
    let entity = entity.expect("one entity alive");
    assert_eq!(
        *kernel.world().get::<Position>(entity).unwrap(),
        Position(3.0, 6.0)
    );

    assert_eq!(log.borrow()[0], "spawned 0");
    kernel.shutdown();
    assert_eq!(log.borrow()[1], "despawned");
    assert_eq!(kernel.world().entity_count(), 0);
}

/// Recycle-slot generation safety across ticks: a despawned entity's slot
/// must not let stale components leak through to a recycled handle.
#[test]
fn recycled_slot_has_no_stale_components_via_kernel() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(Recycler::default())).unwrap();
    kernel.init().unwrap();
    kernel.run(3).unwrap();
    kernel.shutdown();
}

#[derive(Default)]
struct Recycler {
    a: Option<Entity>,
    b: Option<Entity>,
}

impl Subsystem for Recycler {
    fn name(&self) -> &'static str {
        "recycler"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
    fn init(&mut self, ctx: &mut KernelContext<'_>) {
        let world = ctx.world_write();
        let a = world.spawn();
        world.insert(a, Position(1.0, 0.0));
        let b = world.spawn();
        world.insert(b, Position(2.0, 0.0));
        self.a = Some(a);
        self.b = Some(b);
    }
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        let world = ctx.world_write();
        if let Some(a) = self.a.take() {
            world.despawn(a);
        } else if let Some(b) = self.b.take() {
            world.despawn(b);
        } else {
            // Recycles the lowest freed slot (an "a").
            let c = world.spawn();
            assert!(
                world.get::<Position>(c).is_none(),
                "recycled handle must not see old component"
            );
        }
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}
