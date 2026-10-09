use crate::CompareFunction;

/// Updates operate on the full stored byte before applying the write mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StencilOperation {
    Keep,
    Zero,
    Replace,
    IncrementClamp,
    DecrementClamp,
    Invert,
    IncrementWrap,
    DecrementWrap,
}

/// Comparison and updates for one triangle facing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilFaceState {
    pub compare: CompareFunction,
    pub fail_op: StencilOperation,
    pub depth_fail_op: StencilOperation,
    pub pass_op: StencilOperation,
}

/// Immutable front/back rules with shared reference and bit masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilState {
    pub front: StencilFaceState,
    pub back: StencilFaceState,
    pub reference: u8,
    pub read_mask: u8,
    pub write_mask: u8,
}

impl StencilState {
    /// Mark passing fragments with reference; use a colour mask for invisible marking.
    pub fn replace(reference: u8) -> Self {
        let face = StencilFaceState {
            compare: CompareFunction::Always,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Replace,
        };
        Self {
            front: face,
            back: face,
            reference,
            read_mask: 255,
            write_mask: 255,
        }
    }

    /// Draw only matching pixels; preserve every stencil bit.
    pub fn test_equal(reference: u8) -> Self {
        let face = StencilFaceState {
            compare: CompareFunction::Equal,
            pass_op: StencilOperation::Keep,
            ..Self::replace(reference).front
        };
        Self {
            front: face,
            back: face,
            reference,
            read_mask: 255,
            write_mask: 0,
        }
    }

    pub(crate) fn passes(self, face: StencilFaceState, stored: u8) -> bool {
        // Every u8 is represented exactly in f32; reuse the shared comparison.
        face.compare.test(
            f32::from(self.reference & self.read_mask),
            f32::from(stored & self.read_mask),
        )
    }

    pub(crate) fn update(self, operation: StencilOperation, stored: u8) -> u8 {
        let value = match operation {
            StencilOperation::Keep => stored,
            StencilOperation::Zero => 0,
            StencilOperation::Replace => self.reference,
            StencilOperation::IncrementClamp => stored.saturating_add(1),
            StencilOperation::DecrementClamp => stored.saturating_sub(1),
            StencilOperation::Invert => !stored,
            StencilOperation::IncrementWrap => stored.wrapping_add(1),
            StencilOperation::DecrementWrap => stored.wrapping_sub(1),
        };
        (stored & !self.write_mask) | (value & self.write_mask)
    }
}
