use crate::{
    Buffer, BufferUsage, Command, CommandEncoder, Framebuffer, FramebufferFormat, GraphicsPipeline,
    ShaderStage,
};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::ops::Range;
mod index;
pub use index::{IndexFormat, IndexedTriangles};

/// Recording misuse leaves previous work and the current binding unchanged.
#[derive(Debug)]
pub enum RenderEncodeError {
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
}

impl RenderCommands {
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

/// Exclusively borrows an encoder. Finish commits; dropping discards this pass.
#[derive(Debug)]
#[must_use = "finish the render pass to record its work; dropping discards it"]
pub struct RenderPass<'a> {
    encoder: &'a mut CommandEncoder,
    commands: RenderCommands,
    binding: Option<(Buffer, u64)>,
    pipeline: Option<GraphicsPipeline>,
    index_binding: Option<(Buffer, u64, IndexFormat)>,
}

impl CommandEncoder {
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
        if !target.belongs_to(&self.device) {
            return Err(RenderEncodeError::ForeignFramebuffer {
                label: target.label().to_owned(),
            });
        }
        Ok(RenderPass {
            encoder: self,
            commands: RenderCommands {
                target: target.clone(),
                clear,
                draws: Vec::new(),
            },
            binding: None,
            pipeline: None,
            index_binding: None,
        })
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
    /// Fallible temporary vertex/image reservation failed; no target writes occurred.
    AllocationFailed,
}

impl fmt::Display for RenderExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexStoragePoisoned { label } => write!(f, "index buffer {label:?} storage is poisoned"),
            Self::InvalidVertexIndex { index_position, index_value, base_vertex, vertex_label, available_vertices } => write!(f, "index element {index_position} value {index_value} plus base vertex {base_vertex} cannot address a complete u32-indexed vertex in buffer {vertex_label:?} ({available_vertices} available records)"),
            Self::VertexStoragePoisoned { label } => write!(f, "vertex buffer {label:?} storage is poisoned"),
            Self::FramebufferStoragePoisoned { label } => write!(f, "framebuffer {label:?} storage is poisoned"),
            Self::NonFiniteVertex { label, vertex_index } => write!(f, "vertex {vertex_index} in buffer {label:?} contains NaN or infinity; positions must be finite"),
            Self::InvalidClipW { vertex_index } => write!(f, "vertex {vertex_index} produced nonpositive clip w; this reference pipeline requires w > 0"),
            Self::ShaderInputCount { stage, label, invocation, expected, actual } => write!(f, "{stage:?} shader {label:?} invocation {invocation} expected {expected} inputs, received {actual}"),
            Self::NonFiniteShaderInput { stage, label, invocation, location } => write!(f, "{stage:?} shader {label:?} invocation {invocation} input {location} is non-finite"),
            Self::NonFiniteShaderResult { stage, label, invocation, instruction } => write!(f, "{stage:?} shader {label:?} invocation {invocation} instruction {instruction} produced a non-finite result"),
            Self::AllocationFailed => f.write_str("cannot reserve temporary render vertices or image"),
        }
    }
}
impl Error for RenderExecutionError {}
