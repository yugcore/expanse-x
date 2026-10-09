use crate::Device;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

const MAX_EXTENT: u32 = crate::profile::PREVIEW_LIMITS.max_image_extent;

/// Borrowed creation metadata; allocation owns its label and depth values initially 1.
#[derive(Debug, Clone, Copy)]
pub struct DepthBufferDescriptor<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn setup() -> (Device, Framebuffer, DepthBuffer) {
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
        (d, c, z)
    }
    #[test]
    fn depth_poison_preserves_both_attachments_and_readback_destination() {
        let (d, c, z) = setup();
        assert!(std::panic::catch_unwind(|| {
            let _guard = z.state.depths.lock().unwrap();
            panic!("poison depth");
        })
        .is_err());
        let mut values = [9.; 4];
        assert!(matches!(
            z.read_depths(&mut values),
            Err(DepthBufferAccessError::StoragePoisoned { .. })
        ));
        assert_eq!(values, [9.; 4]);
        assert!(matches!(
            z.depth(0, 0),
            Err(DepthBufferAccessError::StoragePoisoned { .. })
        ));
        let mut e = d.create_command_encoder();
        e.begin_render_pass_with_depth(&c, Some([7; 4]), &z, Some(0.5))
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(
            d.default_queue().submit(&e.finish()),
            Err(QueueSubmitError::RenderFailed {
                command_index: 0,
                source: RenderExecutionError::DepthStoragePoisoned {
                    label: "z".to_owned()
                }
            })
        );
        assert_eq!(c.pixel(0, 0).unwrap(), [0; 4]);
        assert_eq!(
            *z.state.depths.lock().unwrap_err().into_inner(),
            vec![1.; 4]
        );
    }
    #[test]
    fn depth_lives_through_commands_and_dropped_unfinished_passes_release_it() {
        let (d, c, z) = setup();
        let weak = Arc::downgrade(&z.state);
        let mut e = d.create_command_encoder();
        e.begin_render_pass_with_depth(&c, None, &z, Some(0.5))
            .unwrap()
            .finish()
            .unwrap();
        let commands = e.finish();
        drop(z);
        d.default_queue().submit(&commands).unwrap();
        assert!(weak.upgrade().is_some());
        drop(commands);
        assert!(weak.upgrade().is_none());
        let z = d
            .create_depth_buffer(DepthBufferDescriptor {
                label: "drop",
                width: 2,
                height: 2,
            })
            .unwrap();
        let weak = Arc::downgrade(&z.state);
        let mut e = d.create_command_encoder();
        let pass = e.begin_render_pass_with_depth(&c, None, &z, None).unwrap();
        drop(z);
        drop(pass);
        assert!(weak.upgrade().is_none());
        assert!(e.commands().is_empty());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthBufferDescriptorError {
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

impl DepthBufferDescriptor<'_> {
    /// Dimensions are 1..=16384 per axis; fixed f32 depth storage.
    pub fn validate(&self) -> Result<(), DepthBufferDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(DepthBufferDescriptorError::ZeroExtent {
                width: self.width,
                height: self.height,
            });
        }
        if self.width > MAX_EXTENT || self.height > MAX_EXTENT {
            return Err(DepthBufferDescriptorError::ExtentTooLarge {
                width: self.width,
                height: self.height,
                maximum: MAX_EXTENT,
            });
        }
        Ok(())
    }
}

impl fmt::Display for DepthBufferDescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => write!(f, "depth buffer extent {width}x{height} must be positive on both axes"),
            Self::ExtentTooLarge { width, height, maximum } => write!(f, "depth buffer extent {width}x{height} exceeds reference maximum {maximum} on either axis"),
        }
    }
}
impl Error for DepthBufferDescriptorError {}

#[derive(Debug)]
pub enum DepthBufferCreateError {
    InvalidDescriptor(DepthBufferDescriptorError),
    AllocationFailed { source: TryReserveError },
}

impl fmt::Display for DepthBufferCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create depth buffer: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve depth buffer values or label: {source}")
            }
        }
    }
}
impl Error for DepthBufferCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(source) => Some(source),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepthBufferAccessError {
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

impl fmt::Display for DepthBufferAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPixelCount { expected, actual } => write!(
                f,
                "depth buffer readback needs {expected} depths, received {actual}"
            ),
            Self::OutOfBounds {
                x,
                y,
                width,
                height,
            } => write!(
                f,
                "pixel ({x}, {y}) is outside depth buffer {width}x{height}"
            ),
            Self::StoragePoisoned { label } => write!(
                f,
                "depth buffer {label:?} storage is poisoned after an interrupted operation"
            ),
        }
    }
}
impl Error for DepthBufferAccessError {}

struct DepthBufferState {
    device: Device,
    label: String,
    width: u32,
    height: u32,
    depths: Mutex<Vec<f32>>,
}

/// Owned reference depths, shared by clones; row-major f32 with top-left origin.
#[derive(Clone)]
pub struct DepthBuffer {
    state: Arc<DepthBufferState>,
}

impl fmt::Debug for DepthBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DepthBuffer")
            .field("label", &self.label())
            .field("width", &self.width())
            .field("height", &self.height())
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Allocates depth values initially 1. This does not record or execute a clear.
    pub fn create_depth_buffer(
        &self,
        descriptor: DepthBufferDescriptor<'_>,
    ) -> Result<DepthBuffer, DepthBufferCreateError> {
        descriptor
            .validate()
            .map_err(DepthBufferCreateError::InvalidDescriptor)?;
        // The validated extent caps f32 depth storage at 1 GiB, fitting 32/64-bit
        // usize and isize contiguous-vector limits before multiplication.
        let count = descriptor.width as usize * descriptor.height as usize;
        let mut depths = Vec::new();
        depths
            .try_reserve_exact(count)
            .map_err(|source| DepthBufferCreateError::AllocationFailed { source })?;
        depths.resize(count, 1.0);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| DepthBufferCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(DepthBuffer {
            state: Arc::new(DepthBufferState {
                device: self.clone(),
                label,
                width: descriptor.width,
                height: descriptor.height,
                depths: Mutex::new(depths),
            }),
        })
    }
}

impl DepthBuffer {
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

    /// Reads the whole image into exactly width*height f32 depth entries, atomically
    /// against submissions. Errors leave the caller's destination unchanged.
    pub fn read_depths(&self, destination: &mut [f32]) -> Result<(), DepthBufferAccessError> {
        let expected = self.width() as usize * self.height() as usize;
        if destination.len() != expected {
            return Err(DepthBufferAccessError::WrongPixelCount {
                expected,
                actual: destination.len(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let depths = self
            .state
            .depths
            .lock()
            .map_err(|_| self.poisoned_error())?;
        destination.copy_from_slice(&depths);
        Ok(())
    }

    /// Returns one depth value; (0,0) is the top-left pixel, not a vertex coordinate.
    pub fn depth(&self, x: u32, y: u32) -> Result<f32, DepthBufferAccessError> {
        if x >= self.width() || y >= self.height() {
            return Err(DepthBufferAccessError::OutOfBounds {
                x,
                y,
                width: self.width(),
                height: self.height(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let depths = self
            .state
            .depths
            .lock()
            .map_err(|_| self.poisoned_error())?;
        Ok(depths[y as usize * self.width() as usize + x as usize])
    }

    // Reference executor only. Caller holds the device gate before storage.
    pub(crate) fn with_depths_mut<R>(&self, f: impl FnOnce(&mut [f32]) -> R) -> Result<R, ()> {
        let mut depths = self.state.depths.lock().map_err(|_| ())?;
        Ok(f(&mut depths))
    }

    fn poisoned_error(&self) -> DepthBufferAccessError {
        DepthBufferAccessError::StoragePoisoned {
            label: self.label().to_owned(),
        }
    }
}
