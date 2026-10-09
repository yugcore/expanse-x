use crate::{
    Buffer, BufferUsage, Command, CommandEncoder, Framebuffer, FramebufferFormat, GraphicsPipeline,
    ShaderStage,
};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::ops::Range;
mod index;
mod textures;
pub use index::{IndexFormat, IndexedTriangles};
pub use textures::TextureBinding;

/// Recording misuse leaves previous work and the current binding unchanged.
#[derive(Debug)]
pub enum RenderEncodeError {
    ForeignStencilBuffer {
        label: String,
    },
    StencilExtentMismatch {
        colour_width: u32,
        colour_height: u32,
        stencil_width: u32,
        stencil_height: u32,
    },
    MissingStencilBuffer,
    ForeignDepthBuffer {
        label: String,
    },
    DepthExtentMismatch {
        colour_width: u32,
        colour_height: u32,
        depth_width: u32,
        depth_height: u32,
    },
    InvalidDepthClear,
    MissingDepthBuffer,
    InvalidTextureSlot {
        binding: u32,
    },
    ForeignTexture {
        label: String,
    },
    ForeignSampler {
        label: String,
    },
    MissingTextureBinding {
        binding: u32,
    },
    ForeignFramebuffer {
        label: String,
    },
    ForeignVertexBuffer {
        label: String,
    },
    MissingVertexUsage {
        label: String,
    },
    VertexOffsetOutOfBounds {
        label: String,
        offset: u64,
        buffer_size: u64,
    },
    UnalignedVertexOffset {
        offset: u64,
    },
    MissingVertexBuffer,
    MissingIndexBuffer,
    ForeignIndexBuffer {
        label: String,
    },
    MissingIndexUsage {
        label: String,
    },
    IndexOffsetOutOfBounds {
        label: String,
        offset: u64,
        buffer_size: u64,
    },
    UnalignedIndexOffset {
        offset: u64,
        alignment: u64,
    },
    InvalidIndexRange {
        start: u32,
        end: u32,
    },
    IndexRangeOutOfBounds {
        label: String,
        first_index: u32,
        index_count: u32,
        offset: u64,
        buffer_size: u64,
    },
    MissingPipeline,
    ForeignPipeline {
        label: String,
    },
    IncompatibleTargetFormat {
        pipeline_format: FramebufferFormat,
        target_format: FramebufferFormat,
    },
    InvalidVertexRange {
        start: u32,
        end: u32,
    },
    IncompleteTriangle {
        vertex_count: u32,
    },
    VertexRangeOutOfBounds {
        label: String,
        first_vertex: u32,
        vertex_count: u32,
        offset: u64,
        buffer_size: u64,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}

impl fmt::Display for RenderEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignStencilBuffer { label } => write!(f, "stencil buffer {label:?} belongs to another device"),
            Self::StencilExtentMismatch { colour_width, colour_height, stencil_width, stencil_height } => write!(f, "stencil extent {stencil_width}x{stencil_height} must equal colour extent {colour_width}x{colour_height}"),
            Self::MissingStencilBuffer => f.write_str("stencil drawing or clearing requires a stencil attachment"),
            Self::ForeignDepthBuffer { label } => write!(f, "depth buffer {label:?} belongs to another device"),
            Self::DepthExtentMismatch { colour_width, colour_height, depth_width, depth_height } => write!(f, "depth extent {depth_width}x{depth_height} must equal colour extent {colour_width}x{colour_height}"),
            Self::InvalidDepthClear => f.write_str("depth clear must be finite and in [0,1]"),
            Self::MissingDepthBuffer => f.write_str("depth-enabled drawing requires a depth attachment"),
            Self::InvalidTextureSlot { binding } => write!(f, "texture slot {binding} is outside 0..8"),
            Self::ForeignTexture { label } => write!(f, "texture {label:?} belongs to another device"),
            Self::ForeignSampler { label } => write!(f, "sampler {label:?} belongs to another device"),
            Self::MissingTextureBinding { binding } => write!(f, "bind a texture/sampler pair at required slot {binding}"),
            Self::ForeignFramebuffer { label } => write!(f, "framebuffer {label:?} belongs to another device; use the encoder's device"),
            Self::ForeignVertexBuffer { label } => write!(f, "vertex buffer {label:?} belongs to another device"),
            Self::MissingVertexUsage { label } => write!(f, "buffer {label:?} needs Vertex usage"),
            Self::VertexOffsetOutOfBounds { label, offset, buffer_size } => write!(f, "vertex binding offset {offset} exceeds buffer {label:?} size {buffer_size}"),
            Self::UnalignedVertexOffset { offset } => write!(f, "vertex binding offset {offset} must be aligned to 4 bytes"),
            Self::MissingVertexBuffer => f.write_str("bind a Vertex buffer before drawing"),
            Self::MissingIndexBuffer => f.write_str("bind an Index buffer before indexed drawing"),
            Self::ForeignIndexBuffer { label } => write!(f, "index buffer {label:?} belongs to another device"),
            Self::MissingIndexUsage { label } => write!(f, "buffer {label:?} needs Index usage"),
            Self::IndexOffsetOutOfBounds { label, offset, buffer_size } => write!(f, "index binding offset {offset} exceeds buffer {label:?} size {buffer_size}"),
            Self::UnalignedIndexOffset { offset, alignment } => write!(f, "index binding offset {offset} must align to {alignment} bytes"),
            Self::InvalidIndexRange { start, end } => write!(f, "index range {start}..{end} must be ascending"),
            Self::IndexRangeOutOfBounds { label, first_index, index_count, offset, buffer_size } => write!(f, "{index_count} indices from {first_index} at offset {offset} do not fit buffer {label:?} ({buffer_size} bytes)"),
            Self::MissingPipeline => f.write_str("bind a graphics pipeline before programmable drawing"),
            Self::ForeignPipeline { label } => write!(f, "pipeline {label:?} belongs to another device"),
            Self::IncompatibleTargetFormat { pipeline_format, target_format } => write!(f, "pipeline colour format {pipeline_format:?} does not match target {target_format:?}"),
            Self::InvalidVertexRange { start, end } => write!(f, "vertex range {start}..{end} must be ascending"),
            Self::IncompleteTriangle { vertex_count } => write!(f, "triangle-list vertex count {vertex_count} must be divisible by three"),
            Self::VertexRangeOutOfBounds { label, first_vertex, vertex_count, offset, buffer_size } => write!(f, "{vertex_count} vertices from {first_vertex} at binding offset {offset} do not fit in buffer {label:?} ({buffer_size} bytes)"),
            Self::AllocationFailed { source } => write!(f, "cannot reserve recorded render work: {source}"),
        }
    }
}
impl Error for RenderEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

/// One triangle-list draw retaining live vertex bytes and optional pipeline.
#[derive(Debug)]
pub struct DrawTriangles {
    vertex_buffer: Buffer,
    vertex_offset: u64,
    vertices: Range<u32>,
    colour: [u8; 4],
    pipeline: Option<GraphicsPipeline>,
    indexed: Option<IndexedTriangles>,
    textures: [Option<TextureBinding>; 8],
}

impl DrawTriangles {
    pub fn vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }
    pub fn vertex_offset(&self) -> u64 {
        self.vertex_offset
    }
    pub fn vertices(&self) -> Range<u32> {
        self.vertices.clone()
    }
    pub fn colour(&self) -> [u8; 4] {
        self.colour
    }
    /// None selects the fixed-layout/colour path; Some selects shader execution.
    pub fn pipeline(&self) -> Option<&GraphicsPipeline> {
        self.pipeline.as_ref()
    }
    /// Borrow a retained shader-used texture slot; out-of-range slots return None.
    pub fn texture(&self, binding: u32) -> Option<&TextureBinding> {
        self.textures.get(binding as usize).and_then(Option::as_ref)
    }
    /// Some uses retained indices; None uses the direct vertex range.
    pub fn indexed(&self) -> Option<&IndexedTriangles> {
        self.indexed.as_ref()
    }
}

/// A finished render scope. Only RenderPass can construct its validated draws.
#[derive(Debug)]
pub struct RenderCommands {
    target: Framebuffer,
    clear: Option<[u8; 4]>,
    draws: Vec<DrawTriangles>,
    depth: Option<crate::DepthBuffer>,
    clear_depth: Option<f32>,
    stencil: Option<crate::StencilBuffer>,
    clear_stencil: Option<u8>,
}

impl RenderCommands {
    pub fn stencil_buffer(&self) -> Option<&crate::StencilBuffer> {
        self.stencil.as_ref()
    }
    pub fn clear_stencil(&self) -> Option<u8> {
        self.clear_stencil
    }
    pub fn depth_buffer(&self) -> Option<&crate::DepthBuffer> {
        self.depth.as_ref()
    }
    pub fn clear_depth(&self) -> Option<f32> {
        self.clear_depth
    }
    pub fn target(&self) -> &Framebuffer {
        &self.target
    }
    pub fn clear_colour(&self) -> Option<[u8; 4]> {
        self.clear
    }
    pub fn draws(&self) -> &[DrawTriangles] {
        &self.draws
    }
}

/// Borrowed attachments; recording retains their handles, not these references.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderAttachments<'a> {
    pub depth: Option<&'a crate::DepthBuffer>,
    pub clear_depth: Option<f32>,
    pub stencil: Option<&'a crate::StencilBuffer>,
    pub clear_stencil: Option<u8>,
}

/// Exclusively borrows an encoder. Finish commits; dropping discards this pass.
#[derive(Debug)]
#[must_use = "finish the render pass to record its work; dropping discards it"]
pub struct RenderPass<'a> {
    encoder: &'a mut CommandEncoder,
    commands: RenderCommands,
    binding: Option<(Buffer, u64)>,
    pipeline: Option<GraphicsPipeline>,
    index_binding: Option<(Buffer, u64, IndexFormat)>,
    textures: [Option<TextureBinding>; 8],
}

impl CommandEncoder {
    /// Open a scope retaining same-device, same-extent depth storage. None clears preserve data.
    pub fn begin_render_pass_with_depth(
        &mut self,
        target: &Framebuffer,
        clear: Option<[u8; 4]>,
        depth: &crate::DepthBuffer,
        clear_depth: Option<f32>,
    ) -> Result<RenderPass<'_>, RenderEncodeError> {
        self.begin_render_pass_with_attachments(
            target,
            clear,
            RenderAttachments {
                depth: Some(depth),
                clear_depth,
                ..RenderAttachments::default()
            },
        )
    }
    /// Retain optional same-device, same-extent depth/stencil storage and clears.
    pub fn begin_render_pass_with_attachments(
        &mut self,
        target: &Framebuffer,
        clear: Option<[u8; 4]>,
        attachments: RenderAttachments<'_>,
    ) -> Result<RenderPass<'_>, RenderEncodeError> {
        if !target.belongs_to(&self.device) {
            return Err(RenderEncodeError::ForeignFramebuffer {
                label: target.label().to_owned(),
            });
        }
        if let Some(depth) = attachments.depth {
            if !depth.belongs_to(&self.device) {
                return Err(RenderEncodeError::ForeignDepthBuffer {
                    label: depth.label().to_owned(),
                });
            }
            if target.width() != depth.width() || target.height() != depth.height() {
                return Err(RenderEncodeError::DepthExtentMismatch {
                    colour_width: target.width(),
                    colour_height: target.height(),
                    depth_width: depth.width(),
                    depth_height: depth.height(),
                });
            }
            if attachments
                .clear_depth
                .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            {
                return Err(RenderEncodeError::InvalidDepthClear);
            }
        } else if attachments.clear_depth.is_some() {
            return Err(RenderEncodeError::MissingDepthBuffer);
        }
        if let Some(stencil) = attachments.stencil {
            if !stencil.belongs_to(&self.device) {
                return Err(RenderEncodeError::ForeignStencilBuffer {
                    label: stencil.label().to_owned(),
                });
            }
            if target.width() != stencil.width() || target.height() != stencil.height() {
                return Err(RenderEncodeError::StencilExtentMismatch {
                    colour_width: target.width(),
                    colour_height: target.height(),
                    stencil_width: stencil.width(),
                    stencil_height: stencil.height(),
                });
            }
        } else if attachments.clear_stencil.is_some() {
            return Err(RenderEncodeError::MissingStencilBuffer);
        }
        Ok(RenderPass {
            encoder: self,
            commands: RenderCommands {
                target: target.clone(),
                clear,
                draws: Vec::new(),
                depth: attachments.depth.cloned(),
                clear_depth: attachments.clear_depth,
                stencil: attachments.stencil.cloned(),
                clear_stencil: attachments.clear_stencil,
            },
            binding: None,
            pipeline: None,
            index_binding: None,
            textures: std::array::from_fn(|_| None),
        })
    }
    /// Opens an offscreen render recorder. Pixels change only on submission.
    ///
    /// ```compile_fail,E0499
    /// use expansex_core::{Instance, FramebufferDescriptor, FramebufferFormat};
    /// let device = Instance::new().default_adapter().create_device();
    /// let target = device.create_framebuffer(FramebufferDescriptor {
    ///     label: "target", width: 4, height: 4, format: FramebufferFormat::Rgba8Unorm,
    /// }).unwrap();
    /// let mut encoder = device.create_command_encoder();
    /// let pass = encoder.begin_render_pass(&target, None).unwrap();
    /// let nested = encoder.begin_render_pass(&target, None); // Still borrowed.
    /// pass.finish().unwrap();
    /// ```
    pub fn begin_render_pass(
        &mut self,
        target: &Framebuffer,
        clear: Option<[u8; 4]>,
    ) -> Result<RenderPass<'_>, RenderEncodeError> {
        self.begin_render_pass_with_attachments(target, clear, RenderAttachments::default())
    }
}

impl RenderPass<'_> {
    /// Binds a vertex source. Legacy draws use x/y pairs; programmable draws use
    /// the current pipeline's layout. Offset is always aligned to four bytes.
    pub fn set_vertex_buffer(
        &mut self,
        buffer: &Buffer,
        offset: u64,
    ) -> Result<(), RenderEncodeError> {
        if !buffer.belongs_to(&self.encoder.device) {
            return Err(RenderEncodeError::ForeignVertexBuffer {
                label: buffer.label().to_owned(),
            });
        }
        if !buffer.usages().contains(&BufferUsage::Vertex) {
            return Err(RenderEncodeError::MissingVertexUsage {
                label: buffer.label().to_owned(),
            });
        }
        if offset > buffer.size() {
            return Err(RenderEncodeError::VertexOffsetOutOfBounds {
                label: buffer.label().to_owned(),
                offset,
                buffer_size: buffer.size(),
            });
        }
        if !offset.is_multiple_of(4) {
            return Err(RenderEncodeError::UnalignedVertexOffset { offset });
        }
        self.binding = Some((buffer.clone(), offset));
        Ok(())
    }

    /// Records ascending ranges containing complete triangles. Byte ranges are
    /// checked now; finite vertex contents are checked when submitted.
    pub fn draw(&mut self, vertices: Range<u32>, colour: [u8; 4]) -> Result<(), RenderEncodeError> {
        self.record_draw(vertices, colour, None)
    }

    /// Bind a retained pipeline after device/target checks. Failure preserves
    /// the previous binding. Each pass starts with no pipeline bound.
    pub fn set_pipeline(&mut self, pipeline: &GraphicsPipeline) -> Result<(), RenderEncodeError> {
        if !pipeline.belongs_to(&self.encoder.device) {
            return Err(RenderEncodeError::ForeignPipeline {
                label: pipeline.label().to_owned(),
            });
        }
        if pipeline.colour_format() != self.commands.target.format() {
            return Err(RenderEncodeError::IncompatibleTargetFormat {
                pipeline_format: pipeline.colour_format(),
                target_format: self.commands.target.format(),
            });
        }
        self.pipeline = Some(pipeline.clone());
        Ok(())
    }

    /// Record a triangle list controlled by the current pipeline's shaders.
    pub fn draw_programmable(&mut self, vertices: Range<u32>) -> Result<(), RenderEncodeError> {
        let pipeline = self
            .pipeline
            .clone()
            .ok_or(RenderEncodeError::MissingPipeline)?;
        self.record_draw(vertices, [0; 4], Some(pipeline))
    }

    fn record_draw(
        &mut self,
        vertices: Range<u32>,
        colour: [u8; 4],
        pipeline: Option<GraphicsPipeline>,
    ) -> Result<(), RenderEncodeError> {
        let (buffer, offset) = self
            .binding
            .as_ref()
            .ok_or(RenderEncodeError::MissingVertexBuffer)?;
        let count = vertices.end.checked_sub(vertices.start).ok_or(
            RenderEncodeError::InvalidVertexRange {
                start: vertices.start,
                end: vertices.end,
            },
        )?;
        if !count.is_multiple_of(3) {
            return Err(RenderEncodeError::IncompleteTriangle {
                vertex_count: count,
            });
        }
        let stride = pipeline.as_ref().map_or(8, GraphicsPipeline::vertex_stride);
        if u64::from(vertices.end)
            .checked_mul(stride)
            .and_then(|bytes| offset.checked_add(bytes))
            .filter(|&end| end <= buffer.size())
            .is_none()
        {
            return Err(RenderEncodeError::VertexRangeOutOfBounds {
                label: buffer.label().to_owned(),
                first_vertex: vertices.start,
                vertex_count: count,
                offset: *offset,
                buffer_size: buffer.size(),
            });
        }
        let textures = self.draw_resources(pipeline.as_ref())?;
        self.commands
            .draws
            .try_reserve(1)
            .map_err(|source| RenderEncodeError::AllocationFailed { source })?;
        self.commands.draws.push(DrawTriangles {
            vertex_buffer: buffer.clone(),
            vertex_offset: *offset,
            vertices,
            colour,
            pipeline,
            indexed: None,
            textures,
        });
        Ok(())
    }

    /// Consumes this scope and commits one render command. A failed reservation
    /// discards this pass and leaves the encoder's earlier commands unchanged.
    ///
    /// ```compile_fail,E0382
    /// use expansex_core::{Instance, FramebufferDescriptor, FramebufferFormat};
    /// let device = Instance::new().default_adapter().create_device();
    /// let target = device.create_framebuffer(FramebufferDescriptor {
    ///     label: "target", width: 4, height: 4, format: FramebufferFormat::Rgba8Unorm,
    /// }).unwrap();
    /// let mut encoder = device.create_command_encoder();
    /// let pass = encoder.begin_render_pass(&target, None).unwrap();
    /// pass.finish().unwrap();
    /// pass.finish().unwrap(); // Already consumed.
    /// ```
    pub fn finish(self) -> Result<(), RenderEncodeError> {
        self.encoder
            .commands
            .try_reserve(1)
            .map_err(|source| RenderEncodeError::AllocationFailed { source })?;
        self.encoder
            .commands
            .push(Command::RenderPass(self.commands));
        Ok(())
    }
}

/// Expected reference render failures occur before any target pixels change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderExecutionError {
    /// Internal invariant: graphics shaders cannot contain storage loads.
    UnexpectedStorageBinding {
        binding: u32,
    },
    MissingStencilBuffer,
    StencilStoragePoisoned {
        label: String,
    },
    MissingDepthBuffer,
    DepthStoragePoisoned {
        label: String,
    },
    TextureStoragePoisoned {
        label: String,
    },
    MissingTextureBinding {
        binding: u32,
    },
    IndexStoragePoisoned {
        label: String,
    },
    InvalidVertexIndex {
        index_position: u32,
        index_value: u32,
        base_vertex: i32,
        vertex_label: String,
        available_vertices: u64,
    },
    VertexStoragePoisoned {
        label: String,
    },
    FramebufferStoragePoisoned {
        label: String,
    },
    NonFiniteVertex {
        label: String,
        vertex_index: u32,
    },
    InvalidClipW {
        vertex_index: u32,
    },
    ShaderInputCount {
        stage: ShaderStage,
        label: String,
        invocation: u64,
        expected: u32,
        actual: usize,
    },
    NonFiniteShaderInput {
        stage: ShaderStage,
        label: String,
        invocation: u64,
        location: usize,
    },
    NonFiniteShaderResult {
        stage: ShaderStage,
        label: String,
        invocation: u64,
        instruction: usize,
    },
    /// Fallible vertex/texture-snapshot/attachment reservation failed; no target writes occurred.
    AllocationFailed,
}

impl fmt::Display for RenderExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedStorageBinding { binding } => write!(f, "graphics shader unexpectedly requested compute storage slot {binding}"),
            Self::MissingStencilBuffer => f.write_str("stencil-enabled drawing requires a stencil attachment"),
            Self::StencilStoragePoisoned { label } => write!(f, "stencil buffer {label:?} storage is poisoned"),
            Self::MissingDepthBuffer => f.write_str("depth-enabled drawing requires a depth attachment"),
            Self::DepthStoragePoisoned { label } => write!(f, "depth buffer {label:?} storage is poisoned"),
            Self::TextureStoragePoisoned { label } => write!(f, "texture {label:?} storage is poisoned"),
            Self::MissingTextureBinding { binding } => write!(f, "shader sample requires texture slot {binding}"),
            Self::IndexStoragePoisoned { label } => write!(f, "index buffer {label:?} storage is poisoned"),
            Self::InvalidVertexIndex { index_position, index_value, base_vertex, vertex_label, available_vertices } => write!(f, "index element {index_position} value {index_value} plus base vertex {base_vertex} cannot address a complete u32-indexed vertex in buffer {vertex_label:?} ({available_vertices} available records)"),
            Self::VertexStoragePoisoned { label } => write!(f, "vertex buffer {label:?} storage is poisoned"),
            Self::FramebufferStoragePoisoned { label } => write!(f, "framebuffer {label:?} storage is poisoned"),
            Self::NonFiniteVertex { label, vertex_index } => write!(f, "vertex {vertex_index} in buffer {label:?} contains NaN or infinity; positions must be finite"),
            Self::InvalidClipW { vertex_index } => write!(f, "vertex {vertex_index} produced nonpositive clip w; this reference pipeline requires w > 0"),
            Self::ShaderInputCount { stage, label, invocation, expected, actual } => write!(f, "{stage:?} shader {label:?} invocation {invocation} expected {expected} inputs, received {actual}"),
            Self::NonFiniteShaderInput { stage, label, invocation, location } => write!(f, "{stage:?} shader {label:?} invocation {invocation} input {location} is non-finite"),
            Self::NonFiniteShaderResult { stage, label, invocation, instruction } => write!(f, "{stage:?} shader {label:?} invocation {invocation} instruction {instruction} produced a non-finite result"),
            Self::AllocationFailed => f.write_str("cannot reserve temporary render vertices, texture snapshots, or attachments"),
        }
    }
}
impl Error for RenderExecutionError {}
