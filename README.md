# Expanse-X

Expanse-X (EX) is a graphics API written in Rust, with its own command model,
shader interpreter and CPU rasterizer. The current backend renders offscreen
images and executes compute work synchronously.

The explicit core is working. GPU acceleration, window presentation and a managed
API are planned. The API is provisional; see the [roadmap](docs/ROADMAP.md) for
what is implemented and what comes next.

## Try the SDK

Download the Windows x64 ZIP and `SHA256SUMS.txt` from the
[v0.1.0 Preview release](https://github.com/yugcore/expanse-x/releases/tag/v0.1.0).
Check the ZIP's SHA-256 against the checksum file, then extract it into a directory
you can keep in place. The archive includes `ex.exe` and its source dependencies.

You will need:

- Stable Rust, with rustfmt and Clippy.
- The Windows MSVC build and linking tools.
- `glslangValidator` and `spirv-val` on PATH. You can also set `EX_GLSLANG` and
  `EX_SPIRV_VAL` to their executable paths.

From the extracted SDK directory in PowerShell:

```powershell
.\bin\ex.exe doctor
.\bin\ex.exe new my-ex-app --name my-ex-app
Set-Location my-ex-app
cargo run --locked --offline
```

The app writes `target/ex-starter.png`. Edit `shaders/triangle.vert` or
`shaders/triangle.frag` to change its shaders, or `src/renderer.rs` to change the
resources and draw commands. Generated manifests use absolute paths to the SDK;
update both runtime and build dependencies if you move it.

The shader importer accepts a [limited GLSL subset](docs/SHADER-IMPORT.md):
straight-line vertex and fragment programs using float/vec4 values, varyings and
uniform blocks. Native tools compile and validate SPIR-V at build time; EX
interprets the resulting EX instructions when rendering. Core builds do not
require the shader tools.

For the same workflow from a source checkout:

```text
cargo run -p ex-sdk --locked --offline -- doctor
cargo run -p ex-sdk --locked --offline -- new target/my-ex-app --name my-ex-app
```

Then run `cargo run --locked --offline` from `target/my-ex-app`.
[SDK.md](docs/SDK.md) covers the CLI, setup and error handling.

## What works

- Device discovery, owned buffers and images, CPU access permissions and transfers.
- Command recording, synchronous submission, retained resources and explicit
  whole-resource dependencies.
- Vertex and fragment programs, vertex layouts, perspective interpolation and
  u16/u32 indexed drawing.
- RGBA8 textures, rectangular views, nearest/bilinear sampling, depth/stencil
  attachments and colour blending.
- Compute dispatches, storage buffers, uniform parameters and reusable binding
  layouts/groups.
- SDK project generation, shader import/build/reflection and Cargo incremental
  shader rebuilds.

[Core Preview 1](docs/CORE-PROFILE.md) defines the supported limits. There is one
synchronous queue per device. Richer formats, mipmaps, asynchronous completion,
multiple queues and hardware drivers are future work. There is no selected
project license yet; Cargo registry publication is disabled.

## Examples

Run an example with:

```text
cargo run -p ex-api --example core_preview --locked --offline
```

`core_preview` uploads an image, computes indexed geometry, renders two passes
with depth/stencil and blending, then checks readback and failure handling.
It writes `target/core-preview.png`. The integration test uses the same workload.

| Example | What it demonstrates |
| --- | --- |
| `discover` | Device and queue ownership |
| `buffer_roundtrip` | CPU upload/readback and access permissions |
| `record_copy`, `submit_copy` | Recording versus execution, copy ordering and readback |
| `software_triangle` | Fixed-function triangle rendering |
| `shader_evaluation` | Individual shader invocations |
| `programmable_triangle`, `indexed_quad` | Vertex/fragment shaders, interpolation and indices |
| `textured_quad` | Nearest and bilinear sampling |
| `depth_overlap`, `blended_triangles`, `stencil_mask` | Attachment tests and colour blending |
| `compute_tiles` | Compute-generated geometry |
| `explicit_dependencies` | Transitions between copy, compute and vertex reads |
| `shared_parameters` | One binding group shared across shader stages |
| `texture_flow` | Upload, render-to-texture, sampling and padded readback |
| `api_draft` | Original descriptor and label-recording prototype |

Rendering examples write PNG files under `target/`; pass another output path
after `--` where supported. PNG encoding belongs to the examples, while EX
produces the pixels.

## Build and check

The workspace uses the Rust standard library and local crates. Once Rust and
the native shader tools are installed, the checks run offline:

```text
cargo fmt --all -- --check
cargo build --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
cargo doc --workspace --no-deps --locked --offline
```

`scripts/check.ps1` runs these in order. The workspace currently has 272 tests,
including executable and compile-fail documentation. SDK integration tests
invoke the native shader tools; core-only tests do not.

## Project layout

| Path | Contents |
| --- | --- |
| `crates/ex-api/` | Public crate, imported as `ex`, with examples and integration tests |
| `crates/expansex-core/` | Resources, validation, execution and the software renderer |
| `crates/ex-sdk/` | CLI and application templates |
| `crates/ex-shader/` | Native shader tools, SPIR-V lowering and reflection |
| `api.md` | Public API catalog |
| `docs/` | Feature contracts, architecture, SDK instructions and roadmap |
| `scripts/` | Checks and SDK packaging |

EX's [design](DOCUMENTATION.md) calls for two control levels over the same core:
explicit commands and dependencies for applications that need control, and a
managed layer that derives execution plans. Only the explicit core is implemented
today. Both are intended to share resources and execution rules.

Start with the [API catalog](api.md), [architecture](docs/ARCHITECTURE.md) or
[contributing guide](docs/CONTRIBUTING.md). The [release guide](docs/RELEASING.md)
describes how SDK downloads are built and checked.
