# EX current API contract

Status: Phase 1 buffer-copy recording; public names and behavior are provisional.
Applications depend on package `ex-api` and import its library as `ex`. The
implementation remains in `expansex-core`; the public crate re-exports its types.
This contract describes implemented behavior only. See [architecture](ARCHITECTURE.md)
for planned runtime responsibilities.

## Discovery and logical ownership

`Instance::new()` exposes one built-in software reference adapter via `adapters()`
and `default_adapter()`. `Adapter::info()` reports its name and backend.
`create_device()` creates a distinct logical device; `default_queue()` returns
that device's one endpoint. Device and queue clones share ownership. A queue can
return its original device via `device()` and test membership via `belongs_to()`.
`Device::is_same()` compares logical identity, not equal adapter metadata.

Owned child handles remain valid after parent handles drop; the final owner
releases shared state. Discovery/device constructors perform no driver calls or
work execution and have no recoverable EX error path. Buffer allocation is
fallible, as described below. Read the
[object contract](OBJECT-MODEL.md) for borrowing, cleanup, and design decisions.

```text
cargo run -p ex-api --example discover --locked --offline
```

This example demonstrates discovery and retained ownership only. The label
recorder below is not yet connected to a device or queue.

## Buffer descriptions

`BufferDescriptor<'a>` borrows a label and a usage slice. `size` is a `u64` byte
count; `host_access: HostAccess` explicitly chooses CPU read/write permissions.
Multiple roles are permitted; duplicate roles have no extra meaning. An
empty label is valid. `validate()` rejects zero size first, then missing usages.
`BufferDescriptorError` exposes `ZeroSize` and `MissingUsage`, implements
`Display`, and implements `std::error::Error`.

Successful validation means only those two rules passed. It does not establish
backend support, allocation success, data initialization, or synchronization.
The descriptor contains no bytes and owns no resource storage. Its derived
`Copy` implementation copies values and shared references. Creation copies the
borrowed metadata into the new resource.

## Allocated buffers

`device.create_buffer(descriptor)` returns `Result<Buffer, BufferCreateError>`.
It creates fixed, zero-initialized reference storage and retains the logical
device. Clones share one allocation. `Buffer::is_same()` compares allocations;
`belongs_to()` compares the creating device; `device()` retains that original
device. Borrowed metadata getters expose label, size, usages, and host access.

`write(offset, &[u8])` copies CPU bytes into storage. `read(offset, &mut [u8])`
copies into caller-owned memory. Both require their respective host permission,
validate the full byte range with overflow checks, and lock private storage
through the copy. Errors never partially modify the copy destination. They are
synchronous CPU operations, independent of device-side transfer roles and queues.
See [the buffer contract](BUFFERS.md) for permissions, creation errors, lifetime,
limits, and the intentional new descriptor field.

```text
cargo run -p ex-api --example buffer_roundtrip --locked --offline
```

## Device-bound copy recording

`device.create_command_encoder()` returns an owned `CommandEncoder` in that
device's domain. `copy_buffer(&source, source_offset, &destination,
destination_offset, size)` returns `Result<(), CommandEncodeError>`. It validates
both devices, both transfer usages, checked ranges, and same-allocation overlap
before appending a `Command::CopyBuffer(BufferCopy)` that retains both resources.
No host permission or four-byte alignment is required. Zero-byte copies at valid
end offsets are allowed. Errors append nothing and leave previous work intact.

`finish(self)` consumes the encoder and returns a device-bound `CommandBuffer`.
Both expose `device()`, `belongs_to()`, and borrowed `commands()`. `BufferCopy`
exposes `source()`, `destination()`, `source_offset()`, `destination_offset()`, and
`size()`. The finished sequence is immutable and not cloneable. Buffers stay live,
not snapshotted; changes through other permitted CPU handles remain observable.
Recording and finishing do not read/write bytes or execute work. There is no
submission API yet. See [the recording contract](COMMANDS.md) for precedence,
ownership, errors, source-sampling plans, and limits.

```text
cargo run -p ex-api --example record_copy --locked --offline
```

## Standalone prototype labels

`CommandRecorder` owns a `Vec<String>`. `new()` and `default()` start an empty
recording. `insert_debug_marker(&mut self, &str)` stores an owned label copy.
`finish(self)` consumes the recorder and moves the sequence into `DebugMarkerBuffer`.
`debug_markers(&self)` returns a borrowed slice in insertion order. Empty command
buffers are allowed; finished buffers are not cloneable in this prototype.

Only labels exist. Finishing never submits, allocates backend command storage, or
executes graphics work. Rust prevents recording through a finished buffer and
using a consumed recorder, but establishes no GPU completion guarantee.

## Verification

Five core unit tests cover descriptor errors, accepted multiple usages, label
ownership/order, and empty recording. A compile-fail documentation test checks
recorder reuse after finishing. A public `ex` documentation example exercises
description validation and recording. The application example runs through `ex`:

```text
cargo run -p ex-api --example api_draft --locked --offline
```

It prints a validated description and stored labels, then explicitly reports
that no GPU memory or execution occurred. A label named 'draw triangle' is text.

Five public object-model integration tests cover discovery, identity, membership,
and handles surviving parent drops. One internal lifecycle test checks final
state cleanup. A second public documentation example exercises discovery.
Ten public buffer tests cover allocation metadata/initialization, permissions,
range safety, shared identity, and CPU alias access. Three internal resource
tests verify storage/device retention and poisoned-lock behavior. A third public
documentation example exercises allocation and readback. The complete workspace
now also has ten public command-recording tests, three additional lifecycle tests,
and executable/compile-fail encoder documentation. It runs 43 tests, including
documentation tests. Tests do not induce allocator OOM by exhausting host memory.

## Next contract

Next, Phase 1 must execute the recorded copies through queue submission and
establish submission order, completion visibility, host concurrency, execution
failure behavior, and resource retention through completion.
Implement and test a small CPU execution path before adding rasterization.
There is no implemented execution-backend ABI yet.
