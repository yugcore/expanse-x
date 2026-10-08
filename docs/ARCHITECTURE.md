# EX architecture

Status: Phase 1 copy recording. Discovery, ownership, reference allocation, CPU
access, and validated copy recording work; command execution is still planned.

The [development roadmap](ROADMAP.md) tracks verified work, pending steps, and
phase completion gates. This document describes architectural responsibilities.

## Philosophy and boundaries

The [project design](../DOCUMENTATION.md) governs this architecture.
EX has two intended control levels over one core: managed work derives a valid
plan from resource-use declarations; explicit work supplies a plan that EX
validates. Both share resources, execution rules, and completion semantics.
Neither control level is implemented as an execution API today. Build the
explicit core first, then add managed conveniences over proven contracts.

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
| `ex-api`, library name `ex` | Public discovery, buffers, CPU access, and copy recording; runnable examples | Submission entry point |
| `expansex-core` | Discovery, ownership, allocation, CPU access, and EX command enum with retained copy resources | Reference execution and submission contracts |
| Validation | Descriptor, capacity, host access, ranges, transfer roles, recording device/overlap, and poisoned CPU storage | Submission ownership, execution failures, and richer hazards |
| Reference backend | Owned system-memory buffer storage in core; no command executor | Execute EX's recorded operations on CPU; split working execution code when justified |
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

Next connect these descriptions to queue submission and EX's CPU executor.
Define execution order, visibility, failures, and host access before introducing
draws. Command pools and allocator reuse follow demonstrated requirements.

## Resources and memory

Descriptors borrow labels and usage lists; copying them copies borrowed views,
not memory. Creation now owns metadata and zero-initialized system-memory storage.
It validates positive size, nonempty usages, and contiguous-capacity limits, and
reports reservation errors. See [the buffer contract](BUFFERS.md) for exact scope,
including ordinary allocation failures that EX does not recover.

`HostAccess` grants explicit CPU read/write permissions independently of
device-side usages. CPU upload/readback copy bytes through a private storage
mutex after full-range checks. Buffer clones retain one allocation. No public
mutable slice is exposed. Individual CPU copies serialize, but this does not
define future queue ordering or GPU visibility. Heaps, device-local placement,
mapping, residency, alignment rules, and resource transitions remain deferred.

## Synchronization and lifetime

No queue execution or completion primitive exists yet. The first submission
contract must specify execution order, when writes become visible, how completion
is observed, and whether submission blocks the caller. A synchronous reference
implementation can establish those rules, but asynchronous hardware must not be
forced by accident into the same blocking implementation.

Rust moving a handle does not prove a GPU has finished reading its allocation.
Recorded work now retains its resources; submitted work must preserve them until
completion. Cross-device resources and overlapping ranges within a copy are
already checked. Dependencies between executed operations remain future work.
Multiple queues,
barriers, semaphores, and richer timelines remain future decisions, not claimed
features. Synchronization will be part of Phase 1's contract, not postponed until
after a renderer exists.

## Shader strategy and rendering

Phase 2 starts with vertex positions and constant colour. Phase 3 introduces a
small EX-owned representation of vertex and fragment operations with software
execution. This avoids choosing an external shader format as EX's permanent
contract. There is no shader type, instruction language, compiler, or interpreter
today. Arbitrary Rust closures are not assumed to be portable GPU shaders.

Longer term: EX source -> frontend -> EX IR -> backend compiler -> target ISA or
software execution. IR means intermediate representation; ISA means instruction
set architecture. Define stage inputs/outputs and invalid-program errors before
expanding instruction coverage. External formats can be studied or supported
later without replacing EX's semantics.

The first graphics backend will progressively add framebuffer storage, vertex
fetch, primitive assembly, viewport mapping, rasterization, constant fragment
colour, and image output. Clipping limits must be documented and tested when
rasterization is introduced. Depth/stencil, blending, programmable interpolation,
and textures are later steps, not hidden claims of full GPU conformance.

## Validation and errors

`BufferDescriptorError` is machine-matchable and implements `Display` and
`std::error::Error`. Current messages explain zero size and missing usage.
`BufferCreateError` preserves descriptor/reservation sources; `BufferAccessError`
identifies resource labels, forbidden CPU access, invalid ranges, and storage
poisoning. `CommandEncodeError` reports foreign source/destination resources,
missing transfer roles, invalid ranges, overlap, and command reservation failures.
Keep checks enabled in all builds initially. Release performance policy is a
future decision supported by measurement, not a reason to hide correctness bugs.

Before submission exists, define failures for wrong device, unsupported usage,
invalid command order, incompatible pipeline/attachment state, and resource
hazards. Report the resource label/identity, operation, and expected condition.
Only introduce error variants when an operation can actually produce them.

## Decision and verification discipline

Describe the behavior, ownership, validation and failure rules of a
major abstraction before implementation. Keep its EX contract with the code.

Implement one piece, exercise it through the public API, test its failure cases,
review the diff, and commit a coherent change. The triangle milestone is complete
only when the EX public API records/submits work and EX's own software pipeline
produces an image. The Phase 0 example does not meet that criterion.
