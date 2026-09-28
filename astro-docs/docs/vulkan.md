# Native Vulkan and Platform Plan

## Platform policy

- Windows native validation runs directly on Windows, not WSL.
- Linux compilation and headless tests may run in WSL.
- Linux surface/present validation requires a native Linux host with a configured display
  server and Vulkan driver.
- A missing display, loader, extension, feature, or device must select a documented
  headless/fallback state.

## Composition-root sequence

The native composition root should:

1. read a validated renderer policy;
2. create a Vulkan instance with required and optional extensions;
3. create the platform surface when a window is requested;
4. enumerate physical devices;
5. score devices and validate required queue families/features;
6. create the logical device and queues;
7. create the swapchain and image views;
8. allocate per-frame command pools and synchronization;
9. initialize the renderer subsystem;
10. run acquire, record, submit, and present;
11. recreate the swapchain on resize or out-of-date status;
12. drain and destroy resources in dependency order.

Each stage returns a typed error and includes enough context for diagnostics.

The renderer-facing contract is `RenderBackend::submit`. Headless CI uses a validating
backend; native Linux and Windows implementations must consume the same prepared frame,
then own Vulkan buffers, command recording, queue submission, and present transitions.
The PAL now owns a bounded `NativeBuffer` create/bind/destroy path with physical-memory
type selection on both platforms. Mapping, staging copies, and renderer frame ownership
are still separate follow-up steps.

## Queue policy

The existing queue-family policy should remain explicit:

- graphics queue;
- compute queue;
- transfer queue;
- present queue, which may be distinct from graphics.

Prefer dedicated queues only when synchronization and ownership-transfer costs do not
outweigh the benefit. The first correct implementation may use one universal queue.

## Synchronization

Every frame-in-flight owns:

- command pool and primary command buffer;
- image-available semaphore;
- render-finished semaphore;
- fence or timeline completion state;
- transient upload and descriptor lifetime.

The renderer must wait before recycling frame-owned resources. Acquire, command recording,
submit, and present are separate state transitions with explicit error handling.

## Surface backends

Linux:

- keep X11 and future Wayland policy behind platform modules;
- request only extensions required by the selected surface;
- make WSL/headless behavior explicit;
- test loader discovery, surface creation, resize, and present on native Linux.

Windows:

- use the Win32 surface extension;
- validate HWND ownership and lifecycle;
- handle resize and occlusion without busy loops;
- test loader discovery, surface creation, resize, and present on a Windows host.

## Device loss and recovery

The renderer must distinguish:

- recoverable swapchain invalidation;
- transient acquire/present status;
- out-of-device-memory;
- device loss;
- unsupported feature;
- programmer/configuration error.

Recovery policy:

- stop issuing new work;
- capture diagnostics and frame counters;
- destroy/recreate swapchain when valid;
- reinitialize device-dependent resources when supported;
- fall back to headless or fail startup with a clear error when recovery is unsafe.

## Testing matrix

| Area | WSL | Native Linux | Windows |
|---|---:|---:|---:|
| cargo check/test | yes | yes | yes |
| headless renderer | yes | yes | yes |
| Vulkan loader discovery | conditional | yes | yes |
| X11/Wayland surface | no by default | yes | n/a |
| Win32 surface | n/a | n/a | yes |
| swapchain/present | no by default | yes | yes |
| shader/manifest validation | yes | yes | yes |

No CI job should claim native presentation coverage when it only executed a headless path.

## Exit criteria

- No unchecked native handle crossing the safe renderer boundary.
- No Box or Vec growth on an unbounded input path.
- Native smoke tests produce a machine-readable result and diagnostic artifact.
- CPU fallback remains available for every unsupported capability set.
