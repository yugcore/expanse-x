# EX architecture

Status: Phase 2 complete. Owned buffers/framebuffers, validated recording,
synchronous copy execution, and fixed-function software triangles work.

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
| `ex-api`, library name `ex` | Owned buffers/framebuffers, render scopes, submission/readback, and PNG triangle example | Programmable pipeline entry points |
| `expansex-core` | Ownership, allocation, copy/render command enum, and synchronous queue policy | Shader/pipeline contracts |
| Validation | Resource identity, usages, ranges, extents, render binding/ranges, finite vertices, and storage poison | Pipeline/stage compatibility and richer hazards |
| Reference backend | Private reference.rs executes copies, fetches/clips vertices, and rasterizes triangles under the device gate | Shader interpretation and programmable fragments |
| Shader subsystem | Absent | Small EX internal representation and interpreter |

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
Textures, views, samplers, layouts, graphics/compute pipelines, surfaces, and
presentation are deferred until they have actual behavior. Offscreen image
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
Next introduce programmable stages. Command pools and allocator reuse follow real needs.

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

Phase 2 now renders vertex positions with constant colour. Phase 3 introduces a
small EX-owned representation of vertex and fragment operations with software
execution. This avoids choosing an external shader format as EX's permanent
contract. There is no shader type, instruction language, compiler, or interpreter
today. Arbitrary Rust closures are not assumed to be portable GPU shaders.

Longer term: EX source -> frontend -> EX IR -> backend compiler -> target ISA or
software execution. IR means intermediate representation; ISA means instruction
set architecture. Define stage inputs/outputs and invalid-program errors before
expanding instruction coverage. External formats can be studied or supported
later without replacing EX's semantics.

The private reference module now implements vertex fetch, triangle assembly,
four-plane 2D clipping, full-target viewport mapping, 1/256-pixel rasterization,
and constant fragments. Image readback feeds an example-only PNG serializer.
Each render command preflights all vertex data before changing its target.
Depth/stencil, blending, programmable interpolation,
and textures are later steps, not hidden claims of full GPU conformance.

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

Before adding programmable rendering, define failures for unsupported usage, invalid draw
state, incompatible pipeline/attachment state, and resource hazards. Report the
resource label/identity, operation, and expected condition.
Only introduce error variants when an operation can actually produce them.

## Decision and verification discipline

Describe the behavior, ownership, validation and failure rules of a
major abstraction before implementation. Keep its EX contract with the code.

Implement one piece, exercise it through the public API, test its failure cases,
review the diff, and commit a coherent change. The triangle milestone is complete
only when the EX public API records/submits work and EX's own software pipeline
produces an image. The Phase 0 example does not meet that criterion.
