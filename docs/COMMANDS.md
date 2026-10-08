# EX recorded buffer copies

Status: implemented Phase 1 recording contract. Queue submission/execution is the
next step; this contract does not make recording execute a copy. Public names
remain provisional.

## Design

EX needs to describe work independently of execution while preserving the
resources that work will use. `Device::create_command_encoder()` creates an owned
`CommandEncoder` for that logical device. `copy_buffer(source, source_offset,
destination, destination_offset, size)` appends an EX-owned `Command::CopyBuffer`
with retained buffer handles. `finish(self)` moves those commands and their
device into an immutable `CommandBuffer`.

The core representation is an enum rather than a closure or an existing API's
command handle. A future EX executor can interpret the operations, validate
submission ownership, and retain resources independently of the application.
No command pool, shader, driver ABI, or scheduler is needed just to record copies.

Existing APIs also record copy operations, with different public contracts:
[Vulkan](https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdCopyBuffer.html)
accepts a list of regions and requires common-device resources, transfer usages,
and nonconflicting source/destination memory;
[Direct3D 12](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12graphicscommandlist-copybufferregion)
records one region with byte offsets on a command list;
[Metal](https://developer.apple.com/documentation/metal/mtlblitcommandencoder/copy%28from%3Asourceoffset%3Ato%3Adestinationoffset%3Asize%3A%29)
uses a blit encoder and documents platform-dependent alignment requirements.
EX chooses one byte-granular region per call, checked `Result` errors, and Rust
handle retention. The reference contract introduces no four-byte alignment rule.
Future hardware must implement supported semantics or explicitly report limits;
it cannot silently reinterpret EX's byte ranges.

## Validation and failure effects

Checks apply in this order:

1. Source device, then destination device, must match the encoder's logical device.
2. Source requires `TransferSource`; destination requires `TransferDestination`.
3. Source range, then destination range, must fit with checked `u64` addition.
4. If both handles share an allocation, nonempty ranges must not overlap.
5. Reserve space for the command, then retain the buffers and append it.

CPU `HostAccess` does not grant or restrict recording. A transfer-usable buffer
with `HostAccess::None` can be recorded; recording never reads or locks its bytes.
An empty copy is allowed at any in-range offset, including exactly the end, but
still requires both device and usage checks. Its command still retains resources.
Offsets beyond the end are invalid even for zero bytes. Overflow is a range error.
Nonempty identical ranges are overlapping. Adjacent/disjoint same-buffer regions
are allowed, including when callers use cloned handles.

`CommandEncodeError` identifies the source/destination role, resource label,
missing usage, invalid range, or overlapping offsets as appropriate. A failed
reservation retains its standard `TryReserveError` source. Rejected calls append
nothing, leave previous commands intact, and never modify resource contents.
Callers can correct a request and continue using the encoder. Error-label and
small handle allocations still follow ordinary Rust OOM behavior; universal OOM
recovery is not promised.

## Ownership, inspection, and source contents

Encoders and finished command buffers retain the creating device even when empty.
Each recorded copy retains its source/destination allocations through `Buffer`
clones; dropping original handles does not release them. Dropping the last owner
releases the state. There are no device-to-command ownership cycles.

`device()`, `belongs_to()`, and `commands()` expose ownership and a borrowed,
read-only command sequence. `BufferCopy` exposes its retained buffers, byte
offsets, and size through getters. Commands are not cloneable in this step.
Immutable commands do not imply immutable buffer contents: public CPU access
through another handle still obeys `HostAccess` and may change the bytes.

Recording retains references, not a snapshot. The intended execution contract
reads source contents when a copy executes, including writes made before its
execution and earlier commands' results. There is no executor today to verify
that behavior. Concurrent host access, submission order, completion, poisoning,
and partial-execution errors must be defined in the submission step. Recording
does not establish a GPU dependency or reserve an allocation against CPU access.

## Prototype compatibility

The original standalone `CommandRecorder` remains a label-only prototype.
Its unchanged `new`, `insert_debug_marker`, and `finish` method flow now returns
`DebugMarkerBuffer`, whose `debug_markers()` still borrows the stored labels.
Previously that output type was named `CommandBuffer`. This intentional provisional
API change makes the real `CommandBuffer` always device-bound; explicitly typed
old label results must use `DebugMarkerBuffer`. Inferred existing examples work.

The new `CommandEncoder` records copies only in this step; it does not yet offer
marker insertion. Neither kind of finished recording can be submitted yet.

## Verification and next step

Public tests and `record_copy` exercise validation, unchanged bytes, ownership,
command order, retained resources, empty ranges, aliasing, and continued recording
after errors. Compile-fail documentation checks use after consuming an encoder.
Internal lifecycle checks verify release after the final command buffer drops.
Tests do not induce allocator OOM by exhausting memory.

Next: queue submission must execute these operations under a documented order,
visibility, host-concurrency, and failure contract. Only after submission/readback
works is Phase 1 complete.
