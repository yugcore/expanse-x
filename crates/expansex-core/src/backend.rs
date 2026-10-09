//! Private execution interface. Runtime policy owns validation, ordering and gate.
use crate::{
    BackendKind, Command, ComputeExecutionError, CopyBufferRole, CoreProfile, QueueSubmitError,
    RenderExecutionError, TextureTransferExecutionError,
};
use std::fmt;

/// Execute one validated operation while the device gate is held. Do not call
/// public host access or submission APIs, retain work, or choose command order.
pub(crate) trait ExecutionBackend:
    fmt::Debug + Send + Sync + std::panic::RefUnwindSafe + std::panic::UnwindSafe
{
    fn core_profile(&self) -> CoreProfile;
    fn execute(&self, command: &Command) -> Result<(), ExecutionError>;
}

/// Backend failures have no command index: the runtime assigns that context.
#[derive(Debug)]
pub(crate) enum ExecutionError {
    BufferCopyPoisoned { role: CopyBufferRole, label: String },
    TextureTransfer(TextureTransferExecutionError),
    Compute(ComputeExecutionError),
    Render(RenderExecutionError),
}
impl ExecutionError {
    pub(crate) fn at_command(self, command_index: usize) -> QueueSubmitError {
        match self {
            Self::BufferCopyPoisoned { role, label } => QueueSubmitError::StoragePoisoned {
                command_index,
                role,
                label,
            },
            Self::TextureTransfer(source) => QueueSubmitError::TextureTransferFailed {
                command_index,
                source,
            },
            Self::Compute(source) => QueueSubmitError::ComputeFailed {
                command_index,
                source,
            },
            Self::Render(source) => QueueSubmitError::RenderFailed {
                command_index,
                source,
            },
        }
    }
}

pub(crate) fn core_profile(kind: BackendKind) -> CoreProfile {
    match kind {
        BackendKind::Reference => CoreProfile::Preview1,
    }
}

pub(crate) fn create(kind: BackendKind) -> Box<dyn ExecutionBackend> {
    match kind {
        BackendKind::Reference => Box::new(crate::reference::ReferenceBackend),
    }
}
