# EX architecture

Status: Phases 0–3 complete. Owned resources, validated recording, synchronous
copies, shaders, graphics pipelines, vertex layouts, interpolation, and indexed
software triangles work. Phase 4 now adds sampled textures/views, samplers, and
fragment resource bindings, plus retained depth attachments and pipeline tests.
Colour blending, front/back stencil rules, and storage-buffer compute dispatch also work.

The [development roadmap](ROADMAP.md) tracks verified work, pending steps, and
phase completion gates. This document describes architectural responsibilities.

## Philosophy and boundaries

The [project design](../DOCUMENTATION.md) governs this architecture.
EX has two intended control levels over one core: managed work derives a valid
plan from resource-use declarations; explicit work supplies a plan that EX
validates. Both share resources, execution rules, and completion semantics.
The explicit recording/submission path now executes copies and offscreen triangles.
Managed planning remains future work over those proven contracts.

EX is an independent graphics API. An application specifies resources and work;
EX's runtime validates and schedules that work; EX's driver layer implements the
execution contract for its target. The first target is an EX software GPU running
on the CPU. No other graphics API performs EX's rendering.

```text
Application / game engine
  -> EX managed or explicit level (Rust import: ex)
  -> EX core / shared contracts
  -> EX runtime
  -> EX driver layer
  -> EX reference GPU, later supported hardware targets
```

This is the intended architecture, not an implemented execution chain. Driver
integration needs operating-system interfaces, hardware command formats, memory
management, and shader instruction support. Adding a crate or vendor name does
not supply those capabilities. Hardware work is deferred.

## Current module boundaries

| Component | Implemented now | Next responsibility |
| --- | --- | --- |
| `ex-api`, library name `ex` | Resources, render scopes, submission/readback, shaders/pipelines/layouts, indexed/textured/blended/stencil drawing, compute entry points, and public examples | Explicit resource dependencies |
| `expansex-core` | Ownership, allocation, recording, queue policy, shaders, validated pipelines, indices, sampled textures, depth attachments, immutable blend state, stencil storage/rules, and compute contracts | Explicit resource dependencies |
| Validation | Resource identity/usages/ranges, layouts/stages, render state, finite shader arithmetic, clip w, storage poison, live index references, texture views/slots/devices, depth/stencil attachment/clear checks, compute slots/ranges/aliases/grid limits | Explicit hazards/dependencies |
| Reference backend | reference.rs and reference/programmable.rs execute copies and fixed/programmed triangles, including indexed fetch, texture sampling, depth testing, blending, stencil updates, and staged attachment commits; compute.rs executes storage dispatch with staged outputs | Explicit execution/memory dependencies |
| Shader subsystem | shader.rs owns/validates EX instructions and evaluates standalone or allocation-free renderer invocations | Richer instructions and later external import |

The public crate solves a packaging problem: callers have one `ex` import while
implementation responsibilities can grow behind it. The old core crate and its
working types are retained. This is a source boundary, not a virtual driver or a
GPU abstraction that pretends to execute work. No empty validation, shader,
reference, or hardware crates are introduced. Split code when working behavior
and dependency direction justify it.

## Object model

Today `BufferDescriptor` describes requested bytes and roles; it is not a buffer.
`CommandEncoder` and `CommandBuffer` retain one device and a validated copy
sequence. The legacy `CommandRecorder` returns a separate `DebugMarkerBuffer`.
Public names are provisional. See [the current contract](API-DRAFT.md).

`Instance` now discovers one adapter named `EX Reference (software)`.
`Adapter::create_device()` creates a distinct logical ownership domain each time.
Device clones and default queues share that domain; a queue retains device state
after the original device handle drops. Discovery does not detect installed GPUs.
See [the object ownership contract](OBJECT-MODEL.md) for rationale, API comparisons,
identity checks, and cleanup. Buffers now retain their creating device, have
independent allocation identity, and share storage when cloned.

Owned Framebuffer pixels and fixed-function RenderPass recorders now exist.
Graphics pipelines and single-buffer vertex layouts also exist.
Sampled 2D textures, rectangular views, and immutable samplers also exist, with
eight paired fragment slots inferred from shader instructions. General resource
layouts, surfaces, and presentation remain deferred. Compute pipelines now exist. Offscreen image
output is sufficient for the first triangle; a window and swapchain are not
required. Choose Rust ownership, enums, borrowing, and `Result` for actual
contracts instead of reproducing every object from another graphics API.

## Command model

Recording and execution are distinct. `finish(self)` consumes the device-bound
encoder; Rust rejects subsequent use. Its finished EX-owned command sequence is
inspectable and immutable, while referenced buffer contents remain live. Copy
recording validates both devices, transfer roles, checked ranges, and same-buffer
overlap. See [the recording contract](COMMANDS.md), including the provisional
rename of the prototype's finished labels to `DebugMarkerBuffer`.

`Queue::submit` now borrows reusable descriptions and executes them in order on
the CPU. See [submission](SUBMISSION.md) for completion, failure, and concurrency.
Finished render scopes retain their framebuffer and each draw's vertex binding.
Their exclusive encoder borrow enforces recording scope; unfinished scopes discard.
See [rendering](RENDERING.md) for the fixed vertex layout and failure boundaries.
Programmable draws retain each pipeline and run interpolated shaders; see
[pipelines](PIPELINES.md). Indexed draws also retain index source/format/range
and signed base vertex; [indexed drawing](INDEXED-DRAWING.md) defines execution-time
reference checks and sequential source locking. Draws retain shader-used texture/
sampler pairs; [textures](TEXTURES.md) defines live uploads, view snapshots, and
sampling. Command pools and allocator reuse follow real needs.

## Resources and memory

Descriptors borrow labels and usage lists; copying them copies borrowed views,
not memory. Creation now owns metadata and zero-initialized system-memory storage.
It validates positive size, nonempty usages, and contiguous-capacity limits, and
reports reservation errors. See [the buffer contract](BUFFERS.md) for exact scope,
including ordinary allocation failures that EX does not recover.

`HostAccess` grants explicit CPU read/write permissions independently of
device-side usages. CPU upload/readback copy bytes through a private storage
mutex after full-range checks. Buffer clones retain one allocation. No public
mutable slice is exposed. Whole submissions and CPU copies serialize per device;
this does not define future asynchronous GPU visibility. Heaps, device-local placement,
mapping, residency, alignment rules, and resource transitions remain deferred.

## Synchronization and lifetime

Reference submission blocks the caller and executes copies in recording order.
Successful return establishes completion. Storage failure preserves the completed
prefix and skips remaining work. The device gate serializes entire submissions
against CPU access; storage locks are acquired only afterward. No separate fence
is needed for this path. Future asynchronous hardware needs explicit completion
and is not forced into the reference implementation's CPU scheduling mechanism.

Rust moving a handle does not prove a GPU has finished reading its allocation.
Recorded work retains its resources, and the synchronous borrow preserves them
through completion. Cross-device work and overlapping copy ranges are checked.
Ordered chains observe earlier copy writes; general execution dependencies remain future work.
Multiple queues,
barriers, semaphores, and richer timelines remain future decisions, not claimed
features. The synchronous contract already covers render scopes and framebuffer
readback; it does not establish asynchronous hardware synchronization.

## Shader strategy and rendering

Phase 2 renders vertex positions with constant colour. Phase 3 now supplies a
small EX-owned representation of vertex and fragment operations with independent
evaluation and recorded programmable rendering. See [SHADERS.md](SHADERS.md) for bounded vector interfaces,
acyclic registers, operations, ownership, finite arithmetic, and errors. This
representation participates in recorded rendering; [PIPELINES.md](PIPELINES.md) defines
layout/stage validation, homogeneous clipping, perspective interpolation, and
fragment conversion. No shader frontend, external importer, or compiler exists.
The accepted source-language direction supports established toolchains later
while EX retains execution. Arbitrary Rust closures are not
assumed to be portable GPU shaders.

Longer term: established shader source -> existing compiler -> external format
import -> EX IR -> target compiler or software execution. IR means intermediate
representation; ISA means instruction set architecture. Expand instruction
coverage and define supported input semantics before adding an importer.
A custom EX source language remains a later decision.

The private reference module now implements vertex fetch, triangle assembly,
four-plane 2D clipping, full-target viewport mapping, 1/256-pixel rasterization,
and constant fragments. Image readback feeds an example-only PNG serializer.
Each render command preflights vertices and shades into a temporary image before
committing its target, so late fragment failures preserve the scope's pixels.
Indexed fetch reuses the programmable pipeline and decodes only selected records.
SampleTexture executes fragment sampling from view snapshots taken before shading.
No per-fragment allocation/locking is introduced; snapshots add memory per used
view per nonempty draw. [Depth testing](DEPTH.md) uses z/w mapped to [0,1],
screen-linear interpolation, optional comparison/write state, and a separate
f32 attachment. Both attachments stage/commit under the device gate and distinct
storage guards. [Blending](BLENDING.md) applies independent RGB/alpha equations and
channel masks against staged destination bytes after shading. [Stencil](STENCIL.md)
tests/updates a separate S8 image before depth, with independent front/back rules.
All attached images commit together. [Compute](COMPUTE.md) uses the same shader
interpreter with four built-in vectors and compute-only LoadStorage. Pipeline
outputs target explicit float4 views; 3D global IDs index independent elements.
The executor locks each used allocation once, stages all output ranges, and
commits only after every invocation succeeds. Following draws read these writes
under the same device gate. No shared memory, atomics, parallelism, or full GPU
conformance is claimed.

## Validation and errors

`BufferDescriptorError` is machine-matchable and implements `Display` and
`std::error::Error`. Current messages explain zero size and missing usage.
`BufferCreateError` preserves descriptor/reservation sources; `BufferAccessError`
identifies resource labels, forbidden CPU access, invalid ranges, and storage
poisoning. `CommandEncodeError` reports foreign source/destination resources,
missing transfer roles, invalid ranges, overlap, and command reservation failures.
`QueueSubmitError` reports foreign work or poisoned storage with a failed command
index, buffer role, and label; it defines the completed prefix explicitly.
Framebuffer errors describe invalid extents, allocation, readback size, pixel
bounds, and poisoned image storage. RenderEncodeError describes binding/range
misuse; RenderExecutionError identifies nonfinite live vertices, poisoned storage,
or temporary reservation failure. QueueSubmitError::RenderFailed retains that
cause and guarantees the failed render scope changed no target pixels.
Keep checks enabled in all builds initially. Release performance policy is a
future decision supported by measurement, not a reason to hide correctness bugs.

Programmable rendering now reports stage/layout/binding/numeric failures and stages
scope pixels until complete success. Continue defining unsupported usage, richer
attachment state, and resource hazard errors as those features appear. Report the
resource label/identity, operation, and expected condition.
Only introduce error variants when an operation can actually produce them.

## Decision and verification discipline

Describe the behavior, ownership, validation and failure rules of a
major abstraction before implementation. Keep its EX contract with the code.

Implement one piece, exercise it through the public API, test its failure cases,
review the diff, and commit a coherent change. The triangle milestone is complete
only when the EX public API records/submits work and EX's own software pipeline
produces an image. The Phase 0 example does not meet that criterion.
