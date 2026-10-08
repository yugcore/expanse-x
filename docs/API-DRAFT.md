# EX current API contract

Status: Phases 0–3 complete; Phase 4 sampled textures work. Names and behavior are provisional.
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

## Independent shader evaluation

`Device::create_shader(ShaderDescriptor)` validates and owns a vertex or fragment
program. All values are four-component f32 vectors; result register numbers are
instruction indices. Input, Constant, Add, Multiply, Dot4, and Swizzle execute
through `Shader::evaluate` with bounded local registers. Descriptors require at
most 16 inputs and 256 instructions; vertex programs return position and up to
16 varyings, fragment programs return one RGBA vector. Only earlier registers
can be referenced. All constants, inputs, and evaluated results must be finite.

Clones share immutable source and retain the creating device. Evaluation checks
input count/numbers and arithmetic before returning owned raw vectors; errors
return no partial output or resource mutation. Calls need no device gate and
can execute concurrently. No buffer access, stage linking, projection, colour
conversion, or rendering occurs here. ShaderCreateError retains validation or
reservation sources; ShaderExecutionError reports invocation/numeric failures
and retains output reservation errors. See [SHADERS.md](SHADERS.md).

```text
cargo run -p ex-api --example shader_evaluation --locked --offline
```

## Graphics pipelines and programmable draws

`Device::create_graphics_pipeline(GraphicsPipelineDescriptor)` retains same-device
vertex/fragment shaders, copies a validated single-buffer vertex layout, and checks
varying output/input counts. Float32/x2/x3/x4 attributes expand into four-component
vectors, with missing y/z zero and missing w one. Layout locations cover every
vertex input exactly once; offsets/stride align to four and attributes fit stride.

`RenderPass::set_pipeline` checks device/target format. `draw_programmable(range)`
requires current pipeline/buffer bindings and checks the full stride range. Draws
retain their own pipeline and live source. Legacy `draw(range,colour)` keeps the
fixed layout/colour even with a pipeline bound. Vertex operations produce clip
position and varyings; positive w is required. Six-plane homogeneous clipping,
division/viewport mapping, and perspective interpolation feed fragment operations.
Finite RGBA clamps/rounds to bytes and overwrites covered samples.

Each scope reserves a temporary image, shades into it, and commits only after
every draw succeeds. Late numeric failure leaves the target unchanged, including
scope clear/prior draws. Earlier submission commands still remain completed.
This costs one additional full image; no per-fragment heap allocation occurs.
See [PIPELINES.md](PIPELINES.md) for errors, limitations, and verification.

```text
cargo run -p ex-api --example programmable_triangle --locked --offline
```

## Indexed programmable draws

`RenderPass::set_index_buffer(&Buffer, u64, IndexFormat)` validates same-device
ownership, Index usage, byte offset, and 2/4-byte format alignment.
`draw_indexed(Range<u32>, i32)` requires pipeline, vertex, and index bindings;
validates a complete triangle list and index byte extent; and retains the bindings.
`DrawTriangles::indexed()` exposes `IndexedTriangles` with buffer/offset/format/
range/base getters. Its legacy vertex range is unused 0..0.

At submission, decode live little-endian u16/u32 elements and add the signed base
in i64. Each result must fit u32 and address a complete stride-sized record.
Only referenced vertices execute; repeated references execute repeatedly. Release
index storage before borrowing vertices, so both roles may share one allocation.
Empty draws skip both source locks. Expected index or shading failures discard
the entire scope image; earlier submitted commands remain completed.
See [INDEXED-DRAWING.md](INDEXED-DRAWING.md) for errors and limitations.

```text
cargo run -p ex-api --example indexed_quad --locked --offline
```

## Texture resources and fragment sampling

Device::create_texture allocates a zeroed RGBA8 image with dimensions 1..=16384
and explicit HostAccess for full-image CPU write_pixels/read_pixels. A TextureView
retains a full image or validated positive rectangle. Device::create_sampler owns
immutable nearest/bilinear filtering and independent U/V clamp/repeat addressing.

Fragment SampleTexture reads XY of an earlier register as normalized view-local
UVs. Shader creation validates stage, slot 0..8, and prior reference. Standalone
Shader::evaluate returns MissingTextureBinding at sampling; TextureView::sample
samples live storage synchronously. RenderPass::set_texture binds a same-device
view/sampler pair, and programmable direct/indexed draws require/retain every
shader-used slot, even for empty draws. Existing shader/pipeline descriptors remain
compatible. Finished DrawTriangles exposes retained pairs through texture(slot).

Submission snapshots used view rectangles once per nonempty draw, under the
device gate and sequential storage locks. Empty draws skip texture locks; fragment
sampling allocates/locks nothing per pixel. Texture poison and numeric/reservation
failure preserve the entire failed scope. The extra memory is proportional to
used view areas per draw. No texture transfers, attachments, mipmaps, or general
resource bind groups exist yet. See [TEXTURES.md](TEXTURES.md).

```text
cargo run -p ex-api --example textured_quad --locked --offline
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
compile-fail examples verify public usage. Nine public shader tests, two internal
lifetime checks, and executable shader documentation verify operations, invalid
programs, finite arithmetic, interfaces, owned source, identity, and concurrency.
Seventeen public programmable-rendering tests, one pipeline lifetime test, and
executable pipeline documentation verify transforms, layouts, stage compatibility,
perspective/clipping/edges, numeric errors, atomic scopes, order, reuse, and retention.
Ten public indexed-rendering tests, two internal poison/lifetime tests, and the
indexed public documentation example verify formats, offsets/base, live copies,
aliases, empty draws, errors, retention, ordering, and atomic failure. The quad
example uses four shared vertices; independent decoding and pixel calculations
verify its output. Twelve texture tests, three internal poison/lifetime/gate
tests, and textured public documentation verify resource/sampling/rendering
contracts. Independent checkerboard pixel calculations verify the example image.
The workspace runs 137 tests, including documentation tests.
Tests do not induce allocator OOM by exhausting host memory.

## Next contract

Phases 1–3 are complete: copies and fixed-function triangles execute through
EX and produce observable bytes/pixels. Shader representation, stage interfaces,
and programmable execution now work through pipeline validation, vertex layouts,
interpolation, and indexed drawing. Sampled textures, views, samplers, and eight
fragment resource slots also work. Depth testing is next; language/format import
remains future work.
There is no implemented execution-backend ABI yet.
