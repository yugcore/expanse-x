use crate::Device;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};

const MAX_EXTENT: u32 = 16384;

/// Stored pixel representation. Only linear RGBA bytes are supported currently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FramebufferFormat {
    Rgba8Unorm,
}

/// Borrowed creation metadata; allocation owns its label and zeroed pixels.
#[derive(Debug, Clone, Copy)]
pub struct FramebufferDescriptor<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
    pub format: FramebufferFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferDescriptorError {
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

impl FramebufferDescriptor<'_> {
    /// Dimensions are 1..=16384 per axis; the sole format is RGBA8 unorm.
    pub fn validate(&self) -> Result<(), FramebufferDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(FramebufferDescriptorError::ZeroExtent {
                width: self.width,
                height: self.height,
            });
        }
        if self.width > MAX_EXTENT || self.height > MAX_EXTENT {
            return Err(FramebufferDescriptorError::ExtentTooLarge {
                width: self.width,
                height: self.height,
                maximum: MAX_EXTENT,
            });
        }
        match self.format {
            FramebufferFormat::Rgba8Unorm => Ok(()),
        }
    }
}

impl fmt::Display for FramebufferDescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => write!(f, "framebuffer extent {width}x{height} must be positive on both axes"),
            Self::ExtentTooLarge { width, height, maximum } => write!(f, "framebuffer extent {width}x{height} exceeds reference maximum {maximum} on either axis"),
        }
    }
}
impl Error for FramebufferDescriptorError {}

#[derive(Debug)]
pub enum FramebufferCreateError {
    InvalidDescriptor(FramebufferDescriptorError),
    AllocationFailed { source: TryReserveError },
}

impl fmt::Display for FramebufferCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create framebuffer: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve framebuffer pixels or label: {source}")
            }
        }
    }
}
impl Error for FramebufferCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(source) => Some(source),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramebufferAccessError {
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

impl fmt::Display for FramebufferAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPixelCount { expected, actual } => write!(
                f,
                "framebuffer readback needs {expected} pixels, received {actual}"
            ),
            Self::OutOfBounds {
                x,
                y,
                width,
                height,
            } => write!(
                f,
                "pixel ({x}, {y}) is outside framebuffer {width}x{height}"
            ),
            Self::StoragePoisoned { label } => write!(
                f,
                "framebuffer {label:?} storage is poisoned after an interrupted operation"
            ),
        }
    }
}
impl Error for FramebufferAccessError {}

struct FramebufferState {
    device: Device,
    label: String,
    width: u32,
    height: u32,
    format: FramebufferFormat,
    pixels: Mutex<Vec<[u8; 4]>>,
}

/// Owned reference pixels, shared by clones; row-major RGBA with top-left origin.
#[derive(Clone)]
pub struct Framebuffer {
    state: Arc<FramebufferState>,
}

impl fmt::Debug for Framebuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Framebuffer")
            .field("label", &self.label())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("format", &self.format())
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Allocates zero-initialized pixels. This does not record or execute a clear.
    pub fn create_framebuffer(
        &self,
        descriptor: FramebufferDescriptor<'_>,
    ) -> Result<Framebuffer, FramebufferCreateError> {
        descriptor
            .validate()
            .map_err(FramebufferCreateError::InvalidDescriptor)?;
        // The validated extent caps RGBA storage at 1 GiB, fitting 32/64-bit
        // usize and isize contiguous-vector limits before multiplication.
        let count = descriptor.width as usize * descriptor.height as usize;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|source| FramebufferCreateError::AllocationFailed { source })?;
        pixels.resize(count, [0; 4]);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| FramebufferCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(Framebuffer {
            state: Arc::new(FramebufferState {
                device: self.clone(),
                label,
                width: descriptor.width,
                height: descriptor.height,
                format: descriptor.format,
                pixels: Mutex::new(pixels),
            }),
        })
    }
}

impl Framebuffer {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn width(&self) -> u32 {
        self.state.width
    }
    pub fn height(&self) -> u32 {
        self.state.height
    }
    pub fn format(&self) -> FramebufferFormat {
        self.state.format
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

    /// Reads the whole image into exactly width*height RGBA entries, atomically
    /// against submissions. Errors leave the caller's destination unchanged.
    pub fn read_pixels(&self, destination: &mut [[u8; 4]]) -> Result<(), FramebufferAccessError> {
        let expected = self.width() as usize * self.height() as usize;
        if destination.len() != expected {
            return Err(FramebufferAccessError::WrongPixelCount {
                expected,
                actual: destination.len(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let pixels = self
            .state
            .pixels
            .lock()
            .map_err(|_| self.poisoned_error())?;
        destination.copy_from_slice(&pixels);
        Ok(())
    }

    /// Returns one pixel; (0,0) is the top-left pixel, not a vertex coordinate.
    pub fn pixel(&self, x: u32, y: u32) -> Result<[u8; 4], FramebufferAccessError> {
        if x >= self.width() || y >= self.height() {
            return Err(FramebufferAccessError::OutOfBounds {
                x,
                y,
                width: self.width(),
                height: self.height(),
            });
        }
        let _execution = self.state.device.lock_execution();
        let pixels = self
            .state
            .pixels
            .lock()
            .map_err(|_| self.poisoned_error())?;
        Ok(pixels[y as usize * self.width() as usize + x as usize])
    }

    // Reference executor only. Caller holds the device gate before storage.
    pub(crate) fn with_pixels_mut<R>(&self, f: impl FnOnce(&mut [[u8; 4]]) -> R) -> Result<R, ()> {
        let mut pixels = self.state.pixels.lock().map_err(|_| ())?;
        Ok(f(&mut pixels))
    }

    fn poisoned_error(&self) -> FramebufferAccessError {
        FramebufferAccessError::StoragePoisoned {
            label: self.label().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instance, QueueSubmitError, RenderExecutionError};

    fn target(device: &Device) -> Framebuffer {
        device
            .create_framebuffer(FramebufferDescriptor {
                label: "internal target",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap()
    }

    #[test]
    fn target_poison_preserves_readback_destination_and_stops_later_commands() {
        let device = Instance::new().default_adapter().create_device();
        let target = target(&device);
        let later = self::target(&device);
        assert!(std::panic::catch_unwind(|| {
            let _pixels = target.state.pixels.lock().unwrap();
            panic!("simulate interrupted framebuffer storage");
        })
        .is_err());
        let mut destination = [[99; 4]; 4];
        assert!(matches!(
            target.read_pixels(&mut destination),
            Err(FramebufferAccessError::StoragePoisoned { .. })
        ));
        assert_eq!(destination, [[99; 4]; 4]);
        let mut encoder = device.create_command_encoder();
        encoder
            .begin_render_pass(&target, Some([7; 4]))
            .unwrap()
            .finish()
            .unwrap();
        encoder
            .begin_render_pass(&later, Some([8; 4]))
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(
            device.default_queue().submit(&encoder.finish()),
            Err(QueueSubmitError::RenderFailed {
                command_index: 0,
                source: RenderExecutionError::FramebufferStoragePoisoned {
                    label: "internal target".to_owned()
                },
            })
        );
        assert_eq!(
            *target.state.pixels.lock().unwrap_err().into_inner(),
            [[0; 4]; 4]
        );
        assert_eq!(later.pixel(0, 0).unwrap(), [0; 4]);
    }

    #[test]
    fn recorded_targets_live_until_the_last_owner_and_are_not_retained_by_queue() {
        let device = Instance::new().default_adapter().create_device();
        let queue = device.default_queue();
        let target = target(&device);
        let state = Arc::downgrade(&target.state);
        let mut encoder = device.create_command_encoder();
        encoder
            .begin_render_pass(&target, Some([7; 4]))
            .unwrap()
            .finish()
            .unwrap();
        let commands = encoder.finish();
        drop(target);
        drop(device);
        assert!(state.upgrade().is_some());
        queue.submit(&commands).unwrap();
        drop(commands);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn dropping_an_unfinished_scope_releases_its_target() {
        let device = Instance::new().default_adapter().create_device();
        let target = target(&device);
        let state = Arc::downgrade(&target.state);
        let mut encoder = device.create_command_encoder();
        let pass = encoder.begin_render_pass(&target, None).unwrap();
        drop(target);
        assert!(state.upgrade().is_some());
        drop(pass);
        assert!(state.upgrade().is_none());
        assert!(encoder.commands().is_empty());
    }
}
