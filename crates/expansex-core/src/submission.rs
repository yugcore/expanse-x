use crate::{CommandBuffer, CopyBufferRole, Queue, RenderExecutionError};
use std::error::Error;
use std::fmt;

/// A rejected submission or an interrupted sequence of reference operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueSubmitError {
    /// Failed texture transfer preserves destination and skips later commands.
    TextureTransferFailed {
        command_index: usize,
        source: crate::TextureTransferExecutionError,
    },
    /// No commands executed: the queue and finished work have different devices.
    ForeignCommandBuffer,
    /// Dependency preflight failed; no command executed.
    DependencyFailed {
        source: crate::DependencyValidationError,
    },
    /// Failed dispatch changed no storage output; earlier commands completed.
    ComputeFailed {
        command_index: usize,
        source: crate::ComputeExecutionError,
    },
    /// Earlier commands completed, but this command and later ones did not.
    /// Its destination was not changed. The command index is zero-based.
    StoragePoisoned {
        command_index: usize,
        role: CopyBufferRole,
        label: String,
    },
    /// This render command changed no target pixels; earlier commands completed.
    RenderFailed {
        command_index: usize,
        source: RenderExecutionError,
    },
}

impl fmt::Display for QueueSubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextureTransferFailed { command_index, source } => write!(f, "submission stopped at texture transfer {command_index}: {source}; destination unchanged, earlier commands completed"),
            Self::DependencyFailed { source } => write!(f, "submission dependency validation failed: {source}; no commands executed"),
            Self::ForeignCommandBuffer => f.write_str(
                "cannot submit commands from another device; use their owning device's queue",
            ),
            Self::StoragePoisoned { command_index, role, label } => write!(
                f,
                "submission stopped at command {command_index}: {role} buffer {label:?} storage is poisoned; earlier commands completed, this copy and later commands did not"
            ),
            Self::ComputeFailed { command_index, source } => write!(f, "submission stopped at compute command {command_index}: {source}; outputs unchanged, earlier commands completed"),
            Self::RenderFailed { command_index, source } => write!(f, "submission stopped at render command {command_index}: {source}; this target is unchanged, earlier commands completed"),
        }
    }
}

impl Error for QueueSubmitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::TextureTransferFailed { source, .. } => Some(source),
            Self::DependencyFailed { source } => Some(source),
            Self::RenderFailed { source, .. } => Some(source),
            Self::ComputeFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Queue {
    /// Executes copies, compute dispatches, and render scopes with CPU completion.
    ///
    /// Borrows reusable commands and samples live source bytes at each copy.
    /// Queue clones and CPU accesses on this device serialize around the whole
    /// submission. Concurrent call order is unspecified; separate devices are
    /// independent. Successful return means all submitted operations have finished.
    ///
    /// Foreign work or failed explicit dependency preflight changes nothing.
    /// On storage poison, earlier commands remain
    /// completed; the failed copy changes nothing and later commands are skipped.
    /// HostAccess governs public CPU access, not device-side transfer execution.
    /// Render failures change no pixels in their scope, preserve earlier completed
    /// commands, and skip later work. Compute failures preserve all dispatch
    /// outputs, keep earlier commands, and skip later ones. Readbacks use the gate.
    pub fn submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError> {
        let device = self.device();
        if !commands.belongs_to(&device) {
            return Err(QueueSubmitError::ForeignCommandBuffer);
        }
        commands
            .validate_dependencies()
            .map_err(|source| QueueSubmitError::DependencyFailed { source })?;
        let _execution = device.lock_execution();
        for (command_index, command) in commands.commands().iter().enumerate() {
            crate::reference::execute(command_index, command)?;
        }
        Ok(())
    }
}
