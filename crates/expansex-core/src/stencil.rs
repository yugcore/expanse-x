use crate::Device;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

const MAX_EXTENT: u32 = crate::profile::PREVIEW_LIMITS.max_image_extent;

/// Borrowed creation metadata; allocation owns its label and stencil values initially zero.
#[derive(Debug, Clone, Copy)]
pub struct StencilBufferDescriptor<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StencilBufferDescriptorError {
    ZeroExtent {
        width: u32,
        height: u32,
    },
    ExtentTooLarge {
        width: u32,
        height: u32,
        maximum: u32,
    },
}

impl StencilBufferDescriptor<'_> {
    /// Dimensions are 1..=16384 per axis; fixed u8 stencil storage.
    pub fn validate(&self) -> Result<(), StencilBufferDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(StencilBufferDescriptorError::ZeroExtent {
                width: self.width,
                height: self.height,
            });
        }
        if self.width > MAX_EXTENT || self.height > MAX_EXTENT {
            return Err(StencilBufferDescriptorError::ExtentTooLarge {
                width: self.width,
                height: self.height,
                maximum: MAX_EXTENT,
            });
        }
        Ok(())
    }
}

impl fmt::Display for StencilBufferDescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => write!(f, "stencil buffer extent {width}x{height} must be positive on both axes"),
            Self::ExtentTooLarge { width, height, maximum } => write!(f, "stencil buffer extent {width}x{height} exceeds reference maximum {maximum} on either axis"),
        }
    }
}
impl Error for StencilBufferDescriptorError {}

#[derive(Debug)]
pub enum StencilBufferCreateError {
    InvalidDescriptor(StencilBufferDescriptorError),
    AllocationFailed { source: TryReserveError },
}

impl fmt::Display for StencilBufferCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create stencil buffer: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve stencil buffer values or label: {source}")
            }
        }
    }
}
impl Error for StencilBufferCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(source) => Some(source),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StencilBufferAccessError {
    WrongPixelCount {
        expected: usize,
        actual: usize,
    },
    OutOfBounds {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    StoragePoisoned {
        label: String,
    },
}

impl fmt::Display for StencilBufferAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPixelCount { expected, actual } => write!(
                f,
                "stencil buffer readback needs {expected} stencils, received {actual}"
            ),
            Self::OutOfBounds {
                x,
                y,
                width,
                height,
            } => write!(
                f,
                "pixel ({x}, {y}) is outside stencil buffer {width}x{height}"
            ),
            Self::StoragePoisoned { label } => write!(
                f,
                "stencil buffer {label:?} storage is poisoned after an interrupted operation"
            ),
        }
    }
}
impl Error for StencilBufferAccessError {}

struct StencilBufferState {
    device: Device,
    label: String,
    width: u32,
    height: u32,
    stencils: Mutex<Vec<u8>>,
}

/// Owned reference stencils, shared by clones; row-major u8 with top-left origin.
#[derive(Clone)]
pub struct StencilBuffer {
    state: Arc<StencilBufferState>,
}

impl fmt::Debug for StencilBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StencilBuffer")
            .field("label", &self.label())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Allocates stencil values initially zero. This does not record or execute a clear.
    pub fn create_stencil_buffer(
        &self,
        descriptor: StencilBufferDescriptor<'_>,
    ) -> Result<StencilBuffer, StencilBufferCreateError> {
        descriptor
            .validate()
            .map_err(StencilBufferCreateError::InvalidDescriptor)?;
        // The validated extent caps u8 stencil storage at 256 MiB, fitting 32/64-bit
        // usize and isize contiguous-vector limits before multiplication.
        let count = descriptor.width as usize * descriptor.height as usize;
        let mut stencils = Vec::new();
        stencils
            .try_reserve_exact(count)
            .map_err(|source| StencilBufferCreateError::AllocationFailed { source })?;
        stencils.resize(count, 0);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| StencilBufferCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(StencilBuffer {
            state: Arc::new(StencilBufferState {
                device: self.clone(),
                label,
                width: descriptor.width,
                height: descriptor.height,
                stencils: Mutex::new(stencils),
            }),
        })
    }
}

impl StencilBuffer {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn width(&self) -> u32 {
        self.state.width
    }
    pub fn height(&self) -> u32 {
        self.state.height
    }
    pub fn device(&self) -> Device {
        self.state.device.clone()
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.state.device.is_same(device)
    }
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Reads the whole image into exactly width*height u8 stencil entries, atomically
    /// against submissions. Errors leave the caller's destination unchanged.
    pub fn read_stencils(&self, destination: &mut [u8]) -> Result<(), StencilBufferAccessError> {
        let expected = self.width() as usize * self.height() as usize;
        if destination.len() != expected {
            return Err(StencilBufferAccessError::WrongPixelCount {
                expected,
                actual: destination.len(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let stencils = self
            .state
            .stencils
            .lock()
            .map_err(|_| self.poisoned_error())?;
        destination.copy_from_slice(&stencils);
        Ok(())
    }

    /// Returns one stencil value; (0,0) is the top-left pixel, not a vertex coordinate.
    pub fn stencil(&self, x: u32, y: u32) -> Result<u8, StencilBufferAccessError> {
        if x >= self.width() || y >= self.height() {
            return Err(StencilBufferAccessError::OutOfBounds {
                x,
                y,
                width: self.width(),
                height: self.height(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let stencils = self
            .state
            .stencils
            .lock()
            .map_err(|_| self.poisoned_error())?;
        Ok(stencils[y as usize * self.width() as usize + x as usize])
    }

    // Reference executor only. Caller holds the device gate before storage.
    pub(crate) fn with_stencils_mut<R>(&self, f: impl FnOnce(&mut [u8]) -> R) -> Result<R, ()> {
        let mut stencils = self.state.stencils.lock().map_err(|_| ())?;
        Ok(f(&mut stencils))
    }

    fn poisoned_error(&self) -> StencilBufferAccessError {
        StencilBufferAccessError::StoragePoisoned {
            label: self.label().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn setup() -> (Device, Framebuffer, DepthBuffer, StencilBuffer) {
        let d = Instance::new().default_adapter().create_device();
        let c = d
            .create_framebuffer(FramebufferDescriptor {
                label: "c",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let z = d
            .create_depth_buffer(DepthBufferDescriptor {
                label: "z",
                width: 2,
                height: 2,
            })
            .unwrap();
        let s = d
            .create_stencil_buffer(StencilBufferDescriptor {
                label: "s",
                width: 2,
                height: 2,
            })
            .unwrap();
        (d, c, z, s)
    }

    #[test]
    fn poison_preserves_all_attached_storage_and_readback_destination() {
        let (d, c, z, s) = setup();
        assert!(std::panic::catch_unwind(|| {
            let _guard = s.state.stencils.lock().unwrap();
            panic!("poison stencil");
        })
        .is_err());
        let mut output = [9; 4];
        assert!(matches!(
            s.read_stencils(&mut output),
            Err(StencilBufferAccessError::StoragePoisoned { .. })
        ));
        assert_eq!(output, [9; 4]);
        assert!(matches!(
            s.stencil(0, 0),
            Err(StencilBufferAccessError::StoragePoisoned { .. })
        ));
        let mut e = d.create_command_encoder();
        e.begin_render_pass_with_attachments(
            &c,
            Some([7; 4]),
            RenderAttachments {
                depth: Some(&z),
                clear_depth: Some(0.5),
                stencil: Some(&s),
                clear_stencil: Some(7),
            },
        )
        .unwrap()
        .finish()
        .unwrap();
        assert_eq!(
            d.default_queue().submit(&e.finish()),
            Err(QueueSubmitError::RenderFailed {
                command_index: 0,
                source: RenderExecutionError::StencilStoragePoisoned {
                    label: "s".to_owned()
                }
            })
        );
        assert_eq!(c.pixel(0, 0).unwrap(), [0; 4]);
        assert_eq!(z.depth(0, 0).unwrap(), 1.);
        assert_eq!(
            *s.state.stencils.lock().unwrap_err().into_inner(),
            vec![0; 4]
        );
    }

    #[test]
    fn retained_commands_keep_storage_alive_and_unfinished_scope_releases_it() {
        let (d, c, _, s) = setup();
        let weak = Arc::downgrade(&s.state);
        let mut e = d.create_command_encoder();
        e.begin_render_pass_with_attachments(
            &c,
            None,
            RenderAttachments {
                stencil: Some(&s),
                clear_stencil: Some(5),
                ..RenderAttachments::default()
            },
        )
        .unwrap()
        .finish()
        .unwrap();
        let commands = e.finish();
        drop(s);
        d.default_queue().submit(&commands).unwrap();
        assert!(weak.upgrade().is_some());
        drop(commands);
        assert!(weak.upgrade().is_none());
        let (_, _, _, s) = setup();
        let weak = Arc::downgrade(&s.state);
        let mut e = s.device().create_command_encoder();
        let c = s
            .device()
            .create_framebuffer(FramebufferDescriptor {
                label: "drop",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let pass = e
            .begin_render_pass_with_attachments(
                &c,
                None,
                RenderAttachments {
                    stencil: Some(&s),
                    ..RenderAttachments::default()
                },
            )
            .unwrap();
        drop(s);
        drop(pass);
        assert!(weak.upgrade().is_none());
        assert!(e.commands().is_empty());
    }
}
