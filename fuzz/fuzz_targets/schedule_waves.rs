#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::kernel::{access_waves, dependency_waves, SystemAccess};

static READ_WORLD: [&str; 1] = ["world"];
static WRITE_WORLD: [&str; 1] = ["world"];
static READ_INPUT: [&str; 1] = ["input"];

fuzz_target!(|bytes: &[u8]| {
    // Bound every dimension derived from attacker-controlled bytes. The
    // generated graph is dependency-safe by construction: a node may only
    // depend on an earlier node, so this target exercises wave placement
    // without turning malformed input into an expected panic.
    let node_count = bytes.first().copied().unwrap_or(0) as usize % 32;
    if node_count == 0 {
        return;
    }

    let mut deps = vec![Vec::new(); node_count];
    for node in 1..node_count {
        let byte = bytes.get(node).copied().unwrap_or(0);
        if byte & 1 == 1 {
            deps[node].push((byte as usize % node).min(node - 1));
        }
    }

    let order: Vec<usize> = (0..node_count).collect();
    let accesses: Vec<SystemAccess> = (0..node_count)
        .map(|node| match bytes.get(node + node_count).copied().unwrap_or(0) % 4 {
            0 => SystemAccess::read_only(&READ_WORLD),
            1 => SystemAccess::read_only(&READ_INPUT),
            2 => SystemAccess::read_write(&READ_WORLD, &WRITE_WORLD),
            _ => SystemAccess::exclusive(),
        })
        .collect();

    let dependency = dependency_waves(&order, &deps);
    let access = access_waves(&order, &deps, &accesses);
    assert!(!dependency.is_empty());
    assert!(!access.is_empty());
    assert_eq!(dependency.iter().flatten().count(), node_count);
    assert_eq!(access.iter().flatten().count(), node_count);
});
