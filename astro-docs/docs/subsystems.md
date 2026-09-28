# Audio, physics and scripting

These drivers provide small deterministic contracts that can run headlessly. Their scope is deliberately narrower than a production audio device, collision solver, or general-purpose scripting language.

## Audio voice state

```rust
use rust_kernel_game_engine_kit::subsystems::audio::AudioSubsystem;

let mut audio = AudioSubsystem::with_capacity(8);
let voice = audio.add_voice(1, 500).expect("voice capacity");
assert!(audio.voices()[voice].active);
assert!(audio.stop_voice(voice));
```

The subsystem advances phase state using simulation time. `gain_milli` is stored per voice. This code does not open WASAPI/ALSA or play a sound through speakers; native output remains a separate integration concern.

## Fixed-point physics state

```rust
use rust_kernel_game_engine_kit::subsystems::physics::{Body, PhysicsSubsystem};

let mut physics = PhysicsSubsystem::with_capacity(16);
physics.add_body(Body {
    position: [0, 0],
    velocity: [100, 0],
}).expect("body capacity");
assert_eq!(physics.bodies().len(), 1);
```

The driver integrates two-dimensional integer position/velocity using nanosecond time and saturating arithmetic. Registration requires its input dependency. Adding a body alone does not advance it. This is not documentation for collision detection or a rigid-body constraint solver.

## Bounded scripting VM

```rust
use rust_kernel_game_engine_kit::subsystems::scripting::{
    Instruction, ScriptingSubsystem,
};

let mut vm = ScriptingSubsystem::with_capacity(16, 8, 32);
assert!(vm.load(&[
    Instruction::Push(2), Instruction::Push(3),
    Instruction::Add, Instruction::Halt,
]));
```

Loading resets program state; execution is driven by the subsystem tick. The instruction set consists of integer stack arithmetic and halt. Stack overflow, underflow, allocation failure, and instruction-budget exhaustion have explicit errors. It is not a Lua or JavaScript bridge.

## Integration entry points

The default binary registers these concrete drivers alongside window, input, UI and renderer. Follow `src/main.rs` for the complete dependency set. Read `tests/phase4_headless.rs` for their headless acceptance coverage.
