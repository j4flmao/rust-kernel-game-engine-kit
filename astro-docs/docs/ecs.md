# ECS Runtime Value Index

## Purpose

The runtime sometimes needs to answer questions that the Rust compiler cannot know in
advance:

- Which runtime value is currently associated with an entity?
- Which entities share a value?
- Which values changed this tick?
- Which bounded render or replication fragment should be emitted?

The solution is a side index over typed ECS data. It must not turn the ECS into an
untyped string database.

## Compiler-known information

Rust and monomorphized systems can specialize:

- component and resource types;
- query access shape;
- read/write permissions;
- component layout and alignment;
- system function signatures;
- fixed operations in hot loops.

This is where the kernel gets predictable code generation and data-oriented iteration.

## Runtime-known information

The runtime resolves:

- actual entity membership;
- dynamic value contents;
- change ticks and visibility;
- asset/material/mesh handles;
- network ownership and interest;
- current GPU capacity and device features.

The runtime index stores handles, hashes, compact keys, or interned IDs. It does not
replace the typed value in component storage.

## Proposed model

~~~text
Typed component storage
  Position, MeshRef, MaterialRef, Visibility, ReplicationState
        |
        +--> change ticks and extraction filters
        |
        +--> bounded runtime side indices
              ValueKey -> entity set / fragment list / dirty range
~~~

Recommended properties:

- stable key type instead of raw String in hot paths;
- explicit collision handling;
- generation-aware entity references;
- bounded index capacity and fallible growth;
- deterministic iteration order where output is replicated or tested;
- eviction policy for temporary or weakly referenced values;
- metrics for hit rate, collisions, bytes, and rejected insertions.

## API direction

The eventual API should expose contracts similar to:

- intern_value(value) -> Result<ValueId, ValueIndexError>;
- lookup(ValueId) -> Option<ValueView>;
- attach(Entity, ValueId) -> Result<(), IndexError>;
- detach(Entity, ValueId);
- drain_dirty(range_budget) -> DirtyBatch;
- remove_generation(Entity).

The exact API is intentionally deferred until the storage and extraction benchmarks are
available. Do not commit to a hash map implementation solely because it is convenient.

## Security and stability

- Reject oversized keys and values before hashing or allocation.
- Use a keyed hash when attacker-controlled values can influence table placement.
- Never use an unchecked entity index to address a side table.
- Bound per-value fan-out to prevent one value from creating an unbounded entity list.
- Apply backpressure or return an error when dirty queues are full.
- Treat stale generations as no-ops or explicit errors, never as another live entity.
- Do not expose internal pointers or allow runtime data to control arbitrary GPU offsets.

## Verification

Properties:

1. Interning the same canonical value is idempotent.
2. Different generations never alias.
3. Attach/detach is reversible.
4. Dirty draining never returns more than the configured budget.
5. Removing an entity removes all side-index references.
6. Serialization round-trips preserve IDs or explicitly remap them.
7. A rejected allocation leaves the index unchanged.

Performance measurements:

- lookup latency at cold and warm cache;
- insertion and eviction cost;
- memory bytes per entity/value relation;
- dirty-batch extraction cost;
- collision behavior under adversarial keys.
