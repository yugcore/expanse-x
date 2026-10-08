# EX public API catalog

Updated: 2026-10-08. Scope: every currently exported EX type, inherent method,
public field, enum variant, and explicit standard trait implementation.
There are **21 public types and 39 inherent methods**. These are provisional
Phase 1 APIs, not a completed or stable v1 specification.

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
| Prototype labels | [CommandRecorder](#commandrecorder), [DebugMarkerBuffer](#debugmarkerbuffer) |
| Errors | [BufferDescriptorError](#bufferdescriptorerror), [BufferCreateError](#buffercreateerror), [BufferAccessError](#bufferaccesserror), [CommandEncodeError](#commandencodeerror) |

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
| `BackendKind::Reference` | EX's own CPU/software target; rendering is not implemented yet. |

External matches need a fallback arm because more variants can be added.

### Device

`ex::Device` retains one logical device. Its fields are private. Methods are
implemented across [object.rs](crates/expansex-core/src/object.rs),
[buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs), and
[command.rs](crates/expansex-core/src/command.rs).

| Method | Behavior |
| --- | --- |
| `Device::adapter(&self) -> Adapter` | Retain the selected adapter. |
| `Device::default_queue(&self) -> Queue` | Retain the device's one default endpoint; starts no work. |
| `Device::is_same(&self, other: &Self) -> bool` | Compare logical device identity. |
| `Device::create_buffer(&self, descriptor: BufferDescriptor<'_>) -> Result<Buffer, BufferCreateError>` | Validate and allocate owned zero-initialized reference storage. |
| `Device::create_command_encoder(&self) -> CommandEncoder` | Create an empty encoder bound to this device. |

Cloning retains the same domain; `Adapter::create_device()` creates a new one.
Discovery/device/encoder constructors do not expose a recoverable allocation
error path. Buffer creation has its documented fallible reservation path.

### Queue

`ex::Queue` retains the default endpoint's device. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Queue::device(&self) -> Device` | Retain the original owning device. |
| `Queue::belongs_to(&self, device: &Device) -> bool` | Test device membership. |

There is no `Queue::submit` or execution/completion API yet.

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
descriptor; only transfer roles currently participate in command recording.

| Variant | Role |
| --- | --- |
| `BufferUsage::Vertex` | Future vertex input. |
| `BufferUsage::Index` | Future indexed draw input. |
| `BufferUsage::Uniform` | Future shader parameters. |
| `BufferUsage::Storage` | Future shader-accessible data. |
| `BufferUsage::TransferSource` | Required source role for recorded copies. |
| `BufferUsage::TransferDestination` | Required destination role for recorded copies. |

A usage flag alone does not implement drawing, shaders, or copy execution.

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

Clones share live bytes. Reads/writes are synchronous and serialize per
allocation through private storage. Rejected accesses do not partially change
their destination. Empty accesses require permission and an offset at or before
the end. Transfer usages do not grant CPU access. No mapping or public mutable
slice is exposed; CPU copies do not establish queue synchronization.

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
Submission and execution-time source sampling are not implemented yet.

### Command

`ex::Command` is non-exhaustive and has no inherent methods.

| Variant | Payload |
| --- | --- |
| `Command::CopyBuffer(BufferCopy)` | One validated buffer-copy description. |

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

## Explicit standard trait support

This lists implemented/derived traits, not Rust's blanket implementations or
auto traits. They are separate from the 39 inherent methods above.

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

Standard trait operations include:

| Operation | Implementing types / behavior |
| --- | --- |
| `Clone::clone(&self) -> Self` | Types marked `Clone` above. Device/queue/buffer handles retain shared state; `AdapterInfo` clones its owned metadata. |
| `Default::default() -> Self` | `Instance`, `CommandRecorder`; same behavior as their `new()`. |
| `Debug::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | All 21 types; buffer formatting exposes metadata rather than stored bytes. |
| `PartialEq::eq(&self, other: &Self) -> bool` | Types marked `PartialEq`; handle identity instead uses explicit `is_same`/`belongs_to` methods. |
| `Display::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | Four error types and `CopyBufferRole`. |
| `Error::source(&self) -> Option<&(dyn std::error::Error + 'static)>` | All four error types. Creation's invalid-descriptor/reservation variants and encoding's reservation variant retain a source; other current variants return `None`. |

Other inherited standard trait methods, such as `clone_from` and `ne`, follow
their standard-library contracts. Error trait objects also support standard
`is` and downcasting operations.
`Copy` and `Eq` are marker traits without methods.

## Not implemented

There are currently no public APIs for queue submission/completion, textures,
samplers, shaders, graphics/compute pipelines, draw/dispatch, render passes,
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
