# EX public API catalog

Updated: 2026-10-08. Scope: every currently exported EX type, inherent method,
public field, enum variant, and explicit standard trait implementation.
There are **33 public types and 62 inherent methods**. These are provisional
Phase 2 APIs, not a completed or stable v1 specification.

Applications depend on package `ex-api` and import its library as `ex`:

```toml
[dependencies]
ex = { package = "ex-api", path = "crates/ex-api" }
```

The path above is relative to the application's Cargo.toml; adjust it to the
workspace location. The [public crate](crates/ex-api/src/lib.rs) re-exports the
implementation from `expansex-core`. Implementation modules are private;
use `ex::Buffer`, for example, rather than `ex::buffer::Buffer`.
No public free functions, custom traits, constants, macros, or type aliases are
currently exported.

## Type index

| Area | Exported types |
| --- | --- |
| Discovery and ownership | [Instance](#instance), [Adapter](#adapter), [AdapterInfo](#adapterinfo), [BackendKind](#backendkind), [Device](#device), [Queue](#queue) |
| Buffers | [BufferDescriptor](#bufferdescriptor), [BufferUsage](#bufferusage), [HostAccess](#hostaccess), [Buffer](#buffer) |
| Copy recording | [CommandEncoder](#commandencoder), [CommandBuffer](#commandbuffer), [Command](#command), [BufferCopy](#buffercopy), [CopyBufferRole](#copybufferrole) |
| Framebuffers | [Framebuffer](#framebuffer), [FramebufferDescriptor](#framebufferdescriptor), [FramebufferFormat](#framebufferformat) |
| Rendering | [RenderPass](#renderpass), [RenderCommands](#rendercommands), [DrawTriangles](#drawtriangles) |
| Prototype labels | [CommandRecorder](#commandrecorder), [DebugMarkerBuffer](#debugmarkerbuffer) |
| Errors | [BufferDescriptorError](#bufferdescriptorerror), [BufferCreateError](#buffercreateerror), [BufferAccessError](#bufferaccesserror), [CommandEncodeError](#commandencodeerror), [QueueSubmitError](#queuesubmiterror) |
| Rendering errors | [FramebufferDescriptorError](#framebufferdescriptorerror), [FramebufferCreateError](#framebuffercreateerror), [FramebufferAccessError](#framebufferaccesserror), [RenderEncodeError](#renderencodeerror), [RenderExecutionError](#renderexecutionerror) |

Signatures below use the listed type as the meaning of `Self`. Omitted return
types mean `()`. References borrow their values; returned handles own their
retained state. `finish(self)` consumes its receiver. Byte offsets and counts are
`u64`, not element or vertex counts. See the [API contract](docs/API-DRAFT.md)
for behavior and error rules.

## Discovery and ownership

Source: [object.rs](crates/expansex-core/src/object.rs).

### Instance

`ex::Instance` discovers one built-in `EX Reference (software)` adapter. It does
not detect installed hardware. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Instance::new() -> Self` | Construct reference discovery. |
| `Instance::adapters(&self) -> &[Adapter]` | Borrow discovered adapter handles. |
| `Instance::default_adapter(&self) -> Adapter` | Retain the always-present reference adapter. |

`Default::default()` is equivalent to `new()`. An `Instance` is not cloneable.

### Adapter

`ex::Adapter` retains discovery metadata. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Adapter::info(&self) -> &AdapterInfo` | Borrow target metadata. |
| `Adapter::create_device(&self) -> Device` | Create a new logical ownership domain, even on the same adapter. |

Cloning an adapter shares metadata; it does not create a device.

### AdapterInfo

`ex::AdapterInfo` has these public fields and no inherent methods:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | `String` | Human-readable target name. |
| `backend` | `BackendKind` | EX backend identity. |

An owned metadata value can be constructed or changed, but `Adapter::info()`
returns a shared reference and does not allow mutating the adapter's metadata.

### BackendKind

`ex::BackendKind` is non-exhaustive and has no inherent methods.

| Variant | Meaning |
| --- | --- |
| `BackendKind::Reference` | EX's own CPU/software target; copies and fixed-function triangles. |

External matches need a fallback arm because more variants can be added.

### Device

`ex::Device` retains one logical device. Its fields are private. Methods are
implemented across [object.rs](crates/expansex-core/src/object.rs),
[buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs),
[command.rs](crates/expansex-core/src/command.rs), and
[framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Method | Behavior |
| --- | --- |
| `Device::adapter(&self) -> Adapter` | Retain the selected adapter. |
| `Device::default_queue(&self) -> Queue` | Retain the device's one default endpoint; starts no work. |
| `Device::is_same(&self, other: &Self) -> bool` | Compare logical device identity. |
| `Device::create_buffer(&self, descriptor: BufferDescriptor<'_>) -> Result<Buffer, BufferCreateError>` | Validate and allocate owned zero-initialized reference storage. |
| `Device::create_command_encoder(&self) -> CommandEncoder` | Create an empty encoder bound to this device. |
| `Device::create_framebuffer(&self, descriptor: FramebufferDescriptor<'_>) -> Result<Framebuffer, FramebufferCreateError>` | Validate extents/format and allocate zeroed RGBA reference pixels. |

Cloning retains the same domain; `Adapter::create_device()` creates a new one.
Discovery/device/encoder constructors do not expose a recoverable allocation
error path. Buffer creation has its documented fallible reservation path.

### Queue

`ex::Queue` retains the default endpoint's device. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Queue::device(&self) -> Device` | Retain the original owning device. |
| `Queue::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `Queue::submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError>` | Execute reusable finished copies and render scopes synchronously in order on the CPU. |

Successful submission means all submitted operations finished. Concurrent submissions
and CPU access on the device serialize for the whole operation; concurrent arrival
order is unspecified. Foreign work changes nothing. Storage failure leaves earlier
commands completed, changes nothing in the failed copy, and skips later work.
Expected render failure leaves that scope's target unchanged. See
[SUBMISSION.md](docs/SUBMISSION.md) and [RENDERING.md](docs/RENDERING.md).
No asynchronous completion API exists.

## Buffers

Sources: [buffer.rs](crates/expansex-core/src/buffer.rs) and
[buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).
Contract: [BUFFERS.md](docs/BUFFERS.md).

### BufferDescriptor

`ex::BufferDescriptor<'a>` describes a request; it allocates no storage itself.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Borrowed debug label; empty labels are allowed. |
| `size` | `u64` | Requested bytes; must be greater than zero. |
| `usages` | `&'a [BufferUsage]` | Borrowed nonempty role list; duplicates have no extra effect. |
| `host_access` | `HostAccess` | CPU-copy permissions, independent of device-side usages. |

| Method | Behavior |
| --- | --- |
| `BufferDescriptor::validate(&self) -> Result<(), BufferDescriptorError>` | Check positive size, then nonempty usage list. Does not prove allocation/backend support. |

Creation copies borrowed metadata. Copying the descriptor copies its references
and values, not storage. There is no `Default` implementation.

### BufferUsage

`ex::BufferUsage` has no inherent methods. All variants can be requested in a
descriptor; transfer and Vertex roles currently participate in recording/execution.

| Variant | Role |
| --- | --- |
| `BufferUsage::Vertex` | Fixed-layout triangle vertex input. |
| `BufferUsage::Index` | Future indexed draw input. |
| `BufferUsage::Uniform` | Future shader parameters. |
| `BufferUsage::Storage` | Future shader-accessible data. |
| `BufferUsage::TransferSource` | Required source role for recorded copies. |
| `BufferUsage::TransferDestination` | Required destination role for recorded copies. |

A usage flag alone executes no work. Copies and fixed-function draws execute on submission;
shaders remain planned.

### HostAccess

`ex::HostAccess` has no inherent methods and is not a memory-placement selection.

| Variant | Public CPU access |
| --- | --- |
| `HostAccess::None` | Neither read nor write. |
| `HostAccess::Write` | Write only. |
| `HostAccess::Read` | Read only. |
| `HostAccess::ReadWrite` | Both read and write. |

### Buffer

`ex::Buffer` owns a handle to fixed reference storage and retains its device.
All fields are private; create it through `Device::create_buffer`.

| Method | Behavior |
| --- | --- |
| `Buffer::label(&self) -> &str` | Borrow the owned label. |
| `Buffer::size(&self) -> u64` | Get allocation size in bytes. |
| `Buffer::usages(&self) -> &[BufferUsage]` | Borrow requested roles. |
| `Buffer::host_access(&self) -> HostAccess` | Get CPU permissions. |
| `Buffer::device(&self) -> Device` | Retain the creating device. |
| `Buffer::belongs_to(&self, device: &Device) -> bool` | Test logical device membership. |
| `Buffer::is_same(&self, other: &Self) -> bool` | Test allocation identity, including cloned aliases. |
| `Buffer::write(&self, offset: u64, data: &[u8]) -> Result<(), BufferAccessError>` | Copy CPU bytes after permission/range checks. |
| `Buffer::read(&self, offset: u64, destination: &mut [u8]) -> Result<(), BufferAccessError>` | Copy bytes into caller-owned CPU storage after permission/range checks. |

Clones share live bytes. Reads/writes are synchronous and serialize with submissions
through the device gate, then private storage. Rejected accesses do not partially change
their destination. Empty accesses require permission and an offset at or before
the end. Transfer usages do not grant CPU access. No mapping or public mutable
slice is exposed. CPU access cannot interleave inside a same-device submission;
a sequence of separate calls is not transactional.

## Copy recording

Source: [command.rs](crates/expansex-core/src/command.rs).
Contract: [COMMANDS.md](docs/COMMANDS.md).

### CommandEncoder

`ex::CommandEncoder` is created through `Device::create_command_encoder` and
retains that device. All fields are private.

| Method | Behavior |
| --- | --- |
| `CommandEncoder::device(&self) -> Device` | Retain the owning device. |
| `CommandEncoder::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `CommandEncoder::commands(&self) -> &[Command]` | Inspect recorded operations. |
| `CommandEncoder::copy_buffer(&mut self, source: &Buffer, source_offset: u64, destination: &Buffer, destination_offset: u64, size: u64) -> Result<(), CommandEncodeError>` | Validate and append a retained copy description. |
| `CommandEncoder::finish(self) -> CommandBuffer` | Consume the encoder and move its work into immutable finished commands. |
| `CommandEncoder::begin_render_pass(&mut self, target: &Framebuffer, clear: Option<[u8; 4]>) -> Result<RenderPass<'_>, RenderEncodeError>` | Open an exclusive borrowed render recorder; clear/draws change no pixels until submission. |

Checks run in this order: source/destination devices, source/destination transfer
roles, source/destination checked ranges, same-allocation overlap, command
reservation. Empty copies at valid end offsets are allowed. Nonempty overlapping
aliases are rejected; adjacent/disjoint same-buffer ranges are allowed.
No four-byte alignment or host access is required. Failed calls append nothing
and allow continued recording. Recording/finishing never read or write bytes.

### CommandBuffer

`ex::CommandBuffer` retains the device and recorded resources after encoder
finish. All fields are private.

| Method | Behavior |
| --- | --- |
| `CommandBuffer::device(&self) -> Device` | Retain the owning device. |
| `CommandBuffer::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `CommandBuffer::commands(&self) -> &[Command]` | Inspect immutable finished operations. |

There is no public constructor or clone implementation. The command sequence is
immutable; buffer contents remain live. Resources are retained, not snapshotted.
`Queue::submit` borrows this reusable work and samples live bytes per executed copy.

### Command

`ex::Command` is non-exhaustive and has no inherent methods.

| Variant | Payload |
| --- | --- |
| `Command::CopyBuffer(BufferCopy)` | One validated buffer-copy description. |
| `Command::RenderPass(RenderCommands)` | One finished render scope, retaining target and per-draw vertex bindings. |

External matches need a fallback arm. Descriptions are not executor closures or
another graphics API's handles.

### BufferCopy

`ex::BufferCopy` is created by valid copy recording. All fields are private.

| Method | Behavior |
| --- | --- |
| `BufferCopy::source(&self) -> &Buffer` | Borrow the retained source handle. |
| `BufferCopy::source_offset(&self) -> u64` | Get the source byte offset. |
| `BufferCopy::destination(&self) -> &Buffer` | Borrow the retained destination handle. |
| `BufferCopy::destination_offset(&self) -> u64` | Get the destination byte offset. |
| `BufferCopy::size(&self) -> u64` | Get the byte count. |

### CopyBufferRole

`ex::CopyBufferRole` identifies a copy operand in error reports. It has no
inherent methods.

| Variant | Meaning |
| --- | --- |
| `CopyBufferRole::Source` | Source buffer. |
| `CopyBufferRole::Destination` | Destination buffer. |

Its `Display` representation is `source` or `destination`.

## Framebuffers and fixed-function rendering

Sources: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs) and
[render.rs](crates/expansex-core/src/render.rs). Contract: [RENDERING.md](docs/RENDERING.md).
Pixels are row-major RGBA bytes with top-left origin. Vertex positions are
little-endian f32 x/y pairs, 8-byte stride, in 2D post-projection coordinates.
The reference executor clips to [-1,+1] on both axes, supports both windings,
samples pixel centres on a 1/256-pixel grid, and uses a top-left shared-edge rule.
Zero-area triangles write nothing; later draws overwrite covered RGBA pixels.

### FramebufferFormat

`ex::FramebufferFormat` is non-exhaustive; external matches require a fallback.
It has no inherent methods.

| Variant | Meaning |
| --- | --- |
| `FramebufferFormat::Rgba8Unorm` | Four u8 channels in R,G,B,A order; no sRGB conversion. |

### FramebufferDescriptor

`ex::FramebufferDescriptor<'a>` borrows creation metadata; creation owns the label.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug name; empty allowed. |
| `width` | `u32` | Pixels across; 1..=16384. |
| `height` | `u32` | Pixels down; 1..=16384. |
| `format` | `FramebufferFormat` | Currently Rgba8Unorm only. |

| Method | Behavior |
| --- | --- |
| `FramebufferDescriptor::validate(&self) -> Result<(), FramebufferDescriptorError>` | Check positive extents, maximum extents, then supported format; allocate nothing. |

### Framebuffer

`ex::Framebuffer` has private fields, owns fixed zeroed pixels, and retains its
device. Clones share live pixels. Create through `Device::create_framebuffer`.

| Method | Behavior |
| --- | --- |
| `Framebuffer::label(&self) -> &str` | Borrow owned label. |
| `Framebuffer::width(&self) -> u32` | Width in pixels. |
| `Framebuffer::height(&self) -> u32` | Height in pixels. |
| `Framebuffer::format(&self) -> FramebufferFormat` | Stored pixel format. |
| `Framebuffer::device(&self) -> Device` | Retain creating device. |
| `Framebuffer::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `Framebuffer::is_same(&self, other: &Self) -> bool` | Compare allocation identity, including aliases. |
| `Framebuffer::read_pixels(&self, destination: &mut [[u8; 4]]) -> Result<(), FramebufferAccessError>` | Read the whole image into exactly width*height entries; errors leave destination unchanged. |
| `Framebuffer::pixel(&self, x: u32, y: u32) -> Result<[u8; 4], FramebufferAccessError>` | Read one checked top-left-origin pixel. |

Readbacks serialize with submissions using the device gate then pixel storage.
No mutable image slice, file export method, or window is exposed. The example's
PNG writer is application code, not an EX export.

### RenderPass

`ex::RenderPass<'a>` exclusively borrows its encoder and has private fields.
Create through `CommandEncoder::begin_render_pass`. Nesting or using the encoder
with a live borrowed pass is rejected by Rust. Dropping unfinished work discards it.

| Method | Behavior |
| --- | --- |
| `RenderPass::set_vertex_buffer(&mut self, buffer: &Buffer, offset: u64) -> Result<(), RenderEncodeError>` | Validate exact device, Vertex usage, offset <= size, then 4-byte alignment; retain binding. |
| `RenderPass::draw(&mut self, vertices: Range<u32>, colour: [u8; 4]) -> Result<(), RenderEncodeError>` | Validate binding, ascending range, triangle-list count, checked byte extent; append a retained draw. |
| `RenderPass::finish(self) -> Result<(), RenderEncodeError>` | Reserve/append one render command; consumes this pass. Failed reservation commits no pass work. |

`Range` is `std::ops::Range`, not an EX export. The exclusive end and first vertex
are relative to the binding's byte offset. Count must be divisible by three;
zero-count draws still require a binding and an in-bounds start. Failed binding
or draw calls preserve earlier work/binding. Binding resets for each new pass.

### RenderCommands

`ex::RenderCommands` has private fields and is constructed by finishing a pass.

| Method | Behavior |
| --- | --- |
| `RenderCommands::target(&self) -> &Framebuffer` | Borrow retained target. |
| `RenderCommands::clear_colour(&self) -> Option<[u8; 4]>` | Some clears before draws; None preserves pixels. |
| `RenderCommands::draws(&self) -> &[DrawTriangles]` | Borrow recorded draws in order. |

### DrawTriangles

`ex::DrawTriangles` has private fields. Each draw retains its own vertex binding;
recording retains live source bytes, not a snapshot.

| Method | Behavior |
| --- | --- |
| `DrawTriangles::vertex_buffer(&self) -> &Buffer` | Borrow retained vertex source. |
| `DrawTriangles::vertex_offset(&self) -> u64` | Binding offset in bytes. |
| `DrawTriangles::vertices(&self) -> Range<u32>` | Return an owned copy of the vertex range. |
| `DrawTriangles::colour(&self) -> [u8; 4]` | Constant fragment RGBA; replaces covered pixels without blending. |

## Standalone prototype labels

Source: [markers.rs](crates/expansex-core/src/command/markers.rs).
These APIs record text only and are independent of device-bound work.

### CommandRecorder

`ex::CommandRecorder` has private fields.

| Method | Behavior |
| --- | --- |
| `CommandRecorder::new() -> Self` | Create an empty label recorder. |
| `CommandRecorder::insert_debug_marker(&mut self, label: &str)` | Store an owned label copy. |
| `CommandRecorder::finish(self) -> DebugMarkerBuffer` | Consume the recorder and finish its label sequence. |

`Default::default()` is equivalent to `new()`. This recorder cannot encode copies.

### DebugMarkerBuffer

`ex::DebugMarkerBuffer` has private fields.

| Method | Behavior |
| --- | --- |
| `DebugMarkerBuffer::debug_markers(&self) -> &[String]` | Borrow labels in recording order. |

Earlier prototypes called this result `CommandBuffer`; explicitly typed old label
results must now use `DebugMarkerBuffer`. Empty label sequences are allowed.
Finishing never executes work, even if a label says "draw" or "copy".

## Errors

These enums have public variants and their listed payloads. All implement
`Debug`, `Display`, and `std::error::Error`; none has inherent methods.
Messages describe the failed requirement. `TryReserveError` comes from
`std::collections`, not a separate EX export.

### BufferDescriptorError

Source: [buffer.rs](crates/expansex-core/src/buffer.rs).

| Variant | Meaning |
| --- | --- |
| `BufferDescriptorError::ZeroSize` | Requested size is zero. |
| `BufferDescriptorError::MissingUsage` | Requested role list is empty. |

### BufferCreateError

Source: [buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `BufferCreateError::InvalidDescriptor(BufferDescriptorError)` | Descriptor validation failed. |
| `BufferCreateError::SizeTooLarge { size: u64 }` | Size cannot fit the process's contiguous-vector capacity. |
| `BufferCreateError::AllocationFailed { size: u64, source: TryReserveError }` | Reserving bytes or owned metadata failed. |

### BufferAccessError

Source: [buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `BufferAccessError::ReadNotAllowed { label: String, access: HostAccess }` | CPU reads are forbidden. |
| `BufferAccessError::WriteNotAllowed { label: String, access: HostAccess }` | CPU writes are forbidden. |
| `BufferAccessError::OutOfBounds { label: String, offset: u64, length: u64, buffer_size: u64 }` | Full range does not fit, including overflow. |
| `BufferAccessError::StoragePoisoned { label: String }` | An internal panic interrupted storage access. |

### CommandEncodeError

Source: [command.rs](crates/expansex-core/src/command.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `CommandEncodeError::ForeignBuffer { role: CopyBufferRole, label: String }` | Buffer belongs to a different logical device. |
| `CommandEncodeError::MissingUsage { role: CopyBufferRole, label: String, required: BufferUsage }` | Required transfer role is missing. |
| `CommandEncodeError::OutOfBounds { role: CopyBufferRole, label: String, offset: u64, size: u64, buffer_size: u64 }` | Full copy range does not fit, including overflow. |
| `CommandEncodeError::OverlappingCopy { label: String, source_offset: u64, destination_offset: u64, size: u64 }` | Nonempty ranges overlap in the same allocation. |
| `CommandEncodeError::AllocationFailed { source: TryReserveError }` | Reserving a command-list slot failed. |

Creation/encoding reservation errors do not provide universal OOM recovery:
small handle and error-label allocations still use ordinary Rust allocation behavior.

### QueueSubmitError

Source: [submission.rs](crates/expansex-core/src/submission.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `QueueSubmitError::ForeignCommandBuffer` | Finished work belongs to another device; nothing executed. |
| `QueueSubmitError::StoragePoisoned { command_index: usize, role: CopyBufferRole, label: String }` | Failed nonempty copy at this zero-based index; earlier copies completed, failed destination unchanged, later work skipped. |
| `QueueSubmitError::RenderFailed { command_index: usize, source: RenderExecutionError }` | Failed render command; its target unchanged, earlier commands completed, later work skipped. |

Only RenderFailed supplies an underlying `Error::source`. Aliased copies report poisoned
storage as `Source`; zero-byte copies never lock storage. No rollback is promised.

### FramebufferDescriptorError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferDescriptorError::ZeroExtent { width: u32, height: u32 }` | Either axis is zero. |
| `FramebufferDescriptorError::ExtentTooLarge { width: u32, height: u32, maximum: u32 }` | Either axis exceeds reference maximum 16384. |

### FramebufferCreateError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferCreateError::InvalidDescriptor(FramebufferDescriptorError)` | Invalid extents. Only RGBA8 can currently be requested. |
| `FramebufferCreateError::AllocationFailed { source: TryReserveError }` | Reserving pixels or owned label failed. |

Both variants retain an underlying error source; small handle OOM is not recovered.

### FramebufferAccessError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferAccessError::WrongPixelCount { expected: usize, actual: usize }` | Readback slice length must equal width*height. |
| `FramebufferAccessError::OutOfBounds { x: u32, y: u32, width: u32, height: u32 }` | Requested pixel is outside the image. |
| `FramebufferAccessError::StoragePoisoned { label: String }` | Interrupted internal operation left pixels untrusted. |

### RenderEncodeError

Source: [render.rs](crates/expansex-core/src/render.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `RenderEncodeError::ForeignFramebuffer { label: String }` | Target belongs to another device. |
| `RenderEncodeError::ForeignVertexBuffer { label: String }` | Vertex source belongs to another device. |
| `RenderEncodeError::MissingVertexUsage { label: String }` | Binding requires Vertex usage. |
| `RenderEncodeError::VertexOffsetOutOfBounds { label: String, offset: u64, buffer_size: u64 }` | Binding offset exceeds size. |
| `RenderEncodeError::UnalignedVertexOffset { offset: u64 }` | Binding offset is not a multiple of 4. |
| `RenderEncodeError::MissingVertexBuffer` | Draw requires a bound vertex source. |
| `RenderEncodeError::InvalidVertexRange { start: u32, end: u32 }` | Exclusive end precedes start. |
| `RenderEncodeError::IncompleteTriangle { vertex_count: u32 }` | Count is not divisible by three. |
| `RenderEncodeError::VertexRangeOutOfBounds { label: String, first_vertex: u32, vertex_count: u32, offset: u64, buffer_size: u64 }` | Full fetched byte extent does not fit, including overflow. |
| `RenderEncodeError::AllocationFailed { source: TryReserveError }` | Reserving recorded draw or command slots failed. |

Only AllocationFailed has an underlying source. Expected recording errors change
no bytes or prior recorded work. Nonfinite positions are checked at execution.

### RenderExecutionError

Source: [render.rs](crates/expansex-core/src/render.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `RenderExecutionError::VertexStoragePoisoned { label: String }` | Nonempty draw's vertex source is untrusted. |
| `RenderExecutionError::FramebufferStoragePoisoned { label: String }` | Target pixel storage is untrusted. |
| `RenderExecutionError::NonFiniteVertex { label: String, vertex_index: u32 }` | Absolute fetched vertex index contains NaN or infinity. |
| `RenderExecutionError::AllocationFailed` | Fallible decoded-vertex preparation failed; no lower-level source retained. |

All expected errors occur before that render scope changes target pixels. The
gate prevents concurrent uploads from changing sources during preflight/drawing.

## Explicit standard trait support

This lists implemented/derived traits, not Rust's blanket implementations or
auto traits. They are separate from the 62 inherent methods above.

| Type | Traits |
| --- | --- |
| `Instance` | `Debug`, `Default` |
| `Adapter` | `Debug`, `Clone` |
| `AdapterInfo` | `Debug`, `Clone`, `PartialEq`, `Eq` |
| `BackendKind` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `Device` | `Debug`, `Clone` |
| `Queue` | `Debug`, `Clone` |
| `BufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `BufferUsage` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `HostAccess` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `Buffer` | `Debug`, `Clone` |
| `CommandEncoder` | `Debug` |
| `CommandBuffer` | `Debug` |
| `Command` | `Debug` |
| `BufferCopy` | `Debug` |
| `CopyBufferRole` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display` |
| `CommandRecorder` | `Debug`, `Default` |
| `DebugMarkerBuffer` | `Debug` |
| `BufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `BufferCreateError` | `Debug`, `Display`, `Error` |
| `BufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `CommandEncodeError` | `Debug`, `Display`, `Error` |
| `QueueSubmitError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `FramebufferFormat` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `FramebufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `Framebuffer` | `Debug`, `Clone` |
| `FramebufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `FramebufferCreateError` | `Debug`, `Display`, `Error` |
| `FramebufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `RenderPass<'a>` | `Debug` |
| `RenderCommands` | `Debug` |
| `DrawTriangles` | `Debug` |
| `RenderEncodeError` | `Debug`, `Display`, `Error` |
| `RenderExecutionError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |

Standard trait operations include:

| Operation | Implementing types / behavior |
| --- | --- |
| `Clone::clone(&self) -> Self` | Types marked `Clone` above. Device/queue/buffer/framebuffer handles retain shared state; `AdapterInfo` clones its owned metadata. |
| `Default::default() -> Self` | `Instance`, `CommandRecorder`; same behavior as their `new()`. |
| `Debug::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | All 33 types; buffer/framebuffer formatting exposes metadata rather than stored bytes/pixels. |
| `PartialEq::eq(&self, other: &Self) -> bool` | Types marked `PartialEq`; handle identity instead uses explicit `is_same`/`belongs_to` methods. |
| `Display::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | Ten error types and `CopyBufferRole`. |
| `Error::source(&self) -> Option<&(dyn std::error::Error + 'static)>` | All ten error types. Buffer/framebuffer creation retain descriptor/reservation sources, command/render recording retain reservation sources, and submission RenderFailed retains RenderExecutionError. Other current variants return None. |

Other inherited standard trait methods, such as `clone_from` and `ne`, follow
their standard-library contracts. Error trait objects also support standard
`is` and downcasting operations.
`Copy` and `Eq` are marker traits without methods.

## Not implemented

There are currently no public APIs for asynchronous submission/completion, textures,
samplers, shaders, graphics/compute pipelines, indexed/instanced draws, dispatch,
presentation, mapped memory, hardware discovery/drivers, managed planning,
EX Witness, or EX Proof. Resource branching is an unaccepted discussion idea,
not an implemented or locked API.

See the [roadmap](docs/ROADMAP.md) for planned capabilities and
[DOCUMENTATION.md](DOCUMENTATION.md) for the project design and experimental
directions. Future methods must not be mixed into the implemented tables.

## Maintenance and generated reference

Update this catalog whenever public exports, signatures, fields, variants,
traits, or implemented status change. Keep behavioral contracts under `docs/`
consistent with this inventory. Generated Rust documentation remains the
compiler-derived reference for exact signatures, implementations, and auto traits:

```text
cargo doc --workspace --no-deps --locked --offline
```

The public crate's generated entry point is `target/doc/ex/index.html`.
