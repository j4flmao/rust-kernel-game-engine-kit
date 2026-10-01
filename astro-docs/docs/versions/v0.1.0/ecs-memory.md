# Entities, components and memory

## Create and update a component

`World` owns entity lifetime and typed component storage. An entity handle includes a generation; a reused slot must not make an old handle valid again.

```rust
use rust_kernel_game_engine_kit::kernel::World;

#[derive(Debug)]
struct Position { x: f32, y: f32 }

let mut world = World::new();
let entity = world.spawn();
world.insert(entity, Position { x: 1.0, y: 2.0 });
world.get_mut::<Position>(entity).unwrap().x += 3.0;
assert_eq!(world.get::<Position>(entity).unwrap().x, 4.0);
assert!(world.despawn(entity));
assert!(!world.is_alive(entity));
assert!(world.get::<Position>(entity).is_none());
```

The convenience methods can panic on allocation failure. Use `try_spawn`, `try_insert`, and `try_despawn` where failures need to propagate. Component values require `Send + 'static`. Do not store references to frame-local objects in persistent components.

## Structural changes during a frame

Inside a callback, `ctx.defer_spawn()` and `ctx.defer_despawn(entity)` enqueue bounded structural work. The kernel applies it at its commit boundary. Queueing a spawn does not return a fully initialized entity that can immediately receive components. The current deferred payload API is narrower than a full generic ECS command system.

## Scratch arena

```rust
use rust_kernel_game_engine_kit::kernel::BumpArena;

let mut scratch = BumpArena::new(1024);
{
    let value = scratch.put(42_u64).expect("scratch capacity");
    assert_eq!(*value, 42);
}
assert!(scratch.used() > 0);
scratch.reset();
assert_eq!(scratch.used(), 0);
```

The kernel resets its arena once per frame. `put` accepts `Copy` values; arena reset is not a general destructor pass for arbitrary owned resources. Low-level raw allocation APIs require caller-managed pointer validity.

## Stable pool handles

```rust
use rust_kernel_game_engine_kit::kernel::Pool;

let mut pool = Pool::with_capacity(2);
let handle = pool.alloc(7_u64).unwrap();
assert_eq!(pool.get(handle), Some(&7));
pool.free(handle).unwrap();
assert!(pool.get(handle).is_none());
```

A pool bounds live allocation count and checks handle generations. Choose arena storage for temporary frame work, pools for reusable bounded slots, and ECS storage for entity-associated state. Benchmark their actual workloads rather than comparing unlike allocation lifetimes.

Sources: `src/kernel/ecs/world.rs`, `src/kernel/mem/arena.rs`, `src/kernel/mem/pool.rs`. Regression coverage includes `tests/kernel_world.rs` and the ECS mutation fuzz target.
