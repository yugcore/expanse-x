//! Public entry point for Expanse-X (EX).
//!
//! Phase 1 begins with software discovery, logical devices, and queue ownership,
//! alongside owned reference buffers with explicit host access and validated
//! buffer-copy recording, fixed-function triangles, and synchronous submission.
//! This crate has no dependency on another graphics API.
//! Vertex and fragment shader programs can be evaluated independently or bound
//! through graphics pipelines to render interpolated programmable triangles.
//!
//! ```
//! use ex::{Instance, ShaderDescriptor, ShaderInstruction as Op, ShaderStage};
//! let device = Instance::new().default_adapter().create_device();
//! let shader = device.create_shader(ShaderDescriptor {
//!     label: "translate position", stage: ShaderStage::Vertex, input_count: 1,
//!     instructions: &[
//!         Op::Input { location: 0 },
//!         Op::Constant([0.25, 0.0, 0.0, 0.0]),
//!         Op::Add { left: 0, right: 1 },
//!     ],
//!     outputs: &[2],
//! })?;
//! assert_eq!(shader.evaluate(&[[0.0, 0.5, 0.0, 1.0]])?,
//!            vec![[0.25, 0.5, 0.0, 1.0]]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```
//! use ex::{Instance, ShaderDescriptor, ShaderInstruction as Op, ShaderStage,
//!     GraphicsPipelineDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat,
//!     FramebufferDescriptor, FramebufferFormat, BufferDescriptor, BufferUsage, HostAccess, IndexFormat};
//! use ex::{TextureDescriptor, TextureFormat, SamplerDescriptor, FilterMode, AddressMode};
//! let device = Instance::new().default_adapter().create_device();
//! let vertex = device.create_shader(ShaderDescriptor {
//!     label: "position", stage: ShaderStage::Vertex, input_count: 1,
//!     instructions: &[Op::Input { location: 0 }], outputs: &[0],
//! })?;
//! let fragment = device.create_shader(ShaderDescriptor {
//!     label: "red", stage: ShaderStage::Fragment, input_count: 0,
//!     instructions: &[Op::Constant([0.5; 4]), Op::SampleTexture { binding: 0, coordinates: 0 }], outputs: &[1],
//! })?;
//! let pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
//!     label: "triangle", vertex_shader: &vertex, fragment_shader: &fragment,
//!     vertex_layout: VertexBufferLayout { stride: 8, attributes: &[
//!         VertexAttribute { location: 0, offset: 0, format: VertexFormat::Float32x2 },
//!     ] }, colour_format: FramebufferFormat::Rgba8Unorm,
//! })?;
//! let target = device.create_framebuffer(FramebufferDescriptor {
//!     label: "image", width: 4, height: 4, format: FramebufferFormat::Rgba8Unorm,
//! })?;
//! let data = [-1.0_f32, 1.0, 1.0, 1.0, -1.0, -1.0];
//! let bytes: Vec<u8> = data.iter().flat_map(|n| n.to_le_bytes()).collect();
//! let buffer = device.create_buffer(BufferDescriptor {
//!     label: "positions", size: 24, usages: &[BufferUsage::Vertex], host_access: HostAccess::Write,
//! })?;
//! buffer.write(0, &bytes)?;
//! let indices = device.create_buffer(BufferDescriptor {
//!     label: "indices", size: 6, usages: &[BufferUsage::Index], host_access: HostAccess::Write,
//! })?;
//! indices.write(0, &[0, 0, 1, 0, 2, 0])?; // u16 little-endian 0,1,2
//! let texture = device.create_texture(TextureDescriptor {
//!     label: "red texel", width: 1, height: 1, format: TextureFormat::Rgba8Unorm,
//!     host_access: HostAccess::Write,
//! })?;
//! texture.write_pixels(&[[255, 0, 0, 255]])?;
//! let sampler = device.create_sampler(SamplerDescriptor {
//!     label: "nearest", filter: FilterMode::Nearest,
//!     address_u: AddressMode::ClampToEdge, address_v: AddressMode::ClampToEdge,
//! })?;
//! let mut encoder = device.create_command_encoder();
//! let mut pass = encoder.begin_render_pass(&target, Some([0; 4]))?;
//! pass.set_pipeline(&pipeline)?;
//! pass.set_vertex_buffer(&buffer, 0)?;
//! pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)?;
//! pass.set_texture(0, &texture.view(), &sampler)?;
//! pass.draw_indexed(0..3, 0)?;
//! pass.finish()?;
//! device.default_queue().submit(&encoder.finish())?;
//! assert_eq!(target.pixel(0, 0)?, [255, 0, 0, 255]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```
//! use ex::{BackendKind, Instance};
//!
//! let instance = Instance::new();
//! let adapter = instance.default_adapter();
//! assert_eq!(adapter.info().backend, BackendKind::Reference);
//! let device = adapter.create_device();
//! let queue = device.default_queue();
//! assert!(queue.belongs_to(&device));
//! ```
//!
//! ```
//! use ex::{BufferDescriptor, BufferUsage, HostAccess, Instance};
//!
//! let device = Instance::new().default_adapter().create_device();
//! let buffer = device.create_buffer(BufferDescriptor {
//!     label: "CPU round trip",
//!     size: 4,
//!     usages: &[BufferUsage::Storage],
//!     host_access: HostAccess::ReadWrite,
//! })?;
//! buffer.write(0, &[1, 2, 3, 4])?;
//! let mut output = [0; 4];
//! buffer.read(0, &mut output)?;
//! assert_eq!(output, [1, 2, 3, 4]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Phase 2 renders fixed-layout 2D triangles into owned offscreen RGBA pixels:
//!
//! ```
//! use ex::{BufferDescriptor, BufferUsage, FramebufferDescriptor, FramebufferFormat, HostAccess, Instance};
//! let device = Instance::new().default_adapter().create_device();
//! let target = device.create_framebuffer(FramebufferDescriptor {
//!     label: "image", width: 4, height: 4, format: FramebufferFormat::Rgba8Unorm,
//! })?;
//! let points = [[-1.0_f32, 1.0], [1.0, 1.0], [-1.0, -1.0]];
//! let bytes: Vec<u8> = points.iter().flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes())).collect();
//! let vertices = device.create_buffer(BufferDescriptor {
//!     label: "positions", size: 24, usages: &[BufferUsage::Vertex], host_access: HostAccess::Write,
//! })?;
//! vertices.write(0, &bytes)?;
//! let mut encoder = device.create_command_encoder();
//! let mut pass = encoder.begin_render_pass(&target, Some([0, 0, 0, 255]))?;
//! pass.set_vertex_buffer(&vertices, 0)?;
//! pass.draw(0..3, [255, 0, 0, 255])?;
//! pass.finish()?;
//! device.default_queue().submit(&encoder.finish())?;
//! assert_eq!(target.pixel(0, 0)?, [255, 0, 0, 255]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ```
//! use ex::{BufferDescriptor, BufferUsage, CommandRecorder, HostAccess};
//!
//! let descriptor = BufferDescriptor {
//!     label: "triangle positions",
//!     size: 36,
//!     usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
//!     host_access: HostAccess::None,
//! };
//! descriptor.validate()?;
//! let mut recorder = CommandRecorder::new();
//! recorder.insert_debug_marker("triangle (planned)");
//! let commands = recorder.finish();
//! assert_eq!(commands.debug_markers(), &["triangle (planned)"]);
//! # Ok::<(), ex::BufferDescriptorError>(())
//! ```
//!
//! `finish` consumes the recorder. Markers are labels, not graphics commands.
//! Its finished output is a `DebugMarkerBuffer`, distinct from `CommandBuffer`.
//!
//! Actual copy descriptions are recorded through a device-bound encoder:
//!
//! ```
//! use ex::{BufferDescriptor, BufferUsage, HostAccess, Instance};
//! let device = Instance::new().default_adapter().create_device();
//! let source = device.create_buffer(BufferDescriptor {
//!     label: "source", size: 4, usages: &[BufferUsage::TransferSource],
//!     host_access: HostAccess::Write,
//! })?;
//! let destination = device.create_buffer(BufferDescriptor {
//!     label: "destination", size: 4, usages: &[BufferUsage::TransferDestination],
//!     host_access: HostAccess::Read,
//! })?;
//! source.write(0, &[1, 2, 3, 4])?;
//! let mut encoder = device.create_command_encoder();
//! encoder.copy_buffer(&source, 0, &destination, 0, 4)?;
//! let commands = encoder.finish();
//! assert!(commands.belongs_to(&device));
//! assert_eq!(commands.commands().len(), 1);
//! let mut bytes = [99; 4];
//! destination.read(0, &mut bytes)?;
//! assert_eq!(bytes, [0; 4]); // Recording/finishing do not execute the copy.
//! device.default_queue().submit(&commands)?;
//! destination.read(0, &mut bytes)?;
//! assert_eq!(bytes, [1, 2, 3, 4]); // Successful submission has completed.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The public entry point lets future runtime changes happen behind this crate;
//! it does not imply that these provisional types are a stable specification.
//!
//! Blend state combines a fragment with the destination; depth and stencil are independent:
//!
//! ```
//! use ex::*;
//! let d = Instance::new().default_adapter().create_device();
//! let target = d.create_framebuffer(FramebufferDescriptor {
//!     label: "blend", width: 2, height: 2, format: FramebufferFormat::Rgba8Unorm,
//! })?;
//! let depth = d.create_depth_buffer(DepthBufferDescriptor {label: "z", width: 2, height: 2})?;
//! let stencil = d.create_stencil_buffer(StencilBufferDescriptor {label: "s", width: 2, height: 2})?;
//! let v = d.create_shader(ShaderDescriptor {
//!     label: "position", stage: ShaderStage::Vertex, input_count: 1,
//!     instructions: &[ShaderInstruction::Input { location: 0 }], outputs: &[0],
//! })?;
//! let f = d.create_shader(ShaderDescriptor {
//!     label: "red", stage: ShaderStage::Fragment, input_count: 0,
//!     instructions: &[ShaderInstruction::Constant([1.0, 0.0, 0.0, 0.5])], outputs: &[0],
//! })?;
//! let pipeline = d.create_graphics_pipeline_with_stencil(GraphicsPipelineDescriptor {
//!     label: "transparent", vertex_shader: &v, fragment_shader: &f,
//!     vertex_layout: VertexBufferLayout { stride: 16, attributes: &[VertexAttribute {
//!         location: 0, offset: 0, format: VertexFormat::Float32x4,
//!     }] }, colour_format: FramebufferFormat::Rgba8Unorm,
//! }, GraphicsPipelineOptions {
//!     depth: Some(DepthState { compare: CompareFunction::Less, write_enabled: false }),
//!     blend: Some(BlendState::alpha_blending()),
//! }, StencilState::replace(3))?;
//! let bytes: Vec<_> = [[-1_f32, 1., 0., 1.], [1., 1., 0., 1.], [-1., -1., 0., 1.]]
//!     .iter().flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes())).collect();
//! let vertices = d.create_buffer(BufferDescriptor {
//!     label: "triangle", size: bytes.len() as u64,
//!     usages: &[BufferUsage::Vertex], host_access: HostAccess::Write,
//! })?;
//! vertices.write(0, &bytes)?;
//! let mut e = d.create_command_encoder();
//! let mut pass = e.begin_render_pass_with_attachments(&target, Some([0, 0, 255, 255]),
//!     RenderAttachments { depth: Some(&depth), clear_depth: Some(1.),
//!         stencil: Some(&stencil), clear_stencil: Some(0) })?;
//! pass.set_pipeline(&pipeline)?;
//! pass.set_vertex_buffer(&vertices, 0)?;
//! pass.draw_programmable(0..3)?;
//! pass.finish()?;
//! d.default_queue().submit(&e.finish())?;
//! assert_eq!(target.pixel(0, 0)?, [128, 0, 128, 255]);
//! assert_eq!(depth.depth(0, 0)?, 1.0); // Testing need not write depth.
//! assert_eq!(stencil.stencil(0, 0)?, 3); // Colour/depth policy does not suppress stencil.
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub use expansex_core::{
    Adapter, AdapterInfo, BackendKind, Buffer, BufferAccessError, BufferCopy, BufferCreateError,
    BufferDescriptor, BufferDescriptorError, BufferUsage, Command, CommandBuffer,
    CommandEncodeError, CommandEncoder, CommandRecorder, CopyBufferRole, DebugMarkerBuffer, Device,
    DrawTriangles, Framebuffer, FramebufferAccessError, FramebufferCreateError,
    FramebufferDescriptor, FramebufferDescriptorError, FramebufferFormat, GraphicsPipeline,
    GraphicsPipelineCreateError, GraphicsPipelineDescriptor, HostAccess, IndexFormat,
    IndexedTriangles, Instance, Queue, QueueSubmitError, RenderCommands, RenderEncodeError,
    RenderExecutionError, RenderPass, Shader, ShaderCreateError, ShaderDescriptor,
    ShaderExecutionError, ShaderInstruction, ShaderStage, ShaderValidationError, VertexAttribute,
    VertexBufferLayout, VertexFormat,
};

pub use expansex_core::{
    AddressMode, FilterMode, Sampler, SamplerCreateError, SamplerDescriptor, Texture,
    TextureAccessError, TextureBinding, TextureCreateError, TextureDescriptor,
    TextureDescriptorError, TextureFormat, TextureSampleError, TextureView, TextureViewDescriptor,
};

pub use expansex_core::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, GraphicsPipelineOptions,
};
pub use expansex_core::{
    CompareFunction, DepthBuffer, DepthBufferAccessError, DepthBufferCreateError,
    DepthBufferDescriptor, DepthBufferDescriptorError, DepthState,
};
pub use expansex_core::{
    RenderAttachments, StencilBuffer, StencilBufferAccessError, StencilBufferCreateError,
    StencilBufferDescriptor, StencilBufferDescriptorError, StencilFaceState, StencilOperation,
    StencilState,
};

pub use expansex_core::{
    ComputeBufferBinding, ComputeDispatch, ComputeEncodeError, ComputeExecutionError,
    ComputePipeline, ComputePipelineCreateError, ComputePipelineDescriptor, StorageBufferBinding,
};

pub use expansex_core::{
    DependencyEncodeError, DependencyValidationError, HazardKind, Resource, ResourceAccess,
    ResourceKind, ResourceTransition, ResourceTransitions, ResourceUse, SynchronizationMode,
};

pub use expansex_core::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingCreateError, BindingResource, BindingType, BindingValidationError,
    BufferBinding, ShaderVisibility,
};

pub use expansex_core::{
    BufferTextureCopy, TextureBufferLayout, TextureCopy, TextureTransferEncodeError,
    TextureTransferExecutionError,
};

pub use expansex_core::{CoreLimits, CoreProfile};
