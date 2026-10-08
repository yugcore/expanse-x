# EX current API contract

Status: Phase 2 reference rendering complete; public names and behavior are provisional.
Applications depend on package `ex-api` and import its library as `ex`. The
implementation remains in `expansex-core`; the public crate re-exports its types.
This contract describes implemented behavior only. See [architecture](ARCHITECTURE.md)
for planned runtime responsibilities.

The [public API catalog](../api.md) lists every implemented public type, method,
field, enum variant, and explicit standard trait implementation. This document
describes their behavioral contracts.

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
synchronous CPU operations, independent of device-side transfer roles. They
serialize against whole submissions through the shared device execution gate.
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
Recording and finishing do not read/write bytes or execute work.
See [the recording contract](COMMANDS.md) for precedence,
ownership, errors, and limits.

```text
cargo run -p ex-api --example record_copy --locked --offline
```

## Synchronous reference submission

`queue.submit(&commands)` returns `Result<(), QueueSubmitError>`. Finished work
must belong to the queue's exact device, including empty work. Commands execute
in order, read live source bytes, and complete before successful return. Work is
borrowed and reusable. Device-side transfers ignore public CPU host permissions.

All valid CPU reads/writes and whole submissions on a device serialize through
its shared gate. Concurrent arrival order is unspecified. Same-buffer copies take
one storage lock; distinct-buffer copies lock source and destination under that
gate. Empty copies access no storage. Foreign submission changes nothing;
poisoned storage reports a zero-based failed command and role, preserves earlier
results, leaves the failing destination unchanged, and skips later commands.
There is no rollback, asynchronous execution, or hardware completion primitive.
See [the submission contract](SUBMISSION.md) for exact lock and failure rules.

```text
cargo run -p ex-api --example submit_copy --locked --offline
```

## Framebuffers and render scopes

`device.create_framebuffer(FramebufferDescriptor { label, width, height, format })`
returns owned, zeroed, shared RGBA pixels. Each dimension is 1..=16384; format is
currently Rgba8Unorm. Metadata/identity getters mirror buffer handles. `pixel(x,y)`
checks a top-left-origin pixel, and `read_pixels(&mut [[u8;4]])` requires the exact
pixel count. Readback serializes against whole submissions; errors leave caller
storage unchanged.

`encoder.begin_render_pass(&target, Option<[u8;4]>)` returns an exclusive borrowed
RenderPass. Bind a Vertex-usable buffer using `set_vertex_buffer(&buffer, offset)`;
its fixed layout is little-endian f32 x/y pairs, stride 8 bytes, aligned to 4.
`draw(first..end, colour)` requires an ascending range with count divisible by
three and a checked full byte extent. `pass.finish()` consumes/commits the scope;
dropping it discards pending work. Clears and draws do not execute while recording.

Submission runs Command::RenderPass alongside copies. A private working reference
module reads live finite positions, clips 2D triangles, maps the viewport, samples
pixel centres with a top-left rule, and writes constant RGBA fragments. A render
command preflights all vertices before changing its target; RenderFailed retains
the cause while earlier commands stay completed and later commands are skipped.
See [RENDERING.md](RENDERING.md) for exact coverage, lifetime, and limits.

```text
cargo run -p ex-api --example software_triangle --locked --offline
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
and executable/compile-fail encoder documentation. Nine public submission tests
and five internal execution tests check results, order, reuse, lifetime, aliasing,
foreign work, concurrency, poisoning, partial failure, and gate recovery.
Fifteen public rendering tests and four internal storage/lifecycle checks cover
framebuffers, draws, clipping, edges, winding, degenerates, misuse, live sources,
and failure effects. Executable rendering documentation and two render-scope
compile-fail examples verify public usage. The workspace runs 79 tests, including
documentation tests. Tests do not induce allocator OOM by exhausting host memory.

## Next contract

Phases 1 and 2 are complete: copies and fixed-function triangles execute through
EX and produce observable bytes/pixels. Next define EX shader representation,
stage interfaces, pipeline validation, and programmable reference execution.
There is no implemented execution-backend ABI yet.
