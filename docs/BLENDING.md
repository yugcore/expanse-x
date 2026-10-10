# Colour blending

Blending combines a passing fragment's colour with the current
pixel, enabling transparency and additive effects in EX's own renderer. Existing
APIs offer [separate RGB/alpha factors and operations](https://docs.vulkan.org/spec/latest/chapters/framebuffer.html#framebuffer-blending).
EX uses that arithmetic concept with immutable Rust values and one RGBA8 target;
it does not delegate rendering to another API or claim hardware conformance.

- `GraphicsPipelineOptions { depth, blend }` holds optional DepthState/BlendState.
  `create_graphics_pipeline_with_options(descriptor, options)` uses the existing
  validation/ownership path. Original creation and depth-only creation retain
  their behavior and descriptor compatibility; both disable blending. Default
  options disable depth and blending. `blend_state()` copies optional state.
- BlendState has independent `colour` and `alpha` BlendComponent values and an
  RGBA `[bool;4]` write_mask. Each component selects source/destination factors
  and an operation. Factors: Zero, One, SrcColour, OneMinusSrcColour, DstColour,
  OneMinusDstColour, SrcAlpha, OneMinusSrcAlpha, DstAlpha, OneMinusDstAlpha,
  SrcAlphaSaturate. Saturate is min(src.a,1-dst.a) for RGB and 1 for alpha.
  Colour factors use the current lane (including alpha for the alpha equation).
- Add computes src*src_factor + dst*dst_factor. Subtract subtracts destination
  from source; ReverseSubtract reverses that. Min/Max take the original source
  and destination lane and ignore factors. All state combinations are valid;
  no numeric constants, dynamic state, or new validation error is necessary.
- Clamp finite shader output to [0,1] before factor calculation; normalize the
  staged destination bytes by 255 for factor calculation. Evaluate equations in
  f64 byte units (source*255, destination byte), clamp to [0,255], then round to
  the nearest byte (represented positive ties up). This avoids a needless
  normalize/denormalize round trip for destination colour. Arbitrary f64 equations
  still have ordinary rounding limits; no exact rational or GPU-bitwise promise.
  Apply the write mask last, preserving disabled destination bytes exactly.
  Quantize after each fragment; subsequent fragments see those stored bytes.
  No sRGB/gamma conversion or automatic premultiplication occurs.
- None blend uses the existing f32 conversion/overwrite path unchanged. Some
  state always applies its equation and mask, including One/Zero replacement.
  Masks do not affect clears, fragment evaluation/errors, or enabled depth writes.
  Direct and indexed programmable draws, interpolated colours, and sampled
  textures share this path. Legacy fixed-colour draws continue overwriting.
- Early depth rejection skips shading/blending/all writes. Passing fragments
  shade, blend against staged colour, and update depth when enabled. Alpha zero
  does not discard geometry or suppress depth writes. For conventional transparent
  surfaces, test depth with writes disabled and draw back to front; EX does not
  sort surfaces or implement order-independent transparency.
- `BlendState::alpha_blending()` selects RGB SrcAlpha/OneMinusSrcAlpha and alpha
  One/OneMinusSrcAlpha, Add, all channels. This source-over preset assumes straight
  source RGB; on a transparent target its composited RGB is associated with alpha.
  `premultiplied_alpha()` uses One/OneMinusSrcAlpha for both equations, expecting
  source RGB already multiplied by alpha. `additive()` uses One/One for both.
- State is copied into the retained immutable pipeline. Existing whole-scope
  staging and atomic colour/depth commit guarantees apply without new allocations
  or storage locks per fragment. Failure discards all scope clears/blended writes.
- No blend constants, dual-source factors, logic operations, multiple render
  targets, floating/sRGB targets, multisampling, or GPU execution yet.

The [public tests](../crates/ex-api/tests/blending.rs) cover every factor/operation,
all 16 masks, presets, clamping/quantization, ordering, depth/masks/zero alpha,
indexed/live-texture integration, varying alpha, legacy compatibility, retained
state, validation, and scope rollback. Executable public documentation demonstrates
alpha blending with depth testing and disabled depth writes.

The [comparison example](../crates/ex-api/examples/blended_triangles.rs) writes a
768x384 image: replace RGB while preserving destination alpha (left), source-over
(middle), additive (right). All panels draw identical geometry back to front with
depth writes disabled. Readback assertions verify full depth preservation, opaque
alpha, and selected composited pixels. Independent PNG checksums, coverage, and
rational blending calculations verify all 294,912 pixels, with 42,677 covered per
panel (128,031 total); the image was also visually inspected.
The blending checkpoint passed 163 workspace tests, including twelve blending tests and the new public
depth/blend documentation example. Formatting, build, lint, and documentation
checks also pass. Existing depth and texture tests remain passing.
