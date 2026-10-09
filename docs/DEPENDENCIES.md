# Resource transitions and dependencies

EX now needs an explicit description of how copies, compute, and graphics share
resources. The current software executor already completes commands in order;
that execution contract remains valid. The new explicit mode makes access and
dependency obligations visible, checkable, and suitable for later backend work.

The [Vulkan synchronization specification](https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-dependencies)
distinguishes execution ordering from write availability/visibility. EX chooses
a smaller supported contract: whole-resource access roles and full dependency
points within a synchronous command buffer. These are EX semantics, not Vulkan
stage masks, GPU cache operations, or physical image layouts.

## Execution rules

- `SynchronizationMode::Ordered` is the default for existing encoders. Commands
  complete in order and their writes are visible to following commands. This is
  a defined synchronization contract, not managed planning or hidden repairs.
- `Device::create_command_encoder_with_synchronization(Explicit)` opts into
  checked explicit work. Mode is immutable, survives finish, and is inspectable
  on both encoder and command buffer. Explicit submissions keep the same
  synchronous ordering, storage locks, completion, and execution failure rules.
- `Resource` retains a Buffer, Texture, Framebuffer, DepthBuffer, or StencilBuffer.
  Identity is allocation identity, never label equality. Texture and Framebuffer
  aliases share one identity and permit sampling, transfer and colour roles. All transition resources
  must belong to the encoder's device. Samplers/pipelines are immutable metadata
  and need no mutable-memory transition.
- `ResourceAccess` declares TransferRead/Write/ReadWrite, VertexRead, IndexRead, UniformRead,
  StorageRead/Write/ReadWrite, BufferRead, SampledRead, ColourReadWrite,
  DepthReadWrite, or StencilReadWrite. BufferRead covers supported read-only buffer
  roles. StorageReadWrite covers storage reads/writes; TransferReadWrite covers
  transfer reads/writes. Other roles require an exact match. Recording checks
  resource-kind compatibility and the corresponding BufferUsage permissions.
- `CommandEncoder::transition_resources(&[ResourceTransition])` records one
  immutable ResourceTransitions command. Each request retains a resource and
  declares `before: Option<ResourceAccess>` and `after: ResourceAccess`. None
  introduces a resource not previously declared in that command buffer. It does
  not discard contents or describe undefined memory. Some must exactly match the
  currently declared role. A same-role transition is a dependency barrier too.
- A transition orders all preceding command uses of that resource before later
  uses and makes preceding writes visible to those later uses. Its after role
  governs later access. The software target satisfies this with its ordered
  execution/device gate, so executing the barrier needs no extra byte operation.
  No invalid explicit plan is repaired or supplied with an implicit barrier.
- Recording checks mode, devices, role/kind/usage, and duplicate identities within
  a batch, before retaining/adding anything. Only Explicit supports transitions.
  Empty Explicit batches append nothing. Wrong before-state and missing barriers
  are checked over finished work, rather than changing existing finish signatures.
- `CommandBuffer::resource_uses()` returns owned, command-indexed summaries of
  actual supported accesses without reading any resource contents. Uses of the
  same allocation within one command are combined. Two read buffer roles combine
  as BufferRead, storage read/write as StorageReadWrite, and a disjoint same-buffer
  copy, or disjoint same-image copy, as TransferReadWrite. Command indices include transition commands.
- Nonempty buffer copies and positive rectangular image transfers use their whole
  source/destination allocation. Nonzero compute
  uses only shader-required load and output slots, classified from all instructions
  and output mappings. Unused bindings do not count. Nonempty draws use vertices,
  indices if present, and shader-used textures (the whole underlying texture).
  Every render scope uses colour and all attached depth/stencil storage as
  read/write, even when empty or masked: the current executor stages/commits those
  images. A scope is one coherent operation; its internal draw order needs no
  separate barrier. Empty copies/dispatches/draw sources contribute no memory use.
- Explicit validation starts with no declared resources. A first use needs a
  transition; every access must fit its current role. Read/read uses can repeat
  without a barrier. Read-after-write, write-after-read, and write-after-write
  conflicts between distinct commands require a transition, including repeated
  writes in the same role. Unrelated resource transitions cannot resolve a hazard.
- `validate_dependencies()` checks the entire finished sequence. Ordered mode
  succeeds without explicit analysis. Explicit state/role/hazard errors include
  resource kind/label, command indices, and relevant access roles. Allocation
  failure is reported. Dependency validation does not inspect poison or shader data.
- `Queue::submit` rejects foreign work, then validates dependencies before acquiring
  the gate or executing commands. `DependencyFailed` changes no resources anywhere
  in that submission, including earlier commands. Later storage/shader execution
  failures retain the existing completed-prefix and failed-operation rollback rules.
- State is local to a command buffer/submission. Synchronous completion and the
  device gate establish ordering/visibility between completed submissions and CPU
  accesses. Reusable work can start with the same None declarations on each submit.
  No persistent hardware layout/state, cross-queue ownership, asynchronous boundary,
  host-cache flush, range/subresource tracking, or fence/timeline is implemented.

## Limitations and next work

Tracking is deliberately conservative: disjoint regions of the same allocation
still conflict across commands, and attached image access is read/write even if
the result is unchanged. State transitions do not add resource usages or enable
unsupported operations. Allocation-backed temporary summaries are not a measured
performance optimization. Existing usage/range/layout validation remains enforced.
The explicit path proves the supported dependency contract under EX's reference
execution; it does not prove a future device driver or shader's arbitrary access.

Range/stage/subresource precision and richer memory models require separate
contracts. Shader/resource interfaces now work under [BINDINGS.md](BINDINGS.md);
texture/resource flows are the next Core Preview gate. The SDK
is deferred until core completion.

## Verification

[Fourteen public tests](../crates/ex-api/tests/dependencies.rs), two internal
poison/retention tests and an executable transfer example verify mode compatibility,
batch atomicity, role permissions, allocation identity, all hazard classes, before
state, read/read reuse, disjoint alias conservatism, no-op sources, compute slots,
render attachments, index/vertex aliases, sampled textures, and error boundaries.
The dependency checkpoint passed 209 workspace tests and all project checks.

The [explicit dependency example](../crates/ex-api/examples/explicit_dependencies.rs)
records four transition commands around copy, compute, render and readback. Its
seven actual uses cover four allocations; it verifies 288 generated vertices and
48 tile centres. Independent PNG checksum and coverage checks match every one of
196,608 pixels, including 101,568 cyan pixels. The image was visually inspected.
Console printing belongs to this example; validation APIs return structured errors
and do not print automatically.


Uniform parameter reads from layout-aware groups are summarized only for used
slots/nonempty operations. BufferRead accepts UniformRead. Read-only alias families
combine as BufferRead; a compute storage writer sharing disjoint allocation regions
with uniform reads conservatively combines as StorageReadWrite. CPU copies into
parameters require a transition to UniformRead before consumption. Uniform usage
and pipeline/group checks remain independent of transitions.

## Image resource flow

[TEXTURE-FLOW.md](TEXTURE-FLOW.md) defines padded buffer/image transfers,
rectangular image copies and shared full-image attachments. Declarations through
Texture or Framebuffer cover both aliases. Duplicate cross-kind aliases in one
batch fail. Render-to-sample/readback and upload-to-render still require barriers;
conversion alone performs no dependency or copy. Shader-used attachment feedback
is rejected while recording even in Ordered mode. Whole-image tracking remains
conservative for disjoint copied/sampled rectangles.
