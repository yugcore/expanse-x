# Compute pipelines and dispatch

Compute runs a shader without triangle assembly or rasterization. Applications
can transform arrays, generate geometry, and consume the result in a later copy,
dispatch, or draw. EX owns the interpreter and buffer execution; no other graphics
API executes this work.

The [Khronos compute guide](https://docs.vulkan.org/guide/latest/compute_shaders.html)
describes invocation and workgroup grids. EX uses those familiar concepts, with
a deliberately bounded vector contract suited to its current IR. This is not
Vulkan shader ingestion, hardware conformance, or a parallel execution promise.

## Execution rules

- `ShaderStage::Compute` declares exactly four input vectors and 1..=8 outputs.
  Existing arithmetic, constants, and swizzles work. `LoadStorage { binding }`
  is compute-only and reads the current invocation's element from slot 0..8.
  Texture sampling remains fragment-only. Standalone `Shader::evaluate` supplies
  the four vectors explicitly; a storage load without dispatch reports a missing
  binding.
- `ComputePipelineDescriptor` borrows label, shader, local `workgroup_size:
  [u32;3]`, and `output_bindings: &[u32]`. Creation owns metadata and retains the
  shader/device. Each local axis is 1..=1024; the product is at most 1024.
  Output bindings correspond to shader outputs, are unique, and are in 0..8.
- `CommandEncoder::dispatch(pipeline, workgroups, bindings)` records one immutable
  `Command::Dispatch(ComputeDispatch)`. Each group count is 0..=65535. The total
  invocation count is at most 1,048,576; arithmetic is checked. Any zero axis
  means zero invocations, while all other recording validation still applies.
- A borrowed `StorageBufferBinding` names a slot, buffer, byte offset and size.
  Slots are unique, 0..8; every buffer belongs to the encoder device and has
  `BufferUsage::Storage`. Offset and size are multiples of 16; ranges fit in the
  allocation, including checked addition. All bindings needed by loads and
  outputs must exist and hold at least `invocation_count * 16` bytes.
- One element is four little-endian f32 lanes. Global coordinates flatten with
  X fastest: `linear = x + width * (y + height * z)`. Loads and outputs use this
  same linear element, relative to the binding's offset. There is no arbitrary
  indexing, indirect write, branch, shared memory, barrier, or atomic operation.
- Shader inputs are vectors `(x,y,z,w)`: location 0 global invocation ID with
  global linear index in W; 1 local invocation ID with local linear index in W;
  2 workgroup ID with group linear index in W; 3 workgroup counts with W=0.
  Global ID = group ID * local size + local ID. IDs are exactly representable
  as f32 under the invocation limit. Empty dispatches evaluate no shader.
- Load and write through the same slot support in-place transforms. Across
  distinct slots, overlapping ranges of the same allocation are rejected if
  either slot is an output. Read-only overlap and disjoint views are allowed.
  This prevents ambiguous writes and double-locking while allowing shared buffers.
- Recorded work retains pipeline and buffers, not their old contents. Each
  dispatch observes live contents, including preceding commands in that submit.
  Loading a non-finite element or producing a non-finite result fails execution.
  Unused/tail bytes are neither interpreted nor modified.
- Submission holds the existing device gate. Each unique used allocation is
  locked once, all locks are acquired before execution, and outputs are staged
  separately. Loads always see pre-dispatch bytes. Successful execution commits
  all output ranges together with no subsequent fallible step. Allocation,
  poisoned storage, or shader error leaves every dispatch output unchanged.
  Completed earlier commands remain and later commands are skipped.
- Zero dispatch is a validated no-op: it does not access storage or check poison.
  Extra unused bindings are validated/retained but not locked or read. Public
  CPU permissions remain independent from Storage usage; HostAccess::None can
  execute compute and transfer its result into a readback buffer.

## Limits and tests

Execution is sequential CPU reference execution. Local groups currently provide
identity, not cooperating threads. No performance or hardware parallelism claim
follows from dispatch counts. Integer/control-flow IR, arbitrary storage access,
storage images, push constants and asynchronous queues need separate contracts.
Explicit whole-resource dependencies now work under [DEPENDENCIES.md](DEPENDENCIES.md). Current ordered synchronous commands already make a
compute output visible to a following draw; future hardware dependency APIs must
preserve this defined behavior.

[compute.rs](../crates/expansex-core/src/compute.rs) implements pipeline validation,
recording, and execution through the shared [shader interpreter](../crates/expansex-core/src/shader.rs).
[Twelve public tests](../crates/ex-api/tests/compute.rs), two internal poison/lifetime
tests, and executable compute documentation verify stage/grid/view validation,
all built-in IDs, eight output slots, limits/empty dispatch, aliasing, in-place
updates, ordering, live reuse, retention, device-only storage, complete rollback,
and compute-to-graphics visibility. The compute checkpoint passed 192 workspace tests and project checks.

The [tile example](../crates/ex-api/examples/compute_tiles.rs) runs 288 invocations
in 48 groups and draws 48 generated squares in the same submission. It checks
all 288 readback positions and 48 tile centres. Independent PNG checksums,
float32 geometry, and coverage calculations match all 196,608 pixels, including
101,568 covered pixels; the image was visually inspected. Output is
`target/phase4-compute-tiles.png`. Reservation failures are reported but tests
never deliberately exhaust memory. Handles/message allocations retain normal
Rust OOM behavior; comprehensive allocation recovery is not claimed.


## Reusable groups and uniforms

Layout-aware creation validates compute loads and output slots against a retained
BindGroupLayout. dispatch_with_bind_group uses group storage views and the existing
bounds/alias/atomic staging rules. Legacy dispatch needs a legacy pipeline.
Used uniform views are snapshotted under the gate before storage locks; snapshots
release locks before evaluation to support aliases safely. Uniform views overlapping
writable storage are rejected; disjoint and read-only aliases are supported.
Uniform poison maps to StoragePoisoned with its slot/label. Zero operations and
unused parameters skip reads. See [BINDINGS.md](BINDINGS.md) for the complete contract.
