# Texture transfers and render-to-texture

Selected core contract (2026-10-09). This gate connects upload, rendering,
sampling in a later pass, and readback using real shared image storage. Implementation
evidence for this bounded gate is recorded below.

## Purpose and selected profile

CPU texture writes alone cannot express a device-side upload or readback chain.
An offscreen image must also be usable as an attachment and as a later shader
source. EX retains its independent software execution and existing descriptors.
The selected profile is one linear RGBA8 unorm 2D image, one mip level, positive
rectangular transfers, and full-image colour attachments. All such images support
sampling, transfer source/destination, and colour attachment roles. These are
fixed capabilities, not inferred host permissions or new usage flags. Future
formats may need an explicit creation capability contract.

An explicit barrier describes visibility and access; it does not copy pixels.
Buffer transfers perform actual copies. This distinction also exists in larger
graphics APIs, but this bounded EX profile has no hardware image layouts or
format conversion to manage.

## Shared storage and permissions

`Texture::as_framebuffer()` and `Framebuffer::as_texture()` return retained
full-image aliases without allocating or copying pixels. Both keep dimensions,
label, device, and allocation identity. Existing render entry points accept the
framebuffer alias, including depth, stencil, blending, and scope rollback.
Repeated conversion compares equal through `is_same`; cross-kind
`Resource::is_same` recognizes the same image too. Subrectangle views can be
sampled or transferred, but cannot be render attachments in this profile.

HostAccess remains a permission on the shared allocation. A texture with None
or Write cannot be read through its framebuffer alias; framebuffer readback
returns `ReadForbidden`. Standalone framebuffers retain their existing CPU read
behavior and expose Read access through the texture alias. Device operations
ignore host permissions and use buffer transfer usages where applicable.

A shader-used texture cannot alias the current colour attachment, even through
a disjoint view. Direct and indexed programmable draw recording rejects this
feedback with its binding slot and label; failure leaves the pass unchanged.
The check applies to empty draws too, matching existing structural binding
validation. Unused extra bindings and fixed draws do not sample and are allowed.
Use separate images and a later pass for feedback effects.

## Transfers and validation

`copy_buffer_to_texture`, `copy_texture_to_buffer`, and `copy_texture` record
retained live sources. Buffer/image payloads retain a buffer, its
`TextureBufferLayout { offset, bytes_per_row }`, and a validated `TextureView`.
Image/image payloads retain two views of equal dimensions. Existing views are
positive and in bounds, so there is no zero-area transfer command.

Buffer layout offsets and strides are in bytes, multiples of four. Stride is at
least `width * 4`, including one-row copies. The exclusive end is checked as
`offset + (height - 1) * bytes_per_row + width * 4`; overflow or exceeding buffer
size fails recording. Final-row padding need not exist. Upload ignores padding;
readback preserves padding and all bytes outside copied row spans. Image copies
preserve all texels outside the destination view, perform no filtering, and
reject overlapping rectangles in the same allocation. Disjoint self-copies are
supported with a source snapshot and never lock the same storage twice.

Device, buffer usage, layout, extent, overlap, and command reservation failures
append nothing and touch no contents. Immutable command descriptions cannot be
fabricated publicly. Submission serializes under the device gate and reads live
source contents; command buffers can be reused after updates.

## Dependencies and failure boundaries

Transfers contribute TransferRead/TransferWrite on whole allocations. Disjoint
self-copies combine into TransferReadWrite. Texture and framebuffer aliases
permit the same image roles; declaring one alias covers the other. Explicit
mode requires transitions for upload-to-render, render-to-sample, and
render-to-readback hazards. Duplicate aliases in one transition batch fail.
Missing declarations or dependencies reject the entire submission before any
operation executes. Ordered mode retains synchronous command visibility.

Transfer poison or temporary snapshot allocation failures leave the failed
destination unchanged, preserve the completed command prefix, and skip later
commands. Transfer errors include command index through QueueSubmitError and
storage role/kind/label when applicable. Render-to-texture uses the existing
whole-scope colour/depth/stencil atomic commit. Public CPU accesses share the
same device gate. No production path reacquires that gate inside execution.

## Limits

No mipmaps, layers, cubes, 3D images, compressed/sRGB/depth image transfers,
storage images, resolve/blit/filter copies, subrectangle attachments, asynchronous
completion, hardware backend, or new SDK is included. Large image-copy snapshots
are fallibly reserved; small Arc allocations and error labels follow ordinary
Rust allocation behavior. Whole-resource dependencies deliberately remain more
conservative than rectangle overlap checks.

## Tests and examples

Implemented in [transfers](../crates/expansex-core/src/transfer.rs),
[texture storage](../crates/expansex-core/src/texture.rs),
[attachment aliases](../crates/expansex-core/src/framebuffer.rs),
[feedback validation](../crates/expansex-core/src/render/textures.rs), and
[dependencies](../crates/expansex-core/src/dependency.rs).
Fifteen [public tests](../crates/ex-api/tests/texture_flow.rs), two
[internal tests](../crates/expansex-core/src/buffer/texture_transfer_tests.rs),
and executable alias documentation cover rows/rectangles, padding, overflow,
permissions, device roles, live reuse, identity, feedback, alias barriers,
preflight rejection, six poisoned-endpoint cases, retention, and render rollback
across shared colour/depth/stencil storage. The workspace passes 245 tests.
Allocation failure branches exist; tests do not exhaust host memory to induce OOM.

The [public workload](../crates/ex-api/examples/texture_flow.rs) uses HostAccess::None
images, padded upload, an atlas rectangle copy, compute-generated geometry,
shared render attachments, stage-checked texture/uniform groups, two render
passes, and padded buffer readback. Its 13 recorded commands and 17 combined
resource uses validate in Explicit mode. Both submissions check all 204,800
pixels and padding bytes after changing only upload data. Independent PNG CRC,
dimension, decompression and pixel checks verify the final 640x320 image,
including 131,072 panel pixels. The PNG was visually inspected. The right panel
is vertically flipped using a vertex uniform; the first pass renders the image
that both second-pass panels sample. This proves this gate, not the final
supported-profile/backend completion gate or stable v1.
