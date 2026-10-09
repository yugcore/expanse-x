# Indexed drawing

Indexed triangles reuse stored vertices: four vertex records plus indices
`[0,1,2,2,3,0]` describe a quad. EX needs this before growing into modern resource
features. Vulkan also separates index-buffer binding, index count/start, and a
signed vertex offset ([indexed draw](https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdDrawIndexed.html)).
EX defines its own retained command and synchronous reference execution.

## Selected contract

- `IndexFormat::Uint16` and `Uint32` store unsigned little-endian indices (2/4
  bytes). All values are ordinary indices; there is no primitive restart.
- `RenderPass::set_index_buffer(buffer, byte_offset, format)` checks device,
  Index usage, offset <= size, and format alignment. Failed binding keeps prior
  state. A new pass has no index binding. Host permissions do not grant Index usage.
- `draw_indexed(indices: Range<u32>, base_vertex: i32)` requires a pipeline,
  vertex buffer, and index buffer in that order, then validates ascending range,
  triangle-list count, and checked full index-byte extent. First index is relative
  to the index binding offset. Zero draws still require valid bindings/range.
- Each draw retains its pipeline and both buffer bindings. `DrawTriangles::indexed`
  exposes its `IndexedTriangles` description. Its legacy vertices getter returns
  0..0 and colour getter returns [0;4]; those fields are unused for indexed draws.
- Index bytes remain live. At submission, decode the selected range and add the
  signed base vertex using wide arithmetic. Effective indices must fit u32 and
  address complete stride-sized records relative to the vertex binding offset.
  Invalid live indices reject the scope, including its clear, before target changes.
- Decode/validate indices under the index lock, release it, then fetch referenced
  vertices under the vertex lock, all under the device gate. This also supports
  a single allocation with both Index and Vertex usage without locking it twice.
  Empty indexed draws touch neither source storage; target poison still errors.
- Only referenced vertex records are evaluated, in index order, including repeated
  references. No vertex cache/performance claim is made. Shader error invocation
  numbers are effective vertex indices; invalid index errors name the absolute
  index-element position, raw value, base offset, vertex label, and capacity.
- Indexed, direct programmable, and legacy draws can mix in recorded order.
  Existing homogeneous clipping, interpolation, fragment conversion, scope
  staging, completion, and failure rules apply unchanged.

## Errors and limits

Recording reports missing/foreign index buffers, absent Index usage, offset or
alignment misuse, reversed/incomplete index ranges, reservation failure, and
out-of-bounds index byte extents. Actual referenced vertex validity is checked
at execution because earlier copies/late writes can change index contents.
Execution distinguishes poisoned index storage from invalid vertex references.
Expected failure leaves the render scope's target unchanged; earlier submitted
commands stay completed and later commands skip. Buffers/programs stay reusable.

No instancing, primitive restart, other topologies, indirect drawing, async
completion, external shader ingestion, textures, or hardware support is added.
Index sharing saves stored records; it does not establish hardware acceleration.

## Tests and examples

[Recording](../crates/expansex-core/src/render/index.rs) and
[programmable execution](../crates/expansex-core/src/reference/programmable.rs)
implement this contract. [Ten public tests](../crates/ex-api/tests/indexed_rendering.rs)
cover both formats, a shared-vertex quad, offsets/base, live copies/reuse, aliased
buffers, retained resources, invalid state/ranges/references, mixed draw order,
and scope failure effects. Two [internal tests](../crates/expansex-core/src/buffer/resource.rs)
cover poison/empty behavior and final allocation release. The public crate
documentation exercises indexed drawing too. The indexed milestone passed 122
workspace tests; the later texture milestone passed 137 and depth milestone 150.

The [quad example](../crates/ex-api/examples/indexed_quad.rs) stores 80 vertex bytes
and 12 index bytes, instead of 120 bytes for six expanded records in this layout.
Its 512x384 PNG has 110,592 covered pixels and 80,237 foreground colours. Independent
PNG decoding checks CRCs and every pixel against separately calculated coverage/
interpolation; visual inspection also confirms the output. Phase 3 is complete.
