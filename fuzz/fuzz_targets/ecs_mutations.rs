#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::kernel::ecs::{Entity, World};

fuzz_target!(|bytes: &[u8]| {
    const MAX_ENTITIES: usize = 64;
    let mut world = World::new();
    let mut handles = Vec::with_capacity(MAX_ENTITIES);

    for pair in bytes.chunks_exact(2).take(256) {
        let slot = usize::from(pair[1]) % MAX_ENTITIES;
        match pair[0] % 5 {
            0 if handles.len() < MAX_ENTITIES => {
                handles.push(world.try_spawn().expect("bounded spawn must succeed"));
            }
            1 if slot < handles.len() => {
                let entity = handles[slot];
                let was_alive = world.is_alive(entity);
                let _ = world.try_despawn(entity);
                assert!(!world.is_alive(entity));
                if was_alive {
                    assert!(world.get::<u32>(entity).is_none());
                }
            }
            2 if slot < handles.len() => {
                let entity = handles[slot];
                if world.is_alive(entity) {
                    world.try_insert(entity, u32::from_le_bytes([pair[0], pair[1], 0, 0])).unwrap();
                    assert!(world.get::<u32>(entity).is_some());
                }
            }
            3 if slot < handles.len() => {
                let entity = handles[slot];
                let _ = world.remove::<u32>(entity);
                assert!(world.get::<u32>(entity).is_none());
            }
            _ => {
                // Probe an arbitrary stale or never-created handle. It must
                // never alias a live component after generation recycling.
                let probe = Entity::new(slot as u32, pair[1] as u32);
                if !world.is_alive(probe) {
                    assert!(world.get::<u32>(probe).is_none());
                }
            }
        }
    }

    assert_eq!(world.entity_count(), handles.iter().filter(|e| world.is_alive(**e)).count());
    assert!(world.count::<u32>() <= world.entity_count());
});
