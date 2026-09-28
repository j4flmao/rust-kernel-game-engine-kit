# Runtime Schedule and Parallel Execution Plan

## Current state

The repository already has dependency validation, topological ordering, dependency waves,
a bounded ParallelWaveExecutor, and a KernelContext. The kernel's subsystem tick path is
still serial and the parallel executor is an integration primitive rather than the
authoritative execution engine.

## Target model

Systems declare intent instead of relying only on manually ordered subsystem names:

- reads and writes for resources, components, messages, and render extraction data;
- explicit system sets and ordering constraints;
- deferred structural commands;
- run conditions and fixed-step stages;
- optional parallel eligibility.

The scheduler builds a persistent graph. It should not rebuild a full topological order
every frame unless the graph changes.

## Graph construction

1. Register systems and validate unique IDs.
2. Resolve explicit before/after constraints.
3. Convert access declarations into conflict edges.
4. Detect missing dependencies and cycles.
5. Compress the graph into stable execution levels or a ready queue.
6. Cache the validated plan until registration or access metadata changes.

A read/read pair may share a wave. Any write conflict, explicit ordering edge, or
exclusive resource access must serialize.

## Execution contract

~~~text
Stage begin
  -> acquire read/write access tokens
  -> run independent systems in bounded worker pool
  -> collect command buffers and metrics
  -> wait for wave completion
  -> apply deferred structural commands
  -> publish stage events
  -> release access tokens
Stage end
~~~

The executor must provide a serial reference mode. The reference mode is used for tests,
debugging, deterministic replay, and differential comparison with the parallel mode.

## Migration strategy

1. Introduce access metadata beside the existing Subsystem trait.
2. Wrap existing subsystems as exclusive nodes so behavior does not change.
3. Migrate read-only systems first.
4. Migrate systems that use disjoint ECS component sets.
5. Introduce deferred commands for structural ECS mutations.
6. Replace exclusive wrappers only after differential tests pass.
7. Integrate render extraction as a dedicated stage boundary.

## Failure and cancellation

- A failed system returns a typed error and prevents dependent systems from running.
- Independent work may finish, but its deferred commands are discarded unless the stage
  policy explicitly permits partial commit.
- Cancellation must drain worker ownership before resources are released.
- Queue saturation returns backpressure; it must not spawn unbounded threads.
- A worker panic is isolated and reported as a failed stage, not silently ignored.

## Acceptance tests

- deterministic graph output for the same declarations;
- rejection of cycles and missing dependencies;
- no simultaneous conflicting accesses;
- serial/parallel observable-state equivalence;
- deferred command ordering and commit atomicity;
- bounded queue and worker counts;
- cancellation during every stage;
- error propagation and shutdown;
- fixed-step determinism under different worker counts.

## Performance gates

Measure, do not assume:

- graph build time;
- scheduling overhead per system;
- worker utilization;
- wave width and critical path;
- queue contention;
- allocations per tick;
- p50/p95/p99 stage duration;
- frame deadline misses.

The parallel executor is useful only when the work is large enough to amortize scheduling
and synchronization overhead.
