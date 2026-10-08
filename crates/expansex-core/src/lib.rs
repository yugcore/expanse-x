//! Shared Expanse-X discovery, ownership, and initial recording contracts.
//!
//! Discovers the software target, creates logical devices and queue handles,
//! describes buffers, and records debug labels. No resource storage, submission,
//! or rendering is implemented yet.

mod buffer;
mod command;
mod object;

pub use buffer::{BufferDescriptor, BufferDescriptorError, BufferUsage};
pub use command::{CommandBuffer, CommandRecorder};
pub use object::{Adapter, AdapterInfo, BackendKind, Device, Instance, Queue};
