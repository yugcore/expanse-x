//! Shared Expanse-X discovery, ownership, and initial recording contracts.
//!
//! Discovers the software target, creates logical devices and queue handles,
//! allocates reference buffers with explicit CPU access, and records validated
//! buffer-copy descriptions independently of execution. Queues execute recorded
//! copies synchronously on the CPU. Prototype labels remain; rendering is absent.

mod buffer;
mod command;
mod object;
mod submission;

pub use buffer::{
    Buffer, BufferAccessError, BufferCreateError, BufferDescriptor, BufferDescriptorError,
    BufferUsage, HostAccess,
};
pub use command::{
    BufferCopy, Command, CommandBuffer, CommandEncodeError, CommandEncoder, CommandRecorder,
    CopyBufferRole, DebugMarkerBuffer,
};
pub use object::{Adapter, AdapterInfo, BackendKind, Device, Instance, Queue};
pub use submission::QueueSubmitError;
