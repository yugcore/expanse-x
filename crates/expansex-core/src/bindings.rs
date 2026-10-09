//! Retained resource interfaces for the bounded vector shader profile.
use crate::{
    Buffer, BufferUsage, Device, Sampler, Shader, ShaderInstruction, ShaderStage, TextureView,
};
use std::{collections::TryReserveError, error::Error, fmt, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderVisibility {
    Vertex,
    Fragment,
    Graphics,
    Compute,
    All,
}
impl ShaderVisibility {
    pub fn contains(&self, stage: ShaderStage) -> bool {
        matches!(
            (self, stage),
            (Self::All, _)
                | (Self::Vertex, ShaderStage::Vertex)
                | (Self::Fragment, ShaderStage::Fragment)
                | (Self::Graphics, ShaderStage::Vertex | ShaderStage::Fragment)
                | (Self::Compute, ShaderStage::Compute)
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingType {
    UniformBuffer { min_size: u64 },
    StorageBuffer { read_only: bool },
    SampledTexture,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderVisibility,
    pub ty: BindingType,
}
#[derive(Debug, Clone, Copy)]
pub struct BindGroupLayoutDescriptor<'a> {
    pub label: &'a str,
    pub entries: &'a [BindGroupLayoutEntry],
}
#[derive(Debug)]
struct LayoutState {
    device: Device,
    label: String,
    entries: Vec<BindGroupLayoutEntry>,
}
#[derive(Debug, Clone)]
pub struct BindGroupLayout {
    state: Arc<LayoutState>,
}
impl BindGroupLayout {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn entries(&self) -> &[BindGroupLayoutEntry] {
        &self.state.entries
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
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.belongs_to(&other.state.device) && self.entries() == other.entries()
    }
    fn entry(&self, binding: u32) -> Option<&BindGroupLayoutEntry> {
        self.entries().iter().find(|e| e.binding == binding)
    }
}

/// An owned handle with a byte view; cloning retains storage, not a byte snapshot.
#[derive(Debug, Clone)]
pub struct BufferBinding {
    pub buffer: Buffer,
    pub offset: u64,
    pub size: u64,
}
#[derive(Debug, Clone)]
pub enum BindingResource {
    UniformBuffer(BufferBinding),
    StorageBuffer(BufferBinding),
    SampledTexture { view: TextureView, sampler: Sampler },
}
#[derive(Debug, Clone)]
pub struct BindGroupEntry {
    pub binding: u32,
    pub resource: BindingResource,
}
#[derive(Debug, Clone, Copy)]
pub struct BindGroupDescriptor<'a> {
    pub label: &'a str,
    pub layout: &'a BindGroupLayout,
    pub entries: &'a [BindGroupEntry],
}
#[derive(Debug)]
struct GroupState {
    label: String,
    layout: BindGroupLayout,
    entries: Vec<BindGroupEntry>,
}
#[derive(Debug, Clone)]
pub struct BindGroup {
    state: Arc<GroupState>,
}
impl BindGroup {
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn layout(&self) -> &BindGroupLayout {
        &self.state.layout
    }
    pub fn entries(&self) -> &[BindGroupEntry] {
        &self.state.entries
    }
    pub fn device(&self) -> Device {
        self.layout().device()
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.layout().belongs_to(device)
    }
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
    pub(crate) fn entry(&self, slot: u32) -> Option<&BindingResource> {
        self.entries()
            .iter()
            .find(|e| e.binding == slot)
            .map(|e| &e.resource)
    }
    pub(crate) fn snapshot_uniforms(&self, mask: u8) -> Result<Uniforms, SnapshotError> {
        let mut result: Uniforms = Vec::new();
        if mask == 0 {
            return Ok(result);
        }
        result
            .try_reserve_exact(8)
            .map_err(|_| SnapshotError::Allocation)?;
        result.resize_with(8, Vec::new);
        for entry in self
            .entries()
            .iter()
            .filter(|e| mask & (1 << e.binding) != 0)
        {
            if let BindingResource::UniformBuffer(view) = &entry.resource {
                let target = &mut result[entry.binding as usize];
                target
                    .try_reserve_exact(view.size as usize / 16)
                    .map_err(|_| SnapshotError::Allocation)?;
                view.buffer
                    .with_bytes(
                        view.offset as usize..(view.offset + view.size) as usize,
                        |bytes| {
                            target.extend(bytes.as_chunks::<16>().0.iter().map(|record| {
                                std::array::from_fn(|lane| {
                                    let start = lane * 4;
                                    f32::from_le_bytes([
                                        record[start],
                                        record[start + 1],
                                        record[start + 2],
                                        record[start + 3],
                                    ])
                                })
                            }));
                        },
                    )
                    .map_err(|_| SnapshotError::Poison {
                        binding: entry.binding,
                        label: view.buffer.label().to_owned(),
                    })?;
            }
        }
        Ok(result)
    }
}
pub(crate) type Uniforms = Vec<Vec<[f32; 4]>>;
pub(crate) enum SnapshotError {
    Allocation,
    Poison { binding: u32, label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingValidationError {
    InvalidBinding {
        binding: u32,
    },
    DuplicateBinding {
        binding: u32,
    },
    InvalidMinimum {
        binding: u32,
        size: u64,
    },
    ForeignLayout,
    ForeignResource {
        binding: u32,
        label: String,
    },
    MissingBinding {
        binding: u32,
    },
    UnexpectedBinding {
        binding: u32,
    },
    WrongType {
        binding: u32,
        expected: BindingType,
    },
    InvalidRange {
        binding: u32,
        offset: u64,
        size: u64,
        buffer_size: u64,
    },
    MissingUsage {
        binding: u32,
        usage: BufferUsage,
    },
    LayoutRequired,
    MissingGroup,
    IncompatibleLayout,
    StageNotVisible {
        binding: u32,
        stage: ShaderStage,
    },
    UniformTooSmall {
        binding: u32,
        required: u64,
        declared: u64,
    },
    ReadOnlyOutput {
        binding: u32,
    },
    OverlappingUniformStorage {
        uniform: u32,
        storage: u32,
    },
}
impl fmt::Display for BindingValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinding { binding } => write!(f, "binding {binding} is outside 0..8"),
            Self::DuplicateBinding { binding } => write!(f, "binding {binding} occurs twice"),
            Self::InvalidMinimum { binding, size } => write!(f, "uniform binding {binding} minimum {size} must be 16-aligned and in 16..=4096"),
            Self::ForeignLayout => f.write_str("binding layout belongs to another device"),
            Self::ForeignResource { binding, label } => write!(f, "binding {binding} resource {label:?} belongs to another device"),
            Self::MissingBinding { binding } => write!(f, "required binding {binding} is missing"),
            Self::UnexpectedBinding { binding } => write!(f, "binding {binding} is absent from the layout"),
            Self::WrongType { binding, expected } => write!(f, "binding {binding} requires {expected:?}"),
            Self::InvalidRange { binding, offset, size, buffer_size } => write!(f, "binding {binding} view {offset}+{size} needs a nonzero 16-aligned range inside {buffer_size} bytes; uniform views are at most 4096 bytes"),
            Self::MissingUsage { binding, usage } => write!(f, "binding {binding} buffer needs {usage:?} usage"),
            Self::LayoutRequired => f.write_str("this operation requires a layout-aware pipeline"),
            Self::MissingGroup => f.write_str("layout-aware pipeline requires a bind group"),
            Self::IncompatibleLayout => f.write_str("group layout is incompatible with pipeline layout"),
            Self::StageNotVisible { binding, stage } => write!(f, "binding {binding} is not visible to {stage:?}"),
            Self::UniformTooSmall { binding, required, declared } => write!(f, "uniform binding {binding} needs {required} bytes, declared {declared}"),
            Self::ReadOnlyOutput { binding } => write!(f, "compute output binding {binding} must be writable storage"),
            Self::OverlappingUniformStorage { uniform, storage } => write!(f, "uniform binding {uniform} overlaps writable storage binding {storage}"),
        }
    }
}
impl Error for BindingValidationError {}
#[derive(Debug)]
pub enum BindingCreateError {
    InvalidDescriptor(BindingValidationError),
    AllocationFailed { source: TryReserveError },
}
impl fmt::Display for BindingCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create bindings: {source}"),
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve binding metadata: {source}")
            }
        }
    }
}
impl Error for BindingCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(e) => Some(e),
            Self::AllocationFailed { source } => Some(source),
        }
    }
}
impl From<BindingValidationError> for BindingCreateError {
    fn from(source: BindingValidationError) -> Self {
        Self::InvalidDescriptor(source)
    }
}
fn label(text: &str) -> Result<String, BindingCreateError> {
    let mut label = String::new();
    label
        .try_reserve_exact(text.len())
        .map_err(|source| BindingCreateError::AllocationFailed { source })?;
    label.push_str(text);
    Ok(label)
}
impl Device {
    /// Own a reusable immutable shader resource interface.
    ///
    /// ```
    /// use expansex_core::*;
    /// let d = Instance::new().default_adapter().create_device();
    /// let layout = d.create_bind_group_layout(BindGroupLayoutDescriptor {
    ///     label: "parameters", entries: &[BindGroupLayoutEntry {
    ///         binding: 0, visibility: ShaderVisibility::All,
    ///         ty: BindingType::UniformBuffer { min_size: 16 },
    ///     }],
    /// })?;
    /// let parameters = d.create_buffer(BufferDescriptor {
    ///     label: "colour", size: 16, usages: &[BufferUsage::Uniform],
    ///     host_access: HostAccess::Write,
    /// })?;
    /// let group = d.create_bind_group(BindGroupDescriptor {
    ///     label: "material", layout: &layout, entries: &[BindGroupEntry {
    ///         binding: 0, resource: BindingResource::UniformBuffer(BufferBinding {
    ///             buffer: parameters.clone(), offset: 0, size: 16,
    ///         }),
    ///     }],
    /// })?;
    /// drop(parameters); // The group retains its live allocation.
    /// assert!(group.layout().is_compatible(&layout));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn create_bind_group_layout(
        &self,
        descriptor: BindGroupLayoutDescriptor<'_>,
    ) -> Result<BindGroupLayout, BindingCreateError> {
        use BindingValidationError as E;
        let mut seen = 0u8;
        for entry in descriptor.entries {
            if entry.binding >= 8 {
                return Err(E::InvalidBinding {
                    binding: entry.binding,
                }
                .into());
            }
            if seen & (1 << entry.binding) != 0 {
                return Err(E::DuplicateBinding {
                    binding: entry.binding,
                }
                .into());
            }
            seen |= 1 << entry.binding;
            if let BindingType::UniformBuffer { min_size } = entry.ty {
                if !(16..=4096).contains(&min_size) || !min_size.is_multiple_of(16) {
                    return Err(E::InvalidMinimum {
                        binding: entry.binding,
                        size: min_size,
                    }
                    .into());
                }
            }
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(descriptor.entries.len())
            .map_err(|source| BindingCreateError::AllocationFailed { source })?;
        entries.extend_from_slice(descriptor.entries);
        entries.sort_unstable_by_key(|e| e.binding);
        Ok(BindGroupLayout {
            state: Arc::new(LayoutState {
                device: self.clone(),
                label: label(descriptor.label)?,
                entries,
            }),
        })
    }
    pub fn create_bind_group(
        &self,
        descriptor: BindGroupDescriptor<'_>,
    ) -> Result<BindGroup, BindingCreateError> {
        use BindingValidationError as E;
        if !descriptor.layout.belongs_to(self) {
            return Err(E::ForeignLayout.into());
        }
        let mut seen = 0u8;
        for entry in descriptor.entries {
            let slot = entry.binding;
            if slot >= 8 {
                return Err(E::InvalidBinding { binding: slot }.into());
            }
            if seen & (1 << slot) != 0 {
                return Err(E::DuplicateBinding { binding: slot }.into());
            }
            seen |= 1 << slot;
            let expected = descriptor
                .layout
                .entry(slot)
                .ok_or(E::UnexpectedBinding { binding: slot })?
                .ty;
            let foreign = |text: &str| E::ForeignResource {
                binding: slot,
                label: text.to_owned(),
            };
            let (view, uniform) = match (&entry.resource, expected) {
                (BindingResource::UniformBuffer(v), BindingType::UniformBuffer { .. }) => {
                    (Some(v), true)
                }
                (BindingResource::StorageBuffer(v), BindingType::StorageBuffer { .. }) => {
                    (Some(v), false)
                }
                (
                    BindingResource::SampledTexture { view, sampler },
                    BindingType::SampledTexture,
                ) => {
                    if !view.texture().belongs_to(self) {
                        return Err(foreign(view.texture().label()).into());
                    }
                    if !sampler.belongs_to(self) {
                        return Err(foreign(sampler.label()).into());
                    }
                    (None, false)
                }
                _ => {
                    return Err(E::WrongType {
                        binding: slot,
                        expected,
                    }
                    .into())
                }
            };
            if let Some(v) = view {
                if !v.buffer.belongs_to(self) {
                    return Err(foreign(v.buffer.label()).into());
                }
                let usage = if uniform {
                    BufferUsage::Uniform
                } else {
                    BufferUsage::Storage
                };
                if !v.buffer.usages().contains(&usage) {
                    return Err(E::MissingUsage {
                        binding: slot,
                        usage,
                    }
                    .into());
                }
                if v.size == 0
                    || !v.offset.is_multiple_of(16)
                    || !v.size.is_multiple_of(16)
                    || v.offset
                        .checked_add(v.size)
                        .is_none_or(|end| end > v.buffer.size())
                    || (uniform && v.size > 4096)
                {
                    return Err(E::InvalidRange {
                        binding: slot,
                        offset: v.offset,
                        size: v.size,
                        buffer_size: v.buffer.size(),
                    }
                    .into());
                }
                if let BindingType::UniformBuffer { min_size } = expected {
                    if v.size < min_size {
                        return Err(E::UniformTooSmall {
                            binding: slot,
                            required: min_size,
                            declared: v.size,
                        }
                        .into());
                    }
                }
            }
        }
        for entry in descriptor.layout.entries() {
            if seen & (1 << entry.binding) == 0 {
                return Err(E::MissingBinding {
                    binding: entry.binding,
                }
                .into());
            }
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(descriptor.entries.len())
            .map_err(|source| BindingCreateError::AllocationFailed { source })?;
        entries.extend_from_slice(descriptor.entries);
        entries.sort_unstable_by_key(|e| e.binding);
        Ok(BindGroup {
            state: Arc::new(GroupState {
                label: label(descriptor.label)?,
                layout: descriptor.layout.clone(),
                entries,
            }),
        })
    }
}

pub(crate) fn validate_shader(
    device: &Device,
    layout: Option<&BindGroupLayout>,
    shader: &Shader,
    outputs: &[u32],
) -> Result<(), BindingValidationError> {
    use BindingValidationError as E;
    let Some(layout) = layout else {
        return if shader.required_uniforms() != 0 {
            Err(E::LayoutRequired)
        } else {
            Ok(())
        };
    };
    if !layout.belongs_to(device) {
        return Err(E::ForeignLayout);
    }
    let require = |binding, expected: BindingType, size: u64| {
        let entry = layout.entry(binding).ok_or(E::MissingBinding { binding })?;
        if !entry.visibility.contains(shader.stage()) {
            return Err(E::StageNotVisible {
                binding,
                stage: shader.stage(),
            });
        }
        match (entry.ty, expected) {
            (BindingType::UniformBuffer { min_size }, BindingType::UniformBuffer { .. }) => {
                if min_size < size {
                    return Err(E::UniformTooSmall {
                        binding,
                        required: size,
                        declared: min_size,
                    });
                }
            }
            (BindingType::StorageBuffer { .. }, BindingType::StorageBuffer { .. }) => {}
            (BindingType::SampledTexture, BindingType::SampledTexture) => {}
            _ => return Err(E::WrongType { binding, expected }),
        }
        Ok(())
    };
    for op in shader.instructions() {
        match *op {
            ShaderInstruction::LoadUniform { binding, element } => require(
                binding,
                BindingType::UniformBuffer { min_size: 16 },
                (u64::from(element) + 1) * 16,
            )?,
            ShaderInstruction::LoadStorage { binding } => {
                require(binding, BindingType::StorageBuffer { read_only: true }, 0)?
            }
            ShaderInstruction::SampleTexture { binding, .. } => {
                require(binding, BindingType::SampledTexture, 0)?
            }
            _ => {}
        }
    }
    for &binding in outputs {
        require(binding, BindingType::StorageBuffer { read_only: false }, 0)?;
        if layout
            .entry(binding)
            .is_some_and(|e| matches!(e.ty, BindingType::StorageBuffer { read_only: true }))
        {
            return Err(E::ReadOnlyOutput { binding });
        }
    }
    Ok(())
}
pub(crate) fn check_group(
    layout: Option<&BindGroupLayout>,
    group: Option<&BindGroup>,
) -> Result<(), BindingValidationError> {
    if let Some(layout) = layout {
        let group = group.ok_or(BindingValidationError::MissingGroup)?;
        if !layout.is_compatible(group.layout()) {
            return Err(BindingValidationError::IncompatibleLayout);
        }
    }
    Ok(())
}

pub(crate) fn load_uniform(
    uniforms: &Uniforms,
    binding: u32,
    element: u32,
) -> Result<[f32; 4], crate::ShaderExecutionError> {
    uniforms
        .get(binding as usize)
        .and_then(|values| values.get(element as usize))
        .copied()
        .ok_or(crate::ShaderExecutionError::MissingUniformBinding { binding, element })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_group_and_layout_release_with_final_command() {
        let d = crate::Instance::new().default_adapter().create_device();
        let layout = d
            .create_bind_group_layout(BindGroupLayoutDescriptor {
                label: "empty",
                entries: &[],
            })
            .unwrap();
        let group = d
            .create_bind_group(BindGroupDescriptor {
                label: "empty",
                layout: &layout,
                entries: &[],
            })
            .unwrap();
        let layout_state = Arc::downgrade(&layout.state);
        let group_state = Arc::downgrade(&group.state);
        let vertex = d
            .create_shader(crate::ShaderDescriptor {
                label: "v",
                stage: ShaderStage::Vertex,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([0., 0., 0., 1.])],
                outputs: &[0],
            })
            .unwrap();
        let fragment = d
            .create_shader(crate::ShaderDescriptor {
                label: "f",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([1.; 4])],
                outputs: &[0],
            })
            .unwrap();
        let pipeline = d
            .create_graphics_pipeline_with_bindings(
                crate::GraphicsPipelineDescriptor {
                    label: "p",
                    vertex_shader: &vertex,
                    fragment_shader: &fragment,
                    vertex_layout: crate::VertexBufferLayout {
                        stride: 4,
                        attributes: &[],
                    },
                    colour_format: crate::FramebufferFormat::Rgba8Unorm,
                },
                crate::GraphicsPipelineOptions::default(),
                None,
                &layout,
            )
            .unwrap();
        let vertices = d
            .create_buffer(crate::BufferDescriptor {
                label: "v",
                size: 4,
                usages: &[BufferUsage::Vertex],
                host_access: crate::HostAccess::None,
            })
            .unwrap();
        let target = d
            .create_framebuffer(crate::FramebufferDescriptor {
                label: "t",
                width: 1,
                height: 1,
                format: crate::FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let mut e = d.create_command_encoder();
        let mut pass = e.begin_render_pass(&target, None).unwrap();
        pass.set_pipeline(&pipeline).unwrap();
        pass.set_vertex_buffer(&vertices, 0).unwrap();
        pass.set_bind_group(&group).unwrap();
        pass.draw_programmable(0..0).unwrap();
        pass.finish().unwrap();
        let commands = e.finish();
        drop(group);
        drop(layout);
        drop(pipeline);
        assert!(group_state.upgrade().is_some());
        assert!(layout_state.upgrade().is_some());
        drop(commands);
        assert!(group_state.upgrade().is_none());
        assert!(layout_state.upgrade().is_none());
    }
}
