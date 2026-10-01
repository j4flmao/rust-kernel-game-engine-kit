# Optional native acceleration

The `native-accel` Cargo feature exposes a small, batch-oriented C ABI for
CPU-side 3D transforms. It is deliberately optional:

```powershell
cargo test --all-features
cargo bench --bench native_accel --features native-accel
```

The safe Rust API validates slice lengths before calling C. Linux/macOS Unix
targets compile the C object through the repository build script. Windows
currently uses the identical Rust fallback behind the same API; this keeps
MSVC builds reproducible until a Windows C toolchain is explicitly selected.
The C code must never be called once per object; callers should submit large
contiguous batches so the FFI boundary remains amortized.

The benchmark compares the native path with the scalar Rust baseline. It is a
measurement tool, not a promise that C is faster on every compiler or CPU.

The report compares all four operations against checked scalar Rust at 1K,
64K, 1M and 1,000,003 items. Culling includes 0%, 50% and 100% visible inputs.
There are 48 benchmark cases. Labels reflect the actual backend: Linux with
the feature uses "c"; Windows uses "rust-fallback". Allocation is outside the
timed loops; input validation is timed on both paths. These are CPU measurements.

## Correctness and sanitizer gates

The Rust API panics before FFI on malformed lengths, out-of-range triangle
indices, negative radii and NaN/infinite inputs. Empty batches are valid;
empty AABB input returns None. Finite inputs can still overflow intermediate
floating-point arithmetic. Differential tests use bounded finite data and
relative tolerance for transform/normal results.

- native-correctness.yml: GCC/Clang on Linux, Windows Rust fallback and
  feature-disabled differential tests.
- native-sanitizers.yml: 4,096 deterministic C cases under ASan/UBSan,
  60 seconds of libFuzzer, and Rust wrappers calling GCC UBSan-instrumented C.
- native-performance.yml: separate GCC/Clang Criterion HTML artifacts and
  CPU/compiler metadata. No shared-runner FPS pass/fail gate.

```sh
cargo test --locked --test native_differential --features native-accel
cargo test --locked --test native_differential --no-default-features
# Linux/GCC: instrument C and link UBSan into the Rust test executable.
CC=gcc RKE_NATIVE_UBSAN=1 cargo test --locked --test native_differential --features native-accel
```

CC and AR select executable paths, not shell command strings. The build script
tracks them and RKE_NATIVE_UBSAN and enforces strict C compiler warnings.
SIMD dispatch and voxel greedy meshing remain future work.

## Platform matrix

| Target | Native C batch path | Rust fallback | Shader path |
|---|---:|---:|---|
| Linux | enabled with `native-accel` | yes | Vulkan GLSL → SPIR-V |
| macOS | enabled with `native-accel` | yes | compile/validation only unless a Vulkan runtime is configured |
| Windows | fallback by default | yes | Vulkan GLSL → SPIR-V |

Normal builds do not require a C compiler. Native Linux builds require `cc`
and `ar`; shader builds require `glslc` only when
`RKE_REQUIRE_GLSLC=1` is set. The Vulkan runtime and driver are still
platform concerns; compiling a shader does not prove that presentation works.

## Strict validation

```powershell
$env:RKE_REQUIRE_GLSLC="1"
cargo check --all-targets --all-features
cargo test --all-features
```

For the native benchmark report:

```powershell
cargo bench --bench native_accel --features native-accel
```
