# EX public API catalog

Updated: 2026-10-09. Scope: every currently exported EX type, inherent method,
public field, enum variant, and explicit standard trait implementation.
There are **114 public types and 216 inherent methods**. Phase 3 is complete;
these remain provisional APIs, not a completed or stable v1 specification.
The immediate milestone is [core completion](docs/CORE-MILESTONE.md); SDK
development follows that gate. This catalog describes working APIs, not planned SDK commands.

Applications depend on package `ex-api` and import its library as `ex`:

```toml
[dependencies]
ex = { package = "ex-api", path = "crates/ex-api" }
```

The path above is relative to the application's Cargo.toml; adjust it to the
workspace location. The [public crate](crates/ex-api/src/lib.rs) re-exports the
implementation from `expansex-core`. Implementation modules are private;
use `ex::Buffer`, for example, rather than `ex::buffer::Buffer`.
No public free functions, custom traits, constants, macros, or type aliases are
currently exported.

## Type index

| Area | Exported types |
| --- | --- |
| Discovery and ownership | [Instance](#instance), [Adapter](#adapter), [AdapterInfo](#adapterinfo), [BackendKind](#backendkind), [Device](#device), [Queue](#queue) |
| Buffers | [BufferDescriptor](#bufferdescriptor), [BufferUsage](#bufferusage), [HostAccess](#hostaccess), [Buffer](#buffer) |
| Copy recording | [CommandEncoder](#commandencoder), [CommandBuffer](#commandbuffer), [Command](#command), [BufferCopy](#buffercopy), [CopyBufferRole](#copybufferrole) |
| Shader bindings | [ShaderVisibility](#shadervisibility), [BindingType](#bindingtype), [BindGroupLayoutEntry](#bindgrouplayoutentry), [BindGroupLayoutDescriptor](#bindgrouplayoutdescriptor), [BindGroupLayout](#bindgrouplayout), [BufferBinding](#bufferbinding), [BindingResource](#bindingresource), [BindGroupEntry](#bindgroupentry), [BindGroupDescriptor](#bindgroupdescriptor), [BindGroup](#bindgroup), [BindingValidationError](#bindingvalidationerror), [BindingCreateError](#bindingcreateerror) |
| Dependencies | [SynchronizationMode](#synchronizationmode), [Resource](#resource), [ResourceKind](#resourcekind), [ResourceAccess](#resourceaccess), [ResourceTransition](#resourcetransition), [ResourceTransitions](#resourcetransitions), [ResourceUse](#resourceuse), [HazardKind](#hazardkind), [DependencyEncodeError](#dependencyencodeerror), [DependencyValidationError](#dependencyvalidationerror) |
| Framebuffers | [Framebuffer](#framebuffer), [FramebufferDescriptor](#framebufferdescriptor), [FramebufferFormat](#framebufferformat) |
| Rendering | [RenderPass](#renderpass), [RenderCommands](#rendercommands), [DrawTriangles](#drawtriangles), [IndexFormat](#indexformat), [IndexedTriangles](#indexedtriangles) |
| Textures and sampling | [Texture](#texture), [TextureDescriptor](#texturedescriptor), [TextureFormat](#textureformat), [TextureView](#textureview), [TextureViewDescriptor](#textureviewdescriptor), [Sampler](#sampler), [SamplerDescriptor](#samplerdescriptor), [FilterMode](#filtermode), [AddressMode](#addressmode), [TextureBinding](#texturebinding) |
| Texture errors | [TextureDescriptorError](#texturedescriptorerror), [TextureCreateError](#texturecreateerror), [TextureAccessError](#textureaccesserror), [SamplerCreateError](#samplercreateerror), [TextureSampleError](#texturesampleerror) |
| Depth | [DepthBuffer](#depthbuffer), [DepthBufferDescriptor](#depthbufferdescriptor), [DepthBufferDescriptorError](#depthbufferdescriptorerror), [DepthBufferCreateError](#depthbuffercreateerror), [DepthBufferAccessError](#depthbufferaccesserror), [DepthState](#depthstate), [CompareFunction](#comparefunction) |
| Blending | [GraphicsPipelineOptions](#graphicspipelineoptions), [BlendState](#blendstate), [BlendComponent](#blendcomponent), [BlendFactor](#blendfactor), [BlendOperation](#blendoperation) |
| Stencil | [StencilBuffer](#stencilbuffer), [StencilBufferDescriptor](#stencilbufferdescriptor), [StencilBufferDescriptorError](#stencilbufferdescriptorerror), [StencilBufferCreateError](#stencilbuffercreateerror), [StencilBufferAccessError](#stencilbufferaccesserror), [StencilOperation](#stenciloperation), [StencilFaceState](#stencilfacestate), [StencilState](#stencilstate), [RenderAttachments](#renderattachments) |
| Shaders | [Shader](#shader), [ShaderDescriptor](#shaderdescriptor), [ShaderStage](#shaderstage), [ShaderInstruction](#shaderinstruction) |
| Compute | [ComputePipeline](#computepipeline), [ComputePipelineDescriptor](#computepipelinedescriptor), [StorageBufferBinding](#storagebufferbinding), [ComputeBufferBinding](#computebufferbinding), [ComputeDispatch](#computedispatch), [ComputePipelineCreateError](#computepipelinecreateerror), [ComputeEncodeError](#computeencodeerror), [ComputeExecutionError](#computeexecutionerror) |
| Pipelines and layouts | [GraphicsPipeline](#graphicspipeline), [GraphicsPipelineDescriptor](#graphicspipelinedescriptor), [VertexBufferLayout](#vertexbufferlayout), [VertexAttribute](#vertexattribute), [VertexFormat](#vertexformat), [GraphicsPipelineCreateError](#graphicspipelinecreateerror) |
| Prototype labels | [CommandRecorder](#commandrecorder), [DebugMarkerBuffer](#debugmarkerbuffer) |
| Errors | [BufferDescriptorError](#bufferdescriptorerror), [BufferCreateError](#buffercreateerror), [BufferAccessError](#bufferaccesserror), [CommandEncodeError](#commandencodeerror), [QueueSubmitError](#queuesubmiterror) |
| Rendering errors | [FramebufferDescriptorError](#framebufferdescriptorerror), [FramebufferCreateError](#framebuffercreateerror), [FramebufferAccessError](#framebufferaccesserror), [RenderEncodeError](#renderencodeerror), [RenderExecutionError](#renderexecutionerror) |
| Shader errors | [ShaderValidationError](#shadervalidationerror), [ShaderCreateError](#shadercreateerror), [ShaderExecutionError](#shaderexecutionerror) |

Signatures below use the listed type as the meaning of `Self`. Omitted return
types mean `()`. References borrow their values; returned handles own their
retained state. `finish(self)` consumes its receiver. Byte offsets and counts are
`u64`, not element or vertex counts. See the [API contract](docs/API-DRAFT.md)
for behavior and error rules.

## Discovery and ownership

Source: [object.rs](crates/expansex-core/src/object.rs).

### Instance

`ex::Instance` discovers one built-in `EX Reference (software)` adapter. It does
not detect installed hardware. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Instance::new() -> Self` | Construct reference discovery. |
| `Instance::adapters(&self) -> &[Adapter]` | Borrow discovered adapter handles. |
| `Instance::default_adapter(&self) -> Adapter` | Retain the always-present reference adapter. |

`Default::default()` is equivalent to `new()`. An `Instance` is not cloneable.

### Adapter

`ex::Adapter` retains discovery metadata. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Adapter::info(&self) -> &AdapterInfo` | Borrow target metadata. |
| `Adapter::create_device(&self) -> Device` | Create a new logical ownership domain, even on the same adapter. |

Cloning an adapter shares metadata; it does not create a device.

### AdapterInfo

`ex::AdapterInfo` has these public fields and no inherent methods:

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | `String` | Human-readable target name. |
| `backend` | `BackendKind` | EX backend identity. |

An owned metadata value can be constructed or changed, but `Adapter::info()`
returns a shared reference and does not allow mutating the adapter's metadata.

### BackendKind

`ex::BackendKind` is non-exhaustive and has no inherent methods.

| Variant | Meaning |
| --- | --- |
| `BackendKind::Reference` | EX's own CPU/software target; copies and fixed/programmed triangles. |

External matches need a fallback arm because more variants can be added.

### Device

`ex::Device` retains one logical device. Its fields are private. Methods are
implemented across [object.rs](crates/expansex-core/src/object.rs),
[buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs),
[command.rs](crates/expansex-core/src/command.rs),
[framebuffer.rs](crates/expansex-core/src/framebuffer.rs),
[shader.rs](crates/expansex-core/src/shader.rs), and [pipeline.rs](crates/expansex-core/src/pipeline.rs).

| Method | Behavior |
| --- | --- |
| `Device::adapter(&self) -> Adapter` | Retain the selected adapter. |
| `Device::default_queue(&self) -> Queue` | Retain the device's one default endpoint; starts no work. |
| `Device::is_same(&self, other: &Self) -> bool` | Compare logical device identity. |
| `Device::create_buffer(&self, descriptor: BufferDescriptor<'_>) -> Result<Buffer, BufferCreateError>` | Validate and allocate owned zero-initialized reference storage. |
| `Device::create_command_encoder(&self) -> CommandEncoder` | Create an empty Ordered encoder bound to this device. |
| `Device::create_command_encoder_with_synchronization(&self, synchronization: SynchronizationMode) -> CommandEncoder` | Select immutable Ordered or Explicit synchronization. |
| `Device::create_bind_group_layout(&self, descriptor: BindGroupLayoutDescriptor<'_>) -> Result<BindGroupLayout, BindingCreateError>` | Validate and own a sorted reusable resource interface. |
| `Device::create_bind_group(&self, descriptor: BindGroupDescriptor<'_>) -> Result<BindGroup, BindingCreateError>` | Validate exact entries, own metadata and retain live resource handles. |
| `Device::create_framebuffer(&self, descriptor: FramebufferDescriptor<'_>) -> Result<Framebuffer, FramebufferCreateError>` | Validate extents/format and allocate zeroed RGBA reference pixels. |
| `Device::create_shader(&self, descriptor: ShaderDescriptor<'_>) -> Result<Shader, ShaderCreateError>` | Validate and own immutable EX instruction source; no pipeline binding or rendering. |
| `Device::create_compute_pipeline(&self, descriptor: ComputePipelineDescriptor<'_>) -> Result<ComputePipeline, ComputePipelineCreateError>` | Validate compute stage, local size and output slots; retain shader and own metadata. |
| `Device::create_compute_pipeline_with_bindings(&self, descriptor: ComputePipelineDescriptor<'_>, layout: &BindGroupLayout) -> Result<ComputePipeline, ComputePipelineCreateError>` | Validate all compute resource reads/outputs against a retained layout. |
| `Device::create_graphics_pipeline(&self, descriptor: GraphicsPipelineDescriptor<'_>) -> Result<GraphicsPipeline, GraphicsPipelineCreateError>` | Validate shader devices/stages, vertex layout, and varying interface; own metadata and retain shaders. |
| `Device::create_graphics_pipeline_with_bindings(&self, descriptor: GraphicsPipelineDescriptor<'_>, options: GraphicsPipelineOptions, stencil: Option<StencilState>, layout: &BindGroupLayout) -> Result<GraphicsPipeline, GraphicsPipelineCreateError>` | Validate both shader interfaces and retain the layout alongside immutable optional rendering state. |

Cloning retains the same domain; `Adapter::create_device()` creates a new one.
Discovery/device/encoder constructors do not expose a recoverable allocation
error path. Buffer creation has its documented fallible reservation path.

### Queue

`ex::Queue` retains the default endpoint's device. Its fields are private.

| Method | Behavior |
| --- | --- |
| `Queue::device(&self) -> Device` | Retain the original owning device. |
| `Queue::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `Queue::submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError>` | Execute reusable finished copies, compute dispatches, and render scopes synchronously in order on the CPU. |

Successful submission means all submitted operations finished. Concurrent submissions
and CPU access on the device serialize for the whole operation; concurrent arrival
order is unspecified. Foreign work or failed explicit dependency preflight changes nothing anywhere in the submission. Storage failure leaves earlier
commands completed, changes nothing in the failed copy, and skips later work.
Expected render failure leaves that scope's target unchanged. See
[SUBMISSION.md](docs/SUBMISSION.md) and [RENDERING.md](docs/RENDERING.md).
No asynchronous completion API exists.

## Buffers

Sources: [buffer.rs](crates/expansex-core/src/buffer.rs) and
[buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).
Contract: [BUFFERS.md](docs/BUFFERS.md).

### BufferDescriptor

`ex::BufferDescriptor<'a>` describes a request; it allocates no storage itself.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Borrowed debug label; empty labels are allowed. |
| `size` | `u64` | Requested bytes; must be greater than zero. |
| `usages` | `&'a [BufferUsage]` | Borrowed nonempty role list; duplicates have no extra effect. |
| `host_access` | `HostAccess` | CPU-copy permissions, independent of device-side usages. |

| Method | Behavior |
| --- | --- |
| `BufferDescriptor::validate(&self) -> Result<(), BufferDescriptorError>` | Check positive size, then nonempty usage list. Does not prove allocation/backend support. |

Creation copies borrowed metadata. Copying the descriptor copies its references
and values, not storage. There is no `Default` implementation.

### BufferUsage

`ex::BufferUsage` has no inherent methods. All variants can be requested in a
descriptor; transfer, Vertex, Index, and Storage roles participate in recording/execution.

| Variant | Role |
| --- | --- |
| `BufferUsage::Vertex` | Vertex input for fixed-layout or pipeline-controlled triangles. |
| `BufferUsage::Index` | Indexed triangle input; binding validates this role. |
| `BufferUsage::Uniform` | Read-only parameter views for LoadUniform in all shader stages. |
| `BufferUsage::Storage` | Float4 loads/outputs through compute dispatch; CPU access still follows HostAccess. |
| `BufferUsage::TransferSource` | Required source role for recorded copies. |
| `BufferUsage::TransferDestination` | Required destination role for recorded copies. |

A usage flag alone executes no work. Copies, compute dispatches, and triangle draws execute on submission.
Standalone shader evaluation accepts input vectors independently of buffers.

### HostAccess

`ex::HostAccess` has no inherent methods and is not a memory-placement selection.

| Variant | Public CPU access |
| --- | --- |
| `HostAccess::None` | Neither read nor write. |
| `HostAccess::Write` | Write only. |
| `HostAccess::Read` | Read only. |
| `HostAccess::ReadWrite` | Both read and write. |

### Buffer

`ex::Buffer` owns a handle to fixed reference storage and retains its device.
All fields are private; create it through `Device::create_buffer`.

| Method | Behavior |
| --- | --- |
| `Buffer::label(&self) -> &str` | Borrow the owned label. |
| `Buffer::size(&self) -> u64` | Get allocation size in bytes. |
| `Buffer::usages(&self) -> &[BufferUsage]` | Borrow requested roles. |
| `Buffer::host_access(&self) -> HostAccess` | Get CPU permissions. |
| `Buffer::device(&self) -> Device` | Retain the creating device. |
| `Buffer::belongs_to(&self, device: &Device) -> bool` | Test logical device membership. |
| `Buffer::is_same(&self, other: &Self) -> bool` | Test allocation identity, including cloned aliases. |
| `Buffer::write(&self, offset: u64, data: &[u8]) -> Result<(), BufferAccessError>` | Copy CPU bytes after permission/range checks. |
| `Buffer::read(&self, offset: u64, destination: &mut [u8]) -> Result<(), BufferAccessError>` | Copy bytes into caller-owned CPU storage after permission/range checks. |

Clones share live bytes. Reads/writes are synchronous and serialize with submissions
through the device gate, then private storage. Rejected accesses do not partially change
their destination. Empty accesses require permission and an offset at or before
the end. Transfer usages do not grant CPU access. No mapping or public mutable
slice is exposed. CPU access cannot interleave inside a same-device submission;
a sequence of separate calls is not transactional.

## Copy recording

Source: [command.rs](crates/expansex-core/src/command.rs).
Contract: [COMMANDS.md](docs/COMMANDS.md).

### CommandEncoder

`ex::CommandEncoder` is created through `Device::create_command_encoder` and
retains that device. All fields are private.

| Method | Behavior |
| --- | --- |
| `CommandEncoder::device(&self) -> Device` | Retain the owning device. |
| `CommandEncoder::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `CommandEncoder::commands(&self) -> &[Command]` | Inspect recorded operations. |
| `CommandEncoder::copy_buffer(&mut self, source: &Buffer, source_offset: u64, destination: &Buffer, destination_offset: u64, size: u64) -> Result<(), CommandEncodeError>` | Validate and append a retained copy description. |
| `CommandEncoder::dispatch(&mut self, pipeline: &ComputePipeline, workgroups: [u32; 3], bindings: &[StorageBufferBinding<'_>]) -> Result<(), ComputeEncodeError>` | Validate and retain one compute dispatch; no execution during recording. |
| `CommandEncoder::synchronization_mode(&self) -> SynchronizationMode` | Inspect the fixed synchronization contract. |
| `CommandEncoder::transition_resources(&mut self, transitions: &[ResourceTransition]) -> Result<(), DependencyEncodeError>` | Record validated retained dependency points; Explicit only, no execution. |
| `CommandEncoder::dispatch_with_bind_group(&mut self, pipeline: &ComputePipeline, workgroups: [u32; 3], group: &BindGroup) -> Result<(), ComputeEncodeError>` | Check layout compatibility, storage capacities/aliases and uniform/writer overlap; retain group. |
| `CommandEncoder::finish(self) -> CommandBuffer` | Consume the encoder and move its work into immutable finished commands. |
| `CommandEncoder::begin_render_pass(&mut self, target: &Framebuffer, clear: Option<[u8; 4]>) -> Result<RenderPass<'_>, RenderEncodeError>` | Open an exclusive borrowed render recorder; clear/draws change no pixels until submission. |

Checks run in this order: source/destination devices, source/destination transfer
roles, source/destination checked ranges, same-allocation overlap, command
reservation. Empty copies at valid end offsets are allowed. Nonempty overlapping
aliases are rejected; adjacent/disjoint same-buffer ranges are allowed.
No four-byte alignment or host access is required. Failed calls append nothing
and allow continued recording. Recording/finishing never read or write bytes.

### CommandBuffer

`ex::CommandBuffer` retains the device and recorded resources after encoder
finish. All fields are private.

| Method | Behavior |
| --- | --- |
| `CommandBuffer::device(&self) -> Device` | Retain the owning device. |
| `CommandBuffer::belongs_to(&self, device: &Device) -> bool` | Test device membership. |
| `CommandBuffer::commands(&self) -> &[Command]` | Inspect immutable finished operations. |
| `CommandBuffer::synchronization_mode(&self) -> SynchronizationMode` | Inspect the retained mode. |
| `CommandBuffer::resource_uses(&self) -> Result<Vec<ResourceUse>, DependencyValidationError>` | Inspect combined whole-allocation uses per command without reading bytes. |
| `CommandBuffer::validate_dependencies(&self) -> Result<(), DependencyValidationError>` | Preflight the entire Explicit plan; Ordered succeeds without analysis. |

There is no public constructor or clone implementation. The command sequence is
immutable; buffer contents remain live. Resources are retained, not snapshotted.
`Queue::submit` borrows this reusable work and samples live bytes per executed copy.

### Command

`ex::Command` is non-exhaustive and has no inherent methods.

| Variant | Payload |
| --- | --- |
| `Command::ResourceTransitions(ResourceTransitions)` | Retained explicit access-role changes and full dependency points. |
| `Command::CopyBuffer(BufferCopy)` | One validated buffer-copy description. |
| `Command::Dispatch(ComputeDispatch)` | One validated compute grid with retained pipeline/storage views. |
| `Command::RenderPass(RenderCommands)` | One finished render scope, retaining target and per-draw vertex/pipeline bindings. |

External matches need a fallback arm. Descriptions are not executor closures or
another graphics API's handles.

### BufferCopy

`ex::BufferCopy` is created by valid copy recording. All fields are private.

| Method | Behavior |
| --- | --- |
| `BufferCopy::source(&self) -> &Buffer` | Borrow the retained source handle. |
| `BufferCopy::source_offset(&self) -> u64` | Get the source byte offset. |
| `BufferCopy::destination(&self) -> &Buffer` | Borrow the retained destination handle. |
| `BufferCopy::destination_offset(&self) -> u64` | Get the destination byte offset. |
| `BufferCopy::size(&self) -> u64` | Get the byte count. |

### CopyBufferRole

`ex::CopyBufferRole` identifies a copy operand in error reports. It has no
inherent methods.

| Variant | Meaning |
| --- | --- |
| `CopyBufferRole::Source` | Source buffer. |
| `CopyBufferRole::Destination` | Destination buffer. |

Its `Display` representation is `source` or `destination`.

## Framebuffers and fixed-function rendering

Sources: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs) and
[render.rs](crates/expansex-core/src/render.rs). Contract: [RENDERING.md](docs/RENDERING.md).
Pixels are row-major RGBA bytes with top-left origin. Vertex positions are
little-endian f32 x/y pairs, 8-byte stride, in 2D post-projection coordinates.
The fixed path clips to [-1,+1] on both axes, supports both windings,
samples pixel centres on a 1/256-pixel grid, and uses a top-left shared-edge rule.
Zero-area triangles write nothing; later draws overwrite covered RGBA pixels.

### FramebufferFormat

`ex::FramebufferFormat` is non-exhaustive; external matches require a fallback.
It has no inherent methods.

| Variant | Meaning |
| --- | --- |
| `FramebufferFormat::Rgba8Unorm` | Four u8 channels in R,G,B,A order; no sRGB conversion. |

### FramebufferDescriptor

`ex::FramebufferDescriptor<'a>` borrows creation metadata; creation owns the label.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug name; empty allowed. |
| `width` | `u32` | Pixels across; 1..=16384. |
| `height` | `u32` | Pixels down; 1..=16384. |
| `format` | `FramebufferFormat` | Currently Rgba8Unorm only. |

| Method | Behavior |
| --- | --- |
| `FramebufferDescriptor::validate(&self) -> Result<(), FramebufferDescriptorError>` | Check positive extents, maximum extents, then supported format; allocate nothing. |

### Framebuffer

`ex::Framebuffer` has private fields, owns fixed zeroed pixels, and retains its
device. Clones share live pixels. Create through `Device::create_framebuffer`.

| Method | Behavior |
| --- | --- |
| `Framebuffer::label(&self) -> &str` | Borrow owned label. |
| `Framebuffer::width(&self) -> u32` | Width in pixels. |
| `Framebuffer::height(&self) -> u32` | Height in pixels. |
| `Framebuffer::format(&self) -> FramebufferFormat` | Stored pixel format. |
| `Framebuffer::device(&self) -> Device` | Retain creating device. |
| `Framebuffer::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `Framebuffer::is_same(&self, other: &Self) -> bool` | Compare allocation identity, including aliases. |
| `Framebuffer::read_pixels(&self, destination: &mut [[u8; 4]]) -> Result<(), FramebufferAccessError>` | Read the whole image into exactly width*height entries; errors leave destination unchanged. |
| `Framebuffer::pixel(&self, x: u32, y: u32) -> Result<[u8; 4], FramebufferAccessError>` | Read one checked top-left-origin pixel. |

Readbacks serialize with submissions using the device gate then pixel storage.
No mutable image slice, file export method, or window is exposed. The example's
PNG writer is application code, not an EX export.

### RenderPass

`ex::RenderPass<'a>` exclusively borrows its encoder and has private fields.
Create through `CommandEncoder::begin_render_pass`. Nesting or using the encoder
with a live borrowed pass is rejected by Rust. Dropping unfinished work discards it.

| Method | Behavior |
| --- | --- |
| `RenderPass::set_vertex_buffer(&mut self, buffer: &Buffer, offset: u64) -> Result<(), RenderEncodeError>` | Validate exact device, Vertex usage, offset <= size, then 4-byte alignment; retain binding. |
| `RenderPass::draw(&mut self, vertices: Range<u32>, colour: [u8; 4]) -> Result<(), RenderEncodeError>` | Validate binding, ascending range, triangle-list count, checked byte extent; append a retained draw. |
| `RenderPass::set_pipeline(&mut self, pipeline: &GraphicsPipeline) -> Result<(), RenderEncodeError>` | Check device then target colour format and retain pipeline binding. |
| `RenderPass::set_bind_group(&mut self, group: &BindGroup) -> Result<(), RenderEncodeError>` | Check device and retain current group; layout compatibility is checked per programmable/indexed draw. |
| `RenderPass::draw_programmable(&mut self, vertices: Range<u32>) -> Result<(), RenderEncodeError>` | Require pipeline then vertex buffer; check complete triangle list and checked full range with pipeline stride. Retain current bindings for this draw. |
| `RenderPass::set_index_buffer(&mut self, buffer: &Buffer, offset: u64, format: IndexFormat) -> Result<(), RenderEncodeError>` | Check exact device, Index usage, offset <= size, then format alignment; retain binding. |
| `RenderPass::draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32) -> Result<(), RenderEncodeError>` | Require pipeline, vertex buffer, then index buffer. Check ascending triangle-list index range and full index byte extent; retain all three bindings. Validate live references at submission. |
| `RenderPass::set_texture(&mut self, binding: u32, view: &TextureView, sampler: &Sampler) -> Result<(), RenderEncodeError>` | Check slot 0..8, texture device, then sampler device; retain pair. Failure preserves prior binding. |
| `RenderPass::finish(self) -> Result<(), RenderEncodeError>` | Reserve/append one render command; consumes this pass. Failed reservation commits no pass work. |

`Range` is `std::ops::Range`, not an EX export. The exclusive end and first vertex
are relative to the binding's byte offset. Count must be divisible by three;
zero-count draws still require a binding and an in-bounds start. Failed binding
or draw calls preserve earlier work/binding. Vertex, index, and pipeline bindings reset
for each new pass. Legacy `draw` always uses the fixed 8-byte layout and supplied
colour, including when a pipeline is bound. Programmable drawing is described in
[PIPELINES.md](docs/PIPELINES.md). Indexed ranges count elements relative to the
index binding offset; the signed base is added to each decoded index before
vertex fetch relative to the vertex binding offset. All index values are ordinary
references; no primitive restart exists. Empty indexed draws still validate state
and range, but skip both source storage locks. See [INDEXED-DRAWING.md](docs/INDEXED-DRAWING.md).

Direct/indexed programmable draws also require every slot referenced by their
fragment shader's SampleTexture instructions, including unused results and empty
draws. Each draw retains required pairs; legacy draws ignore texture bindings.
New passes reset all slots. Submission snapshots used views only for nonempty
draws; missing bindings are reported after vertex/index range validation.

### RenderCommands

`ex::RenderCommands` has private fields and is constructed by finishing a pass.

| Method | Behavior |
| --- | --- |
| `RenderCommands::target(&self) -> &Framebuffer` | Borrow retained target. |
| `RenderCommands::clear_colour(&self) -> Option<[u8; 4]>` | Some clears before draws; None preserves pixels. |
| `RenderCommands::draws(&self) -> &[DrawTriangles]` | Borrow recorded draws in order. |
| `RenderCommands::depth_buffer(&self) -> Option<&DepthBuffer>` | Borrow retained optional depth attachment. |
| `RenderCommands::clear_depth(&self) -> Option<f32>` | Some clears depth before draws; None preserves it. |

### DrawTriangles

`ex::DrawTriangles` has private fields. Each draw retains its own vertex/pipeline and optional index binding;
recording retains live source bytes, not a snapshot.

| Method | Behavior |
| --- | --- |
| `DrawTriangles::bind_group(&self) -> Option<&BindGroup>` | Borrow the captured layout-aware group; legacy draws retain none. |
| `DrawTriangles::vertex_buffer(&self) -> &Buffer` | Borrow retained vertex source. |
| `DrawTriangles::vertex_offset(&self) -> u64` | Binding offset in bytes. |
| `DrawTriangles::vertices(&self) -> Range<u32>` | Return an owned direct vertex range; indexed draws store unused 0..0. |
| `DrawTriangles::colour(&self) -> [u8; 4]` | Legacy constant RGBA; programmable draws store [0;4] here and ignore it. |
| `DrawTriangles::pipeline(&self) -> Option<&GraphicsPipeline>` | Some selects programmable execution with a retained pipeline; None selects fixed layout/colour. |
| `DrawTriangles::indexed(&self) -> Option<&IndexedTriangles>` | Some selects retained indexed fetch; None selects direct fetch. Indexed draws always have a pipeline. |
| `DrawTriangles::texture(&self, binding: u32) -> Option<&TextureBinding>` | Borrow a retained shader-used pair; unused/out-of-range slots return None. |

### IndexFormat

`ex::IndexFormat` is an exhaustive enum of unsigned little-endian elements.
All values, including the maximum, are ordinary indices. No restart sentinel exists.

| Variant | Bytes / binding alignment |
| --- | --- |
| `IndexFormat::Uint16` | 2 |
| `IndexFormat::Uint32` | 4 |

| Method | Behavior |
| --- | --- |
| `IndexFormat::size(&self) -> u64` | Element size and required offset alignment in bytes. |

### IndexedTriangles

`ex::IndexedTriangles` has private fields and is created by `draw_indexed`.
Source: [render/index.rs](crates/expansex-core/src/render/index.rs).

| Method | Behavior |
| --- | --- |
| `IndexedTriangles::buffer(&self) -> &Buffer` | Borrow retained live index source. |
| `IndexedTriangles::offset(&self) -> u64` | Binding byte offset. |
| `IndexedTriangles::format(&self) -> IndexFormat` | Unsigned element encoding. |
| `IndexedTriangles::indices(&self) -> Range<u32>` | Owned copy of element range relative to binding offset. |
| `IndexedTriangles::base_vertex(&self) -> i32` | Signed addition to each decoded index. |

Submission widens the addition to i64, then requires a u32 effective index and
a complete stride-sized vertex record. Only referenced vertices are evaluated,
including repeated references; no vertex cache exists. Index and vertex locks
are taken sequentially under the device gate, so both may share one allocation.

## Shader representation and evaluation

Source: [shader.rs](crates/expansex-core/src/shader.rs).
Contract: [SHADERS.md](docs/SHADERS.md). Programs execute independently through
`evaluate` or through recorded programmable draws using a graphics pipeline.
All values are four-component f32 vectors. Register numbers are instruction
indices, and all operands must refer to earlier instructions.

### ShaderStage

`ex::ShaderStage` has no inherent methods.

| Variant | Interface meaning |
| --- | --- |
| `ShaderStage::Vertex` | First output is homogeneous clip position x,y,z,w; remaining outputs are pipeline varyings. |
| `ShaderStage::Fragment` | Exactly one RGBA output vector. |
| `ShaderStage::Compute` | Four built-in input vectors, 1..=8 float4 storage outputs; independent dispatch invocations. |

### ShaderInstruction

`ex::ShaderInstruction` has no inherent methods. Each instruction creates one
result vector. Constants must be finite; references are register indices.

| Variant and public payload | Operation |
| --- | --- |
| `ShaderInstruction::Input { location: u32 }` | Read an input vector with location less than input_count. |
| `ShaderInstruction::LoadUniform { binding: u32, element: u32 }` | Read a static float4 element (0..256) relative to uniform slot 0..8; all stages. |
| `ShaderInstruction::LoadStorage { binding: u32 }` | Compute-only: read current global linear element from storage slot 0..8. |
| `ShaderInstruction::SampleTexture { binding: u32, coordinates: u32 }` | Fragment-only sampling from slot 0..8; XY of an earlier register are normalized view-local UVs. |
| `ShaderInstruction::Constant([f32; 4])` | Literal vector. |
| `ShaderInstruction::Add { left: u32, right: u32 }` | Componentwise addition. |
| `ShaderInstruction::Multiply { left: u32, right: u32 }` | Componentwise multiplication. |
| `ShaderInstruction::Dot4 { left: u32, right: u32 }` | Sum x/y/z/w products in order, broadcast result to all four lanes. |
| `ShaderInstruction::Swizzle { value: u32, components: [u8; 4] }` | Select four source lanes; each component is in 0..4, repetition allowed. |

### ShaderDescriptor

`ex::ShaderDescriptor<'a>` borrows source; creation copies it into owned state.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug label; empty allowed. |
| `stage` | `ShaderStage` | Vertex, fragment, or compute interface. |
| `input_count` | `u32` | Exact invocation input count, at most 16; compute requires four built-ins. |
| `instructions` | `&'a [ShaderInstruction]` | At most 256 instructions in execution order. |
| `outputs` | `&'a [u32]` | Register indices in return order, duplicates allowed; vertex 1..=17, fragment exactly 1, compute 1..=8. |

| Method | Behavior |
| --- | --- |
| `ShaderDescriptor::validate(&self) -> Result<(), ShaderValidationError>` | Check input/instruction limits, stage output count, each instruction's validity, then output indices; no allocation or execution. |

### Shader

`ex::Shader` has private fields. Create through `Device::create_shader`.
Clones share immutable source and identity and retain the creating device.

| Method | Behavior |
| --- | --- |
| `Shader::label(&self) -> &str` | Borrow owned label. |
| `Shader::stage(&self) -> ShaderStage` | Stage interface meaning. |
| `Shader::input_count(&self) -> u32` | Exact number of input vectors required. |
| `Shader::instructions(&self) -> &[ShaderInstruction]` | Borrow immutable instructions. |
| `Shader::outputs(&self) -> &[u32]` | Borrow output register indices. |
| `Shader::device(&self) -> Device` | Retain creating device. |
| `Shader::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `Shader::is_same(&self, other: &Self) -> bool` | Compare shared program identity, independently of source equality. |
| `Shader::evaluate(&self, inputs: &[[f32; 4]]) -> Result<Vec<[f32; 4]>, ShaderExecutionError>` | Interpret one invocation and return raw vectors in output order. |

Evaluation checks exact input count, then finite inputs, then every instruction
result, including unused inputs/results. It uses f32 arithmetic and bounded local
registers; output reservation is fallible. No resource is changed and no partial
output is returned on error. Calls can run repeatedly or concurrently without
the device gate. No clipping, interpolation, projection, colour clamping, byte
conversion, stage linking, or queue submission occurs.
Standalone evaluation encountering SampleTexture returns MissingTextureBinding;
bound fragment execution supplies retained resources. All sample/load instructions contribute required slots even when their results
are unused. Standalone LoadStorage reports MissingStorageBinding; dispatch supplies
pre-dispatch elements. Compute built-in input meaning is defined in [COMPUTE.md](docs/COMPUTE.md).

## Graphics pipelines and vertex layouts

Source: [pipeline.rs](crates/expansex-core/src/pipeline.rs).
Contract: [PIPELINES.md](docs/PIPELINES.md). Programmable vertex output requires
positive w, clips against -w <= x,y,z <= w, and divides by w before viewport
mapping. Varyings interpolate with perspective correction at pixel centres on
the 1/256-pixel grid. Without blending, fragment output clamps to [0,1], scales by
255, and rounds to bytes. Optional blending combines clamped output with staged
destination bytes. Indexed fetch and sampled fragment textures are implemented.
Depth, stencil, and blending use optional immutable pipeline state. Instancing remains unsupported.
Every scope shades into a temporary image and commits only after all draws succeed.

### VertexFormat

`ex::VertexFormat` describes little-endian f32 storage. Omitted y/z lanes are
zero; omitted w is one. It is exhaustive in this provisional version.

| Variant | Bytes |
| --- | --- |
| `VertexFormat::Float32` | 4 |
| `VertexFormat::Float32x2` | 8 |
| `VertexFormat::Float32x3` | 12 |
| `VertexFormat::Float32x4` | 16 |

| Method | Behavior |
| --- | --- |
| `VertexFormat::size(&self) -> u64` | Attribute size in bytes. |

### VertexAttribute

`ex::VertexAttribute` has no inherent methods.

| Public field | Type | Meaning |
| --- | --- | --- |
| `location` | `u32` | Vertex shader input vector location. |
| `offset` | `u64` | Four-byte-aligned byte offset within each vertex. |
| `format` | `VertexFormat` | Stored component count. |

### VertexBufferLayout

`ex::VertexBufferLayout<'a>` has no inherent methods.

| Public field | Type | Meaning |
| --- | --- | --- |
| `stride` | `u64` | Positive, four-byte-aligned distance between vertices. |
| `attributes` | `&'a [VertexAttribute]` | Cover all declared vertex input locations exactly once, in any order. Overlap is allowed; every attribute must fit within stride. |

### GraphicsPipelineDescriptor

`ex::GraphicsPipelineDescriptor<'a>` has no inherent methods. Creation validates
in this order: vertex shader device/stage, fragment shader device/stage, stride,
attribute count, each attribute's location/duplicates/alignment/checked extent,
varying interface count, then metadata reservations.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Borrowed debug label; empty allowed. |
| `vertex_shader` | `&'a Shader` | Vertex-stage shader from the creating device. |
| `fragment_shader` | `&'a Shader` | Fragment-stage shader from the creating device. |
| `vertex_layout` | `VertexBufferLayout<'a>` | One buffer's vertex input layout. |
| `colour_format` | `FramebufferFormat` | Required target format; currently Rgba8Unorm only. |

Vertex output zero is position. Remaining output count must equal fragment input
count; sequential varying outputs map to input locations 0 onward. A shader with
zero vertex inputs accepts an empty attribute list but draws still require a
vertex buffer and valid full-stride range. No built-in vertex index is supplied.

### GraphicsPipeline

`ex::GraphicsPipeline` has private fields. It owns layout/label and retains shaders
and creating device. Clones share immutable state and identity.

| Method | Behavior |
| --- | --- |
| `GraphicsPipeline::label(&self) -> &str` | Borrow owned debug label. |
| `GraphicsPipeline::binding_layout(&self) -> Option<&BindGroupLayout>` | Borrow the retained resource interface; legacy pipelines return None. |
| `GraphicsPipeline::vertex_shader(&self) -> &Shader` | Borrow retained vertex program. |
| `GraphicsPipeline::fragment_shader(&self) -> &Shader` | Borrow retained fragment program. |
| `GraphicsPipeline::vertex_stride(&self) -> u64` | Byte distance between vertices. |
| `GraphicsPipeline::vertex_attributes(&self) -> &[VertexAttribute]` | Borrow owned attributes in descriptor order. |
| `GraphicsPipeline::colour_format(&self) -> FramebufferFormat` | Required render-target format. |
| `GraphicsPipeline::device(&self) -> Device` | Retain creating device. |
| `GraphicsPipeline::belongs_to(&self, device: &Device) -> bool` | Test logical device identity. |
| `GraphicsPipeline::is_same(&self, other: &Self) -> bool` | Test shared pipeline identity, independently of descriptor equality. |

## Standalone prototype labels

Source: [markers.rs](crates/expansex-core/src/command/markers.rs).
These APIs record text only and are independent of device-bound work.

### CommandRecorder

`ex::CommandRecorder` has private fields.

| Method | Behavior |
| --- | --- |
| `CommandRecorder::new() -> Self` | Create an empty label recorder. |
| `CommandRecorder::insert_debug_marker(&mut self, label: &str)` | Store an owned label copy. |
| `CommandRecorder::finish(self) -> DebugMarkerBuffer` | Consume the recorder and finish its label sequence. |

`Default::default()` is equivalent to `new()`. This recorder cannot encode copies.

### DebugMarkerBuffer

`ex::DebugMarkerBuffer` has private fields.

| Method | Behavior |
| --- | --- |
| `DebugMarkerBuffer::debug_markers(&self) -> &[String]` | Borrow labels in recording order. |

Earlier prototypes called this result `CommandBuffer`; explicitly typed old label
results must now use `DebugMarkerBuffer`. Empty label sequences are allowed.
Finishing never executes work, even if a label says "draw" or "copy".

## Errors

These enums have public variants and their listed payloads. All implement
`Debug`, `Display`, and `std::error::Error`; none has inherent methods.
Messages describe the failed requirement. `TryReserveError` comes from
`std::collections`, not a separate EX export.

### BufferDescriptorError

Source: [buffer.rs](crates/expansex-core/src/buffer.rs).

| Variant | Meaning |
| --- | --- |
| `BufferDescriptorError::ZeroSize` | Requested size is zero. |
| `BufferDescriptorError::MissingUsage` | Requested role list is empty. |

### BufferCreateError

Source: [buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `BufferCreateError::InvalidDescriptor(BufferDescriptorError)` | Descriptor validation failed. |
| `BufferCreateError::SizeTooLarge { size: u64 }` | Size cannot fit the process's contiguous-vector capacity. |
| `BufferCreateError::AllocationFailed { size: u64, source: TryReserveError }` | Reserving bytes or owned metadata failed. |

### BufferAccessError

Source: [buffer/resource.rs](crates/expansex-core/src/buffer/resource.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `BufferAccessError::ReadNotAllowed { label: String, access: HostAccess }` | CPU reads are forbidden. |
| `BufferAccessError::WriteNotAllowed { label: String, access: HostAccess }` | CPU writes are forbidden. |
| `BufferAccessError::OutOfBounds { label: String, offset: u64, length: u64, buffer_size: u64 }` | Full range does not fit, including overflow. |
| `BufferAccessError::StoragePoisoned { label: String }` | An internal panic interrupted storage access. |

### CommandEncodeError

Source: [command.rs](crates/expansex-core/src/command.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `CommandEncodeError::ForeignBuffer { role: CopyBufferRole, label: String }` | Buffer belongs to a different logical device. |
| `CommandEncodeError::MissingUsage { role: CopyBufferRole, label: String, required: BufferUsage }` | Required transfer role is missing. |
| `CommandEncodeError::OutOfBounds { role: CopyBufferRole, label: String, offset: u64, size: u64, buffer_size: u64 }` | Full copy range does not fit, including overflow. |
| `CommandEncodeError::OverlappingCopy { label: String, source_offset: u64, destination_offset: u64, size: u64 }` | Nonempty ranges overlap in the same allocation. |
| `CommandEncodeError::AllocationFailed { source: TryReserveError }` | Reserving a command-list slot failed. |

Creation/encoding reservation errors do not provide universal OOM recovery:
small handle and error-label allocations still use ordinary Rust allocation behavior.

### QueueSubmitError

Source: [submission.rs](crates/expansex-core/src/submission.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `QueueSubmitError::ForeignCommandBuffer` | Finished work belongs to another device; nothing executed. |
| `QueueSubmitError::DependencyFailed { source: DependencyValidationError }` | Full explicit preflight failed; no command executed, all submission resources unchanged. Retains source. |
| `QueueSubmitError::StoragePoisoned { command_index: usize, role: CopyBufferRole, label: String }` | Failed nonempty copy at this zero-based index; earlier copies completed, failed destination unchanged, later work skipped. |
| `QueueSubmitError::RenderFailed { command_index: usize, source: RenderExecutionError }` | Failed render command; its target unchanged, earlier commands completed, later work skipped. |
| `QueueSubmitError::ComputeFailed { command_index: usize, source: ComputeExecutionError }` | Failed dispatch preserves all outputs and earlier commands; later commands skipped; source retained. |

Only RenderFailed supplies an underlying `Error::source`. Aliased copies report poisoned
storage as `Source`; zero-byte copies never lock storage. No rollback is promised.

### FramebufferDescriptorError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferDescriptorError::ZeroExtent { width: u32, height: u32 }` | Either axis is zero. |
| `FramebufferDescriptorError::ExtentTooLarge { width: u32, height: u32, maximum: u32 }` | Either axis exceeds reference maximum 16384. |

### FramebufferCreateError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferCreateError::InvalidDescriptor(FramebufferDescriptorError)` | Invalid extents. Only RGBA8 can currently be requested. |
| `FramebufferCreateError::AllocationFailed { source: TryReserveError }` | Reserving pixels or owned label failed. |

Both variants retain an underlying error source; small handle OOM is not recovered.

### FramebufferAccessError

Source: [framebuffer.rs](crates/expansex-core/src/framebuffer.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `FramebufferAccessError::WrongPixelCount { expected: usize, actual: usize }` | Readback slice length must equal width*height. |
| `FramebufferAccessError::OutOfBounds { x: u32, y: u32, width: u32, height: u32 }` | Requested pixel is outside the image. |
| `FramebufferAccessError::StoragePoisoned { label: String }` | Interrupted internal operation left pixels untrusted. |

### RenderEncodeError

Source: [render.rs](crates/expansex-core/src/render.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `RenderEncodeError::Bindings { source: BindingValidationError }` | Invalid/missing resource interface or group; retains validation source. |
| `RenderEncodeError::ForeignFramebuffer { label: String }` | Target belongs to another device. |
| `RenderEncodeError::ForeignStencilBuffer { label: String }` | Stencil attachment belongs to another device. |
| `RenderEncodeError::StencilExtentMismatch { colour_width: u32, colour_height: u32, stencil_width: u32, stencil_height: u32 }` | Stencil and colour extents differ. |
| `RenderEncodeError::MissingStencilBuffer` | Stencil-enabled programmable draw or clear requires an attachment, including empty draws. |
| `RenderEncodeError::ForeignDepthBuffer { label: String }` | Depth attachment belongs to another device. |
| `RenderEncodeError::DepthExtentMismatch { colour_width: u32, colour_height: u32, depth_width: u32, depth_height: u32 }` | Colour and depth dimensions differ. |
| `RenderEncodeError::InvalidDepthClear` | Depth clear is nonfinite or outside [0,1]. |
| `RenderEncodeError::MissingDepthBuffer` | Depth-enabled programmable draw or clear requires an attachment, including empty draws. |
| `RenderEncodeError::InvalidTextureSlot { binding: u32 }` | Texture binding slot is outside 0..8. |
| `RenderEncodeError::ForeignTexture { label: String }` | Bound view's texture belongs to another device. |
| `RenderEncodeError::ForeignSampler { label: String }` | Bound sampler belongs to another device. |
| `RenderEncodeError::MissingTextureBinding { binding: u32 }` | Programmable draw requires a shader-used texture/sampler pair. |
| `RenderEncodeError::ForeignVertexBuffer { label: String }` | Vertex source belongs to another device. |
| `RenderEncodeError::MissingVertexUsage { label: String }` | Binding requires Vertex usage. |
| `RenderEncodeError::VertexOffsetOutOfBounds { label: String, offset: u64, buffer_size: u64 }` | Binding offset exceeds size. |
| `RenderEncodeError::UnalignedVertexOffset { offset: u64 }` | Binding offset is not a multiple of 4. |
| `RenderEncodeError::MissingVertexBuffer` | Draw requires a bound vertex source. |
| `RenderEncodeError::MissingIndexBuffer` | Indexed draw requires a bound index source. |
| `RenderEncodeError::ForeignIndexBuffer { label: String }` | Index source belongs to another device. |
| `RenderEncodeError::MissingIndexUsage { label: String }` | Index binding requires Index usage. |
| `RenderEncodeError::IndexOffsetOutOfBounds { label: String, offset: u64, buffer_size: u64 }` | Index binding offset exceeds size. |
| `RenderEncodeError::UnalignedIndexOffset { offset: u64, alignment: u64 }` | Index offset violates the format's 2/4-byte alignment. |
| `RenderEncodeError::InvalidIndexRange { start: u32, end: u32 }` | Exclusive index end precedes start. |
| `RenderEncodeError::IndexRangeOutOfBounds { label: String, first_index: u32, index_count: u32, offset: u64, buffer_size: u64 }` | Full index byte extent exceeds storage, including overflow. |
| `RenderEncodeError::MissingPipeline` | Programmable draw requires a bound pipeline. |
| `RenderEncodeError::ForeignPipeline { label: String }` | Pipeline belongs to another logical device. |
| `RenderEncodeError::IncompatibleTargetFormat { pipeline_format: FramebufferFormat, target_format: FramebufferFormat }` | Pipeline's required colour format differs from target. Only one format is currently constructible. |
| `RenderEncodeError::InvalidVertexRange { start: u32, end: u32 }` | Exclusive end precedes start. |
| `RenderEncodeError::IncompleteTriangle { vertex_count: u32 }` | Vertex occurrences (index count for indexed draws) are not divisible by three. |
| `RenderEncodeError::VertexRangeOutOfBounds { label: String, first_vertex: u32, vertex_count: u32, offset: u64, buffer_size: u64 }` | Full fetched byte extent does not fit, including overflow. |
| `RenderEncodeError::AllocationFailed { source: TryReserveError }` | Reserving recorded draw or command slots failed. |

Only AllocationFailed has an underlying source. Expected recording errors change
no bytes or prior recorded work. Nonfinite positions are checked at execution.

### RenderExecutionError

Source: [render.rs](crates/expansex-core/src/render.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `RenderExecutionError::UniformStoragePoisoned { binding: u32, label: String }` | Used parameter snapshot is untrusted; scope outputs unchanged. |
| `RenderExecutionError::MissingUniformBinding { binding: u32, element: u32 }` | Internal uniform context is incomplete; validated recording prevents ordinary misuse. |
| `RenderExecutionError::VertexStoragePoisoned { label: String }` | Nonempty draw's vertex source is untrusted. |
| `RenderExecutionError::StencilStoragePoisoned { label: String }` | Attached stencil storage is untrusted, including empty scopes. |
| `RenderExecutionError::MissingStencilBuffer` | Internal invariant failure; validated public recording prevents ordinary misuse from reaching it. |
| `RenderExecutionError::DepthStoragePoisoned { label: String }` | Scope depth storage is untrusted, including empty/clear-only scopes. |
| `RenderExecutionError::MissingDepthBuffer` | Internal depth context is absent; validated public recording prevents ordinary misuse from reaching this. |
| `RenderExecutionError::TextureStoragePoisoned { label: String }` | Nonempty draw's sampled texture source is untrusted. |
| `RenderExecutionError::MissingTextureBinding { binding: u32 }` | Internal sampling context is incomplete; validated public draw recording prevents ordinary misuse from reaching this. |
| `RenderExecutionError::UnexpectedStorageBinding { binding: u32 }` | Internal invariant failure: validated graphics shaders cannot request compute storage. |
| `RenderExecutionError::IndexStoragePoisoned { label: String }` | Nonempty indexed draw's index source is untrusted. |
| `RenderExecutionError::InvalidVertexIndex { index_position: u32, index_value: u32, base_vertex: i32, vertex_label: String, available_vertices: u64 }` | Raw index plus base cannot address a complete u32-indexed vertex record. Position is the index element relative to the binding offset; capacity excludes trailing incomplete records. |
| `RenderExecutionError::FramebufferStoragePoisoned { label: String }` | Target pixel storage is untrusted. |
| `RenderExecutionError::NonFiniteVertex { label: String, vertex_index: u32 }` | Absolute fetched vertex index contains NaN or infinity. |
| `RenderExecutionError::InvalidClipW { vertex_index: u32 }` | Vertex shader output position w is zero or negative. |
| `RenderExecutionError::ShaderInputCount { stage: ShaderStage, label: String, invocation: u64, expected: u32, actual: usize }` | Interpreter input count mismatch; validated pipeline construction/binding prevents ordinary application misuse from reaching this. |
| `RenderExecutionError::NonFiniteShaderInput { stage: ShaderStage, label: String, invocation: u64, location: usize }` | Shader invocation input contains NaN/infinity. |
| `RenderExecutionError::NonFiniteShaderResult { stage: ShaderStage, label: String, invocation: u64, instruction: usize }` | Shader arithmetic produced a non-finite result. |
| `RenderExecutionError::AllocationFailed` | Temporary vertex/texture-snapshot/attachment reservation failed; no lower-level source retained. |

Invocation is absolute vertex index for Vertex, row-major pixel index for Fragment.
Late fragment errors discard the temporary image, including clear/prior draws in
that scope. All expected errors leave that render scope's target unchanged. The
gate prevents concurrent uploads from changing sources during preflight/drawing.

### ShaderValidationError

Source: [shader.rs](crates/expansex-core/src/shader.rs). All variants have no source.

| Variant and public payload | Meaning |
| --- | --- |
| `ShaderValidationError::TooManyInputs { count: u32, maximum: u32 }` | More than 16 input vectors. |
| `ShaderValidationError::TextureSamplingStage { instruction: usize, stage: ShaderStage }` | SampleTexture is supported only in Fragment programs. |
| `ShaderValidationError::InvalidUniformLoad { instruction: usize, binding: u32, element: u32 }` | Uniform slot outside 0..8 or element outside 0..256. |
| `ShaderValidationError::StorageLoadingStage { instruction: usize, stage: ShaderStage }` | LoadStorage requires Compute. |
| `ShaderValidationError::InvalidStorageBinding { instruction: usize, binding: u32 }` | Storage slot outside 0..8. |
| `ShaderValidationError::ComputeInputCount { count: u32 }` | Compute requires exactly four built-in input vectors. |
| `ShaderValidationError::InvalidTextureBinding { instruction: usize, binding: u32 }` | SampleTexture slot is outside 0..8. |
| `ShaderValidationError::TooManyInstructions { count: usize, maximum: usize }` | More than 256 instructions. |
| `ShaderValidationError::InvalidOutputCount { stage: ShaderStage, count: usize }` | Vertex needs 1..=17 outputs; fragment exactly 1; compute 1..=8. |
| `ShaderValidationError::InvalidInputLocation { instruction: usize, location: u32, input_count: u32 }` | Input location is outside declared inputs. |
| `ShaderValidationError::NonFiniteConstant { instruction: usize }` | Constant contains NaN or infinity. |
| `ShaderValidationError::InvalidOperand { instruction: usize, operand: u32 }` | Operand is not an earlier register. |
| `ShaderValidationError::InvalidSwizzle { instruction: usize, lane: usize, component: u8 }` | Selected component is outside 0..4. |
| `ShaderValidationError::InvalidOutput { output: usize, register: u32, instruction_count: usize }` | Output references a missing register. |

### ShaderCreateError

| Variant and public payload | Meaning |
| --- | --- |
| `ShaderCreateError::InvalidDescriptor(ShaderValidationError)` | Invalid source/interface; source retained. |
| `ShaderCreateError::AllocationFailed { source: TryReserveError }` | Owned source or label reservation failed; source retained. |

### ShaderExecutionError

| Variant and public payload | Meaning |
| --- | --- |
| `ShaderExecutionError::WrongInputCount { expected: u32, actual: usize }` | Exact input count mismatch. |
| `ShaderExecutionError::MissingTextureBinding { binding: u32 }` | Standalone evaluation encountered sampling without bound resources. |
| `ShaderExecutionError::MissingUniformBinding { binding: u32, element: u32 }` | No bound uniform element; standalone evaluation cannot supply resources. |
| `ShaderExecutionError::MissingStorageBinding { binding: u32 }` | Standalone evaluation encountered LoadStorage without dispatch bindings. |
| `ShaderExecutionError::NonFiniteInput { location: usize }` | Input has a non-finite component. |
| `ShaderExecutionError::NonFiniteResult { instruction: usize }` | Instruction produced a non-finite vector. |
| `ShaderExecutionError::AllocationFailed { source: TryReserveError }` | Returning owned outputs failed; only this variant retains a source. |

### GraphicsPipelineCreateError

Source: [pipeline.rs](crates/expansex-core/src/pipeline.rs).

| Variant and public payload | Meaning |
| --- | --- |
| `GraphicsPipelineCreateError::Bindings { source: BindingValidationError }` | Invalid/missing resource interface or group; retains validation source. |
| `GraphicsPipelineCreateError::ForeignShader { stage: ShaderStage, label: String }` | Shader for this stage belongs to another device. |
| `GraphicsPipelineCreateError::WrongShaderStage { expected: ShaderStage, actual: ShaderStage }` | Shader stage does not match its pipeline role. |
| `GraphicsPipelineCreateError::InvalidStride { stride: u64 }` | Stride is zero or not four-byte aligned. |
| `GraphicsPipelineCreateError::AttributeCountMismatch { expected: u32, actual: usize }` | Layout must provide exactly one attribute per vertex input. |
| `GraphicsPipelineCreateError::InvalidAttributeLocation { location: u32, input_count: u32 }` | Location is outside declared inputs. |
| `GraphicsPipelineCreateError::DuplicateAttributeLocation { location: u32 }` | Location appears twice. |
| `GraphicsPipelineCreateError::UnalignedAttributeOffset { location: u32, offset: u64 }` | Offset is not four-byte aligned. |
| `GraphicsPipelineCreateError::AttributeOutOfBounds { location: u32, offset: u64, size: u64, stride: u64 }` | Attribute extent overflows or exceeds stride. |
| `GraphicsPipelineCreateError::StageInterfaceMismatch { vertex_varyings: usize, fragment_inputs: u32 }` | Varying output/input counts differ. |
| `GraphicsPipelineCreateError::AllocationFailed { source: TryReserveError }` | Owned attributes/label reservation failed; only this variant has a source. |

## Textures, views, and samplers

Sources: [texture.rs](crates/expansex-core/src/texture.rs),
[sampler.rs](crates/expansex-core/src/texture/sampler.rs), and
[render/textures.rs](crates/expansex-core/src/render/textures.rs).
Contract: [TEXTURES.md](docs/TEXTURES.md). All handles have private fields.
Storage is linear RGBA8, top-left row-major, with a single mip level.

| Device method | Behavior |
| --- | --- |
| `Device::create_texture(&self, descriptor: TextureDescriptor<'_>) -> Result<Texture, TextureCreateError>` | Validate extent, own label/metadata, allocate zero-initialized sampled storage. |
| `Device::create_sampler(&self, descriptor: SamplerDescriptor<'_>) -> Result<Sampler, SamplerCreateError>` | Own immutable sampling settings and label; retain device. |

### TextureFormat

Non-exhaustive enum, no inherent methods. External matches require a fallback.

| Variant | Meaning |
| --- | --- |
| `TextureFormat::Rgba8Unorm` | Four linear unsigned normalized bytes; sample returns lane/255, without sRGB conversion. |

### TextureDescriptor

Borrowed creation request, copied into owned texture metadata.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug label; empty allowed. |
| `width` | `u32` | Texel columns, 1..=16384. |
| `height` | `u32` | Texel rows, 1..=16384. |
| `format` | `TextureFormat` | Pixel encoding. |
| `host_access` | `HostAccess` | CPU copy permission, independent of device sampling. |

| Method | Behavior |
| --- | --- |
| `TextureDescriptor::validate(&self) -> Result<(), TextureDescriptorError>` | Check positive extents, then maximum; no allocation. |

### Texture

Clones share identity, live pixels, metadata, and device. All current textures
are sampled images; attachment/storage/transfer usage is not exposed yet.

| Method | Behavior |
| --- | --- |
| `Texture::label(&self) -> &str` | Borrow owned label. |
| `Texture::width(&self) -> u32` | Texel columns. |
| `Texture::height(&self) -> u32` | Texel rows. |
| `Texture::format(&self) -> TextureFormat` | Encoding. |
| `Texture::host_access(&self) -> HostAccess` | CPU permission. |
| `Texture::device(&self) -> Device` | Retain creating device. |
| `Texture::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `Texture::is_same(&self, other: &Self) -> bool` | Compare shared allocation identity. |
| `Texture::write_pixels(&self, pixels: &[[u8; 4]]) -> Result<(), TextureAccessError>` | Full-image upload; check Write/ReadWrite permission, exact width*height count, then storage poison. |
| `Texture::read_pixels(&self, pixels: &mut [[u8; 4]]) -> Result<(), TextureAccessError>` | Full-image readback; check Read/ReadWrite permission, exact count, then poison. |
| `Texture::create_view(&self, region: TextureViewDescriptor) -> Result<TextureView, TextureAccessError>` | Retain a positive in-bounds rectangle with overflow-safe checks. |
| `Texture::view(&self) -> TextureView` | Retain a full-image view; no additional allocation. |

CPU copies serialize under the device gate, then storage lock. Failed access
changes neither pixels nor caller readback. Creation zeroes storage without a
recorded clear. Fallible large reservations do not guarantee recovery from all OOM.

### TextureViewDescriptor

No inherent methods. Rectangle coordinates are parent texture texels.

| Public field | Type | Meaning |
| --- | --- | --- |
| `x` | `u32` | Left column. |
| `y` | `u32` | Top row. |
| `width` | `u32` | Positive number of columns. |
| `height` | `u32` | Positive number of rows. |

### TextureView

Clones retain the same texture and rectangle. UVs refer to the selected rectangle;
clamp/repeat never read neighbouring texels outside the view.

| Method | Behavior |
| --- | --- |
| `TextureView::texture(&self) -> &Texture` | Borrow retained parent. |
| `TextureView::region(&self) -> TextureViewDescriptor` | Return rectangle by value. |
| `TextureView::sample(&self, sampler: &Sampler, uv: [f32; 2]) -> Result<[f32; 4], TextureSampleError>` | Check sampler device then finite UVs; acquire gate/storage and sample live pixels. HostAccess is unnecessary. |

Sampling uses f64 coordinate/filter arithmetic and returns f32 RGBA in [0,1].
Nearest uses floor(uv*extent); Linear uses uv*extent-0.5 and four neighbours.
U/V address independently, including repeat across bilinear seams. Finite extreme
coordinates are reduced/bounded before indexing. Sampling does no framebuffer write.

### FilterMode

Exhaustive enum, no inherent methods.

| Variant | Meaning |
| --- | --- |
| `FilterMode::Nearest` | Select one texel; centres are (i+0.5)/extent. |
| `FilterMode::Linear` | Bilinear interpolation of four addressed texels. |

### AddressMode

Exhaustive enum, no inherent methods.

| Variant | Meaning |
| --- | --- |
| `AddressMode::ClampToEdge` | Clamp each neighbour to the view's edge. |
| `AddressMode::Repeat` | Wrap each neighbour within the view. |

### SamplerDescriptor

Borrowed creation request; no inherent methods. No mip/LOD/anisotropy settings exist.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Owned on creation; empty allowed. |
| `filter` | `FilterMode` | Sole mip-level filtering. |
| `address_u` | `AddressMode` | Horizontal addressing. |
| `address_v` | `AddressMode` | Vertical addressing. |

### Sampler

Clones share immutable settings and identity.

| Method | Behavior |
| --- | --- |
| `Sampler::label(&self) -> &str` | Borrow owned label. |
| `Sampler::filter(&self) -> FilterMode` | Selected filter. |
| `Sampler::address_u(&self) -> AddressMode` | Horizontal addressing. |
| `Sampler::address_v(&self) -> AddressMode` | Vertical addressing. |
| `Sampler::device(&self) -> Device` | Retain creating device. |
| `Sampler::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `Sampler::is_same(&self, other: &Self) -> bool` | Compare shared sampler identity. |

### TextureBinding

Created by render recording; clones retain view and sampler.

| Method | Behavior |
| --- | --- |
| `TextureBinding::view(&self) -> &TextureView` | Borrow retained view. |
| `TextureBinding::sampler(&self) -> &Sampler` | Borrow retained sampler. |

### TextureDescriptorError

No inherent methods or lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `TextureDescriptorError::ZeroExtent { width: u32, height: u32 }` | Either dimension is zero. |
| `TextureDescriptorError::ExtentTooLarge { width: u32, height: u32, maximum: u32 }` | Either dimension exceeds 16384. |

### TextureCreateError

No inherent methods; both variants retain their lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `TextureCreateError::InvalidDescriptor(TextureDescriptorError)` | Bad extent. |
| `TextureCreateError::AllocationFailed { source: TryReserveError }` | Storage/label reservation failed. |

### TextureAccessError

No inherent methods or lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `TextureAccessError::ReadForbidden { label: String }` | CPU read permission absent. |
| `TextureAccessError::WriteForbidden { label: String }` | CPU write permission absent. |
| `TextureAccessError::WrongPixelCount { expected: usize, actual: usize }` | Slice does not contain exactly width*height pixels. |
| `TextureAccessError::InvalidView { region: TextureViewDescriptor, width: u32, height: u32 }` | Rectangle is empty or outside parent extent, including overflowing extents. |
| `TextureAccessError::StoragePoisoned { label: String }` | Pixel storage is untrusted. |

### SamplerCreateError

No inherent methods; the sole variant retains its source.

| Variant and public payload | Meaning |
| --- | --- |
| `SamplerCreateError::AllocationFailed { source: TryReserveError }` | Label reservation failed. |

### TextureSampleError

No inherent methods or lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `TextureSampleError::ForeignSampler { label: String }` | Sampler belongs to another device. |
| `TextureSampleError::NonFiniteCoordinates` | U or V is NaN/infinity. |
| `TextureSampleError::StoragePoisoned { label: String }` | Texture pixel storage is untrusted. |

## Depth attachments and testing

Source: [depth.rs](crates/expansex-core/src/depth.rs),
[pipeline.rs](crates/expansex-core/src/pipeline.rs), and the reference executor.
Contract: [DEPTH.md](docs/DEPTH.md). Depth is fixed D32 float storage, not a texture.

| Method | Behavior |
| --- | --- |
| `Device::create_depth_buffer(&self, descriptor: DepthBufferDescriptor<'_>) -> Result<DepthBuffer, DepthBufferCreateError>` | Validate extent, own metadata, allocate depth values initially 1. |
| `Device::create_graphics_pipeline_with_depth(&self, descriptor: GraphicsPipelineDescriptor<'_>, depth: DepthState) -> Result<GraphicsPipeline, GraphicsPipelineCreateError>` | Same shader/layout checks as ordinary creation, with retained immutable depth state. |
| `GraphicsPipeline::depth_state(&self) -> Option<DepthState>` | Copy optional comparison/write state; ordinary creation returns None. |
| `CommandEncoder::begin_render_pass_with_depth(&mut self, target: &Framebuffer, clear: Option<[u8; 4]>, depth: &DepthBuffer, clear_depth: Option<f32>) -> Result<RenderPass<'_>, RenderEncodeError>` | Check colour device, depth device, equal dimensions, then finite clear in [0,1]; retain attachment. |

Depth-enabled direct/indexed programmable draws require an attachment after range
checks and before texture-slot checks. Legacy and depth-disabled draws ignore
depth, while explicitly recorded clears still apply. None depth clear preserves
stored values. Both colour/depth stage and commit together under their storage
locks and the device gate; any expected error preserves both failed-scope targets.

### DepthBufferDescriptor

Borrowed metadata; creation owns its label and storage.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug label; empty allowed. |
| `width` | `u32` | Columns, 1..=16384. |
| `height` | `u32` | Rows, 1..=16384. |

| Method | Behavior |
| --- | --- |
| `DepthBufferDescriptor::validate(&self) -> Result<(), DepthBufferDescriptorError>` | Check positive extent then maximum, without allocation. |

### DepthBuffer

Private fields; clones retain one allocation/device. Top-left row-major f32
values start at 1 and remain in [0,1]. No CPU upload, packed stencil component, or sampled view exists.

| Method | Behavior |
| --- | --- |
| `DepthBuffer::label(&self) -> &str` | Borrow owned label. |
| `DepthBuffer::width(&self) -> u32` | Columns. |
| `DepthBuffer::height(&self) -> u32` | Rows. |
| `DepthBuffer::device(&self) -> Device` | Retain creating device. |
| `DepthBuffer::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `DepthBuffer::is_same(&self, other: &Self) -> bool` | Compare allocation identity. |
| `DepthBuffer::read_depths(&self, destination: &mut [f32]) -> Result<(), DepthBufferAccessError>` | Exact width*height readback under gate/storage; error preserves destination. |
| `DepthBuffer::depth(&self, x: u32, y: u32) -> Result<f32, DepthBufferAccessError>` | Check coordinates then read one synchronized value. |

### CompareFunction

Exhaustive enum; no public inherent methods. Compare incoming projected f32
depth against stored f32. Equality is exact, without epsilon.

| Variant | Pass condition |
| --- | --- |
| `CompareFunction::Never` | false |
| `CompareFunction::Less` | incoming < stored |
| `CompareFunction::Equal` | incoming == stored |
| `CompareFunction::LessEqual` | incoming <= stored |
| `CompareFunction::Greater` | incoming > stored |
| `CompareFunction::NotEqual` | incoming != stored |
| `CompareFunction::GreaterEqual` | incoming >= stored |
| `CompareFunction::Always` | true |

### DepthState

No inherent methods. Pipeline creation retains this value.

| Public field | Type | Meaning |
| --- | --- | --- |
| `compare` | `CompareFunction` | Early depth comparison. |
| `write_enabled` | `bool` | Passing successfully shaded samples update stored depth when true. |

Clip position uses -w <= z <= w. Convert to z/w*0.5+0.5 and interpolate linearly
with screen barycentric weights on the quantized coverage grid, clamp rounding
drift to [0,1], then convert to f32. Failed tests skip fragment shading and all
writes. Vertex/resource preflight still executes. Greater with clear 0 supports
reverse-Z. Shader-written depth, bias, and multisampling are absent.
Optional blending combines passing fragments with staged colour; zero alpha and
disabled colour writes do not suppress depth writes.

### DepthBufferDescriptorError

No inherent methods or lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `DepthBufferDescriptorError::ZeroExtent { width: u32, height: u32 }` | Either axis is zero. |
| `DepthBufferDescriptorError::ExtentTooLarge { width: u32, height: u32, maximum: u32 }` | Either axis exceeds 16384. |

### DepthBufferCreateError

No inherent methods; both variants retain their source.

| Variant and public payload | Meaning |
| --- | --- |
| `DepthBufferCreateError::InvalidDescriptor(DepthBufferDescriptorError)` | Extent validation failed. |
| `DepthBufferCreateError::AllocationFailed { source: TryReserveError }` | Depth storage/label reservation failed. |

### DepthBufferAccessError

No inherent methods or lower-level source.

| Variant and public payload | Meaning |
| --- | --- |
| `DepthBufferAccessError::WrongPixelCount { expected: usize, actual: usize }` | Readback slice length differs from width*height. |
| `DepthBufferAccessError::OutOfBounds { x: u32, y: u32, width: u32, height: u32 }` | Coordinate is outside the attachment. |
| `DepthBufferAccessError::StoragePoisoned { label: String }` | Depth values are untrusted. |

## Colour blending and pipeline options

Source: [blend.rs](crates/expansex-core/src/pipeline/blend.rs) and
[pipeline.rs](crates/expansex-core/src/pipeline.rs). Contract: [BLENDING.md](docs/BLENDING.md).
Direct/indexed, interpolated/textured programmable draws share this implementation.
Legacy fixed-colour draws ignore bound pipeline blending. Original pipeline
creation and depth-only creation disable blending without changing descriptors.

| Method | Behavior |
| --- | --- |
| `Device::create_graphics_pipeline_with_options(&self, descriptor: GraphicsPipelineDescriptor<'_>, options: GraphicsPipelineOptions) -> Result<GraphicsPipeline, GraphicsPipelineCreateError>` | Existing shader/layout checks and retained ownership, with independently optional depth/blend state. |
| `GraphicsPipeline::blend_state(&self) -> Option<BlendState>` | Copy retained optional blending state. |

### GraphicsPipelineOptions

No inherent methods. `Default` disables depth and blending. All state combinations
are valid; no additional validation errors exist.

| Public field | Type | Meaning |
| --- | --- | --- |
| `depth` | `Option<DepthState>` | None bypasses depth; Some enables comparison and selected write control. |
| `blend` | `Option<BlendState>` | None uses existing overwrite conversion; Some applies equations and mask. |

### BlendState

State is copied into the immutable retained pipeline. False mask entries preserve
destination bytes exactly; masks affect neither clears nor depth writes nor shader
evaluation/errors. Tests still require all normal vertex and shader resources.

| Public field | Type | Meaning |
| --- | --- | --- |
| `colour` | `BlendComponent` | Equation for R/G/B. |
| `alpha` | `BlendComponent` | Separate equation for A. |
| `write_mask` | `[bool; 4]` | R/G/B/A channel writes. |

| Method | Behavior |
| --- | --- |
| `BlendState::alpha_blending() -> Self` | RGB SrcAlpha/OneMinusSrcAlpha Add; alpha One/OneMinusSrcAlpha Add; all writes. Straight source RGB source-over. |
| `BlendState::premultiplied_alpha() -> Self` | Both equations One/OneMinusSrcAlpha Add; all writes. Source RGB must already include alpha. |
| `BlendState::additive() -> Self` | Both equations One/One Add; all writes, with storage clamping. |

### BlendComponent

No inherent methods; no invalid combinations.

| Public field | Type | Meaning |
| --- | --- | --- |
| `source_factor` | `BlendFactor` | Multiplier for source. |
| `destination_factor` | `BlendFactor` | Multiplier for destination. |
| `operation` | `BlendOperation` | Combine scaled lanes, except Min/Max ignore factors. |

### BlendFactor

Exhaustive enum; no public inherent methods. Source S is clamped shader output;
destination D is the staged byte normalized by 255. For lane i:

| Variant | Factor |
| --- | --- |
| `BlendFactor::Zero` | 0 |
| `BlendFactor::One` | 1 |
| `BlendFactor::SrcColour` | S[i] |
| `BlendFactor::OneMinusSrcColour` | 1-S[i] |
| `BlendFactor::DstColour` | D[i] |
| `BlendFactor::OneMinusDstColour` | 1-D[i] |
| `BlendFactor::SrcAlpha` | S[3] |
| `BlendFactor::OneMinusSrcAlpha` | 1-S[3] |
| `BlendFactor::DstAlpha` | D[3] |
| `BlendFactor::OneMinusDstAlpha` | 1-D[3] |
| `BlendFactor::SrcAlphaSaturate` | min(S[3],1-D[3]) for RGB; 1 for alpha. |

### BlendOperation

Exhaustive enum; no public inherent methods. In byte units, s=S[i]*255*source_factor,
d=destination_byte*destination_factor. Compute f64 arithmetic, clamp to [0,255],
round to the nearest byte (represented positive ties up), then apply the mask.
Min/Max use the unscaled source/destination. This avoids destination normalization
round trips for equations, but arbitrary f64 arithmetic retains rounding limits.

| Variant | Equation |
| --- | --- |
| `BlendOperation::Add` | s+d |
| `BlendOperation::Subtract` | s-d |
| `BlendOperation::ReverseSubtract` | d-s |
| `BlendOperation::Min` | min(S[i]*255,destination_byte), ignoring factors. |
| `BlendOperation::Max` | max(S[i]*255,destination_byte), ignoring factors. |

Subsequent fragments read the quantized staged bytes. Depth rejection skips
shading/blending; successful scopes publish colour/depth together. Alpha zero does
not discard. Transparent applications typically disable depth writes and sort back
to front themselves. No sRGB conversion, automatic premultiplication, sorting,
blend constants, dual-source factors, logic operations, multiple targets,
or floating colour targets exist.

## Stencil and optional render attachments

Sources: [resource](crates/expansex-core/src/stencil.rs),
[state](crates/expansex-core/src/pipeline/stencil.rs), [recording](crates/expansex-core/src/render.rs).
Contract: [STENCIL.md](docs/STENCIL.md). All existing descriptors/options remain
compatible; original pipeline creators disable stencil. Depth and stencil are separate storage.

| Method | Behavior |
| --- | --- |
| `Device::create_stencil_buffer(&self, descriptor: StencilBufferDescriptor<'_>) -> Result<StencilBuffer, StencilBufferCreateError>` | Validate extent, own label, allocate zeroed S8 values. |
| `Device::create_graphics_pipeline_with_stencil(&self, descriptor: GraphicsPipelineDescriptor<'_>, options: GraphicsPipelineOptions, stencil: StencilState) -> Result<GraphicsPipeline, GraphicsPipelineCreateError>` | Same shader/layout validation and ownership, plus copied stencil and optional depth/blend. |
| `GraphicsPipeline::stencil_state(&self) -> Option<StencilState>` | Copy optional immutable rules. |
| `CommandEncoder::begin_render_pass_with_attachments(&mut self, target: &Framebuffer, clear: Option<[u8; 4]>, attachments: RenderAttachments<'_>) -> Result<RenderPass<'_>, RenderEncodeError>` | Check colour then depth then stencil ownership/extent/clears; retain optional resources. |
| `RenderCommands::stencil_buffer(&self) -> Option<&StencilBuffer>` | Borrow retained optional stencil attachment. |
| `RenderCommands::clear_stencil(&self) -> Option<u8>` | Optional clear performed at execution. |

### RenderAttachments

No inherent methods. Borrowed same-device/same-extent storage; recording retains
cloned handles and copied clears. Default has all None. Missing storage with a
requested clear is an error. Clears ignore pipeline masks; None preserves live values.
Old begin methods delegate with no stencil. All attached images commit together.

| Public field | Type | Meaning |
| --- | --- | --- |
| `depth` | `Option<&'a DepthBuffer>` | Optional retained depth image. |
| `clear_depth` | `Option<f32>` | Optional finite [0,1] clear. |
| `stencil` | `Option<&'a StencilBuffer>` | Optional retained stencil image. |
| `clear_stencil` | `Option<u8>` | Optional stencil clear; every u8 is valid. |

### StencilBufferDescriptor

Creation owns the borrowed label and new storage.

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Debug label; empty allowed. |
| `width` | `u32` | Columns, 1..=16384. |
| `height` | `u32` | Rows, 1..=16384. |

| Method | Behavior |
| --- | --- |
| `StencilBufferDescriptor::validate(&self) -> Result<(), StencilBufferDescriptorError>` | Check positive then maximum extent, without allocation. |

### StencilBuffer

Private fields. One row-major, top-left u8 per pixel, initially zero. Clones share
allocation/device. No public upload or sampled view.

| Method | Behavior |
| --- | --- |
| `StencilBuffer::label(&self) -> &str` | Borrow owned label. |
| `StencilBuffer::width(&self) -> u32` | Columns. |
| `StencilBuffer::height(&self) -> u32` | Rows. |
| `StencilBuffer::device(&self) -> Device` | Retain creating device. |
| `StencilBuffer::belongs_to(&self, device: &Device) -> bool` | Compare device identity. |
| `StencilBuffer::is_same(&self, other: &Self) -> bool` | Compare storage identity. |
| `StencilBuffer::read_stencils(&self, destination: &mut [u8]) -> Result<(), StencilBufferAccessError>` | Exact width*height readback under gate/storage lock; error preserves destination. |
| `StencilBuffer::stencil(&self, x: u32, y: u32) -> Result<u8, StencilBufferAccessError>` | Check coordinates, then synchronized single-value readback. |

### StencilState

Private storage is not involved: these are copied immutable pipeline rules.

| Public field | Type | Meaning |
| --- | --- | --- |
| `front` | `StencilFaceState` | Positive quantized screen-area triangle rules. |
| `back` | `StencilFaceState` | Negative area rules, selected before winding normalization. |
| `reference` | `u8` | Shared comparison/replacement reference. |
| `read_mask` | `u8` | AND both reference and stored byte before comparison. |
| `write_mask` | `u8` | Select updated bits after the full-byte operation. |

| Method | Behavior |
| --- | --- |
| `StencilState::replace(reference: u8) -> Self` | Both faces Always, fail/depth-fail Keep, pass Replace; full masks. |
| `StencilState::test_equal(reference: u8) -> Self` | Both faces Equal/all Keep, full read mask, zero write mask. |

Compare masked reference against masked stored value using CompareFunction's
eight comparisons, in that order. Front is clockwise in top-left screen coordinates
(counterclockwise clip x/y). Clipped fan pieces use their quantized area; degenerates
update nothing. No configurable facing/culling. State-disabled and legacy draws
bypass stencil. Empty stencil-enabled programmable draws still require an attachment.

### StencilFaceState

No inherent methods; every combination is valid.

| Public field | Type | Meaning |
| --- | --- | --- |
| `compare` | `CompareFunction` | Reference-versus-stored comparison after masking. |
| `fail_op` | `StencilOperation` | Stencil failure: update, then skip depth/shading/colour. |
| `depth_fail_op` | `StencilOperation` | Stencil passes, depth fails: update, skip shading/colour. |
| `pass_op` | `StencilOperation` | Both pass (or depth disabled): update after successful shading. |

### StencilOperation

Exhaustive enum; no public inherent methods. Generate new from the complete old
byte, then write `(old & !write_mask) | (new & write_mask)`. A zero write mask
preserves storage without disabling comparison, shaders, or depth policy.

| Variant | Generated value |
| --- | --- |
| `StencilOperation::Keep` | old |
| `StencilOperation::Zero` | 0 |
| `StencilOperation::Replace` | unmasked reference |
| `StencilOperation::IncrementClamp` | min(old+1,255) |
| `StencilOperation::DecrementClamp` | max(old-1,0) |
| `StencilOperation::Invert` | bitwise !old |
| `StencilOperation::IncrementWrap` | old+1 modulo 256 |
| `StencilOperation::DecrementWrap` | old-1 modulo 256 |

All updates are staged. A late expected failure discards colour/depth/stencil
clears and draws in that scope; completed prefix commands remain, later ones skip.
Colour write masks and alpha zero do not disable stencil operations. There is no
dynamic reference, shader stencil output, packed depth/stencil, or attachment sampling.

### StencilBufferDescriptorError

No inherent methods or source.

| Variant and public payload | Meaning |
| --- | --- |
| `StencilBufferDescriptorError::ZeroExtent { width: u32, height: u32 }` | Either axis is zero. |
| `StencilBufferDescriptorError::ExtentTooLarge { width: u32, height: u32, maximum: u32 }` | Either axis exceeds 16384. |

### StencilBufferCreateError

No inherent methods; both variants retain their source.

| Variant and public payload | Meaning |
| --- | --- |
| `StencilBufferCreateError::InvalidDescriptor(StencilBufferDescriptorError)` | Extent validation failed. |
| `StencilBufferCreateError::AllocationFailed { source: TryReserveError }` | Values/label reservation failed. |

### StencilBufferAccessError

No inherent methods or source.

| Variant and public payload | Meaning |
| --- | --- |
| `StencilBufferAccessError::WrongPixelCount { expected: usize, actual: usize }` | Readback length differs from width*height. |
| `StencilBufferAccessError::OutOfBounds { x: u32, y: u32, width: u32, height: u32 }` | Coordinates are outside storage. |
| `StencilBufferAccessError::StoragePoisoned { label: String }` | Storage is untrusted after an interrupted operation. |

## Compute pipelines and dispatch

Source: [compute.rs](crates/expansex-core/src/compute.rs).
Contract: [COMPUTE.md](docs/COMPUTE.md). Elements are little-endian float4 vectors;
each invocation loads/writes its global linear element. Global coordinates flatten
X fastest. Input locations 0/1/2/3 are global/local/group IDs and group counts;
the first three include the respective linear index in W, group counts use W=0.
All values are finite; each dispatch commits all outputs together after successful
evaluation. Resources are live and retained. In-place transforms use one slot;
overlapping distinct views involving a writer are rejected. CPU permissions are
independent from Storage usage. Zero dispatches validate but access no bytes.

### ComputePipelineDescriptor

Borrowed configuration, `ex::ComputePipelineDescriptor<'a>`:

| Public field | Type | Meaning |
| --- | --- | --- |
| `label` | `&'a str` | Owned on creation; empty allowed. |
| `shader` | `&'a Shader` | Retained Compute-stage program from the creating device. |
| `workgroup_size` | `[u32; 3]` | Each axis 1..=1024; product at most 1024. |
| `output_bindings` | `&'a [u32]` | Unique slots in 0..8, one per shader output in output order; copied on creation. |

No inherent methods. Use Device::create_compute_pipeline.

### ComputePipeline

Private fields; clones share immutable configuration/identity and retain device/shader.

| Method | Behavior |
| --- | --- |
| `ComputePipeline::label(&self) -> &str` | Borrow owned label. |
| `ComputePipeline::binding_layout(&self) -> Option<&BindGroupLayout>` | Borrow the retained resource interface; legacy pipelines return None. |
| `ComputePipeline::shader(&self) -> &Shader` | Borrow retained shader. |
| `ComputePipeline::workgroup_size(&self) -> [u32; 3]` | Local invocation dimensions. |
| `ComputePipeline::output_bindings(&self) -> &[u32]` | Output slots in shader output order. |
| `ComputePipeline::device(&self) -> Device` | Retain original device. |
| `ComputePipeline::belongs_to(&self, device: &Device) -> bool` | Logical device membership. |
| `ComputePipeline::is_same(&self, other: &Self) -> bool` | Shared pipeline identity. |

### StorageBufferBinding

Borrowed recording request, `ex::StorageBufferBinding<'a>`. No inherent methods.

| Public field | Type | Meaning |
| --- | --- | --- |
| `binding` | `u32` | Unique slot in 0..8. |
| `buffer` | `&'a Buffer` | Same-device allocation with Storage usage. |
| `offset` | `u64` | Byte offset, aligned to 16. |
| `size` | `u64` | Byte range size, multiple of 16; zero allowed. |

Offset plus size must fit. Required slots hold at least invocation_count * 16
bytes. Unused bindings are validated and retained but not read or locked.

### ComputeBufferBinding

Validated owned view inside a recorded dispatch; fields private. Cloning retains storage.

| Method | Behavior |
| --- | --- |
| `ComputeBufferBinding::binding(&self) -> u32` | Slot number. |
| `ComputeBufferBinding::buffer(&self) -> &Buffer` | Retained live allocation. |
| `ComputeBufferBinding::offset(&self) -> u64` | Byte offset. |
| `ComputeBufferBinding::size(&self) -> u64` | Bound byte count. |

### ComputeDispatch

Private fields; only dispatch recording constructs this description. No Clone.
Group counts are 0..=65535 per axis; local size comes from the pipeline. Total
invocations are at most 1,048,576. A zero group axis makes the dispatch empty.

| Method | Behavior |
| --- | --- |
| `ComputeDispatch::bind_group(&self) -> Option<&BindGroup>` | Borrow retained group; legacy dispatches return None. |
| `ComputeDispatch::pipeline(&self) -> &ComputePipeline` | Retained pipeline. |
| `ComputeDispatch::workgroups(&self) -> [u32; 3]` | Recorded group counts. |
| `ComputeDispatch::invocation_count(&self) -> u32` | Product of global dimensions, or zero. |
| `ComputeDispatch::bindings(&self) -> &[ComputeBufferBinding]` | Retained storage views in recording request order. |

### ComputePipelineCreateError

| Variant and public payload | Meaning |
| --- | --- |
| `ComputePipelineCreateError::Bindings { source: BindingValidationError }` | Invalid/missing resource interface or group; retains validation source. |
| `ComputePipelineCreateError::ForeignShader { label: String }` | Shader belongs to another device. |
| `ComputePipelineCreateError::WrongShaderStage { actual: ShaderStage }` | Shader is not Compute. |
| `ComputePipelineCreateError::InvalidWorkgroupSize { size: [u32; 3] }` | Zero/oversized local axis or product exceeds 1024. |
| `ComputePipelineCreateError::OutputCountMismatch { expected: usize, actual: usize }` | Output-slot count differs from shader output count. |
| `ComputePipelineCreateError::InvalidOutputBinding { binding: u32 }` | Slot outside 0..8. |
| `ComputePipelineCreateError::DuplicateOutputBinding { binding: u32 }` | Two outputs target the same slot. |
| `ComputePipelineCreateError::AllocationFailed { source: TryReserveError }` | Metadata reservation failed; source retained. |

### ComputeEncodeError

Failed recording appends nothing, modifies no resource, and retains no new handles.
Legacy validation checks missing group first for layout-aware pipelines, then pipeline device, group axes, total invocations, supplied slots/devices/
usages/alignment/ranges, required slots/capacity, overlap, reservations.

| Variant and public payload | Meaning |
| --- | --- |
| `ComputeEncodeError::Bindings { source: BindingValidationError }` | Invalid/missing resource interface or group; retains validation source. |
| `ComputeEncodeError::ForeignPipeline { label: String }` | Pipeline belongs to another device. |
| `ComputeEncodeError::InvalidWorkgroups { counts: [u32; 3] }` | Group axis exceeds 65535. |
| `ComputeEncodeError::TooManyInvocations { workgroups: [u32; 3], workgroup_size: [u32; 3] }` | Product overflow or more than 1,048,576 invocations. |
| `ComputeEncodeError::InvalidBinding { binding: u32 }` | Slot outside 0..8. |
| `ComputeEncodeError::DuplicateBinding { binding: u32 }` | Duplicate supplied slot. |
| `ComputeEncodeError::ForeignBuffer { binding: u32, label: String }` | Buffer belongs to another device. |
| `ComputeEncodeError::MissingStorageUsage { binding: u32, label: String }` | Buffer lacks Storage usage. |
| `ComputeEncodeError::UnalignedRange { binding: u32, offset: u64, size: u64 }` | Offset or size is not a multiple of 16. |
| `ComputeEncodeError::OutOfBounds { binding: u32, offset: u64, size: u64, buffer_size: u64 }` | Range overflows or exceeds allocation. |
| `ComputeEncodeError::MissingBinding { binding: u32 }` | Required load/output slot absent, even for empty dispatch. |
| `ComputeEncodeError::InsufficientElements { binding: u32, required_bytes: u64, size: u64 }` | View too small for all invocations. |
| `ComputeEncodeError::OverlappingBindings { first: u32, second: u32 }` | Same-allocation views overlap and at least one is an output. |
| `ComputeEncodeError::AllocationFailed { source: TryReserveError }` | Recorded-view or command reservation failed; source retained. |

### ComputeExecutionError

| Variant and public payload | Meaning |
| --- | --- |
| `ComputeExecutionError::StoragePoisoned { binding: u32, label: String }` | Used allocation lock poisoned; checked before any output commit. |
| `ComputeExecutionError::AllocationFailed` | Staged output reservation failed; no source retained. |
| `ComputeExecutionError::ShaderFailed { invocation: [u32; 3], message: String }` | Shader failure with global invocation coordinates and original explanation. |

All variants leave every dispatch output unchanged. These variants retain no
underlying error source; QueueSubmitError::ComputeFailed retains this error.
Completed earlier commands remain; later work is skipped. Used uniform views are snapshotted before storage locks. All used unique storage buffers
stay locked through evaluation and commit under the device gate; loads see starting
bytes, tail bytes remain unchanged. There are no shared-memory/barrier/atomic,
arbitrary indexing, texture, integer, or branching compute instructions yet.


## Reusable shader bindings

Source: [bindings.rs](crates/expansex-core/src/bindings.rs).
Contract and supported shader profile: [BINDINGS.md](docs/BINDINGS.md).
One immutable group with eight sparse slots supports uniform views, compute storage
views and paired fragment texture/samplers. Resources remain live across submissions.

### ShaderVisibility

Variants: `Vertex`, `Fragment`, `Graphics` (Vertex/Fragment), `Compute`, `All`.

| Method | Behavior |
| --- | --- |
| `ShaderVisibility::contains(&self, stage: ShaderStage) -> bool` | Test the supported stage set. |

### BindingType

No inherent methods. Variants: `UniformBuffer { min_size: u64 }`,
`StorageBuffer { read_only: bool }`, `SampledTexture`. Uniform minimum is positive,
16-aligned, at most 4096 bytes. Writable storage is required for compute outputs.

### BindGroupLayoutEntry

No inherent methods. Public fields: `binding: u32` (0..8),
`visibility: ShaderVisibility`, `ty: BindingType`.

### BindGroupLayoutDescriptor

No inherent methods. Borrowed public fields: `label: &'a str`,
`entries: &'a [BindGroupLayoutEntry]`. Creation owns a sorted copy. Empty layouts
are supported; duplicate/out-of-range slots and invalid uniform minima are rejected.

### BindGroupLayout

Private shared immutable state retaining device/label/entries.

| Method | Behavior |
| --- | --- |
| `BindGroupLayout::label(&self) -> &str` | Borrow owned label. |
| `BindGroupLayout::entries(&self) -> &[BindGroupLayoutEntry]` | Borrow definitions sorted by slot. |
| `BindGroupLayout::device(&self) -> Device` | Retain original device. |
| `BindGroupLayout::belongs_to(&self, device: &Device) -> bool` | Check ownership. |
| `BindGroupLayout::is_same(&self, other: &Self) -> bool` | Compare creation identity. |
| `BindGroupLayout::is_compatible(&self, other: &Self) -> bool` | Same device and identical sorted definitions; labels/identity ignored. |

### BufferBinding

No inherent methods. Public fields: `buffer: Buffer`, `offset: u64`, `size: u64`.
Owns a handle, not copied bytes. Group creation requires a nonzero 16-aligned
in-bounds view; uniform size is at most 4096 and covers the layout's minimum.
HostAccess does not authorize or restrict shader reads.

### BindingResource

No inherent methods. Public variants: `UniformBuffer(BufferBinding)`,
`StorageBuffer(BufferBinding)`, `SampledTexture { view: TextureView, sampler: Sampler }`.
Creation checks the expected layout type, device, Uniform/Storage usages and view
range. Both texture and sampler must share the group's device.

### BindGroupEntry

No inherent methods. Public fields: `binding: u32`, `resource: BindingResource`.
Every layout entry must appear once; no extras are allowed.

### BindGroupDescriptor

No inherent methods. Public borrowed fields: `label: &'a str`,
`layout: &'a BindGroupLayout`, `entries: &'a [BindGroupEntry]`.
Creation owns metadata and retains handles; it reads no bytes.

### BindGroup

Private immutable shared state, reusable across structurally compatible pipelines.

| Method | Behavior |
| --- | --- |
| `BindGroup::label(&self) -> &str` | Borrow owned label. |
| `BindGroup::layout(&self) -> &BindGroupLayout` | Borrow retained interface. |
| `BindGroup::entries(&self) -> &[BindGroupEntry]` | Borrow immutable entries sorted by slot. |
| `BindGroup::device(&self) -> Device` | Retain layout's owning device. |
| `BindGroup::belongs_to(&self, device: &Device) -> bool` | Check ownership. |
| `BindGroup::is_same(&self, other: &Self) -> bool` | Compare shared group identity. |

Layout-aware draws/dispatches capture the current group. Each nonempty operation
snapshots used uniform views under the gate before other source/storage locks;
only loaded results must be finite. Extra slots/zero sources skip reads and poison.
Later copies/CPU writes are visible on submission and reuse. Legacy pipelines use
existing texture/storage APIs and ignore a bound graphics group.

### BindingValidationError

No inherent methods; implements Error with no underlying source.

| Variant and public payload | Meaning |
| --- | --- |
| `InvalidBinding { binding: u32 }` | Slot outside 0..8. |
| `DuplicateBinding { binding: u32 }` | Repeated slot. |
| `InvalidMinimum { binding: u32, size: u64 }` | Uniform minimum is not 16-aligned in 16..=4096. |
| `ForeignLayout` | Layout/group belongs to another device. |
| `ForeignResource { binding: u32, label: String }` | Concrete resource has foreign ownership. |
| `MissingBinding { binding: u32 }` | Required layout/group/shader slot absent. |
| `UnexpectedBinding { binding: u32 }` | Group supplied a slot outside its layout. |
| `WrongType { binding: u32, expected: BindingType }` | Shader/concrete resource type differs from expected kind. |
| `InvalidRange { binding: u32, offset: u64, size: u64, buffer_size: u64 }` | Empty, unaligned, overflowing/out-of-bounds view or uniform size exceeds 4096. |
| `MissingUsage { binding: u32, usage: BufferUsage }` | Buffer lacks required shader permission. |
| `LayoutRequired` | Uniform shader creation or group dispatch needs layout-aware construction. |
| `MissingGroup` | Layout-aware operation has no group. |
| `IncompatibleLayout` | Group and pipeline definitions/device differ. |
| `StageNotVisible { binding: u32, stage: ShaderStage }` | Shader stage cannot access the entry. |
| `UniformTooSmall { binding: u32, required: u64, declared: u64 }` | Pipeline minimum/view size does not cover required bytes. |
| `ReadOnlyOutput { binding: u32 }` | Compute output slot is read-only. |
| `OverlappingUniformStorage { uniform: u32, storage: u32 }` | Uniform view overlaps used writable storage view. |

Layout-aware compute group/alias validation precedes existing dispatch range checks.
Graphics checks device on setting a group, compatibility when recording each draw.
Failures append no operation; failed setters preserve current state. Errors are
wrapped by Bindings variants of graphics/compute pipeline creation and recording.

### BindingCreateError

No inherent methods. Variants: `InvalidDescriptor(BindingValidationError)`,
`AllocationFailed { source: std::collections::TryReserveError }`. Both retain their
underlying Error::source. `From<BindingValidationError>` wraps InvalidDescriptor.
Creation validates before retaining metadata; no storage mutation occurs.

## Resource dependencies

Source: [dependency.rs](crates/expansex-core/src/dependency.rs).
Contract: [DEPENDENCIES.md](docs/DEPENDENCIES.md). These are whole-resource,
command-buffer-local roles and full dependencies for synchronous reference work.
They do not describe physical GPU layouts, stage masks, or cache operations.

### SynchronizationMode

No inherent methods. Variants: `Ordered` (default, fully ordered commands and
write visibility), `Explicit` (declared accesses and checked hazards).

### ResourceKind

No inherent methods. Variants: `Buffer`, `Texture`, `Framebuffer`, `DepthBuffer`,
`StencilBuffer`. These identify allocation categories in validation errors.

### Resource

Public variants retain their corresponding handle: `Buffer(Buffer)`,
`Texture(Texture)`, `Framebuffer(Framebuffer)`, `DepthBuffer(DepthBuffer)`,
`StencilBuffer(StencilBuffer)`. Labels are descriptive; identity is allocation identity.

| Method | Behavior |
| --- | --- |
| `Resource::kind(&self) -> ResourceKind` | Get allocation category. |
| `Resource::label(&self) -> &str` | Borrow retained label. |
| `Resource::device(&self) -> Device` | Retain the owning device. |
| `Resource::belongs_to(&self, device: &Device) -> bool` | Check ownership. |
| `Resource::is_same(&self, other: &Self) -> bool` | Compare category and allocation identity. |

### ResourceAccess

| Variant | Required resource and use |
| --- | --- |
| `TransferRead` | Buffer with TransferSource usage. |
| `TransferWrite` | Buffer with TransferDestination usage. |
| `TransferReadWrite` | Buffer with both transfer usages; permits either transfer role. |
| `VertexRead` | Buffer with Vertex usage. |
| `IndexRead` | Buffer with Index usage. |
| `UniformRead` | Buffer with Uniform usage; parameters actually loaded by a shader. |
| `StorageRead` | Buffer with Storage usage, read access. |
| `StorageWrite` | Buffer with Storage usage, write access. |
| `StorageReadWrite` | Buffer with Storage usage; permits either storage role. |
| `BufferRead` | Buffer with at least one supported read usage; permits TransferRead/VertexRead/IndexRead/StorageRead/UniformRead and combined render-source reads. |
| `SampledRead` | Texture, underlying allocation of sampled views. |
| `ColourReadWrite` | Framebuffer attachment. |
| `DepthReadWrite` | DepthBuffer attachment. |
| `StencilReadWrite` | StencilBuffer attachment. |

| Method | Behavior |
| --- | --- |
| `ResourceAccess::is_read(&self) -> bool` | All roles except TransferWrite/StorageWrite. |
| `ResourceAccess::is_write(&self) -> bool` | TransferWrite/ReadWrite, StorageWrite/ReadWrite and attachment roles. |

Creation permissions and existing operation checks remain enforced. Broader
read roles cannot grant missing usages. Other declared/required roles match exactly.

### ResourceTransition

No inherent methods. Public fields: `resource: Resource`,
`before: Option<ResourceAccess>`, `after: ResourceAccess`.
None introduces previously undeclared local state and preserves contents;
Some must exactly match the current declared role. Every transition is a full
dependency point for that allocation, including a same-role transition.

### ResourceTransitions

Private retained batch, constructed only by valid transition recording.

| Method | Behavior |
| --- | --- |
| `ResourceTransitions::transitions(&self) -> &[ResourceTransition]` | Borrow immutable recorded requests. |

### ResourceUse

No inherent methods. Public fields: `command_index: usize`, `resource: Resource`,
`access: ResourceAccess`. Returned owned handles retain allocation storage.
Indices include barriers. Access summarizes a whole allocation within one command:
nonempty copies, required nonzero compute slots, nonempty draw sources, and every
attached render image (even empty/masked scopes). Unused bindings contribute no use.

### HazardKind

No inherent methods. Variants: `ReadAfterWrite`, `WriteAfterRead`, `WriteAfterWrite`.
Read/read needs no barrier. If both uses write, WriteAfterWrite takes precedence.
All conflicting cross-command uses require an intervening resource transition;
disjoint regions still conflict under whole-allocation tracking.

### DependencyEncodeError

No inherent methods. A failed recording call appends no batch.

| Variant | Meaning |
| --- | --- |
| `ExplicitModeRequired` | Ordered encoder cannot record transitions, including an empty request. |
| `ForeignResource { kind: ResourceKind, label: String }` | Allocation belongs to another device. |
| `InvalidAccess { kind: ResourceKind, label: String, access: ResourceAccess }` | Before/after role conflicts with kind or buffer usages. |
| `DuplicateResource { kind: ResourceKind, label: String }` | Batch repeats allocation identity. |
| `AllocationFailed { source: std::collections::TryReserveError }` | Retained batch/command reservation failed; retains source. |

Validation order: mode, then for each request device, before/after role support,
prior duplicate identity, then reservation. Empty Explicit batches append nothing.

### DependencyValidationError

No inherent methods. Preflight checks metadata only; it reads no bytes or poison.

| Variant | Meaning |
| --- | --- |
| `AllocationFailed` | Resource-use or state reservation failed. |
| `MissingTransition { command_index: usize, kind: ResourceKind, label: String, required: ResourceAccess }` | First actual use lacks a declaration. |
| `AccessMismatch { command_index: usize, kind: ResourceKind, label: String, declared: ResourceAccess, required: ResourceAccess }` | Actual role does not fit declared role. |
| `TransitionMismatch { command_index: usize, kind: ResourceKind, label: String, before: Option<ResourceAccess>, actual: Option<ResourceAccess> }` | Before-state differs from current local state. |
| `MissingDependency { command_index: usize, previous_command_index: usize, kind: ResourceKind, label: String, hazard: HazardKind, previous: ResourceAccess, required: ResourceAccess }` | Conflicting uses have no intervening barrier for that resource. |

These variants retain no underlying source. Submission wraps them in DependencyFailed.
Validation rebuilds local state each time: check transitions and, for each actual
use, declaration, hazard, then role. Completed synchronous submissions and CPU
access use the existing device gate; no persistent cross-submission layout exists.
Runtime storage/shader failures retain completed-prefix/atomic-operation rules.

## Explicit standard trait support

This lists implemented/derived traits, not Rust's blanket implementations or
auto traits. They are separate from the 216 inherent methods above.

| Type | Traits |
| --- | --- |
| `Instance` | `Debug`, `Default` |
| `Adapter` | `Debug`, `Clone` |
| `AdapterInfo` | `Debug`, `Clone`, `PartialEq`, `Eq` |
| `BackendKind` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `Device` | `Debug`, `Clone` |
| `Queue` | `Debug`, `Clone` |
| `BufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `BufferUsage` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `HostAccess` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `Buffer` | `Debug`, `Clone` |
| `CommandEncoder` | `Debug` |
| `CommandBuffer` | `Debug` |
| `Command` | `Debug` |
| `BufferCopy` | `Debug` |
| `CopyBufferRole` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display` |
| `CommandRecorder` | `Debug`, `Default` |
| `DebugMarkerBuffer` | `Debug` |
| `BufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `BufferCreateError` | `Debug`, `Display`, `Error` |
| `BufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `CommandEncodeError` | `Debug`, `Display`, `Error` |
| `QueueSubmitError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `FramebufferFormat` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `FramebufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `Framebuffer` | `Debug`, `Clone` |
| `FramebufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `FramebufferCreateError` | `Debug`, `Display`, `Error` |
| `FramebufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `RenderPass<'a>` | `Debug` |
| `RenderCommands` | `Debug` |
| `DrawTriangles` | `Debug` |
| `IndexFormat` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `IndexedTriangles` | `Debug` |
| `RenderEncodeError` | `Debug`, `Display`, `Error` |
| `RenderExecutionError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `ShaderStage` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `ShaderInstruction` | `Debug`, `Clone`, `Copy`, `PartialEq` |
| `ShaderDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `Shader` | `Debug`, `Clone` |
| `ShaderValidationError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `ShaderCreateError` | `Debug`, `Display`, `Error` |
| `ShaderExecutionError` | `Debug`, `Display`, `Error` |
| `VertexFormat` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `VertexAttribute` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `VertexBufferLayout<'a>` | `Debug`, `Clone`, `Copy` |
| `GraphicsPipelineDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `GraphicsPipeline` | `Debug`, `Clone` |
| `GraphicsPipelineCreateError` | `Debug`, `Display`, `Error` |
| `TextureFormat` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `TextureDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `TextureDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `TextureCreateError` | `Debug`, `Display`, `Error` |
| `TextureAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `TextureViewDescriptor` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `TextureView` | `Debug`, `Clone` |
| `Texture` | `Debug`, `Clone` |
| `FilterMode` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `AddressMode` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `SamplerDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `SamplerCreateError` | `Debug`, `Display`, `Error` |
| `Sampler` | `Debug`, `Clone` |
| `TextureSampleError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `TextureBinding` | `Debug`, `Clone` |
| `DepthBuffer` | `Debug`, `Clone` |
| `DepthBufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `DepthBufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `DepthBufferCreateError` | `Debug`, `Display`, `Error` |
| `DepthBufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `DepthState` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `CompareFunction` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `GraphicsPipelineOptions` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Default` |
| `BlendState` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BlendComponent` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BlendFactor` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BlendOperation` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `StencilBuffer` | `Debug`, `Clone` |
| `StencilBufferDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `StencilBufferDescriptorError` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `Error` |
| `StencilBufferCreateError` | `Debug`, `Display`, `Error` |
| `StencilBufferAccessError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `ComputePipeline` | `Debug`, `Clone` |
| `ComputePipelineDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `StorageBufferBinding<'a>` | `Debug`, `Clone`, `Copy` |
| `ComputeBufferBinding` | `Debug`, `Clone` |
| `ComputeDispatch` | `Debug` |
| `ComputePipelineCreateError` | `Debug`, `Display`, `Error` |
| `ComputeEncodeError` | `Debug`, `Display`, `Error` |
| `ComputeExecutionError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `StencilState` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `StencilFaceState` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `StencilOperation` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `RenderAttachments<'a>` | `Debug`, `Clone`, `Copy`, `Default` |
| `SynchronizationMode` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Default` |
| `ResourceKind` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `Resource` | `Debug`, `Clone` |
| `ResourceAccess` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `ResourceTransition` | `Debug`, `Clone` |
| `ResourceTransitions` | `Debug` |
| `ResourceUse` | `Debug`, `Clone` |
| `HazardKind` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `DependencyEncodeError` | `Debug`, `Display`, `Error` |
| `DependencyValidationError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `ShaderVisibility` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BindingType` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BindGroupLayoutEntry` | `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` |
| `BindGroupLayoutDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `BindGroupLayout` | `Debug`, `Clone` |
| `BufferBinding` | `Debug`, `Clone` |
| `BindingResource` | `Debug`, `Clone` |
| `BindGroupEntry` | `Debug`, `Clone` |
| `BindGroupDescriptor<'a>` | `Debug`, `Clone`, `Copy` |
| `BindGroup` | `Debug`, `Clone` |
| `BindingValidationError` | `Debug`, `Clone`, `PartialEq`, `Eq`, `Display`, `Error` |
| `BindingCreateError` | `Debug`, `Display`, `Error`, `From<BindingValidationError>` |

Standard trait operations include:

| Operation | Implementing types / behavior |
| --- | --- |
| `Clone::clone(&self) -> Self` | Types marked `Clone` above. Device/queue/buffer/framebuffer/depth/stencil/shader/pipeline/texture/view/sampler/binding handles retain shared state; `AdapterInfo` clones its owned metadata. |
| `Default::default() -> Self` | `Instance`, `CommandRecorder`: same as `new()`. GraphicsPipelineOptions: depth/blend None. RenderAttachments: all None. SynchronizationMode: Ordered. |
| `Debug::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | All 114 types; buffer/framebuffer/depth/stencil formatting exposes metadata rather than stored data. |
| `PartialEq::eq(&self, other: &Self) -> bool` | Types marked `PartialEq`; handle identity instead uses explicit `is_same`/`belongs_to` methods. |
| `Display::fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | Thirty-two error types and `CopyBufferRole`. |
| `Error::source(&self) -> Option<&(dyn std::error::Error + 'static)>` | All thirty-two error types. Buffer/framebuffer/depth/stencil/shader/texture creation retain descriptor/reservation sources, sampler creation retains reservation sources, pipeline creation/command/render recording and shader evaluation retain reservation sources, and submission RenderFailed/ComputeFailed retain RenderExecutionError/ComputeExecutionError. Compute creation/recording and DependencyEncodeError::AllocationFailed retain reservation sources; submission DependencyFailed retains DependencyValidationError. BindingCreateError retains validation/reservation sources; pipeline/recording Bindings variants retain BindingValidationError. Other current variants return None. |

`From<BindingValidationError>::from(source: BindingValidationError) -> BindingCreateError` wraps InvalidDescriptor.

Other inherited standard trait methods, such as `clone_from` and `ne`, follow
their standard-library contracts. Error trait objects also support standard
`is` and downcasting operations.
`Copy` and `Eq` are marker traits without methods.

## Not implemented

There are currently no public APIs for asynchronous submission/completion, mipmaps,
texture attachments/storage/transfer commands, multiple groups, binding arrays/dynamic offsets,
external shader language/format importer or frontend/compiler,
instanced draws, primitive restart, indirect drawing/dispatch,
presentation, mapped memory, hardware discovery/drivers, managed planning,
EX Witness, or EX Proof. Resource branching is an unaccepted discussion idea,
not an implemented or locked API.

See the [roadmap](docs/ROADMAP.md) for planned capabilities and
[DOCUMENTATION.md](DOCUMENTATION.md) for the project design and experimental
directions. Future methods must not be mixed into the implemented tables.

## Maintenance and generated reference

Update this catalog whenever public exports, signatures, fields, variants,
traits, or implemented status change. Keep behavioral contracts under `docs/`
consistent with this inventory. Generated Rust documentation remains the
compiler-derived reference for exact signatures, implementations, and auto traits:

```text
cargo doc --workspace --no-deps --locked --offline
```

The public crate's generated entry point is `target/doc/ex/index.html`.
