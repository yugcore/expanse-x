//! The supported execution contract, distinct from backend identity.

/// A coherent supported core contract. Preview profiles remain provisional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoreProfile {
    /// Synchronous bounded float4 graphics/compute and whole-resource dependencies.
    Preview1,
}

/// Numeric bounds for valid requests; memory allocation can still fail.
/// Changing a returned copy does not configure an adapter or device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CoreLimits {
    pub max_buffer_size: u64,
    pub max_image_extent: u32,
    pub max_shader_inputs: u32,
    pub max_shader_instructions: u32,
    pub max_vertex_outputs: u32,
    pub binding_slots: u32,
    pub max_uniform_buffer_size: u64,
    /// Each local axis and the product of local axes are bounded by this value.
    pub max_workgroup_size: u32,
    /// Maximum workgroup count on each axis; zero counts are allowed.
    pub max_workgroup_count: u32,
    pub max_compute_invocations: u32,
}

pub(crate) const PREVIEW_LIMITS: CoreLimits = CoreLimits {
    max_buffer_size: isize::MAX as u64,
    max_image_extent: 16384,
    max_shader_inputs: 16,
    max_shader_instructions: 256,
    max_vertex_outputs: 17,
    binding_slots: 8,
    max_uniform_buffer_size: 4096,
    max_workgroup_size: 1024,
    max_workgroup_count: 65535,
    max_compute_invocations: 1_048_576,
};

impl CoreProfile {
    /// Inspect valid-request bounds without allocating or executing work.
    ///
    /// ```
    /// use expansex_core::{CoreProfile, Instance};
    /// let adapter = Instance::new().default_adapter();
    /// assert_eq!(adapter.core_profile(), CoreProfile::Preview1);
    /// let device = adapter.create_device();
    /// assert_eq!(device.core_profile(), adapter.core_profile());
    /// assert_eq!(device.core_profile().limits().binding_slots, 8);
    /// ```
    pub fn limits(&self) -> CoreLimits {
        match self {
            Self::Preview1 => PREVIEW_LIMITS,
        }
    }
}
