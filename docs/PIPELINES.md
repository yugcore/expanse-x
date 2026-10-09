# Graphics pipelines and programmable triangles

This step connects owned EX shaders to recorded draws. A pipeline retains a
vertex shader, fragment shader, explicit single-buffer vertex layout, and target
format. The existing fixed-function `draw(range, colour)` remains available;
`set_pipeline` plus `draw_programmable(range)` selects programmable execution.
Both use the same resources, render scopes, queue, and completion contract.

Established APIs also separate vertex layout and shader stages from draws
([Vulkan vertex inputs](https://docs.vulkan.org/spec/latest/chapters/fxvertex.html),
[shader interfaces](https://docs.vulkan.org/spec/latest/chapters/interfaces.html),
[interpolation](https://docs.vulkan.org/spec/latest/chapters/primsrast.html)).
EX uses its own small contract and CPU executor, with safe Rust retained handles.

## Pipeline behavior

- `GraphicsPipelineDescriptor` borrows label, shader handles, and a
  `VertexBufferLayout` (byte stride plus attributes). Creation owns label/layout
  and retains both shaders and the device. Pipeline clones share identity.
- Vertex formats are little-endian Float32, Float32x2, Float32x3, and Float32x4.
  Missing y/z lanes become zero, missing w becomes one. Each attribute specifies
  input location and byte offset. Locations must cover each vertex input exactly
  once. Attributes may overlap; stride is positive/aligned to four bytes, every
  offset is aligned, and offset + format size must fit within stride.
- Both shaders belong to the creating device and have their declared stages.
  Vertex outputs after position must exactly match fragment input count. All
  varyings are four-component vectors with perspective-correct interpolation.
- A pass has one current vertex buffer and optional current pipeline. Pipeline
  binding checks its device and target format; failed binding preserves prior
  state. Programmable draws require both bindings, a complete triangle list,
  and checked offset + end * stride within the source. Zero draws still require
  valid state/range. Each draw retains its own pipeline/buffer; source bytes are live.
- Fragment SampleTexture instructions infer required paired slots 0..8. Both
  direct and indexed programmable draws validate and retain every required pair,
  including unused sample results/empty draws. [TEXTURES.md](TEXTURES.md) defines
  views, filtering, addressing, live contents, snapshots, and failures.
- Legacy draws always use the Phase 2 layout/colour, even when a pipeline is bound.
  This explicit choice preserves existing examples and their semantics.
- At submission, vertex attributes feed vertex invocations. Output zero is
  homogeneous clip position x,y,z,w. All outputs must be finite and w must be
  strictly positive; nonpositive w rejects the entire scope before target changes.
  This initial implementation deliberately excludes eye-plane-crossing geometry.
- Triangles clip against -w <= x,y,z <= w. Clipping also interpolates varyings in
  homogeneous edge parameter. After division by w, viewport/coverage use the
  existing 1/256-pixel grid and top-left rule. Both windings work; degenerates skip.
  z participates in clipping. Optional [depth state](DEPTH.md) maps z/w into [0,1]
  and interpolates projected depth linearly in screen space before fragment shading.
- Screen barycentric weights use quantized vertex/sample positions. For each
  varying, interpolate `(value / w)` and `(1 / w)` and divide their sums. Result
  vectors feed one fragment invocation per covered sample passing depth testing
  (or with depth disabled). RGBA output clamps
  each finite lane to [0,1], multiplies by 255, and rounds to nearest byte.
  Later draws overwrite unless optional [blending](BLENDING.md) combines source
  and destination with separate RGB/alpha equations and channel masks. No gamma
  conversion, multisampling, or culling.
- Fragment arithmetic can fail late. Render into a fallibly allocated temporary
  image and commit only after every draw succeeds. Any expected failure leaves
  this scope's target unchanged (including its clear); earlier submission commands
  remain completed, later commands skip. This costs one extra full image plus
  prepared vertex data, snapshots of shader-used views, and staged depth when
  attached. Colour/depth commit together on success. Device gate and
  sequential storage lock ordering remain unchanged.
- No per-fragment heap allocation: internal shader evaluation fills fixed local
  output arrays. Public `Shader::evaluate` continues returning an owned Vec.

## Shader language decision

Accepted direction: support established shader languages/toolchains later, rather
than committing to a new EX source language. EX currently accepts its own small
instruction representation. HLSL/GLSL/SPIR-V ingestion remains future work; a
defined supported subset needs richer EX instructions before import is possible.
Using a shader compiler would leave EX responsible for rendering/execution.
Current programmable examples still construct `ShaderInstruction` enums directly.

## Validation, errors, and verification

Creation reports shader device/stage mismatches, invalid stride/attributes,
incompatible stage counts, and owned metadata reservation failure. Recording
reports missing/foreign pipelines and incompatible target format in addition to
existing vertex state/range errors. Execution identifies failed shader stage,
label, invocation (absolute vertex index or row-major pixel index), and numeric
cause. Invalid homogeneous w is a separate error. Storage poison and temporary
reservation failures retain existing behavior. No partial scope output is visible.

Public tests and a PNG example show position transformation, colour variation,
perspective interpolation, clipped varying continuity, error atomicity, recording
order, resource lifetime, and range/device/stage/layout misuse. Evidence:
[pipeline implementation](../crates/expansex-core/src/pipeline.rs),
[programmable executor](../crates/expansex-core/src/reference/programmable.rs),
[seventeen public tests](../crates/ex-api/tests/programmable_rendering.rs), and
[programmable triangle](../crates/ex-api/examples/programmable_triangle.rs).
The 512x384 PNG contains 47,794 shaded pixels and 36,374 distinct foreground
colours; independent decoding verified PNG checksums and every pixel against
separately calculated coverage/interpolation. It was also visually inspected.
[Indexed drawing](INDEXED-DRAWING.md) now reuses these pipelines, layouts, and
interpolation rules. It supports u16/u32 elements and a signed base vertex, with
live-reference validation at submission. Wider GPU features remain future work.
