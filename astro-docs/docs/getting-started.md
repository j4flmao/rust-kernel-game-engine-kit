# Getting started

## Prerequisites

Install Rust with Cargo and a native linker for your OS. The current manifest declares Rust **1.95** as its minimum version. Windows native examples need the Windows build toolchain; Rubik additionally needs a working Vulkan driver. Start with headless tests before diagnosing graphics support.

## Build and test

Run these commands from the repository root:

```sh
cargo check --locked --all-targets --all-features
cargo test --locked --lib --tests --all-features
cargo run --release
```

The default binary registers the demonstration subsystems and runs 120 fixed simulation frames. Its default renderer is headless. Windows can opt into the binary's native bootstrap with `RUST_KERNEL_NATIVE_VULKAN`; bootstrap failure reports a fallback.

## Interactive examples

```powershell
cargo run --example sudoku_game --release
$env:RKE_RUBIK_MAX_FRAMES = "600"
cargo run --example rubik_3d --release
```

Rubik defaults to 600 frames. Increase the limit for a longer session. A value of zero exits after initialization. See [examples](examples.md) for rendering differences and controls.

## Work on this documentation

```sh
cd astro-docs
npm ci
npm run dev
```

Open the local URL printed by Astro. Build a static copy with `npm run build`, then use `npm run preview` to inspect it. Markdown lives in `astro-docs/docs/`. Node dependencies and generated output stay inside `astro-docs`.

## Troubleshooting

Compiler version errors should be resolved against `Cargo.toml` and the repository toolchain policy. A native rendering failure does not mean the headless contracts failed. Record the OS, Vulkan device/driver, launch command, and full error before comparing native runs.
