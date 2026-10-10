# EX recorded buffer copies

Encoders record validated copies without executing them. Finishing an encoder
produces a device-bound command buffer; [queue submission](SUBMISSION.md)
executes its work synchronously.

## Design

EX needs to describe work independently of execution while preserving the
resources that work will use. `Device::create_command_encoder()` creates an owned
`CommandEncoder` for that logical device. `copy_buffer(source, source_offset,
destination, destination_offset, size)` appends an EX-owned `Command::CopyBuffer`
with retained buffer handles. `finish(self)` moves those commands and their
device into an immutable `CommandBuffer`.

The core representation is an enum rather than a closure or an existing API's
command handle. EX's reference executor interprets the operations, validates
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

The same encoder also records render scopes and [compute dispatches](COMPUTE.md).
Command::Dispatch retains a ComputePipeline and validated Storage buffer views;
recording does not evaluate the kernel or modify storage. These operations share
the same consuming finish, immutable inspection, device ownership, retention,
and live-reference rules. The copy-specific checks below remain unchanged.

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

Recording retains references, not a snapshot. The implemented submission contract
reads source contents when a copy executes, including writes made before its
execution and earlier commands' results. Whole submissions serialize against CPU
access on the same device. See the submission contract for ordering, completion,
poisoning, and partial-execution errors. Recording
does not establish a GPU dependency or reserve an allocation against CPU access.

## Prototype compatibility

The original standalone `CommandRecorder` remains a label-only prototype.
Its unchanged `new`, `insert_debug_marker`, and `finish` method flow now returns
`DebugMarkerBuffer`, whose `debug_markers()` still borrows the stored labels.
Previously that output type was named `CommandBuffer`. This intentional provisional
API change makes the real `CommandBuffer` always device-bound; explicitly typed
old label results must use `DebugMarkerBuffer`. Inferred existing examples work.

The `CommandEncoder` now records copies and finished render scopes; it does not offer
marker insertion. Only the device-bound `CommandBuffer` can be submitted.

## Verification and next step

Public tests and `record_copy` exercise validation, unchanged bytes, ownership,
command order, retained resources, empty ranges, aliasing, and continued recording
after errors. Compile-fail documentation checks use after consuming an encoder.
Internal lifecycle checks verify release after the final command buffer drops.
Tests do not induce allocator OOM by exhausting memory.

The `submit_copy` example now executes these operations under the documented
submission contract. Phase 2 now also records/executes triangles under the separate
[rendering contract](RENDERING.md); pipeline-controlled triangles are now covered
by the [pipeline contract](PIPELINES.md).

## Texture flow integration

CommandEncoder now records copy_buffer_to_texture, copy_texture_to_buffer and
copy_texture under [TEXTURE-FLOW.md](TEXTURE-FLOW.md). The three Command variants
retain positive validated TextureViews, equal extents for image copies, and a
four-byte-aligned TextureBufferLayout for buffer/image rows. The command sequence
and resources remain unchanged on recording errors; finished commands read live
source storage. These image layout checks do not alter byte-granular copy_buffer.
