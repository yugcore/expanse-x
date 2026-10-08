use crate::{Device, HostAccess};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
mod sampler;
pub(crate) use sampler::sample;
pub use sampler::{
    AddressMode, FilterMode, Sampler, SamplerCreateError, SamplerDescriptor, TextureSampleError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextureFormat {
    Rgba8Unorm,
}

/// One sampled 2D image; creation copies metadata and zero-initializes storage.
#[derive(Debug, Clone, Copy)]
pub struct TextureDescriptor<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub host_access: HostAccess,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDescriptorError {
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
impl TextureDescriptor<'_> {
    pub fn validate(&self) -> Result<(), TextureDescriptorError> {
        if self.width == 0 || self.height == 0 {
            return Err(TextureDescriptorError::ZeroExtent {
                width: self.width,
                height: self.height,
            });
        }
        if self.width > 16384 || self.height > 16384 {
            return Err(TextureDescriptorError::ExtentTooLarge {
                width: self.width,
                height: self.height,
                maximum: 16384,
            });
        }
        Ok(())
    }
}
impl fmt::Display for TextureDescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => {
                write!(f, "texture extent {width}x{height} must be positive")
            }
            Self::ExtentTooLarge {
                width,
                height,
                maximum,
            } => write!(
                f,
                "texture extent {width}x{height} exceeds maximum {maximum} per axis"
            ),
        }
    }
}
impl Error for TextureDescriptorError {}
#[derive(Debug)]
pub enum TextureCreateError {
    InvalidDescriptor(TextureDescriptorError),
    AllocationFailed { source: TryReserveError },
}
impl fmt::Display for TextureCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create texture: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve texture storage or label: {source}")
            }
        }
    }
}
impl Error for TextureCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(e) => Some(e),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}
/// Rectangle in parent texture texels; its origin is at the upper left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureViewDescriptor {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureAccessError {
    ReadForbidden {
        label: String,
    },
    WriteForbidden {
        label: String,
    },
    WrongPixelCount {
        expected: usize,
        actual: usize,
    },
    InvalidView {
        region: TextureViewDescriptor,
        width: u32,
        height: u32,
    },
    StoragePoisoned {
        label: String,
    },
}
impl fmt::Display for TextureAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadForbidden { label } => {
                write!(f, "texture {label:?} does not permit CPU reads")
            }
            Self::WriteForbidden { label } => {
                write!(f, "texture {label:?} does not permit CPU writes")
            }
            Self::WrongPixelCount { expected, actual } => write!(
                f,
                "texture access needs {expected} pixels, received {actual}"
            ),
            Self::InvalidView {
                region,
                width,
                height,
            } => write!(
                f,
                "view {region:?} must be positive and fit texture {width}x{height}"
            ),
            Self::StoragePoisoned { label } => write!(f, "texture {label:?} storage is poisoned"),
        }
    }
}
impl Error for TextureAccessError {}
struct TextureState {
    device: Device,
    label: String,
    width: u32,
    height: u32,
    format: TextureFormat,
    host_access: HostAccess,
    pixels: Mutex<Vec<[u8; 4]>>,
}
/// Shared sampled image. CPU copies and submissions serialize on its device.
#[derive(Clone)]
pub struct Texture {
    state: Arc<TextureState>,
}
impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Texture")
            .field("label", &self.label())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("format", &self.format())
            .field("host_access", &self.host_access())
            .finish_non_exhaustive()
    }
}
impl Device {
    pub fn create_texture(
        &self,
        descriptor: TextureDescriptor<'_>,
    ) -> Result<Texture, TextureCreateError> {
        descriptor
            .validate()
            .map_err(TextureCreateError::InvalidDescriptor)?;
        // Extent caps RGBA storage at 1 GiB, fitting 32/64-bit contiguous vectors.
        let count = descriptor.width as usize * descriptor.height as usize;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|source| TextureCreateError::AllocationFailed { source })?;
        pixels.resize(count, [0; 4]);
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| TextureCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(Texture {
            state: Arc::new(TextureState {
                device: self.clone(),
                label,
                width: descriptor.width,
                height: descriptor.height,
                format: descriptor.format,
                host_access: descriptor.host_access,
                pixels: Mutex::new(pixels),
            }),
        })
    }
}
impl Texture {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn width(&self) -> u32 {
        self.state.width
    }
    pub fn height(&self) -> u32 {
        self.state.height
    }
    pub fn format(&self) -> TextureFormat {
        self.state.format
    }
    pub fn host_access(&self) -> HostAccess {
        self.state.host_access
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
    /// Full-image row-major upload. Permission/count failure leaves pixels intact.
    pub fn write_pixels(&self, pixels: &[[u8; 4]]) -> Result<(), TextureAccessError> {
        if !matches!(
            self.host_access(),
            HostAccess::Write | HostAccess::ReadWrite
        ) {
            return Err(TextureAccessError::WriteForbidden {
                label: self.label().to_owned(),
            });
        }
        self.check_count(pixels.len())?;
        let _gate = self.state.device.lock_execution();
        self.state
            .pixels
            .lock()
            .map_err(|_| self.poison())?
            .copy_from_slice(pixels);
        Ok(())
    }
    /// Full-image readback; CPU permission is independent of shader sampling.
    pub fn read_pixels(&self, pixels: &mut [[u8; 4]]) -> Result<(), TextureAccessError> {
        if !matches!(self.host_access(), HostAccess::Read | HostAccess::ReadWrite) {
            return Err(TextureAccessError::ReadForbidden {
                label: self.label().to_owned(),
            });
        }
        self.check_count(pixels.len())?;
        let _gate = self.state.device.lock_execution();
        pixels.copy_from_slice(&self.state.pixels.lock().map_err(|_| self.poison())?);
        Ok(())
    }
    fn check_count(&self, actual: usize) -> Result<(), TextureAccessError> {
        let expected = self.width() as usize * self.height() as usize;
        if actual != expected {
            return Err(TextureAccessError::WrongPixelCount { expected, actual });
        }
        Ok(())
    }
    fn poison(&self) -> TextureAccessError {
        TextureAccessError::StoragePoisoned {
            label: self.label().to_owned(),
        }
    }
    pub fn create_view(
        &self,
        region: TextureViewDescriptor,
    ) -> Result<TextureView, TextureAccessError> {
        if region.width == 0
            || region.height == 0
            || region.x > self.width()
            || region.y > self.height()
            || region.width > self.width() - region.x
            || region.height > self.height() - region.y
        {
            return Err(TextureAccessError::InvalidView {
                region,
                width: self.width(),
                height: self.height(),
            });
        }
        Ok(TextureView {
            texture: self.clone(),
            region,
        })
    }
    /// Full-image view without additional allocation.
    pub fn view(&self) -> TextureView {
        TextureView {
            texture: self.clone(),
            region: TextureViewDescriptor {
                x: 0,
                y: 0,
                width: self.width(),
                height: self.height(),
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct TextureView {
    texture: Texture,
    region: TextureViewDescriptor,
}
impl TextureView {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
    pub fn region(&self) -> TextureViewDescriptor {
        self.region
    }
    /// Caller holds the device gate. Snapshot each view with one short storage
    /// borrow; aliases never acquire the same mutex twice simultaneously.
    pub(crate) fn snapshot(&self) -> Result<Vec<[u8; 4]>, crate::RenderExecutionError> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(self.region.width as usize * self.region.height as usize)
            .map_err(|_| crate::RenderExecutionError::AllocationFailed)?;
        let pixels = self.texture.state.pixels.lock().map_err(|_| {
            crate::RenderExecutionError::TextureStoragePoisoned {
                label: self.texture.label().to_owned(),
            }
        })?;
        for y in self.region.y..self.region.y + self.region.height {
            let start = y as usize * self.texture.width() as usize + self.region.x as usize;
            output.extend_from_slice(&pixels[start..start + self.region.width as usize]);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn setup() -> (
        Device,
        Texture,
        Sampler,
        GraphicsPipeline,
        Buffer,
        Framebuffer,
    ) {
        let d = Instance::new().default_adapter().create_device();
        let tex = d
            .create_texture(TextureDescriptor {
                label: "poison",
                width: 1,
                height: 1,
                format: TextureFormat::Rgba8Unorm,
                host_access: HostAccess::ReadWrite,
            })
            .unwrap();
        let s = d
            .create_sampler(SamplerDescriptor {
                label: "sampler",
                filter: FilterMode::Nearest,
                address_u: AddressMode::ClampToEdge,
                address_v: AddressMode::ClampToEdge,
            })
            .unwrap();
        let vs = d
            .create_shader(ShaderDescriptor {
                label: "v",
                stage: ShaderStage::Vertex,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([0.0, 0.0, 0.0, 1.0])],
                outputs: &[0],
            })
            .unwrap();
        let fs = d
            .create_shader(ShaderDescriptor {
                label: "f",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[
                    ShaderInstruction::Constant([0.5; 4]),
                    ShaderInstruction::SampleTexture {
                        binding: 0,
                        coordinates: 0,
                    },
                ],
                outputs: &[1],
            })
            .unwrap();
        let p = d
            .create_graphics_pipeline(GraphicsPipelineDescriptor {
                label: "p",
                vertex_shader: &vs,
                fragment_shader: &fs,
                vertex_layout: VertexBufferLayout {
                    stride: 4,
                    attributes: &[],
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let b = d
            .create_buffer(BufferDescriptor {
                label: "v",
                size: 12,
                usages: &[BufferUsage::Vertex],
                host_access: HostAccess::None,
            })
            .unwrap();
        let t = d
            .create_framebuffer(FramebufferDescriptor {
                label: "target",
                width: 1,
                height: 1,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        (d, tex, s, p, b, t)
    }
    #[test]
    fn poison_errors_preserve_scope_and_empty_draws_skip_texture_storage() {
        let (d, tex, s, p, b, t) = setup();
        let broken = tex.clone();
        assert!(std::thread::spawn(move || {
            let _guard = broken.state.pixels.lock().unwrap();
            panic!("poison texture storage");
        })
        .join()
        .is_err());
        assert!(matches!(
            tex.write_pixels(&[[0; 4]]),
            Err(TextureAccessError::StoragePoisoned { .. })
        ));
        assert!(matches!(
            tex.read_pixels(&mut [[0; 4]]),
            Err(TextureAccessError::StoragePoisoned { .. })
        ));
        assert!(matches!(
            tex.view().sample(&s, [0.5; 2]),
            Err(TextureSampleError::StoragePoisoned { .. })
        ));
        for empty in [false, true] {
            let mut e = d.create_command_encoder();
            let mut pass = e.begin_render_pass(&t, Some([7; 4])).unwrap();
            pass.set_pipeline(&p).unwrap();
            pass.set_vertex_buffer(&b, 0).unwrap();
            pass.set_texture(0, &tex.view(), &s).unwrap();
            pass.draw_programmable(0..if empty { 0 } else { 3 })
                .unwrap();
            pass.finish().unwrap();
            let result = d.default_queue().submit(&e.finish());
            if empty {
                result.unwrap();
                assert_eq!(t.pixel(0, 0).unwrap(), [7; 4]);
            } else {
                assert_eq!(
                    result,
                    Err(QueueSubmitError::RenderFailed {
                        command_index: 0,
                        source: RenderExecutionError::TextureStoragePoisoned {
                            label: "poison".to_owned()
                        }
                    })
                );
                assert_eq!(t.pixel(0, 0).unwrap(), [0; 4]);
            }
        }
    }
    #[test]
    fn finished_draw_retains_texture_until_last_command_owner_drops() {
        let (d, tex, s, p, b, t) = setup();
        let weak = Arc::downgrade(&tex.state);
        let mut e = d.create_command_encoder();
        let mut pass = e.begin_render_pass(&t, None).unwrap();
        pass.set_pipeline(&p).unwrap();
        pass.set_vertex_buffer(&b, 0).unwrap();
        pass.set_texture(0, &tex.view(), &s).unwrap();
        pass.draw_programmable(0..3).unwrap();
        pass.finish().unwrap();
        let c = e.finish();
        drop(tex);
        drop(s);
        drop(p);
        drop(b);
        d.default_queue().submit(&c).unwrap();
        assert!(weak.upgrade().is_some());
        drop(c);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn texture_upload_and_sampling_wait_for_the_device_execution_gate() {
        use std::sync::mpsc;
        let (d, tex, s, _, _, _) = setup();
        let gate = d.lock_execution();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            tex.write_pixels(&[[255; 4]]).unwrap();
            done_tx
                .send(tex.view().sample(&s, [0.5; 2]).unwrap())
                .unwrap();
        });
        started_rx.recv().unwrap();
        assert!(done_rx
            .recv_timeout(std::time::Duration::from_millis(30))
            .is_err());
        drop(gate);
        assert_eq!(
            done_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            [1.0; 4]
        );
        worker.join().unwrap();
    }
}
