//! Validated rectangular RGBA8 copies, executed only under the device gate.
use crate::{
    Buffer, BufferUsage, Command, CommandEncoder, CopyBufferRole, Resource, ResourceKind,
    TextureView, TextureViewDescriptor,
};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;

/// Byte layout of positive RGBA8 rows in a buffer. Final-row padding is optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureBufferLayout {
    pub offset: u64,
    pub bytes_per_row: u64,
}

#[derive(Debug)]
pub enum TextureTransferEncodeError {
    ForeignResource {
        role: CopyBufferRole,
        kind: ResourceKind,
        label: String,
    },
    MissingBufferUsage {
        role: CopyBufferRole,
        label: String,
        required: BufferUsage,
    },
    InvalidBufferLayout {
        layout: TextureBufferLayout,
        width: u32,
        height: u32,
        buffer_size: u64,
    },
    ExtentMismatch {
        source_width: u32,
        source_height: u32,
        destination_width: u32,
        destination_height: u32,
    },
    OverlappingCopy {
        label: String,
        source: TextureViewDescriptor,
        destination: TextureViewDescriptor,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}
impl fmt::Display for TextureTransferEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignResource { role, kind, label } => write!(f, "texture copy {role} {kind:?} {label:?} belongs to another device"),
            Self::MissingBufferUsage { role, label, required } => write!(f, "texture copy {role} buffer {label:?} needs {required:?} usage"),
            Self::InvalidBufferLayout { layout, width, height, buffer_size } => write!(f, "RGBA8 copy layout {layout:?} for {width}x{height} must be four-byte aligned, have stride >= width*4, and fit {buffer_size} bytes without overflow"),
            Self::ExtentMismatch { source_width, source_height, destination_width, destination_height } => write!(f, "texture copy extents {source_width}x{source_height} and {destination_width}x{destination_height} must match"),
            Self::OverlappingCopy { label, source, destination } => write!(f, "texture {label:?} copy rectangles {source:?} and {destination:?} overlap; use disjoint rectangles or separate images"),
            Self::AllocationFailed { source } => write!(f, "cannot reserve a texture copy command: {source}"),
        }
    }
}
impl Error for TextureTransferEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureTransferExecutionError {
    StoragePoisoned {
        role: CopyBufferRole,
        kind: ResourceKind,
        label: String,
    },
    AllocationFailed,
}
impl fmt::Display for TextureTransferExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoragePoisoned { role, kind, label } => write!(
                f,
                "texture copy {role} {kind:?} {label:?} storage is poisoned"
            ),
            Self::AllocationFailed => f.write_str("cannot reserve texture copy source snapshot"),
        }
    }
}
impl Error for TextureTransferExecutionError {}

/// An immutable buffer/image copy. Its Command variant determines direction.
#[derive(Debug)]
pub struct BufferTextureCopy {
    buffer: Buffer,
    layout: TextureBufferLayout,
    texture: TextureView,
}
impl BufferTextureCopy {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn layout(&self) -> TextureBufferLayout {
        self.layout
    }
    pub fn texture(&self) -> &TextureView {
        &self.texture
    }
}

/// Equal-extent image rectangles retaining their live allocations.
#[derive(Debug)]
pub struct TextureCopy {
    source: TextureView,
    destination: TextureView,
}
impl TextureCopy {
    pub fn source(&self) -> &TextureView {
        &self.source
    }
    pub fn destination(&self) -> &TextureView {
        &self.destination
    }
}

impl CommandEncoder {
    /// Record live RGBA8 upload rows. Padding is ignored; HostAccess is unrelated.
    pub fn copy_buffer_to_texture(
        &mut self,
        source: &Buffer,
        layout: TextureBufferLayout,
        destination: &TextureView,
    ) -> Result<(), TextureTransferEncodeError> {
        self.record_buffer_texture(source, layout, destination, true)
    }
    /// Record RGBA8 readback rows, preserving padding and other destination bytes.
    pub fn copy_texture_to_buffer(
        &mut self,
        source: &TextureView,
        destination: &Buffer,
        layout: TextureBufferLayout,
    ) -> Result<(), TextureTransferEncodeError> {
        self.record_buffer_texture(destination, layout, source, false)
    }
    fn check_transfer_resource(
        &self,
        resource: Resource,
        role: CopyBufferRole,
    ) -> Result<(), TextureTransferEncodeError> {
        if !resource.belongs_to(&self.device) {
            return Err(TextureTransferEncodeError::ForeignResource {
                role,
                kind: resource.kind(),
                label: resource.label().to_owned(),
            });
        }
        Ok(())
    }
    fn record_buffer_texture(
        &mut self,
        buffer: &Buffer,
        layout: TextureBufferLayout,
        texture: &TextureView,
        upload: bool,
    ) -> Result<(), TextureTransferEncodeError> {
        use CopyBufferRole::{Destination, Source};
        let (buffer_role, texture_role, required) = if upload {
            (Source, Destination, BufferUsage::TransferSource)
        } else {
            (Destination, Source, BufferUsage::TransferDestination)
        };
        self.check_transfer_resource(Resource::Buffer(buffer.clone()), buffer_role)?;
        self.check_transfer_resource(Resource::Texture(texture.texture().clone()), texture_role)?;
        if !buffer.usages().contains(&required) {
            return Err(TextureTransferEncodeError::MissingBufferUsage {
                role: buffer_role,
                label: buffer.label().to_owned(),
                required,
            });
        }
        let region = texture.region();
        let row_bytes = u64::from(region.width) * 4;
        let end = u64::from(region.height - 1)
            .checked_mul(layout.bytes_per_row)
            .and_then(|span| span.checked_add(layout.offset))
            .and_then(|span| span.checked_add(row_bytes));
        if !layout.offset.is_multiple_of(4)
            || !layout.bytes_per_row.is_multiple_of(4)
            || layout.bytes_per_row < row_bytes
            || end.is_none_or(|end| end > buffer.size())
        {
            return Err(TextureTransferEncodeError::InvalidBufferLayout {
                layout,
                width: region.width,
                height: region.height,
                buffer_size: buffer.size(),
            });
        }
        self.commands
            .try_reserve(1)
            .map_err(|source| TextureTransferEncodeError::AllocationFailed { source })?;
        let copy = BufferTextureCopy {
            buffer: buffer.clone(),
            layout,
            texture: texture.clone(),
        };
        self.commands.push(if upload {
            Command::CopyBufferToTexture(copy)
        } else {
            Command::CopyTextureToBuffer(copy)
        });
        Ok(())
    }
    /// Copy equal positive rectangles without filtering; reject overlapping aliases.
    pub fn copy_texture(
        &mut self,
        source: &TextureView,
        destination: &TextureView,
    ) -> Result<(), TextureTransferEncodeError> {
        self.check_transfer_resource(
            Resource::Texture(source.texture().clone()),
            CopyBufferRole::Source,
        )?;
        self.check_transfer_resource(
            Resource::Texture(destination.texture().clone()),
            CopyBufferRole::Destination,
        )?;
        let a = source.region();
        let b = destination.region();
        if a.width != b.width || a.height != b.height {
            return Err(TextureTransferEncodeError::ExtentMismatch {
                source_width: a.width,
                source_height: a.height,
                destination_width: b.width,
                destination_height: b.height,
            });
        }
        // Validated view edges cannot overflow. Both axes must overlap.
        if source.texture().is_same(destination.texture())
            && a.x < b.x + b.width
            && b.x < a.x + a.width
            && a.y < b.y + b.height
            && b.y < a.y + a.height
        {
            return Err(TextureTransferEncodeError::OverlappingCopy {
                label: source.texture().label().to_owned(),
                source: a,
                destination: b,
            });
        }
        self.commands
            .try_reserve(1)
            .map_err(|source| TextureTransferEncodeError::AllocationFailed { source })?;
        self.commands.push(Command::CopyTexture(TextureCopy {
            source: source.clone(),
            destination: destination.clone(),
        }));
        Ok(())
    }
}

fn poison(resource: Resource, role: CopyBufferRole) -> TextureTransferExecutionError {
    TextureTransferExecutionError::StoragePoisoned {
        role,
        kind: resource.kind(),
        label: resource.label().to_owned(),
    }
}

pub(crate) fn execute_buffer_texture(
    copy: &BufferTextureCopy,
    upload: bool,
) -> Result<(), TextureTransferExecutionError> {
    use CopyBufferRole::{Destination, Source};
    let image = copy.texture.texture();
    let region = copy.texture.region();
    let (buffer_role, image_role) = if upload {
        (Source, Destination)
    } else {
        (Destination, Source)
    };
    // No shared storage can alias between a buffer and an image. Acquire both
    // guards before any destination write; conversion/commit then cannot fail.
    let mut bytes = copy
        .buffer
        .lock_bytes()
        .map_err(|_| poison(Resource::Buffer(copy.buffer.clone()), buffer_role))?;
    image
        .with_pixels_mut(|pixels| {
            for row in 0..region.height as usize {
                let pixel_start =
                    (region.y as usize + row) * image.width() as usize + region.x as usize;
                let byte_start =
                    (copy.layout.offset + row as u64 * copy.layout.bytes_per_row) as usize;
                let row_pixels = &mut pixels[pixel_start..pixel_start + region.width as usize];
                let row_bytes = &mut bytes[byte_start..byte_start + region.width as usize * 4];
                for (pixel, rgba) in row_pixels.iter_mut().zip(row_bytes.as_chunks_mut::<4>().0) {
                    if upload {
                        *pixel = *rgba;
                    } else {
                        *rgba = *pixel;
                    }
                }
            }
        })
        .map_err(|_| poison(Resource::Texture(image.clone()), image_role))
}

pub(crate) fn execute_texture(copy: &TextureCopy) -> Result<(), TextureTransferExecutionError> {
    let source = copy.source.texture();
    let destination = copy.destination.texture();
    let a = copy.source.region();
    let b = copy.destination.region();
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(a.width as usize * a.height as usize)
        .map_err(|_| TextureTransferExecutionError::AllocationFailed)?;
    source
        .with_pixels(|pixels| {
            for y in a.y..a.y + a.height {
                let start = y as usize * source.width() as usize + a.x as usize;
                snapshot.extend_from_slice(&pixels[start..start + a.width as usize]);
            }
        })
        .map_err(|_| poison(Resource::Texture(source.clone()), CopyBufferRole::Source))?;
    destination
        .with_pixels_mut(|pixels| {
            for row in 0..b.height as usize {
                let start = (b.y as usize + row) * destination.width() as usize + b.x as usize;
                let source_start = row * b.width as usize;
                pixels[start..start + b.width as usize]
                    .copy_from_slice(&snapshot[source_start..source_start + b.width as usize]);
            }
        })
        .map_err(|_| {
            poison(
                Resource::Texture(destination.clone()),
                CopyBufferRole::Destination,
            )
        })
}
