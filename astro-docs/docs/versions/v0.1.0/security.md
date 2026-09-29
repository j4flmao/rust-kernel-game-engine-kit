# Security and Stability Plan

## Threat model

The engine may receive untrusted or semi-trusted input through:

- network packets and replicated state;
- asset and save files;
- runtime value keys;
- fuzzed scheduler/ECS commands;
- shader/material metadata;
- platform and device capability reports.

The design priority is security, then stability, then speed.

## Memory safety and allocation

Every externally sized operation must:

- validate count, byte length, alignment, and multiplication before allocation;
- use fallible growth where failure is possible;
- enforce per-frame, per-queue, per-value, and per-file limits;
- return a typed error or controlled drop policy;
- record rejection metrics without logging sensitive payloads.

Avoid assuming Box or Vec is safe merely because Rust owns it. Overflow can still become
a denial of service through excessive allocation, queue growth, fragmentation, or repeated
resize/copy work.

## GPU safety

- Validate all CPU-provided indices before writing GPU buffers.
- Treat GPU-written counters as untrusted until bounded and synchronized.
- Clamp indirect draw counts and instance ranges.
- Check shader-visible buffer sizes against the negotiated device limits.
- Keep descriptor indexing within a validated table.
- Reject unsupported feature combinations rather than forcing an unsafe path.
- Use a CPU fallback when the GPU path cannot prove its bounds.

## Synchronization safety

- Use explicit ownership for frame-in-flight resources.
- Never recycle a command buffer, staging allocation, descriptor, or buffer slice before
  its fence/timeline completion.
- Make shutdown drain workers, I/O completions, and GPU submissions in dependency order.
- Use futex/WaitOnAddress or platform equivalents only behind a tested safe abstraction.
- Add timeouts or cancellation for waits that can be influenced by external input.

## Networking and replication

- Keep authoritative simulation on the server.
- Separate reliable ordered, reliable unordered, and unreliable channels.
- Bound packet size, fragment count, ACK history, snapshot history, and per-client queues.
- Authenticate messages and use AEAD when confidentiality is required.
- Apply interest management and rate limits before allocating large replication payloads.
- Treat duplicate, late, reordered, and malicious messages as normal input cases.

## WAL and streaming

- Rotate and compact WAL files before they approach configured limits.
- Validate record length, checksum/authentication, sequence, and version before replay.
- Make compaction crash-safe with temporary files, fsync policy, and atomic replacement.
- Bound io_uring SQ/CQ and IOCP outstanding operations.
- Cancel or drain completions during shutdown.
- Do not turn a short read or completion error into an uninitialized buffer.

## Dependency and supply chain

CI should run:

- cargo audit;
- dependency policy and license checks;
- lockfile review;
- SBOM generation;
- secret scanning;
- unsafe-code review;
- Miri and ASan where supported;
- fuzzing of byte-oriented and state-machine boundaries.

A security failure blocks release until it has a documented fix, mitigation, or accepted
risk owned by a named maintainer.

## Incident evidence

Capture:

- commit and toolchain version;
- platform and driver capability;
- failing seed or input hash;
- queue and allocation high-water marks;
- frame/tick number;
- sanitized logs and backtraces;
- reproduction command.

Never upload secrets, full user data, or unrestricted environment dumps to CI artifacts.
