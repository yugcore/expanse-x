//! Public entry point for Expanse-X (EX).
//!
//! Phase 0 exposes the existing descriptor and CPU recording prototype. No
//! device, queue, resource allocation, submission, or rendering exists yet.
//! This crate has no dependency on another graphics API.
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
    BufferDescriptor, BufferDescriptorError, BufferUsage, CommandBuffer, CommandRecorder,
};
