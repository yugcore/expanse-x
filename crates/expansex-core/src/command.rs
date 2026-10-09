use crate::{Buffer, BufferUsage, Device};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;

mod markers;

pub use markers::{CommandRecorder, DebugMarkerBuffer};

/// Identifies the source or destination whose copy requirement failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyBufferRole {
    Source,
    Destination,
}

impl fmt::Display for CopyBufferRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Source => "source",
            Self::Destination => "destination",
        })
    }
}

/// A rejected recording request. No command is appended on failure.
#[derive(Debug)]
pub enum CommandEncodeError {
    ForeignBuffer {
        role: CopyBufferRole,
        label: String,
    },
    MissingUsage {
        role: CopyBufferRole,
        label: String,
        required: BufferUsage,
    },
    OutOfBounds {
        role: CopyBufferRole,
        label: String,
        offset: u64,
        size: u64,
        buffer_size: u64,
    },
    OverlappingCopy {
        label: String,
        source_offset: u64,
        destination_offset: u64,
        size: u64,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}

impl fmt::Display for CommandEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignBuffer { role, label } => write!(
                f,
                "cannot record copy: {role} buffer {label:?} belongs to another device; use the encoder's device"
            ),
            Self::MissingUsage { role, label, required } => write!(
                f,
                "cannot record copy: {role} buffer {label:?} needs {required:?} usage; request it when creating the buffer"
            ),
            Self::OutOfBounds { role, label, offset, size, buffer_size } => write!(
                f,
                "cannot record copy: {size} bytes at offset {offset} do not fit in {role} buffer {label:?} ({buffer_size} bytes)"
            ),
            Self::OverlappingCopy { label, source_offset, destination_offset, size } => write!(
                f,
                "cannot record copy within buffer {label:?}: {size}-byte ranges at {source_offset} and {destination_offset} overlap; use disjoint ranges"
            ),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve space for a recorded command: {source}")
            }
        }
    }
}

impl Error for CommandEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

/// EX's recorded operations. These descriptions do not execute during recording.
#[derive(Debug)]
#[non_exhaustive]
pub enum Command {
    CopyBuffer(BufferCopy),
    CopyBufferToTexture(crate::BufferTextureCopy),
    CopyTextureToBuffer(crate::BufferTextureCopy),
    CopyTexture(crate::TextureCopy),
    ResourceTransitions(crate::ResourceTransitions),
    Dispatch(crate::ComputeDispatch),
    RenderPass(crate::RenderCommands),
}

/// A validated copy description retaining both allocations, not their old bytes.
#[derive(Debug)]
pub struct BufferCopy {
    source: Buffer,
    source_offset: u64,
    destination: Buffer,
    destination_offset: u64,
    size: u64,
}

impl BufferCopy {
    pub fn source(&self) -> &Buffer {
        &self.source
    }
    pub fn source_offset(&self) -> u64 {
        self.source_offset
    }
    pub fn destination(&self) -> &Buffer {
        &self.destination
    }
    pub fn destination_offset(&self) -> u64 {
        self.destination_offset
    }
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Records copies, compute dispatches, and render scopes, retaining resources.
///
/// There is no public constructor independent of a device. Finishing consumes
/// the encoder without executing its work; use Queue::submit afterward.
#[derive(Debug)]
pub struct CommandEncoder {
    pub(crate) device: Device,
    pub(crate) commands: Vec<Command>,
    synchronization: crate::SynchronizationMode,
}

impl Device {
    /// Creates an empty encoder in this logical device's ownership domain.
    pub fn create_command_encoder(&self) -> CommandEncoder {
        self.create_command_encoder_with_synchronization(crate::SynchronizationMode::Ordered)
    }
}

impl Device {
    /// Select ordered execution or checked explicit resource dependencies.
    pub fn create_command_encoder_with_synchronization(
        &self,
        synchronization: crate::SynchronizationMode,
    ) -> CommandEncoder {
        CommandEncoder {
            device: self.clone(),
            commands: Vec::new(),
            synchronization,
        }
    }
}

impl CommandEncoder {
    /// Immutable synchronization contract for this recorded sequence.
    pub fn synchronization_mode(&self) -> crate::SynchronizationMode {
        self.synchronization
    }

    /// Retains the original device, rather than creating another logical device.
    pub fn device(&self) -> Device {
        self.device.clone()
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.device.is_same(device)
    }

    /// Borrows the operations recorded so far without exposing mutable commands.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Appends one byte-granular copy after validating the entire request.
    ///
    /// Both buffers must belong to this device and permit their transfer roles.
    /// Host access is unrelated. Empty ranges are allowed at the buffer end;
    /// nonempty ranges sharing an allocation must be disjoint. On error the
    /// command sequence and buffer contents remain unchanged.
    ///
    /// Only handles and offsets are recorded: no bytes are read, snapshotted,
    /// or written. Queue submission reads live source contents at execution.
    pub fn copy_buffer(
        &mut self,
        source: &Buffer,
        source_offset: u64,
        destination: &Buffer,
        destination_offset: u64,
        size: u64,
    ) -> Result<(), CommandEncodeError> {
        for (buffer, role) in [
            (source, CopyBufferRole::Source),
            (destination, CopyBufferRole::Destination),
        ] {
            if !buffer.belongs_to(&self.device) {
                return Err(CommandEncodeError::ForeignBuffer {
                    role,
                    label: buffer.label().to_owned(),
                });
            }
        }
        for (buffer, role, required) in [
            (source, CopyBufferRole::Source, BufferUsage::TransferSource),
            (
                destination,
                CopyBufferRole::Destination,
                BufferUsage::TransferDestination,
            ),
        ] {
            if !buffer.usages().contains(&required) {
                return Err(CommandEncodeError::MissingUsage {
                    role,
                    label: buffer.label().to_owned(),
                    required,
                });
            }
        }
        let source_end = checked_end(source, source_offset, size, CopyBufferRole::Source)?;
        let destination_end = checked_end(
            destination,
            destination_offset,
            size,
            CopyBufferRole::Destination,
        )?;
        if size != 0
            && source.is_same(destination)
            && source_offset < destination_end
            && destination_offset < source_end
        {
            return Err(CommandEncodeError::OverlappingCopy {
                label: source.label().to_owned(),
                source_offset,
                destination_offset,
                size,
            });
        }
        self.commands
            .try_reserve(1)
            .map_err(|source| CommandEncodeError::AllocationFailed { source })?;
        self.commands.push(Command::CopyBuffer(BufferCopy {
            source: source.clone(),
            source_offset,
            destination: destination.clone(),
            destination_offset,
            size,
        }));
        Ok(())
    }

    /// Consumes this encoder and retains its commands/resources without executing.
    ///
    /// ```compile_fail,E0382
    /// use expansex_core::Instance;
    /// let device = Instance::new().default_adapter().create_device();
    /// let encoder = device.create_command_encoder();
    /// let commands = encoder.finish();
    /// let second = encoder.finish(); // The encoder was moved.
    /// ```
    pub fn finish(self) -> CommandBuffer {
        CommandBuffer {
            device: self.device,
            commands: self.commands,
            synchronization: self.synchronization,
        }
    }
}

fn checked_end(
    buffer: &Buffer,
    offset: u64,
    size: u64,
    role: CopyBufferRole,
) -> Result<u64, CommandEncodeError> {
    offset
        .checked_add(size)
        .filter(|&end| end <= buffer.size())
        .ok_or_else(|| CommandEncodeError::OutOfBounds {
            role,
            label: buffer.label().to_owned(),
            offset,
            size,
            buffer_size: buffer.size(),
        })
}

/// Immutable recorded work for one device, reusable with synchronous submission.
///
/// Retains resources until dropped. Immutability applies to the command sequence,
/// not the live buffer contents. This type cannot be constructed from debug labels.
#[derive(Debug)]
pub struct CommandBuffer {
    device: Device,
    commands: Vec<Command>,
    synchronization: crate::SynchronizationMode,
}

impl CommandBuffer {
    /// The recorded synchronization contract; unchanged by reuse.
    pub fn synchronization_mode(&self) -> crate::SynchronizationMode {
        self.synchronization
    }

    pub fn device(&self) -> Device {
        self.device.clone()
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.device.is_same(device)
    }
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}
