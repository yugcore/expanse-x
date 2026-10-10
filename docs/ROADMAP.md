# Expanse-X roadmap

Updated: 2026-10-10.

Core Preview 1 is complete for the synchronous software backend. The SDK has a
working source workflow and a [v0.1.0 Windows preview](releases/v0.1.0.md).
The next SDK task is safe shader reload. A stable v1 API is a later goal.

EX follows the [shared-core design](../DOCUMENTATION.md): develop the explicit
API first, then build managed planning on the same resources and execution rules.

## Completed foundations

| Phase | Delivered | Details |
| --- | --- | --- |
| 0: Workspace | Public `ex` crate, build settings, checks and project documentation | [Architecture](ARCHITECTURE.md) |
| 1: Resources and commands | Software discovery, device/queue ownership, buffers, recorded copies and synchronous submission | [Objects](OBJECT-MODEL.md), [buffers](BUFFERS.md), [commands](COMMANDS.md), [submission](SUBMISSION.md) |
| 2: First triangle | Framebuffers, render scopes, fixed-function vertex fetch and CPU rasterization | [Rendering](RENDERING.md) |
| 3: Programmability | EX shader instructions, graphics pipelines, vertex layouts, interpolation and indexed draws | [Shaders](SHADERS.md), [pipelines](PIPELINES.md), [indices](INDEXED-DRAWING.md) |

The `core_preview` example and test combine these features with compute, textures
and two-pass rendering. They verify readback, resource retention, missing
dependencies, partial failure and reuse after repair. See
[CORE-PROOF.md](CORE-PROOF.md) for the workload and recorded results.

## Phase 4: resources and execution

The following features are implemented in the software backend:

- [x] RGBA8 textures, rectangular views, nearest/bilinear sampling and eight
  fragment texture/sampler slots. [Contract](TEXTURES.md)
- [x] Depth/stencil attachments, comparisons and updates, channel masks and
  colour blending. [Depth](DEPTH.md), [stencil](STENCIL.md), [blending](BLENDING.md)
- [x] Compute pipelines, invocation IDs, workgroups, float4 storage access and
  atomic dispatch output. [Contract](COMPUTE.md)
- [x] Reusable single-group layouts, bindings and uniform parameters shared by
  graphics and compute. [Contract](BINDINGS.md)
- [x] Explicit whole-resource transitions and dependency validation for copies,
  compute and rendering. [Contract](DEPENDENCIES.md)
- [x] Padded buffer/image transfers, rectangular image copies and shared
  render-to-texture attachments. [Contract](TEXTURE-FLOW.md)
- [x] A supported core profile, queryable limits, a private execution backend
  boundary and reusable adapter tests. [Contract](CORE-PROFILE.md)

Phase 4 still has work beyond the first core profile:

- [ ] Asynchronous completion, fences/timelines, multiple queues and cross-queue
  dependencies.
- [ ] Multithreaded recording, indirect operations and measured allocator/command
  reuse.
- [ ] Presentation and surfaces for an interactive example.
- [ ] Richer memory placement, staging, alignment, mapping and residency.
- [ ] Mipmaps, storage images, more formats, multiple binding groups and dynamic
  bindings.
- [ ] More precise range/stage dependencies and broader shader profiles.
- [ ] Managed execution planning with inspectable decisions and interoperability
  with explicit work.

These features need workloads and tests before they become part of a supported
profile. They are not prerequisites for the existing Core Preview 1.

## SDK Preview

The source SDK uses the explicit software core. Its CLI checks prerequisites,
reports runtime limits and creates standalone Rust/GLSL applications.
[SDK.md](SDK.md) lists commands and [SHADER-IMPORT.md](SHADER-IMPORT.md) defines
the supported shader subset.

- [x] `info`, `doctor` and `new`, including setup errors, destination preservation
  and generated application builds in paths with spaces.
- [x] GLSL compilation, SPIR-V validation, bounded lowering to EX instructions,
  Rust generation and JSON reflection.
- [x] Straight-line float/vec4 vertex/fragment programs, varyings and static
  set-zero uniform blocks.
- [x] Cargo incremental rebuilds when shader inputs or selected tools change.
- [x] Windows x64 SDK archive with `ex.exe`, matching source, provenance and a
  ZIP checksum. [Packaging](RELEASING.md)
- [ ] Safe shader reload: keep the last working pipeline after a failed rebuild
  and check interface compatibility before replacing it.
- [ ] Broader imports and an independent shader artifact cache.
- [ ] Asset helpers and window/engine examples.
- [ ] Diagnostic and capture tools.
- [x] MIT license and contribution terms.
- [ ] Additional platform packages and a release toolchain policy.

The workspace has 272 tests, including SDK CLI tests, importer tests and executable
documentation. The v0.1.0 archive was extracted outside the development checkout;
its generated app built offline and all 4096 output pixels matched the triangle
oracle. The tested binary target is Windows x64. The SDK remains a preview.

## EX Witness: proposed experiment

[Witness](../DOCUMENTATION.md#ex-witness-experimental-differentiation) would turn
a selected failure into an explanation, replay and smaller runnable test.
Start with a buffer-copy validation error or a readback assertion:

- [ ] Define the failure condition and capture point.
- [ ] Capture resource descriptions, starting contents, commands and dependencies.
- [ ] Export and verify a replay without the original application.
- [ ] Reduce the case while preserving the chosen failure.
- [ ] Turn a repaired reproduction into a regression test.
- [ ] Measure capture/replay costs and document the supported scope.

Pixel explanations, shader tracing and frame reduction can follow a successful
copy experiment. Witness is not implemented.

## EX Proof: proposed experiment

[Proof](../DOCUMENTATION.md#ex-proof-experimental-execution-certificates) would
attach a checkable certificate to a plan's resource-access obligations:

- [ ] Define a narrow buffer-copy model, obligations and assumptions.
- [ ] Bind a generated certificate to the exact plan and resource parameters.
- [ ] Verify it with a separate checker without executing copies.
- [ ] Reject missing dependencies, corruption and certificates reused against
  changed plans. Report unsupported inputs separately.
- [ ] Return structured results, with optional console or exported reports.
- [ ] Document the trusted rules, runtime enforcement and checking costs.

A certificate would cover only its stated model and assumptions. Shader and
hardware proofs are later research. Proof is not implemented and can be explored
separately from Witness.

## Hardware backends

Hardware support is deferred. A target needs a viable driver interface, memory
model, command stream and shader toolchain, then tests against EX's supported
profile. No vendor backend exists yet. EX's current rendering does not delegate
execution to Vulkan, Direct3D, Metal, OpenGL or wgpu.

Future work has no committed release dates. For each feature, update its contract,
public API catalog and this roadmap after the implementation and checks pass.
