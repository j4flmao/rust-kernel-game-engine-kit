# Build your first game plugin

Use `GamePlugin` when implementing application rules. `GamePluginHost` adapts it to the kernel's ordinary `Subsystem` interface, so the scheduler does not need game-specific behavior.

## A complete minimal application

Create an example Rust file in a local checkout and run it with `cargo run --example your_example`. This example uses no window and needs no graphics device.

```rust
use rust_kernel_game_engine_kit::kernel::{
    GamePlugin, GamePluginHost, Kernel, KernelContext,
};

struct Game { elapsed_ns: u64 }

impl GamePlugin for Game {
    fn name(&self) -> &'static str { "my_game" }
    fn dependencies(&self) -> &'static [&'static str] { &[] }

    fn tick(&mut self, _ctx: &mut KernelContext<'_>, dt_ns: u64) {
        self.elapsed_ns = self.elapsed_ns.saturating_add(dt_ns);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(GamePluginHost::new(Game { elapsed_ns: 0 })))?;
    kernel.init()?;
    kernel.run(3)?;
    kernel.shutdown();
    Ok(())
}
```

Each callback receives the fixed simulation delta. Three successful ticks accumulate `50_000_001` nanoseconds. This is simulation time, not a measurement of startup or rendering latency.

## Add dependencies deliberately

Return registered names from `dependencies()` when a driver must tick before your plugin. A name alone does not instantiate that driver. Missing registrations and cycles are initialization errors. Start with the exclusive access default; change access metadata only after identifying every logical resource your code reads and writes.

## Put work in the right callback

Allocate and prepare persistent state in `init`. Advance rules in `tick`. Release application-owned resources in `shutdown`. Use messages and deferred commands for engine interactions. Avoid retaining frame scratch references or native handles owned by another driver.

## Learn from the applications

Sudoku separates model, game, input, layout, UI, paint, and native presentation. Rubik separates discrete cube state, move/animation management, camera, scene extraction, and the native composition loop. Read their entry points before reusing a platform-specific assumption.

Source: [game.rs](https://github.com/j4flmao/rust-kernel-game-engine-kit/blob/main/src/kernel/game.rs).
