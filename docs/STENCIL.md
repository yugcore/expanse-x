# Stencil testing

Stencil tests mark pixel regions and condition later draws on
those marks. Unlike depth's distance comparison, stencil is an unsigned bit mask
for clipping, nested masks, and counters. Existing APIs provide [stencil comparisons
and fail/depth-fail/pass operations](https://docs.vulkan.org/spec/latest/chapters/fragops.html#fragops-stencil).
EX implements its own CPU path with separate S8 storage and immutable Rust state;
no packed depth/stencil format or hardware conformance is implied.

- StencilBuffer owns one u8 per pixel, initially zero, with top-left row-major
  layout, device identity, extents 1..=16384, and synchronized full/per-pixel
  readback. No public upload, texture sampling, or arbitrary bit width.
- StencilState has front/back StencilFaceState values plus shared u8 reference,
  read_mask, and write_mask. Faces specify CompareFunction, fail_op,
  depth_fail_op, pass_op. All combinations are valid. Existing options and
  descriptor literals remain compatible: create_graphics_pipeline_with_stencil
  takes (descriptor, GraphicsPipelineOptions, StencilState); all prior creators
  disable stencil. GraphicsPipeline::stencil_state returns an optional copy.
- StencilState::replace(reference) uses Always/Keep/Keep/Replace, all mask bits,
  both faces. test_equal(reference) uses Equal/all Keep/all read bits/no writes.
- RenderAttachments borrows optional depth/stencil handles and optional clears.
  begin_render_pass_with_attachments(target, clear_colour, attachments) retains
  handles after checks: colour device; depth device/extent/finite clear; stencil
  device/extent. A clear without its attachment returns MissingDepthBuffer or
  MissingStencilBuffer. Old begin methods delegate with no stencil. None clears
  preserve live values. Clear ignores pipeline masks. RenderCommands exposes
  stencil_buffer and clear_stencil. Even empty stencil-enabled programmable draws
  need their attachment, checked after depth and before texture requirements.
- Compare (reference & read_mask) against (stored & read_mask), in that order,
  with all eight existing comparison functions. These integer values are exact.
  Fail updates stencil with fail_op, then skips depth/shading/colour. On stencil
  success but depth failure, apply depth_fail_op and skip shading/colour. If depth
  is disabled it counts as passed. Passing successful shading applies pass_op,
  selected depth writes, and colour replacement/blending. Preflight still runs;
  shader failure discards the entire scope, including earlier stencil updates.
- Operations: Keep, Zero, Replace (unmasked reference), IncrementClamp,
  DecrementClamp, Invert, IncrementWrap, DecrementWrap. Increment/decrement operate
  on the full stored u8 before masking; clamp at 0/255 or wrap modulo 256.
  Store (old & !write_mask) | (operation_result & write_mask). A zero write mask
  does not disable stencil comparison. Colour masks/zero alpha do not disable it.
- Facing uses each rasterized clipped triangle's quantized signed screen area
  before winding normalization. Positive area is front (clockwise with top-left
  screen coordinates; counterclockwise in clip x/y). Negative selects back.
  Degenerate triangles update nothing. Both windings render; no configurable
  front-face convention or culling exists. Clipped fan pieces use the same rule.
- Colour, optional depth, and optional stencil stage/commit together while holding
  distinct storage guards under the shared device gate. Any expected failure
  preserves all failed-scope attachments/clears; prior commands remain complete
  and later commands skip. Empty/clear-only scopes check attached storage poison.
  Stencil staging costs one additional byte per pixel. No per-fragment allocation
  or storage lock is introduced. Reservations are fallible, not universal OOM recovery.
- Stencil-disabled programmable and legacy fixed draws ignore stencil; explicit
  clears still apply. There is no dynamic reference/state, shader-written stencil,
  packed depth/stencil, attachment sampling, MSAA, or installed-GPU execution.

[Twelve public tests](../crates/ex-api/tests/stencil.rs) cover allocation/readback,
recording misuse/empty draws, all comparisons/operations, bit masks, rejected
fragments, facing/clipped fans, retention/reuse, indexed/live textures, depth/blend
integration, legacy compatibility, and late rollback of all three attachments.
Two internal tests cover poison and storage lifetimes. Combined public
documentation exercises stencil, depth, and blending through the exported API.
All 177 workspace tests and formatting/build/lint/documentation checks pass.

The [example](../crates/ex-api/examples/stencil_mask.rs) marks an invisible diamond,
then draws a repeating translucent checkerboard only where stencil equals one.
Readback verifies colour/depth/stencil correspondence for every pixel. Independent
PNG checksums, geometry, nearest-repeat sampling, and rational alpha calculations
match all 196,608 pixels: 66,560 masked pixels, evenly split between the two colours.
The image was also inspected. Sources: [storage](../crates/expansex-core/src/stencil.rs),
[rules](../crates/expansex-core/src/pipeline/stencil.rs),
[recording](../crates/expansex-core/src/render.rs),
[staging](../crates/expansex-core/src/reference.rs), and
[rasterizer](../crates/expansex-core/src/reference/programmable.rs).
