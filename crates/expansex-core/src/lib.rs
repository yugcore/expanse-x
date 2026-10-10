//! Expanse-X resources, validation and software execution.
//!
//! This crate implements the public types re-exported by `ex-api`, including
//! command recording, synchronous submission, shader interpretation and CPU
//! rasterization. Devices retain their resources; recorded work retains the handles
//! it uses. Explicit encoders declare whole-resource accesses and dependencies,
//! which are validated before submission.

mod backend;
mod bindings;
mod buffer;
mod command;
mod compute;
mod dependency;
mod depth;
mod framebuffer;
mod object;
mod pipeline;
mod profile;
mod reference;
mod render;
mod shader;
mod stencil;
mod submission;
mod texture;
mod transfer;

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

pub use bindings::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingCreateError, BindingResource, BindingType, BindingValidationError,
    BufferBinding, ShaderVisibility,
};

pub use transfer::{
    BufferTextureCopy, TextureBufferLayout, TextureCopy, TextureTransferEncodeError,
    TextureTransferExecutionError,
};

pub use profile::{CoreLimits, CoreProfile};
