# Expanse-X development roadmap

Updated: 2026-10-08 (Asia/Calcutta).

This roadmap tracks EX implementation and remaining work.

EX defines its own Rust API, runtime, resource/command semantics, and execution.
The first target is EX's own software/reference GPU. Rendering must not be
delegated to Vulkan, Direct3D, Metal, OpenGL, wgpu, or another graphics API.
Hardware-driver development remains deferred.

The [project design](../DOCUMENTATION.md) governs this roadmap: managed and
explicit control levels share one core, resources, and execution rules. Build
and verify the explicit core first. Add managed planning over those contracts
when a working workload justifies it; this is a future capability, not a second
renderer or a reason to postpone the next recorded-copy/submission milestone.

## Status at a glance

| Phase | Status | Completion evidence |
| --- | --- | --- |
| 0 — Repository foundation | Complete | Production workspace, public `ex` entry point, checks, contracts, and local Git history |
| 1 — Minimum runtime | In progress: discovery and owned buffers work | Pending: recorded copies, queue execution, and completion semantics |
| 2 — First software triangle | Not started | Pending: public draw commands produce a triangle image through EX's own rasterizer |
| 3 — Programmability and pipeline milestone | Not started | Pending: EX shader representation and public pipeline produce a programmable triangle |
| 4 — Modern GPU concepts | Deferred until earlier phases work | Per-feature implementation, validation, and executable evidence |
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

## Phase 1 — Minimum API and runtime: in progress

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

### 1.3 Recorded buffer copies: next

- [ ] Define a device-bound encoder and an EX-owned executable command representation.
- [ ] Record a buffer-to-buffer copy independently of execution.
- [ ] Retain source/destination allocations after application handles drop.
- [ ] Validate `TransferSource`, `TransferDestination`, and common logical device ownership.
- [ ] Validate byte ranges, arithmetic overflow, and aliased/overlapping copy policy.
- [ ] Make finishing consume the encoder and produce immutable finished work.
- [ ] Demonstrate that recording/finishing alone leaves destination bytes unchanged.
- [ ] Document how source contents are observed when the recorded operation executes.

The current standalone `CommandRecorder` records labels only. Its `CommandBuffer`
cannot be submitted. Decide how to evolve that prototype when executable commands
are implemented; do not treat the existing names as completed execution support.

### 1.4 Queue submission and basic synchronization: pending

- [ ] Submit finished work to its owning queue and reject a foreign device's commands.
- [ ] Execute recorded copies in order through EX's reference implementation.
- [ ] Establish synchronous reference submission: successful return means copies
      have finished and their results are visible to subsequent permitted CPU reads.
- [ ] Define validation/execution failure behavior, including whether earlier
      commands may already have executed when a later operation fails.
- [ ] Define concurrent submissions/CPU access and deadlock-safe resource locking.
- [ ] Test ordered copy chains, retained resource lifetime, ownership/usage/range errors,
      and the documented completion/failure behavior.
- [ ] Add the public record-submit-readback example and current command/sync contracts.

The intended next flow is upload → record copy → verify unchanged destination →
finish → submit → read back matching bytes. Encoder/submission methods are planned,
not available today. CPU-copy mutexes do not already provide this queue contract.

**Phase 1 completion gate:** this flow runs through `ex`, misuse is reported clearly,
and order, visibility, error behavior, and resource retention are documented and
tested. A synchronous reference contract is the starting point; future asynchronous
execution must introduce explicit completion without losing these semantics.

## Phase 2 — First reference triangle: not started

- [ ] Add a framebuffer/render-target resource with dimensions and format validation.
- [ ] Record a minimal render scope, clear, vertex-buffer binding, and triangle draw.
- [ ] Define the fixed vertex layout and validate vertex fetch and draw ranges.
- [ ] Implement vertex fetch, primitive assembly, viewport mapping, and triangle rasterization.
- [ ] Define supported clipping, winding, edge coverage, and degenerate-triangle behavior.
- [ ] Write constant fragment colour into the framebuffer.
- [ ] Export an offscreen image through an EX example; a window/swapchain is unnecessary here.
- [ ] Test representative pixel coverage, clear/background preservation, and invalid draw state.
- [ ] Separate working reference execution code when its responsibilities justify a backend boundary.

**Phase 2 completion gate:** EX's public API records/submits a draw and EX's own
software pipeline produces a verifiable triangle image. Document supported
coordinates and clipping limits rather than claiming a complete GPU pipeline.
This is the first visual checkpoint; programmable shaders and public pipeline
objects in the original conceptual success program are completed in Phase 3.

## Phase 3 — Programmability and pipelines: not started

- [ ] Define a small EX-owned shader representation, stage interfaces, and invalid-program errors.
- [ ] Interpret minimal vertex and fragment operations in the reference renderer.
- [ ] Introduce actual graphics pipeline objects and the binding/layout contract they require.
- [ ] Add explicit vertex attributes and interpolation with validated stage compatibility.
- [ ] Add indexed drawing and index-buffer usage/range validation.
- [ ] Exercise pipeline/vertex binding and programmable drawing through a public triangle example.
- [ ] Test shader behavior, interpolation, indexed draws, and incompatible pipeline/input errors.

**First major graphics milestone:** the application creates a device, vertices,
and graphics pipeline; records render scope, bindings, and draw; submits through
EX; and receives a triangle image produced entirely by EX's reference pipeline.
This meets the original pipeline-based success criterion. A shader frontend/full
compiler, external shader-format dependency, or real GPU is not required.

## Phase 4 — Modern GPU concepts: deferred

Add one capability at a time after the earlier contracts work. Select the order
by an actual example's needs rather than implementing every object immediately.

- [ ] Textures, views, samplers, and texture/resource binding.
- [ ] Depth/stencil and blending.
- [ ] Compute pipelines and dispatch.
- [ ] Explicit resource transitions, read/write hazards, and execution/memory dependencies.
- [ ] Asynchronous completion, fences/timelines, multiple queues, and cross-queue dependencies.
- [ ] Multithreaded recording, indirect operations, and measured allocator/command reuse.
- [ ] Presentation/surface support when an interactive example requires it.
- [ ] Richer host/device memory placement, staging, alignment, mapping, and residency as required.
- [ ] Managed planning over proven explicit contracts, with inspectable decisions
      and tested interoperability using the same resources and completion rules.

Each feature needs a chosen EX contract, a working reference implementation,
misuse validation, executable evidence, and updated documentation before its box
is checked. Performance or hardware-conformance claims require separate evidence.

## EX Witness — experimental track: not started

The [EX Witness direction](../DOCUMENTATION.md#ex-witness-experimental-differentiation)
explores explanation → replay → smaller executable tests for selected failures.
It uses the same contracts for managed and explicit work. This track adds no new
Phase 1 completion requirement; recorded copies and submission remain next.

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
