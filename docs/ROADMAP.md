# Expanse-X development roadmap

Updated: 2026-10-10 (Asia/Calcutta).

This roadmap tracks EX implementation and remaining work.

EX defines its own Rust API, runtime, resource/command semantics, and execution.
The first target is EX's own software/reference GPU. Rendering must not be
delegated to Vulkan, Direct3D, Metal, OpenGL, wgpu, or another graphics API.
Hardware-driver development remains deferred.

The [project design](../DOCUMENTATION.md) governs this roadmap: managed and
explicit control levels share one core, resources, and execution rules. Build
and verify the explicit core first. Add managed planning over those contracts
when a working workload justifies it; this is a future capability, not a second
renderer or a reason to postpone the programmable rendering milestone.

The bounded [Core Preview](CORE-MILESTONE.md) is complete. SDK Preview is in
progress; a complete/stable v1 API is a later goal. Build new SDK tooling
around this verified explicit reference core; retain the broader
Phase 4 feature roadmap without treating every future feature as a core-launch gate.

## Status at a glance

| Phase | Status | Completion evidence |
| --- | --- | --- |
| 0 — Repository foundation | Complete | Production workspace, public `ex` entry point, checks, contracts, and local Git history |
| 1 — Minimum runtime | Complete | Ordered synchronous copy execution, readback, concurrency/failure contract, public example, and tests |
| 2 — First software triangle | Complete | Public render scopes produce a verified PNG through EX's own clipped fixed-function rasterizer |
| 3 — Programmability and pipeline milestone | Complete | Pipeline-controlled triangles, layouts, perspective interpolation, and validated u16/u32 indexed drawing |
| 4 — Modern GPU concepts | In progress | Sampled RGBA8 textures/views, nearest/bilinear samplers, eight fragment resource slots, depth/stencil attachments/tests, blending, and storage-buffer compute dispatch, reusable bindings/uniforms, checked whole-resource dependencies, and multi-pass image flows |
| Core Preview 1 | Complete | Bounded software profile, selected executor, reusable profile corpus and maintained integrated proof in CORE-PROOF.md; 257 tests and all project checks pass |
| SDK Preview | In progress | Source workflow and bounded EX-GLSL-1 shader build/import/reflection in SDK.md/SHADER-IMPORT.md; standalone GLSL/Rust starter, Cargo incremental builds and 272 passing tests; reload and broader integrations remain pending |
| Future — Hardware drivers | Deferred | A selected target, viable driver interfaces, and demonstrated execution |

Checked items mean implemented and verified behavior.
An example, a type name, or a downloaded reference alone does not complete a task.

## Phase 0 — Repository foundation: complete

- [x] Inspect and preserve the existing buffer-description/label-recording prototype.

- [x] Establish a production-only Rust workspace and public `ex-api` entry point (`use ex::...`).

- [x] Track the lockfile and configure formatting, shared lints, and safe-Rust defaults.

- [x] Provide CI-friendly build/lint/test/documentation commands and a local check script.

- [x] Document the architecture, current contracts, assessment, and coding conventions.

- [x] Initialize local Git history and commit coherent implementation steps.

Evidence: [assessment](ASSESSMENT.md), [workspace](../Cargo.toml),
[check script](../scripts/check.ps1), and [conventions](CONTRIBUTING.md).
There is no hosted CI workflow in this repository; CI-friendly commands do not imply one.
This phase is repository foundation, not completion of the archived Rust syllabus.

## Phase 1 — Minimum API and runtime: complete

### 1.1 Discovery and logical ownership: complete

- [x] Implement `Instance → Adapter → Device → Queue` for the named software target.
- [x] Distinguish device creation from handle cloning and validate identity/membership queries.
- [x] Retain child state after original parent handles drop and release it after the final owner.
- [x] Add a public example, lifecycle tests, and an ownership contract.

Evidence: [object contract](OBJECT-MODEL.md),
[discovery example](../crates/ex-api/examples/discover.rs), and
[public object tests](../crates/ex-api/tests/object_model.rs).
Commit: `e76168e` — `add software discovery and device ownership`.

### 1.2 Owned buffers and explicit CPU access: complete

- [x] Allocate fixed, zero-initialized reference byte storage through `Device::create_buffer`.
- [x] Own metadata, retain the creating device, and share allocation identity through clones.
- [x] Separate `HostAccess` permissions from device-side `BufferUsage` roles.
- [x] Implement synchronous CPU upload/readback with full-range and overflow checks.
- [x] Report invalid descriptors, impossible capacity, reservation failures, forbidden access,
      invalid ranges, and poisoned storage with understandable errors.
- [x] Test metadata lifetime, permissions, range safety, shared storage, and cleanup.

Evidence: [buffer contract](BUFFERS.md),
[round-trip example](../crates/ex-api/examples/buffer_roundtrip.rs), and
[public buffer tests](../crates/ex-api/tests/buffer_storage.rs).
Commit: `48135be` — `allocate owned buffers and check CPU access`.
The buffer milestone passed 28 workspace tests, including documentation tests.
Tests reject impossible capacity without exhausting RAM; actual allocator OOM
recovery is not claimed as verified. Heaps, mapping, residency, and device-local
hardware storage are not implemented.

### 1.3 Recorded buffer copies: complete

- [x] Define a device-bound encoder and an EX-owned executable command representation.
- [x] Record a buffer-to-buffer copy independently of execution.
- [x] Retain source/destination allocations after application handles drop.
- [x] Validate `TransferSource`, `TransferDestination`, and common logical device ownership.
- [x] Validate byte ranges, arithmetic overflow, and aliased/overlapping copy policy.
- [x] Make finishing consume the encoder and produce immutable finished work.
- [x] Demonstrate that recording/finishing alone leaves destination bytes unchanged.
- [x] Document live resource references and the planned execution-time source sampling.

Evidence: [recording contract](COMMANDS.md),
[public example](../crates/ex-api/examples/record_copy.rs), and
[recording tests](../crates/ex-api/tests/command_recording.rs). The workspace
passed 43 tests at the recording milestone, including documentation tests.
Reservation failures are handled but tests do not exhaust RAM to induce OOM.
Execution-time source sampling is now implemented and tested in step 1.4.

The standalone `CommandRecorder` still records labels only and now returns
`DebugMarkerBuffer`; `CommandEncoder::finish()` produces the actual device-bound
`CommandBuffer`. Step 1.4 submits the device-bound result; labels remain separate.

### 1.4 Queue submission and basic synchronization: complete

- [x] Submit finished work to its owning queue and reject a foreign device's commands.
- [x] Execute recorded copies in order through EX's reference implementation.
- [x] Establish synchronous reference submission: successful return means copies
      have finished and their results are visible to subsequent permitted CPU reads.
- [x] Define validation/execution failure behavior, including whether earlier
      commands may already have executed when a later operation fails.
- [x] Define concurrent submissions/CPU access and deadlock-safe resource locking.
- [x] Test ordered copy chains, retained resource lifetime, ownership/usage/range errors,
      and the documented completion/failure behavior.
- [x] Add the public record-submit-readback example and current command/sync contracts.

The complete flow is upload → record copy → verify unchanged destination →
finish → submit → read back matching bytes. Borrowed command buffers are reusable;
each submission samples live source bytes. Device gates serialize whole submissions
against CPU access. Failure preserves completed commands and skips later work.

Evidence: [submission contract](SUBMISSION.md),
[public example](../crates/ex-api/examples/submit_copy.rs),
[submission tests](../crates/ex-api/tests/queue_submission.rs), and internal
poisoning/lifetime/gate checks. The submission milestone passed 57 tests, including
documentation tests. This does not claim hardware or asynchronous execution.

**Phase 1 completion gate: met.** This flow runs through `ex`, misuse is reported clearly,
and order, visibility, error behavior, and resource retention are documented and
tested. A synchronous reference contract is the starting point; future asynchronous
execution must introduce explicit completion without losing these semantics.

## Phase 2 — First reference triangle: complete

- [x] Add a framebuffer/render-target resource with dimensions and format validation.
- [x] Record a minimal render scope, clear, vertex-buffer binding, and triangle draw.
- [x] Define the fixed vertex layout and validate vertex fetch and draw ranges.
- [x] Implement vertex fetch, primitive assembly, viewport mapping, and triangle rasterization.
- [x] Define supported clipping, winding, edge coverage, and degenerate-triangle behavior.
- [x] Write constant fragment colour into the framebuffer.
- [x] Export an offscreen image through an EX example; a window/swapchain is unnecessary here.
- [x] Test representative pixel coverage, clear/background preservation, and invalid draw state.
- [x] Separate working reference execution code when its responsibilities justify a backend boundary.

**Phase 2 completion gate: met.** EX's public API records/submits a draw and EX's own
software pipeline produces a verifiable triangle image. Document supported
coordinates and clipping limits rather than claiming a complete GPU pipeline.
This is the first visual checkpoint; programmable shaders and public pipeline
objects in the original conceptual success program are completed in Phase 3.

Evidence: [rendering contract](RENDERING.md), [private CPU executor](../crates/expansex-core/src/reference.rs),
[public triangle example](../crates/ex-api/examples/software_triangle.rs), and
[rendering tests](../crates/ex-api/tests/reference_rendering.rs). The example writes
a 512x384 PNG with 48,556 foreground pixels; centre/corner assertions and independent
PNG decoding verify the output. The Phase 2 milestone passed 79 tests, including
executable and compile-fail documentation. Floating clipping and 1/256-pixel
coverage rules are documented. Programmable drawing followed in Phase 3;
depth and blending have since arrived in Phase 4; presentation and hardware remain future work.

## Phase 3 — Programmability and pipelines: complete

- [x] Define a small EX-owned shader representation, stage interfaces, and invalid-program errors.
- [x] Interpret minimal vertex and fragment operations in the reference renderer.
- [x] Introduce actual graphics pipeline objects and the binding/layout contract they require.
- [x] Add explicit vertex attributes and interpolation with validated stage compatibility.
- [x] Add indexed drawing and index-buffer usage/range validation.
- [x] Exercise pipeline/vertex binding and programmable drawing through a public triangle example.
- [x] Test shader behavior, interpolation, and incompatible pipeline/input errors.
- [x] Test indexed draw execution, index ranges/usages, and invalid indices.

Shader foundation: `Shader::evaluate` already interprets Input, Constant, Add,
Multiply, Dot4, and Swizzle for independent vertex/fragment invocations. It
validates acyclic register references and bounded vector interfaces, owns source,
and reports invalid inputs and non-finite arithmetic.
Evidence: [shader contract](SHADERS.md), [implementation](../crates/expansex-core/src/shader.rs),
[public example](../crates/ex-api/examples/shader_evaluation.rs), and
[nine public shader tests](../crates/ex-api/tests/shader_evaluation.rs).
The shader foundation milestone passed 91 workspace tests, including documentation.

Pipeline integration now links stages, owns validated single-buffer layouts,
fetches expanded f32 attributes, executes vertex/fragment operations, clips
homogeneous positions, and interpolates varyings with perspective correction.
Scope output commits only after complete shading, preserving target pixels on
late fragment failure. The [pipeline contract](PIPELINES.md),
[public example](../crates/ex-api/examples/programmable_triangle.rs), and
[seventeen public rendering tests](../crates/ex-api/tests/programmable_rendering.rs)
provide evidence. The 512x384 PNG contains 47,794 shaded pixels and 36,374 distinct
foreground colours; independent decoding and pixel calculations verified the
entire image. The pipeline integration checkpoint passed 110 tests including documentation.

Indexed drawing completes Phase 3: u16/u32 elements, signed base vertices,
binding offsets, retained sources, and live-reference validation now work.
[The contract](INDEXED-DRAWING.md), [ten public tests](../crates/ex-api/tests/indexed_rendering.rs),
two internal poison/lifetime tests, and the [indexed quad](../crates/ex-api/examples/indexed_quad.rs)
verify these semantics. Four vertices (80 bytes) and six u16 indices (12 bytes)
produce a 512x384 PNG with 110,592 shaded pixels and 80,237 foreground colours.
Independent PNG checksums, coverage/interpolation calculations, and visual
inspection verify the output. The Phase 3 checkpoint passed 122 tests.

**Phase 3 completion gate: met.** Direct and indexed programmable triangles execute
through the public API with validated state, layouts, stage interfaces, and live
references. Phase 4 has since started with textures, samplers, and resource bindings.
External source-language/format ingestion remains future
work; [the accepted direction](SHADERS.md#source-language-direction-accepted)
uses established toolchains rather than committing to a new source language.

**First major graphics milestone: met.** The application creates a device, vertices,
and graphics pipeline; records render scope, bindings, and draw; submits through
EX; and receives a triangle image produced entirely by EX's reference pipeline.
This meets the original pipeline-based success criterion. A shader frontend/full
compiler, external shader-format dependency, or real GPU is not required.

## Phase 4 — Modern GPU concepts: in progress

Add one capability at a time after the earlier contracts work. Select the order
by an actual example's needs rather than implementing every object immediately.
The [core milestone](CORE-MILESTONE.md) defines the immediate completion boundary;
the checklist below is the wider feature backlog. SDK development follows core completion.

- [x] Sampled 2D RGBA8 textures, rectangular views, nearest/bilinear samplers, and eight fragment texture/sampler slots.
- [x] Depth/stencil and blending - separate attachments, comparisons/updates, channel masks, and atomic commits.
- [x] Compute pipelines and dispatch - bounded float4 storage access, invocation IDs, workgroups, and atomic outputs.
- [x] Reusable single-group shader layouts/bindings and uniform parameter buffers under [BINDINGS.md](BINDINGS.md); broader resource profiles remain future work.
- [x] Explicit resource transitions, read/write hazards, and execution/memory dependencies for synchronous whole-resource work ([contract](DEPENDENCIES.md)); range/stage/async precision remains future work.
- [x] Device buffer/image transfers, rectangular image copies, and shared full-image render-to-texture attachments under [TEXTURE-FLOW.md](TEXTURE-FLOW.md).
- [ ] Asynchronous completion, fences/timelines, multiple queues, and cross-queue dependencies.
- [ ] Multithreaded recording, indirect operations, and measured allocator/command reuse.
- [ ] Presentation/surface support when an interactive example requires it.
- [ ] Richer host/device memory placement, staging, alignment, mapping, and residency as required.
- [ ] Managed planning over proven explicit contracts, with inspectable decisions
      and tested interoperability using the same resources and completion rules.

The first capability is complete under [the texture contract](TEXTURES.md):
explicit CPU upload/readback, retained view/sampler pairs, fragment SampleTexture,
view-local clamp/repeat, perspective-correct UVs, and atomic scope failure.
[Twelve public tests](../crates/ex-api/tests/textures.rs), three internal
poison/lifetime/gate tests, and executable public documentation verify the path.
The [checkerboard example](../crates/ex-api/examples/textured_quad.rs) renders
nearest (left) and bilinear (right) repeat sampling. Independent PNG checksums
and coverage/UV/filter calculations verify all 196,608 pixels, including 119,808
covered samples; the image was also visually inspected. The texture checkpoint passed 137 workspace tests.

The depth component now works under [DEPTH.md](DEPTH.md): separate f32 attachments,
eight comparisons, optional depth writes, clears/preservation, screen-linear
projected depth, reverse-Z, and atomic colour/depth commits. [Eleven public tests](../crates/ex-api/tests/depth.rs)
and two internal poison/lifetime tests verify the contract. The [overlap example](../crates/ex-api/examples/depth_overlap.rs)
renders three triangles identically in both orders, with 85,434 covered pixels.
Independent PNG checksum/coverage/nearest-surface calculations verify all 196,608
pixels; the image was visually inspected. The depth checkpoint passed 150 workspace tests.

The blending component works under [BLENDING.md](BLENDING.md): separate RGB/alpha
equations, eleven factors, five operations, all channel write masks, and alpha,
premultiplied, and additive presets. Unified immutable pipeline options combine
depth/blend state without changing existing descriptor literals. Twelve public
tests and executable documentation verify arithmetic, ordering, textures/indices,
depth, compatibility, and rollback. The [comparison example](../crates/ex-api/examples/blended_triangles.rs)
renders identical triangles in three panels; independent PNG checksum, coverage,
and rational blend calculations verify all 294,912 pixels, including 128,031
covered pixels. The image was visually inspected. The blending checkpoint passed 163 workspace tests.

Stencil works under [STENCIL.md](STENCIL.md): S8 storage, masked comparisons,
eight operations, front/back rules, stencil/depth failure branches, and atomic
colour/depth/stencil commits. Existing pipeline/option literals remain compatible.
Twelve public tests, two internal poison/lifetime tests, and combined public
documentation verify the path. The [diamond-mask example](../crates/ex-api/examples/stencil_mask.rs)
verifies all three attachments; independent PNG checksum, geometry, sampling, and
blend calculations match all 196,608 pixels, including 66,560 masked pixels.
The image was inspected. The stencil checkpoint passed 177 workspace tests. The depth/stencil/blend
group is complete under the current reference contracts, while Phase 4 remains in progress.

Compute works under [COMPUTE.md](COMPUTE.md): immutable local size/output slots,
3D dispatch grids, four built-in input vectors, float4 storage loads/outputs,
in-place transforms, live references, checked aliases, and atomic output commits.
[Twelve public tests](../crates/ex-api/tests/compute.rs), two internal poison/lifetime
tests, and executable documentation verify misuse, all IDs, eight outputs,
zero/limit cases, copy ordering, reuse, device-only storage, shader rollback,
and compute-to-graphics visibility. The [tile example](../crates/ex-api/examples/compute_tiles.rs)
verifies 288 computed vertices and 48 tile centres. Independent PNG checksums,
float32 geometry, and coverage calculations match all 196,608 pixels, including
101,568 covered pixels; the image was visually inspected. The compute checkpoint passed 192 workspace tests.

Explicit dependencies work under [DEPENDENCIES.md](DEPENDENCIES.md): retained
resource roles, command-buffer-local state, read/write hazard validation, and full
dependency points. Fourteen public tests, two internal poison/lifetime tests,
and executable documentation cover all command families and failure boundaries.
The [explicit tile workload](../crates/ex-api/examples/explicit_dependencies.rs)
uses upload, compute, vertex fetch and readback transitions; it verifies 288 vertices,
48 tile centres, and 101,568 covered pixels. An independent full-image check matches
all 196,608 pixels; the image was inspected. The dependency checkpoint passed 209 workspace tests.

Reusable interfaces work under [BINDINGS.md](BINDINGS.md): structurally compatible
layouts, immutable retained groups, stage/type/size validation, uniform float4
loads in every shader stage, paired textures and compute storage. Fifteen public
tests, two internal poison/lifetime tests and executable documentation verify the
profile. The [shared parameter example](../crates/ex-api/examples/shared_parameters.rs)
uses one group in compute, vertex and fragment execution with explicit dependencies.
It verifies 288 generated vertices and 48 tile centres, then reuses unchanged commands
with new material bytes. Independent coverage/checksum calculations match every
pixel in both 512x384 images. All 227 tests and project checks pass.

Texture/resource flow now works under [TEXTURE-FLOW.md](TEXTURE-FLOW.md): padded
buffer/image transfers, image rectangle copies, full-image shared texture/framebuffer
aliases, host permission preservation, shader feedback rejection and alias-aware
explicit dependencies. Fifteen public tests, two internal poison/lifetime tests,
and executable documentation bring the workspace to 245 passing tests. The
[public workload](../crates/ex-api/examples/texture_flow.rs) combines upload,
rectangle copy, compute-generated geometry, two render passes and padded readback.
Two unchanged-command submissions with different upload data verify every pixel
and padding byte. Independent PNG checks match every final 640x320 pixel, including
131,072 panel pixels; the image was visually inspected.

The execution/profile gate now works under [CORE-PROFILE.md](CORE-PROFILE.md):
CoreProfile::Preview1 and queryable limits shared by validators, a private device-owned
ExecutionBackend, runtime-owned sequencing/gate/error indices, and nine reusable
adapter-parameterized cases. The corpus checks observable profile behavior without
private storage inspection, including eight-output compute IDs/late failure and
multi-pass parameterized image rendering. An internal traced executor verifies
preflight, gate ownership, selected execution, failure stop and cleanup. All 256
workspace tests and project checks pass. The existing image-flow workload executes
through this boundary and matches its independently verified image byte for byte.

The final [integrated proof](CORE-PROOF.md) now passes: the example and normal
integration test share one 15-command, 27-use public workload. It verifies padded
uploads, computed indexed geometry, depth/stencil rejection, blending, two render
passes, every pixel/attachment and padded readback in two reusable successful
submissions. Missing-barrier rejection preserves the full plan's outputs; late
compute overflow preserves its output, publishes the completed prefix and skips
later rendering/readback. Repairing live data reuses the same successful plan.
The final 128x64 PNG passes independent CRC/decompression and all-pixel checks
and visual inspection. All 257 workspace tests and project checks pass.

Core Preview 1 is complete within the declared bounded software profile. SDK
Preview is now in progress. Phase 4 remains incomplete.
Mipmaps, storage images, multiple groups,
dynamic bindings and broader shader profiles require separate contracts.

Each feature needs a chosen EX contract, a working reference implementation,
misuse validation, executable evidence, and updated documentation before its box
is checked. Performance or hardware-conformance claims require separate evidence.

## SDK Preview — source and first shader workflow working

[SDK.md](SDK.md) and [SHADER-IMPORT.md](SHADER-IMPORT.md) define the implemented
source workflow and bounded EX-GLSL-1 import gate. info queries runtime limits;
doctor checks Rust and native shader tools; new generates an independent Rust
starter with editable GLSL, a Cargo build hook, explicit rendering and a lockfile.
Shader build/import commands emit Rust modules and JSON reflection. Compiler
SPIR-V is validated before lowering to EX's existing instructions. Straight-line
float/vec4 graphics and uniform blocks work; unsupported language constructs
fail explicitly. Cargo tracks shader/tool inputs and caches successful builds.

Six real CLI tests cover misuse/cleanup/missing tools, command build/import/
reflection and generated format/build/run in paths with spaces. Shader edits
change pixels; invalid GLSL and changed tool selection stop stale builds; repair
recovers output. Import tests verify math, static uniform layouts, two geometry/
colour frames and every varying gradient pixel, plus malformed/unsupported input
and lowering limits. All 272 tests and required checks pass; graphics API counts
remain unchanged. No release or additional platform certification is claimed.

The first v0.1.0 SDK Preview release packages a tested Windows x86_64 executable
with the matching source snapshot, provenance and ZIP checksum under
[RELEASING.md](RELEASING.md). It uses GitHub Releases, with Cargo registry publishing
still disabled and no project license selected. Extracted SDK setup and an
independent generated application must pass the same rendering checks.

Next: safe shader reload with last-good-state preservation and explicit interface
compatibility checks. Broader imports/independent artifact cache, windowing,
assets, capture, onboarding, license policy and broader platform releases remain
separate SDK gates.

## EX Witness — experimental track: not started

The [EX Witness direction](../DOCUMENTATION.md#ex-witness-experimental-differentiation)
explores explanation → replay → smaller executable tests for selected failures.
It uses the same contracts for managed and explicit work. This track adds no new
Phase 1 completion requirement; the main roadmap continues with texture/resource flows.

After the copy/submission foundation works:

- [ ] Define a supported failure condition and capture point for a copy validation
      error or application-provided readback assertion.
- [ ] Capture the resource descriptions, starting state, operation inputs, and
      dependencies needed for an independent reference replay.
- [ ] Export a standalone case with EX version/environment requirements and verify
      that it reproduces the selected failure without the originating application.
- [ ] Attempt reduction while preserving the target failure; reject candidates
      that only introduce unrelated validation errors.
- [ ] Demonstrate converting the fixed reproduction into a passing regression test.
- [ ] Document tracing/replay overhead and the supported scope and limitations.

Pixel explanations, shader tracing, and frame reduction depend on those features
actually executing in EX. They are later experiments, not implemented diagnostics
or a promise to reproduce hardware-specific bugs or find an absolute minimum.

## EX Proof — experimental track: not started

The [EX Proof direction](../DOCUMENTATION.md#ex-proof-experimental-execution-certificates)
explores independently checkable execution certificates. It shares the
managed/explicit contracts and adds no new Phase 1 completion requirement.

After the copy/submission foundation works:

- [ ] Define a narrow buffer-copy access/dependency model, checked obligations,
      assumptions, and unsupported scope.
- [ ] Generate a certificate for a valid plan with model version, coverage, and
      binding to the plan and relevant resource parameters.
- [ ] Verify it with a separate checker without rendering or executing the copies.
- [ ] Test missing required dependencies, corrupted certificates, and reuse
      against changed plans/resource parameters; distinguish unsupported inputs.
- [ ] Return structured explanations; make console reporting opt-in and document
      export/test consumption without claiming editor integration already exists.
- [ ] Document trusted checker rules, runtime enforcement, and verification cost.

Shader/hardware proofs remain later research. A successful check covers the
stated model and assumptions, not the intended image, hardware behavior, or
optimal performance. Proof and Witness can be explored independently; integration
with failure reproductions follows demonstrated behavior in both experiments.

## Future — Hardware backends: deferred

Hardware integration needs a driver interface, virtual memory model, command
stream and shader ISA. No vendor backend exists at this stage. A hardware target
must implement EX's execution contract.

## Completion discipline

Implement, test and review each feature. Update the roadmap, API catalog
and relevant contracts after checks pass.
