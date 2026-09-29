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

The report covers transform and frustum-culling batches at 1K, 64K, and 1M
items. Linux uses the compiled C object; Windows exercises the same safe API
through the Rust fallback.

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
