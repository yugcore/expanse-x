//! Public entry point for Expanse-X (EX).
//!
//! Phase 1 begins with software discovery, logical devices, and queue ownership,
//! alongside owned reference buffers with explicit host access and validated
//! buffer-copy recording, fixed-function triangles, and synchronous submission.
//! This crate has no dependency on another graphics API.
//! Vertex and fragment shader programs can also be validated and evaluated
//! independently; pipeline binding and programmable drawing are the next step.
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

pub use expansex_core::{
    Adapter, AdapterInfo, BackendKind, Buffer, BufferAccessError, BufferCopy, BufferCreateError,
    BufferDescriptor, BufferDescriptorError, BufferUsage, Command, CommandBuffer,
    CommandEncodeError, CommandEncoder, CommandRecorder, CopyBufferRole, DebugMarkerBuffer, Device,
    DrawTriangles, Framebuffer, FramebufferAccessError, FramebufferCreateError,
    FramebufferDescriptor, FramebufferDescriptorError, FramebufferFormat, HostAccess, Instance,
    Queue, QueueSubmitError, RenderCommands, RenderEncodeError, RenderExecutionError, RenderPass,
    Shader, ShaderCreateError, ShaderDescriptor, ShaderExecutionError, ShaderInstruction,
    ShaderStage, ShaderValidationError,
};
