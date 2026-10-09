//! Shared Expanse-X discovery, ownership, and initial recording contracts.
//!
//! Discovers the software target, creates logical devices and queue handles,
//! allocates reference buffers with explicit CPU access, and records validated
//! buffer-copy descriptions independently of execution. Queues execute recorded
//! copies and fixed/programmed triangles synchronously on the CPU. Owned shaders,
//! pipelines, layouts, perspective interpolation, sampled textures, depth tests,
//! colour blending, stencil tests/updates, and storage-buffer compute dispatch work.
//! Ordered encoders preserve synchronous visibility; Explicit encoders declare
//! whole-resource roles and dependencies validated before submission.
//! Prototype labels remain.

mod buffer;
mod command;
mod compute;
mod dependency;
mod depth;
mod framebuffer;
mod object;
mod pipeline;
mod reference;
mod render;
mod shader;
mod stencil;
mod submission;
mod texture;

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
pub use pipeline::{
    GraphicsPipeline, GraphicsPipelineCreateError, GraphicsPipelineDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat,
};
pub use render::{
    DrawTriangles, IndexFormat, IndexedTriangles, RenderCommands, RenderEncodeError,
    RenderExecutionError, RenderPass, TextureBinding,
};
pub use shader::{
    Shader, ShaderCreateError, ShaderDescriptor, ShaderExecutionError, ShaderInstruction,
    ShaderStage, ShaderValidationError,
};
pub use submission::QueueSubmitError;

pub use texture::{
    AddressMode, FilterMode, Sampler, SamplerCreateError, SamplerDescriptor, Texture,
    TextureAccessError, TextureCreateError, TextureDescriptor, TextureDescriptorError,
    TextureFormat, TextureSampleError, TextureView, TextureViewDescriptor,
};

pub use depth::{
    DepthBuffer, DepthBufferAccessError, DepthBufferCreateError, DepthBufferDescriptor,
    DepthBufferDescriptorError,
};
pub use pipeline::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, GraphicsPipelineOptions,
};
pub use pipeline::{CompareFunction, DepthState};
pub use pipeline::{StencilFaceState, StencilOperation, StencilState};
pub use render::RenderAttachments;
pub use stencil::{
    StencilBuffer, StencilBufferAccessError, StencilBufferCreateError, StencilBufferDescriptor,
    StencilBufferDescriptorError,
};

pub use compute::{
    ComputeBufferBinding, ComputeDispatch, ComputeEncodeError, ComputeExecutionError,
    ComputePipeline, ComputePipelineCreateError, ComputePipelineDescriptor, StorageBufferBinding,
};

pub use dependency::{
    DependencyEncodeError, DependencyValidationError, HazardKind, Resource, ResourceAccess,
    ResourceKind, ResourceTransition, ResourceTransitions, ResourceUse, SynchronizationMode,
};
