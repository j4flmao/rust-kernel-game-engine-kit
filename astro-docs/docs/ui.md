# UI Rendering Plan

Status: **partially implemented; production hardening remains planned**

The retained tree, generational IDs, bounded commands, two-pass layout, paint/snapshot extraction, focus/capture, resource tables and fallback text contracts exist in `src/subsystems/ui/`. `tests/ui_rendering.rs` covers integration behavior. [Create and lay out UI](ui-guide.md) is the current usage guide. The phases below describe design and acceptance requirements, not a claim that every listed module is absent.

This document defines the next implementation track after the kernel foundation,
ECS boundaries, deferred mutation path, render-world extraction, bounded GPU
preparation, and native present lanes are in place.

The goal is a production-oriented UI renderer that is deterministic, bounded,
platform-independent at the contract layer, and able to use CPU batching for
small or order-sensitive UI while using GPU instancing and indirect work for
large repeated content.

## 1. Repository baseline

The current repository already provides the following renderer foundations:

- `Kernel` lifecycle, dependency validation, schedule waves, message bus, ECS,
  deferred structural commands, and frame-bounded memory primitives.
- `RendererSubsystem` with explicit initialization and shutdown states.
- A frame-owned `RenderWorld` that copies typed render data out of ECS storage.
- A bounded `CpuRenderManifest` that sorts instances and creates deterministic
  mesh/material batches.
- `GpuFramePreparation`, stable GPU instance records, indirect command encoding,
  checked byte-count arithmetic, and headless submission statistics.
- Linux and Windows native present lanes with surface, swapchain, acquire,
  command recording, queue submit, fence, present, and resize/recreate paths.
- Headless rendering remains the default, so kernel tests do not require a
  window system or a physical GPU.

The current UI subsystem emits renderer-owned UI frame packets. UI data must not be pushed into
the simulation world as arbitrary strings or unbounded component types, and UI
must not call Vulkan directly from ECS systems.

## 2. Design decisions

### 2.1 Two-world ownership

The simulation world owns gameplay state and input intent. A render-owned UI
world owns resolved layout, clip state, glyph runs, and paint items for the
current frame. Extraction copies only bounded, render-ready values across the
boundary.

### 2.2 Retained structure, immediate frame output

Keep a retained UI tree for identity, state, focus, layout dirtiness, and
resource handles. Rebuild a compact frame paint list only for dirty subtrees or
when viewport, theme, font, or asset generations change. The renderer consumes
the paint list, not the tree, on the hot path.

### 2.3 Stable typed IDs instead of runtime type explosion

Use compact IDs such as `UiNodeId`, `StyleId`, `FontId`, `TextureId`, and
`ClipId`. Runtime text, labels, and user values remain data behind typed fields;
they do not become new ECS component types or archetypes.

### 2.4 Batching is constrained by ordering and state

The batch key must preserve correctness. A proposed key is:

```text
(pass, clip, texture, sampler, pipeline, blend_mode, z_order, material)
```

Opaque or reorderable UI can be grouped aggressively. Text, alpha-blended
overlays, nested clips, and accessibility overlays retain stable order. Never
sort across a clip or z-order boundary merely to increase batch size.

### 2.5 Bounded memory is part of the API

Every tree, string, glyph run, clip stack, paint list, upload buffer, and command
list has an explicit capacity. Construction and growth use fallible operations.
Overflow must produce a visible diagnostic and a safe fallback, never a panic or
silent truncation of an input-controlled range.

## 3. Target frame pipeline

```text
Input events
    -> UI event routing and focus policy
    -> typed UI systems and deferred tree mutations
    -> layout invalidation and bounded layout solve
    -> render-world extraction
    -> clip resolution and paint-list generation
    -> stable batch-key sort within legal ordering boundaries
    -> dirty-range upload to the frame ring
    -> CPU draw or GPU instanced/indirect path
    -> Vulkan command recording
    -> queue submit and present
```

The pipeline has explicit synchronization points:

1. Input is drained before UI systems run.
2. Structural tree changes are applied at the deferred boundary.
3. Layout is solved before paint extraction.
4. The render world is immutable while GPU preparation runs.
5. Upload buffers remain alive until the frame fence signals.
6. Present and resize errors return to the platform state machine.

## 4. Implementation phases

### Phase U0: contracts, budgets, and diagnostics

Files and areas:

- Add a UI module under `src/subsystems/ui/` or an equivalent isolated module.
- Add `UiConfig` with maximum nodes, depth, text bytes, glyphs, clips, paint
  items, batches, and per-frame upload bytes.
- Add typed error and diagnostic enums for capacity, invalid parent, cycle,
  depth, layout, resource, and render submission failures.
- Add counters to renderer telemetry: dirty nodes, layout passes, paint items,
  batches, glyphs, uploaded bytes, clipped items, and overflow fallbacks.

Acceptance:

- All limits are checked before indexing or allocation.
- A deliberately exhausted budget returns an error and keeps the kernel alive.
- No UI code owns a Vulkan handle.

### Phase U1: retained UI tree and ECS-facing components

Implement:

- `UiNodeId` with generation checks and stale-handle rejection.
- Bounded parent/child/sibling storage with deterministic traversal.
- Typed components for local transform, visibility, z-order, style, text,
  image, interaction, and accessibility metadata.
- Deferred commands for create, reparent, remove, set-style, set-text, and
  set-visibility.
- Stable focus/capture state driven by input messages.

Acceptance:

- Reparenting cannot create cycles or exceed configured depth.
- Stale node handles cannot mutate replacement nodes.
- Applying the same command stream produces the same tree and change ticks.

### Phase U2: layout engine

Implement a constrained layout model first; do not begin with an unbounded CSS
clone. Support the minimum useful set:

- fixed, min, max, and percentage sizes;
- row and column flow;
- padding, margin, gap, alignment, and absolute overlay;
- text measurement hooks;
- viewport scale and DPI policy;
- dirty-subtree propagation and bounded iterative solving.

Use a two-pass approach:

1. Measure intrinsic content from leaves upward.
2. Assign final rectangles from parents downward.

Acceptance:

- Zero-size, NaN, infinity, negative, and oversized inputs are rejected or
  clamped by policy.
- Layout has a maximum iteration count and reports non-convergence.
- A golden set covers nested panels, scrolling regions, text wrapping, and
  resize/DPI changes on Linux and Windows.

### Phase U3: clip, paint, and ordering model

Implement a render-owned paint list with explicit item variants:

- solid or gradient rectangle;
- border and rounded rectangle;
- image or atlas quad;
- glyph run;
- custom primitive behind a validated contract.

Each item carries a resolved rectangle, clip ID, z-order, resource IDs, and
opacity. Build a clip table with scissor rectangles first; defer stencil or
rounded-mask complexity until the basic path is stable.

Acceptance:

- Paint order is deterministic and stable across runs.
- Nested clips are intersected with checked arithmetic.
- Items outside the viewport are rejected before upload.
- CPU reference output can be compared with the GPU batch plan.

### Phase U4: batching and upload strategy

Add UI-specific batch keys and integrate them with the existing
`CpuRenderManifest`/`GpuFramePreparation` boundary without weakening the mesh
renderer.

Recommended paths:

- Small or order-sensitive UI: CPU-generated indexed quads and one command per
  legal batch.
- Repeated sprites/panels: instanced quad records grouped by texture and clip.
- Large repeated lists: GPU visibility/compaction and indirect draws, with a
  CPU fallback for unsupported devices.
- Text: atlas-backed glyph records; retain order for alpha blending and use
  one atlas/material batch where clip and pipeline permit it.

Acceptance:

- No batch crosses a clip, pipeline, blend, or ordering boundary.
- Dirty rectangles update only the required instance/glyph ranges.
- Upload size and draw count are checked before encoding.
- Vulkan barriers cover host-to-device, compute-to-draw, and draw-to-present
  transitions.

### Phase U5: text, assets, and input integration

Implement resource contracts, not a platform-specific font stack in the kernel:

- font and glyph-provider trait with deterministic metrics;
- atlas allocation with bounded pages and eviction generations;
- asynchronous asset readiness messages;
- pointer, keyboard, text-input, focus, capture, and IME intent messages;
- fallback glyph and missing-resource rendering.

Acceptance:

- Missing or late assets render a safe placeholder without stalling the frame.
- Input is consumed once and routed according to capture/focus rules.
- Text byte, glyph, atlas, and upload quotas are enforced.

### Phase U6: renderer and native backend wiring

Wire the UI render world into `RendererSubsystem`:

1. UI systems publish typed intent and deferred tree commands.
2. UI extraction copies resolved values into a render-owned frame snapshot.
3. The renderer builds UI paint items after layout and before GPU preparation.
4. Native Linux and Windows lanes consume the same backend-neutral upload plan.
5. Headless mode validates ordering, counts, clipping, and batch equivalence.

Do not put OS window callbacks or Vulkan loader calls in the UI tree module.
Platform modules only provide window metrics, input events, surfaces, and GPU
submission capabilities.

Acceptance:

- Existing headless renderer tests remain unchanged and pass.
- Linux X11/Wayland policy and Win32 paths use identical UI contracts.
- Resize, minimize, swapchain out-of-date, device loss, and surface teardown
  do not leave stale UI resources reachable.

### Phase U7: verification, performance, and hardening

Add tests and CI gates:

- property tests for tree operations, layout invariants, clip intersection,
  batch boundaries, and stale handles;
- fuzz targets for hostile tree commands, text input, style values, and paint
  list limits;
- Criterion benchmarks for layout, paint generation, batching, glyph lookup,
  dirty upload, and full headless UI frame;
- Linux lavapipe UI smoke and native X11 validation when available;
- Windows 2022 and Windows 2025 headless/native smoke;
- ASan, Miri for safe reference paths, and long-running allocation/resize soak;
- HTML benchmark reports and a tracked regression threshold.

Production gates:

- no unchecked arithmetic on external or asset-controlled counts;
- no unbounded `String`, `Vec`, or command stream growth in a frame;
- no unsafe UI-to-Vulkan handle crossing outside the renderer backend;
- no data race between simulation extraction and render preparation;
- deterministic headless output for the same input trace.

## 5. Suggested module layout

```text
src/subsystems/ui/
  mod.rs              public UI contract and subsystem lifecycle
  id.rs               generational node/resource IDs
  tree.rs             bounded retained tree and traversal
  components.rs       typed UI component values
  commands.rs         deferred structural and property commands
  input.rs            focus, capture, hit testing, routed input
  layout.rs           measure and assign passes
  paint.rs            clip table, paint items, stable ordering
  text.rs             glyph runs and font-provider contract
  resources.rs        atlas and asset readiness handles
  budget.rs           quotas and diagnostics
  tests.rs             deterministic unit and property helpers

src/subsystems/renderer.rs
  UI extraction adapter
  UI batch key and upload integration
  headless reference submission

src/platform/{linux,windows}/
  window metrics and input translation only
  native UI upload and command submission adapters
```

## 6. Performance model

The first implementation should optimize bounded work and predictable memory,
not maximize a synthetic batch count.

Target complexity per frame:

- input routing: `O(events * bounded hit-test depth)`;
- dirty propagation: `O(dirty nodes + affected descendants)`;
- layout: `O(visited nodes * bounded iterations)`;
- paint generation: `O(visible nodes + glyphs)`;
- deterministic batching: `O(items log items)` within legal ordering segments;
- upload: `O(dirty bytes)` when resource generations are unchanged.

When profiling shows sort cost dominating, replace global sorting with stable
bucket insertion by pass and clip, but only after golden output proves ordering
equivalence. Keep the CPU reference manifest as an oracle for the optimized path.

## 7. Delivery order

1. U0 contracts and budgets.
2. U1 tree and deferred commands.
3. U2 layout with golden tests.
4. U3 paint and clip reference path.
5. U4 CPU batching and headless upload validation.
6. U5 text/assets/input contracts.
7. U6 native Vulkan integration and resize/device-loss paths.
8. U7 fuzz, benchmarks, CI, soak, and production sign-off.

Do not skip U0 or U7. UI rendering is a high-volume input boundary; the
correctness and memory ceilings are part of the renderer design, not follow-up
cleanup.

## 8. Definition of done

The UI track is complete only when:

- retained tree mutations, layout, paint, clipping, text, and batching have
  deterministic headless tests;
- the same bounded UI render snapshot can feed Linux, Windows, and headless
  backends;
- Vulkan command recording uses validated frame-owned buffers and fences;
- resize, minimized windows, missing assets, device loss, and budget overflow
  have explicit behavior;
- property/fuzz/benchmark/CI evidence is stored by the repository workflow;
- no UI contract requires callers to know platform Vulkan handles.
