# EX Phase 2 reference rendering

Status: implemented Phase 2 contract. Names are provisional. The software_triangle
example exports the first EX triangle. The working programmable path is documented
separately in [PIPELINES.md](PIPELINES.md); this document defines the legacy path.

## Capability and choices

An application allocates an owned framebuffer, records a render scope with an
optional clear, binds a vertex buffer, records triangle-list draws with constant
RGBA colour, finishes the scope/encoder, and submits through the same queue as
copies. EX's software implementation fetches vertices and writes real pixels.
This is the fixed-function path; programmable draws use the same render scope/queue.

[Vulkan dynamic rendering](https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdBeginRendering.html)
also establishes a rendering scope with attachments. EX starts with one owned
offscreen framebuffer and a borrowed Rust render-pass recorder. Microsoft describes
pixel-centre sampling and a top-left shared-edge rule in its
[rasterization rules](https://learn.microsoft.com/en-us/windows/win32/direct3d11/d3d10-graphics-programming-guide-rasterizer-stage-rules).
EX specifies its own small reference rasterizer rather than claiming API conformance.

## Framebuffer

`Device::create_framebuffer(FramebufferDescriptor)` allocates a fixed, initially
zero RGBA image and owns its borrowed label. Dimensions must each be 1..=16384.
The only supported format is `FramebufferFormat::Rgba8Unorm`: four stored u8
channels in R,G,B,A order. Clear and draw colours are `[u8; 4]`, with no conversion,
gamma correction, blending, or alpha test. Pixels are row-major with origin at the
top left. Allocation/capacity errors are returned; small handle allocations still
follow ordinary Rust OOM behavior. Clones share pixels and retain the device.

`read_pixels(&mut [[u8; 4]])` requires exactly width*height entries and takes the
device gate before storage. A mismatch or poisoned storage changes no destination
pixels. `pixel(x, y)` returns one checked pixel. There is no mutable mapped image.
Metadata and Debug formatting do not expose pixel storage.

## Recording and Rust scope

`encoder.begin_render_pass(&target, clear: Option<[u8; 4]>)` checks target ownership
and returns `RenderPass<'_>` exclusively borrowing the encoder. Bind using
`set_vertex_buffer(&buffer, byte_offset)`; draw using
`draw(vertices: Range<u32>, colour: [u8; 4])`. Calling `pass.finish()` consumes the
pass and appends one retained render command. It may fail to reserve a command slot;
no command is appended on failure. Dropping an unfinished pass discards all its
work. Recording, including clears, never touches pixels.

The borrow prevents nesting passes, recording copies on the same encoder while a
pass exists, and finishing the encoder with a live pass. Binding persists only
within that pass; each recorded draw retains its own binding. A failed binding or
draw leaves earlier pass work and binding unchanged. Empty finished scopes are
valid; `None` preserves old pixels and `Some(colour)` clears even without draws.

The vertex buffer must share the encoder device and have Vertex usage. HostAccess
is unrelated. The fixed layout is two little-endian f32 values, x then y, with an
8-byte stride. Binding offsets must be 4-byte aligned and at most the buffer size.
Draw ranges must be ascending and have a count divisible by three; a binding is
required even for a zero-count draw. The complete fetched range must fit using
checked arithmetic, including an empty draw's start offset. First vertex is
relative to the binding offset. No indexed draw or instance count exists yet.

## Execution and failure

The reference executor is a private working module, separate from public command
recording and queue policy. It runs under the existing device gate. Earlier copies
can upload the bytes a later render command fetches. Finished work retains live
resources; later CPU uploads affect drawing on the next submission.

Before changing a render target, a render command reserves temporary decoded
vertex storage, locks/reads each nonempty draw's source, and validates every fetched
x/y as finite. Error reports identify the command index, source label, and absolute
vertex index when applicable. Then it locks the target, reserves/copies a temporary
image, clears/draws into that image, and commits only on complete success. The
temporary image costs one extra framebuffer allocation even for legacy scopes;
reservation failure occurs before target changes. All
expected failure paths leave this render command's target unchanged; earlier
commands in the submission stay completed and later commands are skipped.
No rollback applies across commands. Zero-count draws access no vertex storage.
Poisoned target storage remains an error, including for an empty render scope.

## Coordinates, clipping, and coverage

Vertices are 2D post-projection coordinates: visible x and y range from -1 to +1,
with +y upward. No z, w, perspective division, or depth is supplied. Finite vertices
outside this square are clipped against all four boundaries using f64 convex
polygon clipping. A clipped polygon is triangulated as a fan with the same colour.
The viewport maps (-1,+1) to the top-left framebuffer boundary and (+1,-1) to
the bottom-right boundary. Boundaries are between pixels, not sample positions.

Screen positions round to a 1/256-pixel grid. Integer edge equations sample pixel
centres (x+0.5,y+0.5). Both input windings render; there is no culling. Orient each
screen triangle to positive signed area; include an exact edge sample only when
its directed edge points upward, or is horizontal and points right. This top-left
rule gives shared edges one owner. Zero-area triangles, including triangles
collapsed by clipping/subpixel rounding, write nothing. All pixel indices are
bounded to the framebuffer. Integer arithmetic fits i64 at the stated extent limit.

Draws execute in order; later covered fragments replace all four stored channels.
This legacy path has no interpolation or shader execution. No multisampling,
texture, depth/stencil, blend, scissor, configurable viewport, swapchain, or
hardware driver is implied. Clipping uses floating
arithmetic, so no bit-for-bit hardware-conformance claim is made.

## Completion gate

The public software_triangle example must export a real offscreen image, assert
representative foreground/background pixels, and demonstrate record/submit/readback.
Tests must cover extents, format/initialization, ownership, colour/clear, range and
binding misuse, winding, shared edges, clipping, degenerates, live vertices,
copy-before-draw ordering, retention, poisoned storage, and failure effects.
Update the roadmap, API catalog and architecture.

This gate is met by [software_triangle](../crates/ex-api/examples/software_triangle.rs)
and [public tests](../crates/ex-api/tests/reference_rendering.rs), plus private poison/
lifetime checks and compile-fail/executable documentation. Its PNG contains 48,556
triangle pixels in a 512x384 image and was independently decoded and visually checked.
The [example PNG writer](../crates/ex-api/examples/triangle/png.rs) serializes EX's
readback using the [PNG format](https://www.w3.org/TR/png-3/); it is not a renderer
or a public EX codec. Tests do not exhaust RAM to induce allocation failure.
