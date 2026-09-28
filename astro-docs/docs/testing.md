# Testing and Benchmark Plan

## Test layers

### Unit tests

Cover pure contracts and bounded data structures:

- entity generations;
- sparse storage and bitsets;
- arenas, pools, rings, checked arithmetic;
- dependency graph validation;
- queue selection and feature policy;
- serialization, authentication, WAL record validation.

Use Arrange/Act/Assert structure. Test successful, error, empty, maximum, and
allocation-failure cases.

### Integration tests

Cover boundaries:

- kernel lifecycle and shutdown;
- message bus and deferred commands;
- scheduler serial versus parallel execution;
- render extraction;
- CPU render manifest;
- native lane state transitions with a fake backend;
- io_uring and IOCP adapters where the host capability exists;
- WAL rotation and replay.

### Property tests

Properties must be bounded, reproducible, and paired with example tests:

- graph order respects all dependencies;
- no conflicting systems overlap;
- entity generation prevents stale aliasing;
- storage operations preserve invariants under random mutation sequences;
- extraction never emits out-of-range GPU indices;
- WAL compaction preserves committed records;
- replication apply is idempotent;
- bounded queues never exceed capacity;
- serialize/deserialize round trips preserve the contract.

Run a fast bounded set on pull requests and a larger set nightly. Capture seeds and add
every discovered counterexample as a regression test.

### Fuzzing

Initial fuzz targets:

- scheduler declarations;
- ECS mutation command streams;
- extraction descriptors;
- replication/WAL record bytes;
- renderer capability and frame-state transitions.

Fuzzing must assert no panic, no undefined behavior under supported unsafe boundaries, no
unbounded allocation, and no hang. Store crash inputs as artifacts and promote stable
cases into tests.

### Smoke tests

Keep a small deterministic suite under five minutes:

- package builds;
- headless kernel starts and shuts down;
- one fixed frame completes;
- renderer fallback completes;
- scheduler executes a known dependency graph;
- bounded memory errors are surfaced;
- native jobs report skip rather than false pass when capability is absent.

### Soak tests

Nightly or pre-release:

- millions of scheduler ticks;
- repeated swapchain recreation;
- streaming queue pressure;
- WAL rotation and crash/restart simulation;
- replication with loss, duplication, reordering, and reconnect;
- long-lived allocation high-water monitoring.

## Benchmark suites

Use Criterion HTML reports for:

- dependency graph build and cached plan execution;
- serial versus parallel schedule execution;
- ECS query iteration and extraction;
- value-index lookup/intern/dirty drain;
- arena, pool, and ring operations;
- message bus publish/drain;
- WAL append/rotation/compaction;
- replication snapshot/delta encode/decode;
- renderer CPU manifest generation;
- GPU-driven culling and indirect command preparation when hardware exists.

Each benchmark records workload shape, entity count, component density, queue depth,
worker count, feature set, and platform.

## Benchmark rules

- Establish a baseline before optimization.
- Compare p50, p95, p99 where timing distribution matters.
- Report allocations, bytes copied, queue depth, and peak memory when possible.
- Do not fail a benchmark for ordinary hosted-runner noise without a stable threshold.
- Use trend detection and manual review for hardware-dependent GPU results.
- Fail on large, repeated regressions only after calibrating the runner variance.
- Store the HTML report as a CI artifact on every benchmark run.

## Performance acceptance examples

These are starting gates, not universal promises:

- no unexpected allocation in the fixed tick hot path;
- no unbounded worker or queue growth;
- schedule graph cache hit does not rebuild the graph;
- extraction cost scales with changed/visible data rather than all historical data;
- GPU buffer writes remain within checked capacity;
- memory high-water returns to the expected steady-state envelope;
- native present does not busy-loop on resize or occlusion.

The final numeric thresholds must be derived from representative workloads and recorded
with the benchmark baseline.
