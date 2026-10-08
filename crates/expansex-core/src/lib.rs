//! ExpanseX API design prototype.
//!
//! Describes buffers and demonstrates a recording-to-finished transition.
//! This crate does not create GPU resources, submit work, or render images.

mod buffer;
mod command;

pub use buffer::{BufferDescriptor, BufferDescriptorError, BufferUsage};
pub use command::{CommandBuffer, CommandRecorder};
