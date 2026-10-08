use std::error::Error;
use std::fmt;

/// A role that a future buffer will be permitted to serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Vertex data consumed by a graphics pipeline.
    Vertex,
    /// Indices selecting vertices for a draw.
    Index,
    /// Shader parameters, typically read-only during a draw.
    Uniform,
    /// Shader-accessible data that may be read or written.
    Storage,
    /// Source data for a copy operation.
    TransferSource,
    /// Destination storage for a copy operation.
    TransferDestination,
}

/// A request describing a buffer; it does not allocate memory.
///
/// Multiple roles can be requested, for example vertex data plus a copy
/// destination. Backend limits and compatibility are not checked here.
#[derive(Debug, Clone, Copy)]
pub struct BufferDescriptor<'a> {
    /// Human-readable debugging name; an empty name is allowed.
    pub label: &'a str,
    /// Requested size in bytes, not number of vertices or elements.
    pub size: u64,
    /// Requested roles. Repeated roles have no additional meaning.
    pub usages: &'a [BufferUsage],
}

impl BufferDescriptor<'_> {
    /// Checks the two backend-independent rules in this initial draft.
    pub fn validate(&self) -> Result<(), BufferDescriptorError> {
        if self.size == 0 {
            return Err(BufferDescriptorError::ZeroSize);
        }
        if self.usages.is_empty() {
            return Err(BufferDescriptorError::MissingUsage);
        }
        Ok(())
    }
}

/// An invalid request discovered before contacting a graphics backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferDescriptorError {
    ZeroSize,
    MissingUsage,
}

impl fmt::Display for BufferDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSize => formatter.write_str("buffer size must be greater than zero"),
            Self::MissingUsage => formatter.write_str("buffer must have at least one usage"),
        }
    }
}

impl Error for BufferDescriptorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_vertex_buffer_that_can_receive_copies() {
        let descriptor = BufferDescriptor {
            label: "triangle positions",
            size: 36,
            usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        };
        assert_eq!(descriptor.validate(), Ok(()));
    }

    #[test]
    fn rejects_zero_bytes() {
        let descriptor = BufferDescriptor {
            label: "empty",
            size: 0,
            usages: &[BufferUsage::Vertex],
        };
        assert_eq!(descriptor.validate(), Err(BufferDescriptorError::ZeroSize));
    }

    #[test]
    fn rejects_a_buffer_without_a_role() {
        let descriptor = BufferDescriptor {
            label: "unclassified",
            size: 36,
            usages: &[],
        };
        assert_eq!(
            descriptor.validate(),
            Err(BufferDescriptorError::MissingUsage)
        );
    }
}
