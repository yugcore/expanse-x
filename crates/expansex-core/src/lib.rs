//! Shared Expanse-X discovery, ownership, and initial recording contracts.
//!
//! Discovers the software target, creates logical devices and queue handles,
//! allocates reference buffers with explicit CPU access, and records debug labels.
//! Queue submission and rendering are not implemented yet.

mod buffer;
mod command;
mod object;

pub use buffer::{
    Buffer, BufferAccessError, BufferCreateError, BufferDescriptor, BufferDescriptorError,
    BufferUsage, HostAccess,
};
pub use command::{CommandBuffer, CommandRecorder};
pub use object::{Adapter, AdapterInfo, BackendKind, Device, Instance, Queue};
