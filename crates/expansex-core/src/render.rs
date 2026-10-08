use crate::{Buffer, BufferUsage, Command, CommandEncoder, Framebuffer};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::ops::Range;

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
            Self::InvalidVertexRange { start, end } => write!(f, "vertex range {start}..{end} must be ascending"),
            Self::IncompleteTriangle { vertex_count } => write!(f, "triangle-list vertex count {vertex_count} must be divisible by three"),
            Self::VertexRangeOutOfBounds { label, first_vertex, vertex_count, offset, buffer_size } => write!(f, "{vertex_count} vertices from {first_vertex} at binding offset {offset} do not fit in buffer {label:?} ({buffer_size} bytes; 8-byte stride)"),
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

/// One fixed-layout triangle-list draw, retaining live vertex bytes.
#[derive(Debug)]
pub struct DrawTriangles {
    vertex_buffer: Buffer,
    vertex_offset: u64,
    vertices: Range<u32>,
    colour: [u8; 4],
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
        })
    }
}

impl RenderPass<'_> {
    /// Binds little-endian `[x:f32,y:f32]` vertices with an 8-byte stride.
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
        if offset
            .checked_add(u64::from(vertices.end) * 8)
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
    /// Fallible temporary vertex reservation failed; no target writes occurred.
    AllocationFailed,
}

impl fmt::Display for RenderExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexStoragePoisoned { label } => write!(f, "vertex buffer {label:?} storage is poisoned"),
            Self::FramebufferStoragePoisoned { label } => write!(f, "framebuffer {label:?} storage is poisoned"),
            Self::NonFiniteVertex { label, vertex_index } => write!(f, "vertex {vertex_index} in buffer {label:?} contains NaN or infinity; positions must be finite"),
            Self::AllocationFailed => f.write_str("cannot reserve temporary decoded render vertices"),
        }
    }
}
impl Error for RenderExecutionError {}
