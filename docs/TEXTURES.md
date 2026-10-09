# Textures, views, and sampling

Selected Phase 4 contract (2026-10-08): make an indexed checkerboard quad run
through EX's own reference shader execution. An image stores texels; a view
selects a rectangle; a sampler defines filtering/addressing. Vulkan also separates
[image views and samplers](https://docs.vulkan.org/tutorial/latest/06_Texture_mapping/01_Image_view_and_sampler.html).
EX uses retained paired slots for this first supported resource interface.

## Selected API and execution contract

- Textures own zero-initialized, top-left, row-major RGBA8 unorm
  storage with dimensions 1..=16384, same-device ownership, and explicit HostAccess.
  Full-image CPU upload/readback copies exact-length pixel slices under the device
  gate. Device transfers and shared colour attachments now work under
  [TEXTURE-FLOW.md](TEXTURE-FLOW.md); mapped slices remain unsupported.
- A view retains a texture and a positive, in-bounds rectangle. Its UV coordinates
  are local to that rectangle; addressing never reaches neighbouring parent texels.
- Immutable same-device samplers select Nearest or Linear and independent U/V
  ClampToEdge or Repeat. One filter applies to the sole mip level.
- UVs are finite normalized coordinates; (0,0) is the upper-left edge. Nearest
  takes floor(uv * extent). Linear uses uv * extent - 0.5 and interpolates four
  texels. Each integer neighbour clamps or wraps; repeat blends across the seam.
  Reduce repeat coordinates before scaling and bound clamp coordinates, using
  f64 arithmetic so even finite extreme f32 UVs cannot overflow index conversion.
  Sampling returns four f32 lanes in [0,1]; there is no sRGB conversion.
- Fragment `SampleTexture { binding, coordinates }` samples XY of a prior vec4
  register. Creation checks fragment stage, slot 0..8, and prior register reference.
  Required slots derive from all sample instructions, even unused results.
  Existing descriptor literals and arithmetic-only evaluation remain compatible.
  Standalone Shader::evaluate reports MissingTextureBinding when it encounters
  sampling. TextureView::sample offers synchronized standalone sampling.
- RenderPass::set_texture binds a view/sampler pair to one of eight slots after
  slot/device checks. Failure preserves prior state; a new pass resets slots.
  Programmable direct/indexed draws require every shader-used slot even when
  empty; each draw retains its own required bindings. Legacy draws ignore slots.
- Submission samples live texture contents, snapshots each used view once per
  nonempty draw using fallible reservation, then releases storage before shading.
  Aliased views never hold two storage guards. The device gate excludes uploads
  throughout submission. No per-fragment allocation/locking occurs. Empty draws
  skip texture source locks. Snapshots cost additional memory proportional to
  the sum of used view areas per draw, including aliases.
- Texture poison/reservation/shader errors preserve the entire failed scope,
  including clear/prior draws. Completed earlier commands remain; later commands
  skip. Commands retain live sources and remain reusable.

## Limits and verification

No mipmaps, derivatives/LOD, anisotropy, compressed formats, arrays/cubes/3D,
storage images, subrectangle attachments, multi-group/array/dynamic bindings, asynchronous
execution, external shader importer, or hardware support is included.
The wider Phase 4 remains incomplete after this capability. Small Arc allocations
and error labels follow ordinary Rust allocation behavior; large storage/snapshot/
label reservations are fallible, without comprehensive OOM recovery claims.

The implemented tests verify exact texel centres, bilinear mixtures, edge/repeat seams, view bounds,
negative/extreme/nonfinite coordinates, permissions/device identity, per-draw
retention/rebinding, direct and indexed rendering, perspective-correct UVs,
live uploads/reuse, poison/empty behavior, and scope atomicity. A public checkerboard
example demonstrates the complete path. [Twelve public tests](../crates/ex-api/tests/textures.rs),
three [internal poison/lifetime/gate tests](../crates/expansex-core/src/texture.rs),
and executable public documentation passed with all 137 workspace tests at the
texture milestone; the later depth milestone passes 150. The
[example](../crates/ex-api/examples/textured_quad.rs) writes a 512x384 PNG with
119,808 covered pixels, nearest on the left and bilinear on the right, both repeat.
Independent decoding verifies PNG CRCs and every pixel against separately
calculated coverage, UVs, and filtering. The image was visually inspected too.
Implementation: [resources](../crates/expansex-core/src/texture.rs),
[sampling](../crates/expansex-core/src/texture/sampler.rs),
[bindings](../crates/expansex-core/src/render/textures.rs),
[shader](../crates/expansex-core/src/shader.rs), and
[renderer](../crates/expansex-core/src/reference/programmable.rs).


## Reusable resource interfaces

[BINDINGS.md](BINDINGS.md) adds retained single-group layouts, uniform parameters
in all shader stages, compute storage views and paired fragment texture/samplers.
Layout-aware pipelines validate slot/type/visibility/minimum size; programmable
and indexed draws capture the current compatible group. Legacy pipelines keep
existing bindings. Used uniform snapshots precede evaluation and other source
locks; failures preserve existing operation boundaries. Actual parameter reads
join dependency summaries as UniformRead, with aliases combined conservatively.
