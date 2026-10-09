use super::{DrawTriangles, RenderEncodeError, RenderPass};
use crate::{Buffer, BufferUsage};
use std::ops::Range;

/// Unsigned little-endian index elements. There is no primitive restart value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}
impl IndexFormat {
    /// Element size and required binding alignment, in bytes.
    pub fn size(&self) -> u64 {
        match self {
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }
}

/// Retained live index source for one programmable triangle-list draw.
#[derive(Debug)]
pub struct IndexedTriangles {
    buffer: Buffer,
    offset: u64,
    format: IndexFormat,
    indices: Range<u32>,
    base_vertex: i32,
}
impl IndexedTriangles {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn format(&self) -> IndexFormat {
        self.format
    }
    pub fn indices(&self) -> Range<u32> {
        self.indices.clone()
    }
    pub fn base_vertex(&self) -> i32 {
        self.base_vertex
    }
}

impl RenderPass<'_> {
    /// Bind a same-device Index buffer after offset/alignment checks. Failure
    /// preserves the prior binding. Contents are sampled when submitted.
    pub fn set_index_buffer(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        format: IndexFormat,
    ) -> Result<(), RenderEncodeError> {
        if !buffer.belongs_to(&self.encoder.device) {
            return Err(RenderEncodeError::ForeignIndexBuffer {
                label: buffer.label().to_owned(),
            });
        }
        if !buffer.usages().contains(&BufferUsage::Index) {
            return Err(RenderEncodeError::MissingIndexUsage {
                label: buffer.label().to_owned(),
            });
        }
        if offset > buffer.size() {
            return Err(RenderEncodeError::IndexOffsetOutOfBounds {
                label: buffer.label().to_owned(),
                offset,
                buffer_size: buffer.size(),
            });
        }
        if !offset.is_multiple_of(format.size()) {
            return Err(RenderEncodeError::UnalignedIndexOffset {
                offset,
                alignment: format.size(),
            });
        }
        self.index_binding = Some((buffer.clone(), offset, format));
        Ok(())
    }

    /// Record indexed programmable triangles. Index ranges are checked now;
    /// live references plus signed base vertex are validated when submitted.
    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        base_vertex: i32,
    ) -> Result<(), RenderEncodeError> {
        let pipeline = self
            .pipeline
            .as_ref()
            .ok_or(RenderEncodeError::MissingPipeline)?;
        let (vertex_buffer, vertex_offset) = self
            .binding
            .as_ref()
            .ok_or(RenderEncodeError::MissingVertexBuffer)?;
        let (buffer, offset, format) = self
            .index_binding
            .as_ref()
            .ok_or(RenderEncodeError::MissingIndexBuffer)?;
        let count =
            indices
                .end
                .checked_sub(indices.start)
                .ok_or(RenderEncodeError::InvalidIndexRange {
                    start: indices.start,
                    end: indices.end,
                })?;
        if !count.is_multiple_of(3) {
            return Err(RenderEncodeError::IncompleteTriangle {
                vertex_count: count,
            });
        }
        if u64::from(indices.end)
            .checked_mul(format.size())
            .and_then(|bytes| offset.checked_add(bytes))
            .filter(|&end| end <= buffer.size())
            .is_none()
        {
            return Err(RenderEncodeError::IndexRangeOutOfBounds {
                label: buffer.label().to_owned(),
                first_index: indices.start,
                index_count: count,
                offset: *offset,
                buffer_size: buffer.size(),
            });
        }
        let textures = self.draw_resources(Some(pipeline))?;
        self.commands
            .draws
            .try_reserve(1)
            .map_err(|source| RenderEncodeError::AllocationFailed { source })?;
        self.commands.draws.push(DrawTriangles {
            vertex_buffer: vertex_buffer.clone(),
            vertex_offset: *vertex_offset,
            vertices: 0..0,
            colour: [0; 4],
            pipeline: Some(pipeline.clone()),
            textures,
            indexed: Some(IndexedTriangles {
                buffer: buffer.clone(),
                offset: *offset,
                format: *format,
                indices,
                base_vertex,
            }),
        });
        Ok(())
    }
}
