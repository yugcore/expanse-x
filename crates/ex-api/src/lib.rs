//! Public entry point for Expanse-X (EX).
//!
//! Phase 1 begins with software discovery, logical devices, and queue ownership,
//! alongside owned reference buffers with explicit host access and label-only
//! recording. Queue submission and rendering are not implemented yet.
//! This crate has no dependency on another graphics API.
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
//! The public entry point lets future runtime changes happen behind this crate;
//! it does not imply that these provisional types are a stable specification.

pub use expansex_core::{
    Adapter, AdapterInfo, BackendKind, Buffer, BufferAccessError, BufferCreateError,
    BufferDescriptor, BufferDescriptorError, BufferUsage, CommandBuffer, CommandRecorder, Device,
    HostAccess, Instance, Queue,
};
