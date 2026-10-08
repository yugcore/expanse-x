//! Shared Expanse-X discovery, ownership, and initial recording contracts.
//!
//! Discovers the software target, creates logical devices and queue handles,
//! allocates reference buffers with explicit CPU access, and records validated
//! buffer-copy descriptions independently of execution. Queues execute recorded
//! copies and fixed-function triangles synchronously on the CPU. Independent
//! validated vertex/fragment shader evaluation also works. Prototype labels remain.

mod buffer;
mod command;
mod framebuffer;
mod object;
mod reference;
mod render;
mod shader;
mod submission;

pub use buffer::{
    Buffer, BufferAccessError, BufferCreateError, BufferDescriptor, BufferDescriptorError,
    BufferUsage, HostAccess,
};
pub use command::{
    BufferCopy, Command, CommandBuffer, CommandEncodeError, CommandEncoder, CommandRecorder,
    CopyBufferRole, DebugMarkerBuffer,
};
pub use framebuffer::{
    Framebuffer, FramebufferAccessError, FramebufferCreateError, FramebufferDescriptor,
    FramebufferDescriptorError, FramebufferFormat,
};
pub use object::{Adapter, AdapterInfo, BackendKind, Device, Instance, Queue};
pub use render::{
    DrawTriangles, RenderCommands, RenderEncodeError, RenderExecutionError, RenderPass,
};
pub use shader::{
    Shader, ShaderCreateError, ShaderDescriptor, ShaderExecutionError, ShaderInstruction,
    ShaderStage, ShaderValidationError,
};
pub use submission::QueueSubmitError;
