use crate::{Device, FramebufferFormat, Shader, ShaderStage};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

mod blend;
pub use blend::{BlendComponent, BlendFactor, BlendOperation, BlendState};
mod stencil;
pub use stencil::{StencilFaceState, StencilOperation, StencilState};

/// Optional immutable rendering state. Default preserves overwrite without depth testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GraphicsPipelineOptions {
    pub depth: Option<DepthState>,
    pub blend: Option<BlendState>,
}

/// Little-endian f32 vertex storage; omitted components expand to (0,0,0,1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
}
impl VertexFormat {
    /// Size of one attribute in bytes.
    pub fn size(&self) -> u64 {
        match self {
            Self::Float32 => 4,
            Self::Float32x2 => 8,
            Self::Float32x3 => 12,
            Self::Float32x4 => 16,
        }
    }
}

/// One vertex shader input sourced from the bound vertex buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexAttribute {
    pub location: u32,
    /// Byte offset within each vertex; aligned to four bytes.
    pub offset: u64,
    pub format: VertexFormat,
}

/// Borrowed single-buffer layout; pipeline creation owns the attributes.
#[derive(Debug, Clone, Copy)]
pub struct VertexBufferLayout<'a> {
    /// Byte distance between consecutive vertices; positive and aligned to four.
    pub stride: u64,
    pub attributes: &'a [VertexAttribute],
}

/// Borrowed pipeline request; creation retains shaders and owns metadata/layout.
#[derive(Debug, Clone, Copy)]
pub struct GraphicsPipelineDescriptor<'a> {
    pub label: &'a str,
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
    pub vertex_layout: VertexBufferLayout<'a>,
    pub colour_format: FramebufferFormat,
}

#[derive(Debug)]
pub enum GraphicsPipelineCreateError {
    Bindings {
        source: crate::BindingValidationError,
    },
    ForeignShader {
        stage: ShaderStage,
        label: String,
    },
    WrongShaderStage {
        expected: ShaderStage,
        actual: ShaderStage,
    },
    InvalidStride {
        stride: u64,
    },
    AttributeCountMismatch {
        expected: u32,
        actual: usize,
    },
    InvalidAttributeLocation {
        location: u32,
        input_count: u32,
    },
    DuplicateAttributeLocation {
        location: u32,
    },
    UnalignedAttributeOffset {
        location: u32,
        offset: u64,
    },
    AttributeOutOfBounds {
        location: u32,
        offset: u64,
        size: u64,
        stride: u64,
    },
    StageInterfaceMismatch {
        vertex_varyings: usize,
        fragment_inputs: u32,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}
impl fmt::Display for GraphicsPipelineCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bindings { source } => write!(f, "graphics resource interface: {source}"),
            Self::ForeignShader { stage, label } => write!(f, "{stage:?} shader {label:?} belongs to another device"),
            Self::WrongShaderStage { expected, actual } => write!(f, "pipeline requires a {expected:?} shader, received {actual:?}"),
            Self::InvalidStride { stride } => write!(f, "vertex stride {stride} must be positive and aligned to four bytes"),
            Self::AttributeCountMismatch { expected, actual } => write!(f, "vertex layout needs {expected} attributes, received {actual}"),
            Self::InvalidAttributeLocation { location, input_count } => write!(f, "vertex attribute location {location} is outside {input_count} inputs"),
            Self::DuplicateAttributeLocation { location } => write!(f, "vertex input location {location} occurs more than once"),
            Self::UnalignedAttributeOffset { location, offset } => write!(f, "vertex attribute {location} offset {offset} must be aligned to four bytes"),
            Self::AttributeOutOfBounds { location, offset, size, stride } => write!(f, "vertex attribute {location} ({size} bytes at offset {offset}) exceeds stride {stride}"),
            Self::StageInterfaceMismatch { vertex_varyings, fragment_inputs } => write!(f, "vertex shader provides {vertex_varyings} varyings, fragment shader requires {fragment_inputs} inputs"),
            Self::AllocationFailed { source } => write!(f, "cannot reserve pipeline metadata: {source}"),
        }
    }
}
impl Error for GraphicsPipelineCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bindings { source } => Some(source),
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct PipelineState {
    device: Device,
    label: String,
    vertex_shader: Shader,
    fragment_shader: Shader,
    stride: u64,
    attributes: Vec<VertexAttribute>,
    colour_format: FramebufferFormat,
    depth: Option<DepthState>,
    blend: Option<BlendState>,
    stencil: Option<StencilState>,
    binding_layout: Option<crate::BindGroupLayout>,
}

/// An immutable validated graphics pipeline, shared by clones and recorded draws.
#[derive(Debug, Clone)]
pub struct GraphicsPipeline {
    state: Arc<PipelineState>,
}

impl Device {
    /// Validate shader ownership/stages, layout, and interfaces, then own a pipeline.
    pub fn create_graphics_pipeline(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        self.create_graphics_pipeline_with_options(descriptor, GraphicsPipelineOptions::default())
    }
    /// Add immutable depth comparison/write state without changing existing descriptors.
    pub fn create_graphics_pipeline_with_depth(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
        depth: DepthState,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        self.create_graphics_pipeline_with_options(
            descriptor,
            GraphicsPipelineOptions {
                depth: Some(depth),
                blend: None,
            },
        )
    }
    /// Validate and retain one pipeline with independently optional depth and blending.
    pub fn create_graphics_pipeline_with_options(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
        options: GraphicsPipelineOptions,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        self.create_pipeline(descriptor, options, None, None)
    }
    /// Add immutable stencil rules while preserving existing option literals.
    pub fn create_graphics_pipeline_with_stencil(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
        options: GraphicsPipelineOptions,
        stencil: StencilState,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        self.create_pipeline(descriptor, options, Some(stencil), None)
    }
    /// Retain and validate a reusable resource interface for both shader stages.
    pub fn create_graphics_pipeline_with_bindings(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
        options: GraphicsPipelineOptions,
        stencil: Option<StencilState>,
        layout: &crate::BindGroupLayout,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        self.create_pipeline(descriptor, options, stencil, Some(layout))
    }
    fn create_pipeline(
        &self,
        descriptor: GraphicsPipelineDescriptor<'_>,
        options: GraphicsPipelineOptions,
        stencil: Option<StencilState>,
        binding_layout: Option<&crate::BindGroupLayout>,
    ) -> Result<GraphicsPipeline, GraphicsPipelineCreateError> {
        for (shader, expected) in [
            (descriptor.vertex_shader, ShaderStage::Vertex),
            (descriptor.fragment_shader, ShaderStage::Fragment),
        ] {
            if !shader.belongs_to(self) {
                return Err(GraphicsPipelineCreateError::ForeignShader {
                    stage: expected,
                    label: shader.label().to_owned(),
                });
            }
            if shader.stage() != expected {
                return Err(GraphicsPipelineCreateError::WrongShaderStage {
                    expected,
                    actual: shader.stage(),
                });
            }
        }
        for shader in [descriptor.vertex_shader, descriptor.fragment_shader] {
            crate::bindings::validate_shader(self, binding_layout, shader, &[])
                .map_err(|source| GraphicsPipelineCreateError::Bindings { source })?;
        }
        let layout = descriptor.vertex_layout;
        if layout.stride == 0 || !layout.stride.is_multiple_of(4) {
            return Err(GraphicsPipelineCreateError::InvalidStride {
                stride: layout.stride,
            });
        }
        let input_count = descriptor.vertex_shader.input_count();
        if layout.attributes.len() != input_count as usize {
            return Err(GraphicsPipelineCreateError::AttributeCountMismatch {
                expected: input_count,
                actual: layout.attributes.len(),
            });
        }
        let mut seen = [false; 16];
        for attribute in layout.attributes {
            if attribute.location >= input_count {
                return Err(GraphicsPipelineCreateError::InvalidAttributeLocation {
                    location: attribute.location,
                    input_count,
                });
            }
            if seen[attribute.location as usize] {
                return Err(GraphicsPipelineCreateError::DuplicateAttributeLocation {
                    location: attribute.location,
                });
            }
            seen[attribute.location as usize] = true;
            if !attribute.offset.is_multiple_of(4) {
                return Err(GraphicsPipelineCreateError::UnalignedAttributeOffset {
                    location: attribute.location,
                    offset: attribute.offset,
                });
            }
            if attribute
                .offset
                .checked_add(attribute.format.size())
                .filter(|&end| end <= layout.stride)
                .is_none()
            {
                return Err(GraphicsPipelineCreateError::AttributeOutOfBounds {
                    location: attribute.location,
                    offset: attribute.offset,
                    size: attribute.format.size(),
                    stride: layout.stride,
                });
            }
        }
        let vertex_varyings = descriptor.vertex_shader.outputs().len() - 1;
        if vertex_varyings != descriptor.fragment_shader.input_count() as usize {
            return Err(GraphicsPipelineCreateError::StageInterfaceMismatch {
                vertex_varyings,
                fragment_inputs: descriptor.fragment_shader.input_count(),
            });
        }
        let mut attributes = Vec::new();
        attributes
            .try_reserve_exact(layout.attributes.len())
            .map_err(|source| GraphicsPipelineCreateError::AllocationFailed { source })?;
        attributes.extend_from_slice(layout.attributes);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| GraphicsPipelineCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(GraphicsPipeline {
            state: Arc::new(PipelineState {
                device: self.clone(),
                label,
                vertex_shader: descriptor.vertex_shader.clone(),
                fragment_shader: descriptor.fragment_shader.clone(),
                stride: layout.stride,
                attributes,
                colour_format: descriptor.colour_format,
                depth: options.depth,
                blend: options.blend,
                stencil,
                binding_layout: binding_layout.cloned(),
            }),
        })
    }
}
impl GraphicsPipeline {
    pub fn binding_layout(&self) -> Option<&crate::BindGroupLayout> {
        self.state.binding_layout.as_ref()
    }
    pub fn stencil_state(&self) -> Option<StencilState> {
        self.state.stencil
    }
    pub fn blend_state(&self) -> Option<BlendState> {
        self.state.blend
    }
    pub fn depth_state(&self) -> Option<DepthState> {
        self.state.depth
    }
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn vertex_shader(&self) -> &Shader {
        &self.state.vertex_shader
    }
    pub fn fragment_shader(&self) -> &Shader {
        &self.state.fragment_shader
    }
    pub fn vertex_stride(&self) -> u64 {
        self.state.stride
    }
    pub fn vertex_attributes(&self) -> &[VertexAttribute] {
        &self.state.attributes
    }
    pub fn colour_format(&self) -> FramebufferFormat {
        self.state.colour_format
    }
    pub fn device(&self) -> Device {
        self.state.device.clone()
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.state.device.is_same(device)
    }
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instance, ShaderDescriptor, ShaderInstruction};
    #[test]
    fn final_pipeline_owner_releases_retained_shaders() {
        let device = Instance::new().default_adapter().create_device();
        let vertex = device
            .create_shader(ShaderDescriptor {
                label: "v",
                stage: ShaderStage::Vertex,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([0.0, 0.0, 0.0, 1.0])],
                outputs: &[0],
            })
            .unwrap();
        let fragment = device
            .create_shader(ShaderDescriptor {
                label: "f",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(GraphicsPipelineDescriptor {
                label: "p",
                vertex_shader: &vertex,
                fragment_shader: &fragment,
                vertex_layout: VertexBufferLayout {
                    stride: 4,
                    attributes: &[],
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let state = Arc::downgrade(&pipeline.state);
        let clone = pipeline.clone();
        drop(pipeline);
        drop(vertex);
        drop(fragment);
        drop(device);
        assert!(state.upgrade().is_some());
        assert_eq!(
            clone.fragment_shader().evaluate(&[]).unwrap(),
            vec![[1.0; 4]]
        );
        drop(clone);
        assert!(state.upgrade().is_none());
    }
}

/// Exact comparison for projected depth and masked stencil reference/storage values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}
impl CompareFunction {
    pub(crate) fn test(self, incoming: f32, stored: f32) -> bool {
        match self {
            Self::Never => false,
            Self::Less => incoming < stored,
            Self::Equal => incoming == stored,
            Self::LessEqual => incoming <= stored,
            Self::Greater => incoming > stored,
            Self::NotEqual => incoming != stored,
            Self::GreaterEqual => incoming >= stored,
            Self::Always => true,
        }
    }
}
/// Immutable pipeline depth comparison and write control; attaching storage is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthState {
    pub compare: CompareFunction,
    pub write_enabled: bool,
}
