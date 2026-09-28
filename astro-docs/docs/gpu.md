# GPU-Driven Rendering Plan

## Intent

GPU-driven rendering is the rendering path for large, mostly static or moderately
dynamic scenes. It is not the ECS execution model and it is not enabled by default until
the platform and validation gates are complete.

The target path is:

~~~text
Simulation ECS
  -> bounded extraction
  -> render-world instance tables
  -> compute culling and binning
  -> optional GPU sort
  -> indirect command generation
  -> graphics command buffer
  -> queue submission and present
~~~

## Render-world data

The render world should contain only data needed to prepare a frame:

- stable render instance ID;
- transform or compact transform representation;
- mesh and material IDs;
- bounds and visibility flags;
- layer, view mask, and pass classification;
- change tick and upload generation;
- optional skinning, morph, or animation offsets.

The extraction phase must copy or reference immutable, frame-owned data. It must not hand
the GPU a pointer into mutable ECS storage.

## GPU buffers

Use separate bounded tables for:

1. instance data;
2. mesh and material metadata;
3. visible instance IDs;
4. bin counters;
5. indirect draw commands;
6. optional sort keys.

Every table needs:

- a maximum element count;
- checked byte-size multiplication;
- fallible resize or a controlled overflow policy;
- generation or frame ownership;
- explicit visibility of CPU/GPU synchronization;
- metrics for capacity, high-water mark, and rejected entries.

On overflow, the first response is a controlled fallback or frame rejection, not an unchecked
write. The renderer must report the reason and preserve process stability.

## Culling and binning

The first implementation should use conservative frustum culling and material/mesh bins.
Occlusion culling, GPU sorting, and more advanced compaction are later optimizations.

Recommended sequence:

1. Upload changed instance ranges.
2. Clear bounded counters.
3. Dispatch culling.
4. Write visible IDs and atomic bin counts.
5. Prefix-scan or use a bounded allocation strategy.
6. Write indirect commands.
7. Insert compute-to-draw barriers.
8. Record indirect draw calls.
9. Submit with frame fences and semaphores.

The kernel should not expose raw GPU synchronization to gameplay systems. The renderer
owns barriers and queue submission.

## CPU fallback

A CPU fallback is mandatory for:

- headless tests;
- devices without the selected Vulkan features;
- small scenes where dispatch overhead dominates;
- debugging and differential tests;
- swapchain recreation and device-loss recovery.

The fallback should produce the same logical draw manifest as the GPU path. A manifest is
a testable list of pass, pipeline, material, mesh, instance range, and draw count.

The repository now has the first CPU reference implementation: RenderWorld instances are
sorted deterministically by mesh/material/entity and grouped into bounded DrawBatch
ranges. GpuFramePreparation copies that manifest into bounded instance and indirect
command models, exposes a stable `GpuInstanceRecord` upload layout, encodes the exact
`VkDrawIndexedIndirectCommand` wire shape, and rejects invalid ranges before a native
backend is allowed to submit. GPU indirect generation must match this manifest before it
can become the default.

## Feature negotiation

Do not assume descriptor indexing, indirect-count draws, subgroup operations, or timeline
semaphores. Negotiate features and retain a capability bitset. Select a path in this
order:

1. GPU-driven path with all required features;
2. GPU-driven path with bounded compatibility features;
3. CPU indirect/direct path;
4. headless completion path.

## UI and transparent content

Opaque world geometry is the first GPU-driven target. UI and transparent objects have
different ordering constraints:

- UI may use a separate sorted batch list;
- transparent objects require depth-aware ordering or an explicit approximation;
- text may use atlas/material bins but still needs stable clipping and draw order;
- debug overlays must remain available when GPU-driven mode is disabled.

Do not force every component or UI node through the same generic indirect buffer.

## Verification

Required differential tests:

- fixed scene CPU versus GPU visible set;
- bin counts and draw ranges;
- zero-instance and maximum-capacity cases;
- resize during extraction;
- despawn during frame handoff;
- device feature fallback;
- indirect command count bounds;
- repeated frames with unchanged data;
- device loss and swapchain invalidation.

Required benchmarks:

- extraction time;
- upload bytes and upload time;
- culling dispatch time;
- command count;
- CPU submission time;
- GPU frame time;
- frame latency and missed deadlines;
- memory high-water marks.

## Implementation order

1. Add render-world types and CPU extraction.
2. Add CPU sorting/binning and manifest tests.
3. Add Vulkan buffer ownership and staging uploads.
4. Add compute culling with bounded counters.
5. Add indirect commands and barriers.
6. Add feature negotiation and fallbacks.
7. Add native platform smoke tests.
8. Enable GPU-driven mode behind an explicit feature or runtime policy.
