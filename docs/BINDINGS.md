# Shader bindings and parameter buffers

Status: implemented and verified. This extends the bounded EX
vector shader profile with reusable resource interfaces shared by graphics and
compute. Existing descriptors, texture setters and storage dispatch remain valid.

## Design

An engine should reuse a camera/material resource interface without rebuilding a
shader for each parameter change. A layout describes slots and their permissions;
a group supplies retained resources. Pipelines validate the interface, recording
validates group compatibility, and execution reads live bytes after earlier commands.
The [Vulkan descriptor specification](https://docs.vulkan.org/spec/latest/chapters/descriptorsets.html)
similarly separates resource interface descriptions from concrete resource bindings.
EX chooses its own smaller contract rather than delegating execution to Vulkan.

## Supported profile

- One group per draw/dispatch, with up to eight sparse slots numbered 0..8. A
  slot has one type across its visible stages; duplicate entries are invalid.
- `ShaderVisibility` is Vertex, Fragment, Graphics, Compute or All.
- `BindingType` is UniformBuffer { min_size }, StorageBuffer { read_only }, or
  SampledTexture (a paired view/sampler). Uniform minimum sizes are positive,
  multiples of 16, at most 4096 bytes. Storage remains compute-only and texture
  sampling remains fragment-only under the existing instruction profile.
- A BindGroupLayout owns a sorted copy of entries and a label. Two layouts are
  compatible only on the same device with equal slot/type/visibility/minimum-size
  definitions. Labels and creation identity do not affect compatibility.
- A BindGroup owns immutable entries and retains its layout/resources. Every
  layout slot must be supplied exactly once, with no extra slots. Buffer bindings
  specify offset and size in bytes, aligned to 16; nonzero ranges must fit using
  checked addition. Uniform views require Uniform usage, cover the declared minimum
  and contain at most 4096 bytes. Storage views require Storage usage. Texture
  views and samplers must share the owning device. Creation reads no bytes.
- `LoadUniform { binding, element }` returns a little-endian float4 at static
  element 0..256 relative to the bound view, in any shader stage. Pipeline layout
  minimum size must cover every load, including unused instruction results.
  This supplies parameters, vector transforms and material values; no arbitrary
  indexing, matrix type, integer format, external shader importer or compiler is added.
- Layout-aware pipeline constructors retain a layout and validate all shader
  loads/samples and compute output mappings against it. Output storage slots must
  be writable. Missing slots, wrong types/visibility, insufficient uniform minima,
  foreign layouts and legacy construction of uniform-using shaders return errors.
  Layouts may include unused slots for reuse across multiple pipelines.
- Graphics `set_bind_group` retains a group independently of pipeline selection;
  programmable/indexed draw checks compatibility and captures it. A failed setter
  or draw preserves previous state and appends no draw. Group-based pipelines use
  the group for all shader resources; legacy texture setters do not override it.
  Legacy pipelines/fixed draws ignore any bound group. Empty programmable draws
  still need a compatible group, but execute no source memory reads.
- Compute `dispatch_with_bind_group` checks compatibility and routes storage views
  through existing dimension/range/alias checks. Legacy `dispatch` cannot execute
  a layout-aware pipeline. A recorded dispatch retains the group. Uniform views
  overlapping any used writable storage view are rejected; read/read aliases and
  disjoint views remain supported. Output count/atomic staging rules are unchanged.
- Under the device execution gate, used uniform views are snapshotted once per
  nonempty draw/dispatch before evaluation. Snapshot locks are released before
  other source/output locks, so aliases cannot cause recursive locking. Unused
  slots and zero operations skip bytes/poison. Loads are checked for finite values
  through the existing per-instruction result rule; unreferenced elements need not
  be finite. Snapshots and shader failures precede all operation output commits.
- Recorded commands retain immutable group descriptions, not copied parameter
  bytes. Later CPU writes or ordered copies are visible at submission; repeated
  submissions use updated contents. HostAccess controls CPU permission only.
- Dependency summaries add UniformRead for actually used parameter allocations.
  BufferRead accepts UniformRead. A uniform read combined with vertex/index reads
  becomes BufferRead; a compute allocation combining uniform and storage reads also
  becomes BufferRead. Disjoint uniform/storage writes on one allocation conservatively
  require StorageReadWrite for the combined whole-allocation use. Unused layout slots
  do not require transitions. Transfer writes before parameters require a barrier.
- Invalid layouts/groups/pipeline interfaces and recording misuse use structured
  errors. Poisoned uniform snapshots or nonfinite loads preserve the whole failed
  render scope or dispatch, retain earlier completed commands, and skip later ones.

## Boundaries

Group mutation, descriptor arrays, multiple groups, dynamic offsets, storage images,
graphics storage, compute textures, general shader indexing and reflection/import
remain future contracts. EX's current supported shader profile is its bounded float4
IR plus the resources above; accepting established shader languages later still
requires a demonstrated importer. This step does not complete the whole Core Preview:
resource flows, execution/backend boundary and final integrated proof remain pending.

## Supported shader limits

All values/results are finite float4 vectors. Programs have at most 16 inputs and
256 instructions, with operands referring only to earlier registers. Vertex shaders
produce clip position plus at most 16 varyings and require positive clip w at
execution. Fragment shaders produce one RGBA vector and receive exactly the vertex
varying count. Compute requires four built-in input vectors and 1..=8 outputs,
positive local size/product at most 1024, group axes at most 65535 and at most
1,048,576 invocations. Existing clipping, perspective interpolation, sampling and
atomic output contracts continue to apply. See [SHADERS.md](SHADERS.md),
[PIPELINES.md](PIPELINES.md) and [COMPUTE.md](COMPUTE.md). Programs execute every
instruction, including unused results; a loaded nonfinite uniform fails that rule.

## Verification

[Fifteen public tests](../crates/ex-api/tests/bindings.rs), two internal
poison/lifetime tests and executable documentation verify layout compatibility,
owned metadata, exact slots, resource permissions/ranges, all stages, relative
views/maxima, live updates, paired sampling, aliasing, no-op/unused slots,
explicit dependencies, finite loads and atomic failure boundaries. All 227 tests
and formatting/build/lint/documentation checks pass. Allocator OOM is not induced.

The [shared parameter workload](../crates/ex-api/examples/shared_parameters.rs)
uses one group across compute, vertex and fragment shaders with explicit uploads,
transitions and readback. It checks 288 vertices and 48 tile centres, then changes
material colour and resubmits the same finished commands. Independent PNG CRC,
zlib checksum, float32 geometry and coverage calculations verify all 196,608 pixels
in each blue/orange image, including 101,568 tile pixels. Both images were inspected.
Output files are `target/core-bindings-blue.png` and `target/core-bindings-orange.png`.

## Texture flow integration

Paired SampledTexture bindings now accept an image rendered in an earlier pass
through a shared framebuffer alias. Direct/indexed draw recording rejects a
shader-used pair that aliases the current colour attachment, even through a
subrectangle. Unused entries still contribute no source use. Explicit barriers
track the underlying image across both handle kinds. [TEXTURE-FLOW.md](TEXTURE-FLOW.md)
defines transfers, host permissions and the multi-pass evidence.
