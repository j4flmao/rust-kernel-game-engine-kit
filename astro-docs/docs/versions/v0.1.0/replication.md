# Replication and persistence

`Replicator` manages bounded outgoing state and packet sequencing. The transport is supplied separately; constructing a replicator does not open a network socket.

## Coalesce pending deltas

```rust
use rust_kernel_game_engine_kit::kernel::{Replicator, SyncConfig};

let mut replication = Replicator::new(SyncConfig::default());
let first = replication.enqueue_delta(42, vec![1]).unwrap();
let replacement = replication.enqueue_delta(42, vec![2]).unwrap();
assert_eq!(first, replacement);
assert_eq!(replication.pending_len(), 1);
```

Both writes address the same pending delta key. The later payload replaces the earlier value without adding a second pending entry. Use events for changes that must retain event semantics; choose snapshot payloads for complete application-owned state. Serialization of that payload remains your application's responsibility.

## Budget and backpressure

Default configuration allows 4096 pending packets, 256 KiB per payload, and 128 in-flight packets. Retry policy uses frame counts, with a default resend interval of three frames and eight retries. Handle `Backpressure` and payload errors explicitly instead of allowing a producer to grow an unbounded queue outside the replicator.

## Persistence boundary

`SyncWal::open` and `open_with_limits` attach a file-backed log. Appending, acknowledgement, replay and compaction can fail with typed errors. Decide how application state is recovered before treating replay as equivalent to restoring a complete game. A saved UI screen alone is not an authoritative game snapshot.

## Authenticated packets

The module provides HMAC-SHA256 authentication and session nonce handling. Key provisioning, session establishment, trust policy and transport security remain integration responsibilities. Do not embed production secrets in examples or assume authenticated bytes authorize every requested game action.

Source: `src/kernel/sync.rs`. Read its inline tests for coalescing, replay and authentication behavior before implementing a transport adapter.
