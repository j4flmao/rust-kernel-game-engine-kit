# Contributing to rust-kernel-game-engine-kit

Thank you for contributing. This project is a low-level Rust engine foundation, so changes must preserve explicit boundaries, bounded resource usage, and cross-platform behavior.

## Before you start

1. Read the [README](README.md), especially the platform, memory, synchronization, and production-limitations sections.
2. Keep the kernel platform-agnostic. OS-specific code belongs under `src/platform/` and must not leak into kernel or subsystem contracts.
3. Check existing tests and nearby implementations before adding a new abstraction.
4. Do not commit secrets, private keys, save data, generated artifacts, `target/`, or temporary files.

## Development environments

- Windows native work should use the Windows target and a compatible Vulkan/Win32 toolchain.
- Linux work should run inside WSL or a native Linux host as appropriate. Do not use WSL as evidence that native Windows behavior works.
- Headless tests should remain runnable without a display server.

## Change guidelines

- Prefer safe Rust. Any `unsafe` must be narrowly scoped, documented, and justified at the boundary where it is required.
- Preserve bounded behavior: use checked arithmetic, `try_reserve`, explicit capacity limits, and fallible constructors at external or production boundaries.
- Return typed errors instead of silently dropping data or growing queues without a limit.
- Preserve deterministic lifecycle and dependency ordering.
- Treat Vulkan, io_uring, IOCP, raw syscalls, and OS handles as ownership-sensitive code. Document lifetime, completion, shutdown, and error paths.
- Do not add cryptography by hand. Use audited primitives and document whether a feature provides integrity, authenticity, or confidentiality.
- Keep hot paths allocation-aware and add a benchmark when changing allocator, bus, ECS, scheduler, sync, or serialization behavior.

## Local checks

Run the relevant checks before submitting a change:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo test --all-targets --all-features -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

For benchmark changes:

```bash
cargo bench --bench kernel -- --sample-size 10 --warm-up-time 1 --measurement-time 1
```

The Criterion HTML report is written to `target/criterion/report/index.html`.

For fuzzing and deeper validation, see the CI workflow in `.github/workflows/ci.yml` and the commands in `README.md`.

## Pull requests

Use a focused branch and a descriptive commit/PR title. A pull request should include:

- What changed and why.
- Which platform(s) were tested: Windows, WSL/Linux, or both.
- The exact validation commands and their results.
- Benchmark impact for performance-sensitive changes.
- Security or compatibility implications.
- Follow-up limitations when a native backend cannot be exercised on the current host.

Keep unrelated formatting or generated-file changes out of the pull request. Update documentation and tests when public behavior changes.

## Review priorities

Review order is intentional:

1. Security and memory safety.
2. Correctness, bounded failure behavior, and recovery paths.
3. Cross-platform lifecycle and ownership correctness.
4. Performance, measured with reproducible benchmarks.
5. Style and cleanup.

## Reporting a vulnerability

Please do not disclose security vulnerabilities in a public issue. Follow [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are provided under the repository's [Apache License 2.0](LICENSE).
