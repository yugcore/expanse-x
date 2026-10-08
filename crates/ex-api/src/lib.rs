//! Public entry point for Expanse-X (EX).
//!
//! Phase 1 begins with software discovery, logical devices, and queue ownership,
//! alongside the descriptor and CPU recording prototype. Resource storage,
//! submission, and rendering are not implemented yet.
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
//! use ex::{BufferDescriptor, BufferUsage, CommandRecorder};
//!
//! let descriptor = BufferDescriptor {
//!     label: "triangle positions",
//!     size: 36,
//!     usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
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
    Adapter, AdapterInfo, BackendKind, BufferDescriptor, BufferDescriptorError, BufferUsage,
    CommandBuffer, CommandRecorder, Device, Instance, Queue,
};
