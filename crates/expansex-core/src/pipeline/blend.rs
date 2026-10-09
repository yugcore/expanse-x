//! Immutable colour arithmetic; called only after successful fragment shading.

/// A multiplier for each source or destination lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColour,
    OneMinusSrcColour,
    DstColour,
    OneMinusDstColour,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    /// min(source alpha, 1-destination alpha) for RGB; one for alpha.
    SrcAlphaSaturate,
}

/// Min/Max ignore factors; subtraction results are clamped before storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// One equation applied either to RGB lanes or to alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendComponent {
    pub source_factor: BlendFactor,
    pub destination_factor: BlendFactor,
    pub operation: BlendOperation,
}

/// Separate RGB/alpha equations and an RGBA channel write mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlendState {
    pub colour: BlendComponent,
    pub alpha: BlendComponent,
    /// False preserves the stored channel. Does not affect clears or depth writes.
    pub write_mask: [bool; 4],
}

impl BlendState {
    /// Source-over using straight source RGB and separate alpha accumulation.
    pub fn alpha_blending() -> Self {
        Self {
            colour: BlendComponent {
                source_factor: BlendFactor::SrcAlpha,
                destination_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                source_factor: BlendFactor::One,
                destination_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            write_mask: [true; 4],
        }
    }

    /// Source-over expecting source RGB already multiplied by source alpha.
    pub fn premultiplied_alpha() -> Self {
        let component = BlendComponent {
            source_factor: BlendFactor::One,
            destination_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        };
        Self {
            colour: component,
            alpha: component,
            write_mask: [true; 4],
        }
    }

    /// Sum source and destination in all channels, clamping to normalized storage.
    pub fn additive() -> Self {
        let component = BlendComponent {
            source_factor: BlendFactor::One,
            destination_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        };
        Self {
            colour: component,
            alpha: component,
            write_mask: [true; 4],
        }
    }

    pub(crate) fn apply(self, source: [f32; 4], destination: [u8; 4]) -> [u8; 4] {
        let src = source.map(|v| f64::from(v.clamp(0.0, 1.0)));
        let dst = destination.map(|v| f64::from(v) / 255.0);
        std::array::from_fn(|lane| {
            if !self.write_mask[lane] {
                return destination[lane];
            }
            let component = if lane == 3 { self.alpha } else { self.colour };
            // Evaluate in byte units: avoid dividing a destination byte by 255
            // and multiplying it back, which can move exact half-byte ties down.
            let source_byte = src[lane] * 255.0;
            let destination_byte = f64::from(destination[lane]);
            let s = source_byte * component.source_factor.value(src, dst, lane);
            let d = destination_byte * component.destination_factor.value(src, dst, lane);
            let value = match component.operation {
                BlendOperation::Add => s + d,
                BlendOperation::Subtract => s - d,
                BlendOperation::ReverseSubtract => d - s,
                BlendOperation::Min => source_byte.min(destination_byte),
                BlendOperation::Max => source_byte.max(destination_byte),
            };
            value.clamp(0.0, 255.0).round() as u8
        })
    }
}

impl BlendFactor {
    fn value(self, src: [f64; 4], dst: [f64; 4], lane: usize) -> f64 {
        match self {
            Self::Zero => 0.0,
            Self::One => 1.0,
            Self::SrcColour => src[lane],
            Self::OneMinusSrcColour => 1.0 - src[lane],
            Self::DstColour => dst[lane],
            Self::OneMinusDstColour => 1.0 - dst[lane],
            Self::SrcAlpha => src[3],
            Self::OneMinusSrcAlpha => 1.0 - src[3],
            Self::DstAlpha => dst[3],
            Self::OneMinusDstAlpha => 1.0 - dst[3],
            Self::SrcAlphaSaturate => {
                if lane == 3 {
                    1.0
                } else {
                    src[3].min(1.0 - dst[3])
                }
            }
        }
    }
}
