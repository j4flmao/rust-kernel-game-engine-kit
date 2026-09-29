# CI/CD Plan

## Goals

## Checked-in workflow inventory

| File | Trigger | What to inspect |
| --- | --- | --- |
| `ci.yml` | Push on any branch, PR | Formatting, Linux/Windows, no_std smoke, properties/benchmarks, fuzz, audit, Miri, ASan, MSRV, io_uring, software Vulkan |
| `quality.yml` | PR, main push, manual | Rustdoc, examples and Sudoku contracts |
| `quality-milestone.yml` | Any-branch push, path-filtered PR, manual | CPU milestone report, contracts, shader/software Vulkan, Windows build, repeated stability |
| `astro-docs.yml` | Path-filtered PR/push, manual | Astro build, internal links/anchors, Markdown Rust examples |
| `coverage.yml` | PR, manual | LLVM coverage |
| `nightly.yml` | Weekly Saturday schedule, manual | Extended tests, fuzz and memory soak; despite its name it is not daily |
| `security.yml` | PR, main push, weekly, manual | Dependency review, cargo-deny and locked metadata |
| `workflow-security.yml` | PR, main push, weekly, manual | actionlint and zizmor |
| `codeql.yml` | PR, main push, weekly, manual | CodeQL analysis |
| `scorecard.yml` | Main push, branch protection change, weekly, manual | OpenSSF Scorecard |
| `label.yml` | PR target events | PR labels; not a GPU runner selector |
| `release.yml` | Version tag push | Bundles and draft release; requires tag commit to equal current origin/main and package version to match |

No dedicated hardware GPU workflow is enabled. Shader/software Vulkan checks and native example builds are separate from hardware performance validation. This inventory describes checked-in configuration, not current remote run conclusions or branch protection settings.

### Inspect a failure

```sh
gh run list --limit 10
gh run view RUN_ID --log-failed
gh run download RUN_ID --name quality-milestone-criterion-report
```

Use the failing job's actual command locally before changing a gate. Extract a complete Criterion artifact to preserve chart assets. A docs-only change can trigger the new Astro workflow; the workflow defined here does not publish the site.

### Planning material below

The following sections retain the delivery policy and proposed improvements. They do not assert that every suggested artifact or gate already exists.

CI must enforce the project's priority order:

1. security;
2. correctness and stability;
3. performance;
4. optional native hardware coverage.

The default PR pipeline must remain reproducible on hosted runners. Hardware-dependent
jobs are separate, clearly named, and never disguised as headless tests.

## Current baseline to preserve

The repository already has workflows for combinations of:

- formatting and Linux checks;
- Windows checks;
- wasm/no_std checks;
- property tests and Criterion benchmarks;
- fuzz build and smoke execution;
- dependency auditing;
- Miri;
- AddressSanitizer;
- coverage and nightly reliability.

Before changing required checks, record their exact job names with gh CLI and update branch
protection atomically. Do not rename a required check casually.

## Proposed workflow layout

### Pull request: fast required gate

- checkout with a pinned action major;
- cargo fmt --check;
- cargo check --all-targets --all-features;
- cargo test --all-targets --all-features with one controlled thread where needed;
- clippy with warnings denied for the supported target;
- cargo audit and dependency policy;
- focused property tests;
- headless renderer and scheduler smoke tests;
- documentation link and Markdown validation;
- upload logs on failure.

Target: deterministic and short enough for normal review.

### Pull request: platform matrix

Linux job:

- Ubuntu native hosted runner for Linux standard library behavior;
- WSL remains a developer convenience, not a substitute for native Linux surface tests;
- run headless renderer, io_uring capability-gated tests, and Linux synchronization tests.

Windows job:

- Windows hosted runner;
- run Windows compilation, Win32-gated tests, IOCP capability-gated tests, and headless
  renderer;
- native present smoke is conditional on an installed Vulkan driver and a visible or
  configured headless surface path.

### Scheduled reliability

Nightly jobs should run:

- Miri with a target/toolchain known to support the test;
- ASan with dynamic CRT/libc configuration;
- fuzz targets with bounded runs;
- long property runs;
- soak tests for scheduler, WAL, streaming, and replication;
- Criterion benchmark comparison;
- coverage generation;
- dependency audit and SBOM generation.

Nightly failures must be visible and must not be silently marked successful.

### Hardware validation

Use self-hosted or explicitly provisioned runners for:

- native Linux X11/Wayland Vulkan present;
- Windows Win32 Vulkan present;
- GPU-driven rendering differential tests;
- shader compilation and vendor-specific feature paths.

The hosted PR gate should still validate all contracts with headless and mocked/fake
backend paths.

## Required artifacts

- test logs and JUnit-style summaries;
- Criterion HTML report and raw measurements;
- coverage report;
- fuzz crash inputs and failing seeds;
- Miri/ASan diagnostics;
- Vulkan capability and device report;
- native smoke manifest;
- SBOM and dependency audit output.

Artifacts must have retention appropriate to their purpose. Crash inputs and baseline
reports must be retained long enough to reproduce a regression.

## Branch protection

Required checks should represent stable contract gates, not every experimental nightly
job. Recommended policy:

- require the fast PR gate;
- require Linux and Windows compile/test gates;
- require dependency security gate;
- require one approving reviewer for non-owner contributors;
- require branch up to date before merge if the team accepts the queue cost;
- allow administrator override only for documented emergency procedure;
- do not let an administrator bypass a failing security check silently.

The owner may review contributors' pull requests. If the owner is also the last pusher,
a repository policy requiring a different approving reviewer must be deliberately configured
or relaxed; this is a GitHub branch-protection policy choice, not a CI failure.

## Action and toolchain hygiene

- Pin action versions to reviewed major versions and review updates.
- Pin Rust toolchain policy where reproducibility requires it.
- Cache Cargo registry and target directories carefully; never cache secrets.
- Use least-privilege permissions.
- Avoid uploading workspace secrets or full environment dumps.
- Separate fork-safe PR jobs from trusted push jobs.
- Test failure paths and artifact upload paths.

## Rollout order

1. Inventory existing workflow job names.
2. Add documentation and headless contract checks.
3. Add missing Linux/Windows capability-gated jobs.
4. Add scheduled native self-hosted jobs.
5. Update branch protection only after checks are green on the default branch.
6. Monitor duration and flaky-test rate for two weeks.
7. Promote only stable checks to required status.
