# Expanse-X (EX)

An independent low-level graphics API written in Rust. EX will define its own
public API, runtime, command model, and execution backend. The first backend will
be a software/reference GPU; rendering will not use Vulkan, Direct3D, Metal,
OpenGL, or wgpu.

## Current status: Phase 0

The production workspace builds and contains a tested buffer-description and
debug-label recording prototype. Applications import the `ex-api` package as
`ex`. No instance, adapter, device, queue, resource allocation, submission, shader,
or renderer is implemented yet. The example does not draw a triangle.

```text
cargo run -p ex-api --example api_draft --locked --offline
```

## Build and check

Install Rust through rustup. `rust-toolchain.toml` selects stable Rust with
rustfmt, Clippy, and Rust source; no extra platform target is required. Current
packages use only the Rust standard library and local workspace dependencies.
After the toolchain is installed, these commands work without network access:

```text
cargo fmt --all -- --check
cargo build --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo doc --workspace --no-deps --locked --offline
```

`scripts/check.ps1` runs these checks in order and fails on the first error. Run
`pwsh -NoProfile -File scripts/check.ps1` from any directory, or use the direct
commands from the project root in CI. Tests include executable and compile-fail
documentation examples. Keep the workspace lockfile committed. The stable
toolchain follows stable releases; it is not a pinned compiler-version guarantee.

## Repository map

| Path | Responsibility |
| --- | --- |
| `crates/ex-api/` | Public Rust entry point, imported as `ex`; runnable example |
| `crates/expansex-core/` | Existing shared descriptor and recording implementation |
| `docs/` | Current EX contracts, architecture, assessment, and conventions |
| `scripts/` | Repeatable production checks |

## Next milestones

1. Phase 1: instance → reference adapter → device → queue; allocated buffers;
   recorded CPU operations; submission and an explicit completion contract.
2. Phase 2: framebuffer, vertex fetch, triangle assembly and rasterization,
   constant fragment colour, image output through EX's own software backend.
3. Phase 3: minimal internal shader representation, vertex/fragment execution,
   pipeline objects, attributes, interpolation, and indexed drawing.
4. Phase 4: textures, depth/stencil, blending, compute, binding, richer
   synchronization, multiple queues, and parallel recording, added progressively.

Hardware backends are future work. Do not implement empty vendor modules or
promise hardware support without working driver integration.

Read the [starting assessment](docs/ASSESSMENT.md),
[architecture](docs/ARCHITECTURE.md), [current API contract](docs/API-DRAFT.md),
and [coding conventions](docs/CONTRIBUTING.md).
