use crate::{Command, CommandBuffer, CopyBufferRole, Queue};
use std::error::Error;
use std::fmt;

/// A rejected submission or an interrupted sequence of reference copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueSubmitError {
    /// No commands executed: the queue and finished work have different devices.
    ForeignCommandBuffer,
    /// Earlier commands completed, but this command and later ones did not.
    /// Its destination was not changed. The command index is zero-based.
    StoragePoisoned {
        command_index: usize,
        role: CopyBufferRole,
        label: String,
    },
}

impl fmt::Display for QueueSubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignCommandBuffer => f.write_str(
                "cannot submit commands from another device; use their owning device's queue",
            ),
            Self::StoragePoisoned { command_index, role, label } => write!(
                f,
                "submission stopped at command {command_index}: {role} buffer {label:?} storage is poisoned; earlier commands completed, this copy and later commands did not"
            ),
        }
    }
}

impl Error for QueueSubmitError {}

impl Queue {
    /// Executes finished copies in order on the CPU and waits for completion.
    ///
    /// Borrows reusable commands and samples live source bytes at each copy.
    /// Queue clones and CPU accesses on this device serialize around the whole
    /// submission. Concurrent call order is unspecified; separate devices are
    /// independent. Successful return means all submitted copies have finished.
    ///
    /// Foreign work changes nothing. On storage poison, earlier commands remain
    /// completed; the failed copy changes nothing and later commands are skipped.
    /// HostAccess governs public CPU access, not device-side transfer execution.
    pub fn submit(&self, commands: &CommandBuffer) -> Result<(), QueueSubmitError> {
        let device = self.device();
        if !commands.belongs_to(&device) {
            return Err(QueueSubmitError::ForeignCommandBuffer);
        }
        let _execution = device.lock_execution();
        for (command_index, command) in commands.commands().iter().enumerate() {
            match command {
                Command::CopyBuffer(copy) => copy
                    .source()
                    .copy_to(
                        copy.source_offset(),
                        copy.destination(),
                        copy.destination_offset(),
                        copy.size(),
                    )
                    .map_err(|role| QueueSubmitError::StoragePoisoned {
                        command_index,
                        role,
                        label: match role {
                            CopyBufferRole::Source => copy.source().label(),
                            CopyBufferRole::Destination => copy.destination().label(),
                        }
                        .to_owned(),
                    })?,
            }
        }
        Ok(())
    }
}
