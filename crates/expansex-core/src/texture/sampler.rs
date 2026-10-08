use super::TextureView;
use crate::Device;
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
    Linear,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
}
#[derive(Debug, Clone, Copy)]
pub struct SamplerDescriptor<'a> {
    pub label: &'a str,
    pub filter: FilterMode,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
}
#[derive(Debug)]
pub enum SamplerCreateError {
    AllocationFailed { source: TryReserveError },
}
impl fmt::Display for SamplerCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve sampler label: {source}")
            }
        }
    }
}
impl Error for SamplerCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
        }
    }
}
#[derive(Debug)]
struct SamplerState {
    device: Device,
    label: String,
    filter: FilterMode,
    address_u: AddressMode,
    address_v: AddressMode,
}
#[derive(Debug, Clone)]
pub struct Sampler {
    state: Arc<SamplerState>,
}
impl Device {
    pub fn create_sampler(
        &self,
        descriptor: SamplerDescriptor<'_>,
    ) -> Result<Sampler, SamplerCreateError> {
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| SamplerCreateError::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        Ok(Sampler {
            state: Arc::new(SamplerState {
                device: self.clone(),
                label,
                filter: descriptor.filter,
                address_u: descriptor.address_u,
                address_v: descriptor.address_v,
            }),
        })
    }
}
impl Sampler {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn filter(&self) -> FilterMode {
        self.state.filter
    }
    pub fn address_u(&self) -> AddressMode {
        self.state.address_u
    }
    pub fn address_v(&self) -> AddressMode {
        self.state.address_v
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
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureSampleError {
    ForeignSampler { label: String },
    NonFiniteCoordinates,
    StoragePoisoned { label: String },
}
impl fmt::Display for TextureSampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignSampler { label } => {
                write!(f, "sampler {label:?} belongs to another device")
            }
            Self::NonFiniteCoordinates => f.write_str("texture UV coordinates must be finite"),
            Self::StoragePoisoned { label } => write!(f, "texture {label:?} storage is poisoned"),
        }
    }
}
impl Error for TextureSampleError {}
impl TextureView {
    /// Synchronized standalone sampling; shader sampling needs no HostAccess.
    pub fn sample(&self, sampler: &Sampler, uv: [f32; 2]) -> Result<[f32; 4], TextureSampleError> {
        if !sampler.belongs_to(&self.texture.state.device) {
            return Err(TextureSampleError::ForeignSampler {
                label: sampler.label().to_owned(),
            });
        }
        if !uv.iter().all(|v| v.is_finite()) {
            return Err(TextureSampleError::NonFiniteCoordinates);
        }
        let _gate = self.texture.state.device.lock_execution();
        let pixels =
            self.texture
                .state
                .pixels
                .lock()
                .map_err(|_| TextureSampleError::StoragePoisoned {
                    label: self.texture.label().to_owned(),
                })?;
        Ok(sample(
            self.region.width,
            self.region.height,
            sampler,
            uv,
            |x, y| {
                pixels[(y + self.region.y) as usize * self.texture.width() as usize
                    + (x + self.region.x) as usize]
            },
        ))
    }
}
pub(crate) fn sample(
    width: u32,
    height: u32,
    sampler: &Sampler,
    uv: [f32; 2],
    texel: impl Fn(u32, u32) -> [u8; 4],
) -> [f32; 4] {
    let coordinate = |v: f32, size: u32, mode| {
        let v = f64::from(v);
        let normalized = match mode {
            AddressMode::ClampToEdge => v.clamp(0.0, 1.0),
            AddressMode::Repeat => {
                // Tiny negative coordinates may round their fractional part to
                // exactly one. Keep nearest sampling on the last texel side.
                (v - v.floor()).min(f64::from_bits(1.0_f64.to_bits() - 1))
            }
        };
        normalized * f64::from(size)
    };
    let address = |v: i64, size: u32, mode| -> u32 {
        match mode {
            AddressMode::ClampToEdge => v.clamp(0, i64::from(size) - 1) as u32,
            AddressMode::Repeat => v.rem_euclid(i64::from(size)) as u32,
        }
    };
    let x = coordinate(uv[0], width, sampler.address_u());
    let y = coordinate(uv[1], height, sampler.address_v());
    let read = |x, y| {
        texel(
            address(x, width, sampler.address_u()),
            address(y, height, sampler.address_v()),
        )
        .map(|v| f64::from(v) / 255.0)
    };
    if sampler.filter() == FilterMode::Nearest {
        return read(x.floor() as i64, y.floor() as i64).map(|v| v as f32);
    }
    let x = x - 0.5;
    let y = y - 0.5;
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let a = read(ix, iy);
    let b = read(ix + 1, iy);
    let c = read(ix, iy + 1);
    let d = read(ix + 1, iy + 1);
    std::array::from_fn(|lane| {
        (((a[lane] * (1.0 - fx) + b[lane] * fx) * (1.0 - fy))
            + ((c[lane] * (1.0 - fx) + d[lane] * fx) * fy))
            .clamp(0.0, 1.0) as f32
    })
}
