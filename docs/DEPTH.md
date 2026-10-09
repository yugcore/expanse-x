# Depth testing

determine visible surfaces by depth rather than draw
order, through EX's own software pipeline. Existing APIs likewise separate
[depth testing](https://docs.vulkan.org/spec/latest/chapters/fragops.html) from
colour output. EX retains a separate depth attachment and immutable pipeline state.

- DepthBuffer stores one f32 in [0,1] per pixel, initially 1, with top-left
  row-major layout, extents 1..=16384, device identity, and synchronized readback.
  It is fixed D32 float storage; no packed stencil component, CPU upload, or texture sampling.
- DepthState selects one of eight comparisons and whether passing samples write
  depth. `create_graphics_pipeline_with_depth(descriptor, state)` adds state while
  existing pipeline descriptor literals and creation remain compatible.
- `begin_render_pass_with_depth(target, clear_colour, depth, clear_depth)` checks
  colour/depth ownership, equal extents, and optional finite depth clear in [0,1].
  None preserves stored depth. Each scope retains its attachment and clear.
  Programmable direct/indexed depth-enabled draws require an attachment, even
  empty draws. Depth-disabled and legacy draws ignore depth; explicit clears apply.
- Existing homogeneous clipping uses -w <= z <= w. After division, viewport depth
  is z/w*0.5+0.5. Interpolate these depths linearly with screen barycentric weights
  on the same quantized coverage grid; depth is not a perspective varying.
  Clamp rounding drift to [0,1], convert to f32, then compare with stored f32.
  Equal/NotEqual are exact; Less uses smaller as nearer. Greater allows reverse-Z.
- Test occurs before fragment shading. Failed comparisons skip the fragment shader
  and all writes. Successful shading overwrites or blends colour, then writes depth only
  when enabled. Vertex/resource preflight still executes before rasterization,
  including resources used only by occluded geometry.
- Stage both colour and depth. Hold their distinct storage locks under the device
  gate until both commits finish. Any expected poison/allocation/shading error
  preserves both attachments, including clears/prior draws in the failed scope.
  Earlier commands remain complete and later commands skip. Readback/submissions
  serialize on the same gate. Empty/clear-only scopes still check attachment poison.
- No depth bias/clamp options, shader-written depth, alternate
  viewport depth range, multisampling, attachments as textures, or hardware claims.
  Temporary depth storage adds four bytes per target pixel to staged colour/vertices/
  texture snapshots. Large reservations are fallible, without universal OOM recovery.

Implemented verification covers draw-order-independent overlap, comparisons/equality, write-off,
preserved/cleared depth, indexed/textured integration, projected depth interpolation,
clipping, missing/foreign/mismatched attachments, invalid clears, atomic failure,
poison/lifetime, and an independently verified offscreen overlap example.
[Eleven public tests](../crates/ex-api/tests/depth.rs) and two internal tests pass
with all 150 workspace tests at the depth checkpoint. The [example](../crates/ex-api/examples/depth_overlap.rs)
verifies identical colour/depth readback in both orders and writes a 512x384 PNG
with 85,434 covered pixels (27,648 red, 32,146 green, 25,640 blue). Independent
PNG checksum/coverage/visibility calculations verify every pixel; the image was
also visually inspected. Sources: [resource](../crates/expansex-core/src/depth.rs),
[pipeline](../crates/expansex-core/src/pipeline.rs), [recording](../crates/expansex-core/src/render.rs),
[scope commits](../crates/expansex-core/src/reference.rs), and
[rasterizer](../crates/expansex-core/src/reference/programmable.rs).
