//! Validated independent float4 compute invocations and atomic storage output.
use crate::{
    Buffer, BufferUsage, Command, CommandEncoder, Device, Shader, ShaderInstruction, ShaderStage,
};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Borrowed immutable compute configuration, owned/retained on creation.
#[derive(Debug, Clone, Copy)]
pub struct ComputePipelineDescriptor<'a> {
    pub label: &'a str,
    pub shader: &'a Shader,
    /// Positive local dimensions; each axis and their product are at most 1024.
    pub workgroup_size: [u32; 3],
    /// One unique storage slot in 0..8 per shader output, in output order.
    pub output_bindings: &'a [u32],
}

#[derive(Debug)]
pub enum ComputePipelineCreateError {
    Bindings {
        source: crate::BindingValidationError,
    },
    ForeignShader {
        label: String,
    },
    WrongShaderStage {
        actual: ShaderStage,
    },
    InvalidWorkgroupSize {
        size: [u32; 3],
    },
    OutputCountMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidOutputBinding {
        binding: u32,
    },
    DuplicateOutputBinding {
        binding: u32,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}
impl fmt::Display for ComputePipelineCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bindings { source } => write!(f, "compute resource interface: {source}"),
            Self::ForeignShader { label } => {
                write!(f, "compute shader {label:?} belongs to another device")
            }
            Self::WrongShaderStage { actual } => write!(
                f,
                "compute pipeline needs Compute shader, received {actual:?}"
            ),
            Self::InvalidWorkgroupSize { size } => write!(
                f,
                "local size {size:?} needs positive axes and product at most 1024"
            ),
            Self::OutputCountMismatch { expected, actual } => write!(
                f,
                "compute pipeline needs {expected} output slots, received {actual}"
            ),
            Self::InvalidOutputBinding { binding } => {
                write!(f, "output slot {binding} is outside 0..8")
            }
            Self::DuplicateOutputBinding { binding } => {
                write!(f, "output slot {binding} occurs more than once")
            }
            Self::AllocationFailed { source } => {
                write!(f, "cannot reserve compute pipeline metadata: {source}")
            }
        }
    }
}
impl Error for ComputePipelineCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bindings { source } => Some(source),
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ComputeState {
    device: Device,
    label: String,
    shader: Shader,
    workgroup_size: [u32; 3],
    outputs: Vec<u32>,
    reads: u8,
    binding_layout: Option<crate::BindGroupLayout>,
}

/// Shared immutable compute program retaining its shader and logical device.
#[derive(Debug, Clone)]
pub struct ComputePipeline {
    state: Arc<ComputeState>,
}

impl Device {
    /// Create a compute pipeline with immutable workgroup size and output slots.
    ///
    /// ```
    /// use expansex_core::*;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let device = Instance::new().default_adapter().create_device();
    /// let shader = device.create_shader(ShaderDescriptor {
    ///     label: "IDs", stage: ShaderStage::Compute, input_count: 4,
    ///     instructions: &[ShaderInstruction::Input { location: 0 }], outputs: &[0],
    /// })?;
    /// let pipeline = device.create_compute_pipeline(ComputePipelineDescriptor {
    ///     label: "generate IDs", shader: &shader,
    ///     workgroup_size: [2, 1, 1], output_bindings: &[0],
    /// })?;
    /// let output = device.create_buffer(BufferDescriptor {
    ///     label: "IDs", size: 32, usages: &[BufferUsage::Storage],
    ///     host_access: HostAccess::Read,
    /// })?;
    /// let mut encoder = device.create_command_encoder();
    /// encoder.dispatch(&pipeline, [1, 1, 1], &[StorageBufferBinding {
    ///     binding: 0, buffer: &output, offset: 0, size: 32,
    /// }])?;
    /// device.default_queue().submit(&encoder.finish())?;
    /// let mut bytes = [0; 32]; output.read(0, &mut bytes)?;
    /// assert_eq!(&bytes[16..20], &1.0_f32.to_le_bytes());
    /// # Ok(()) }
    /// ```
    pub fn create_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor<'_>,
    ) -> Result<ComputePipeline, ComputePipelineCreateError> {
        self.create_compute_with_layout(descriptor, None)
    }
    pub fn create_compute_pipeline_with_bindings(
        &self,
        descriptor: ComputePipelineDescriptor<'_>,
        layout: &crate::BindGroupLayout,
    ) -> Result<ComputePipeline, ComputePipelineCreateError> {
        self.create_compute_with_layout(descriptor, Some(layout))
    }
    fn create_compute_with_layout(
        &self,
        descriptor: ComputePipelineDescriptor<'_>,
        binding_layout: Option<&crate::BindGroupLayout>,
    ) -> Result<ComputePipeline, ComputePipelineCreateError> {
        use ComputePipelineCreateError as E;
        if !descriptor.shader.belongs_to(self) {
            return Err(E::ForeignShader {
                label: descriptor.shader.label().to_owned(),
            });
        }
        if descriptor.shader.stage() != ShaderStage::Compute {
            return Err(E::WrongShaderStage {
                actual: descriptor.shader.stage(),
            });
        }
        if descriptor
            .workgroup_size
            .iter()
            .any(|&axis| axis == 0 || axis > crate::profile::PREVIEW_LIMITS.max_workgroup_size)
            || product(descriptor.workgroup_size)
                .is_none_or(|count| count > crate::profile::PREVIEW_LIMITS.max_workgroup_size)
        {
            return Err(E::InvalidWorkgroupSize {
                size: descriptor.workgroup_size,
            });
        }
        if descriptor.output_bindings.len() != descriptor.shader.outputs().len() {
            return Err(E::OutputCountMismatch {
                expected: descriptor.shader.outputs().len(),
                actual: descriptor.output_bindings.len(),
            });
        }
        let mut seen = 0_u8;
        for &binding in descriptor.output_bindings {
            if binding >= crate::profile::PREVIEW_LIMITS.binding_slots {
                return Err(E::InvalidOutputBinding { binding });
            }
            if seen & (1 << binding) != 0 {
                return Err(E::DuplicateOutputBinding { binding });
            }
            seen |= 1 << binding;
        }
        crate::bindings::validate_shader(
            self,
            binding_layout,
            descriptor.shader,
            descriptor.output_bindings,
        )
        .map_err(|source| E::Bindings { source })?;
        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(|source| E::AllocationFailed { source })?;
        label.push_str(descriptor.label);
        let mut outputs = Vec::new();
        outputs
            .try_reserve_exact(descriptor.output_bindings.len())
            .map_err(|source| E::AllocationFailed { source })?;
        outputs.extend_from_slice(descriptor.output_bindings);
        let reads = descriptor
            .shader
            .instructions()
            .iter()
            .fold(0, |mask, op| match *op {
                ShaderInstruction::LoadStorage { binding } => mask | (1 << binding),
                _ => mask,
            });
        Ok(ComputePipeline {
            state: Arc::new(ComputeState {
                device: self.clone(),
                label,
                shader: descriptor.shader.clone(),
                workgroup_size: descriptor.workgroup_size,
                outputs,
                reads,
                binding_layout: binding_layout.cloned(),
            }),
        })
    }
}
impl ComputePipeline {
    pub fn binding_layout(&self) -> Option<&crate::BindGroupLayout> {
        self.state.binding_layout.as_ref()
    }
    pub fn label(&self) -> &str {
        &self.state.label
    }
    pub fn shader(&self) -> &Shader {
        &self.state.shader
    }
    pub fn workgroup_size(&self) -> [u32; 3] {
        self.state.workgroup_size
    }
    pub fn output_bindings(&self) -> &[u32] {
        &self.state.outputs
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
    pub(crate) fn reads(&self, binding: u32) -> bool {
        self.state.reads & (1 << binding) != 0
    }
    pub(crate) fn writes(&self, binding: u32) -> bool {
        self.output_bindings().contains(&binding)
    }
    pub(crate) fn uses(&self, binding: u32) -> bool {
        self.state.reads & (1 << binding) != 0 || self.writes(binding)
    }
}

/// Borrowed storage view. Offsets and sizes are bytes and multiples of 16.
#[derive(Debug, Clone, Copy)]
pub struct StorageBufferBinding<'a> {
    pub binding: u32,
    pub buffer: &'a Buffer,
    pub offset: u64,
    pub size: u64,
}

/// Recorded storage view retaining the live allocation.
#[derive(Debug, Clone)]
pub struct ComputeBufferBinding {
    binding: u32,
    buffer: Buffer,
    offset: u64,
    size: u64,
}
impl ComputeBufferBinding {
    pub fn binding(&self) -> u32 {
        self.binding
    }
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// A validated dispatch description. Recording does not evaluate its shader.
#[derive(Debug)]
pub struct ComputeDispatch {
    pipeline: ComputePipeline,
    workgroups: [u32; 3],
    dimensions: [u32; 3],
    invocation_count: u32,
    bindings: Vec<ComputeBufferBinding>,
    bind_group: Option<crate::BindGroup>,
}
impl ComputeDispatch {
    pub fn bind_group(&self) -> Option<&crate::BindGroup> {
        self.bind_group.as_ref()
    }
    pub fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }
    pub fn workgroups(&self) -> [u32; 3] {
        self.workgroups
    }
    pub fn invocation_count(&self) -> u32 {
        self.invocation_count
    }
    pub fn bindings(&self) -> &[ComputeBufferBinding] {
        &self.bindings
    }
}

/// A rejected request appends no command and changes no resource.
#[derive(Debug)]
pub enum ComputeEncodeError {
    Bindings {
        source: crate::BindingValidationError,
    },
    ForeignPipeline {
        label: String,
    },
    InvalidWorkgroups {
        counts: [u32; 3],
    },
    TooManyInvocations {
        workgroups: [u32; 3],
        workgroup_size: [u32; 3],
    },
    InvalidBinding {
        binding: u32,
    },
    DuplicateBinding {
        binding: u32,
    },
    ForeignBuffer {
        binding: u32,
        label: String,
    },
    MissingStorageUsage {
        binding: u32,
        label: String,
    },
    UnalignedRange {
        binding: u32,
        offset: u64,
        size: u64,
    },
    OutOfBounds {
        binding: u32,
        offset: u64,
        size: u64,
        buffer_size: u64,
    },
    MissingBinding {
        binding: u32,
    },
    InsufficientElements {
        binding: u32,
        required_bytes: u64,
        size: u64,
    },
    OverlappingBindings {
        first: u32,
        second: u32,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}
impl fmt::Display for ComputeEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bindings { source } => write!(f, "compute group: {source}"),
            Self::ForeignPipeline { label } => write!(f, "compute pipeline {label:?} belongs to another device"),
            Self::InvalidWorkgroups { counts } => write!(f, "workgroup counts {counts:?} exceed 65535 per axis"),
            Self::TooManyInvocations { workgroups, workgroup_size } => write!(f, "dispatch {workgroups:?} with local size {workgroup_size:?} exceeds 1048576 invocations"),
            Self::InvalidBinding { binding } => write!(f, "storage slot {binding} is outside 0..8"),
            Self::DuplicateBinding { binding } => write!(f, "storage slot {binding} occurs more than once"),
            Self::ForeignBuffer { binding, label } => write!(f, "storage slot {binding} buffer {label:?} belongs to another device"),
            Self::MissingStorageUsage { binding, label } => write!(f, "storage slot {binding} buffer {label:?} needs Storage usage"),
            Self::UnalignedRange { binding, offset, size } => write!(f, "storage slot {binding} offset {offset} and size {size} must be multiples of 16"),
            Self::OutOfBounds { binding, offset, size, buffer_size } => write!(f, "storage slot {binding}: {size} bytes at {offset} exceed buffer size {buffer_size}"),
            Self::MissingBinding { binding } => write!(f, "compute shader needs storage slot {binding}"),
            Self::InsufficientElements { binding, required_bytes, size } => write!(f, "storage slot {binding} needs {required_bytes} bytes, bound range has {size}"),
            Self::OverlappingBindings { first, second } => write!(f, "storage slots {first} and {second} overlap with an output; use disjoint ranges or one slot for in-place work"),
            Self::AllocationFailed { source } => write!(f, "cannot reserve recorded compute work: {source}"),
        }
    }
}
impl Error for ComputeEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bindings { source } => Some(source),
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

impl CommandEncoder {
    /// Record independent float4 invocations, retaining live pipeline/buffers.
    pub fn dispatch(
        &mut self,
        pipeline: &ComputePipeline,
        workgroups: [u32; 3],
        bindings: &[StorageBufferBinding<'_>],
    ) -> Result<(), ComputeEncodeError> {
        self.record_dispatch(pipeline, workgroups, bindings, None)
    }
    pub fn dispatch_with_bind_group(
        &mut self,
        pipeline: &ComputePipeline,
        workgroups: [u32; 3],
        group: &crate::BindGroup,
    ) -> Result<(), ComputeEncodeError> {
        use crate::{BindingResource, BindingValidationError as B};
        if pipeline.binding_layout().is_none() {
            return Err(ComputeEncodeError::Bindings {
                source: B::LayoutRequired,
            });
        }
        crate::bindings::check_group(pipeline.binding_layout(), Some(group))
            .map_err(|source| ComputeEncodeError::Bindings { source })?;
        let mut views = Vec::new();
        views
            .try_reserve_exact(8)
            .map_err(|source| ComputeEncodeError::AllocationFailed { source })?;
        for entry in group.entries() {
            if let BindingResource::StorageBuffer(view) = &entry.resource {
                views.push(StorageBufferBinding {
                    binding: entry.binding,
                    buffer: &view.buffer,
                    offset: view.offset,
                    size: view.size,
                });
            }
        }
        for entry in group.entries() {
            if let BindingResource::UniformBuffer(uniform) = &entry.resource {
                for storage in views.iter().filter(|b| pipeline.writes(b.binding)) {
                    if uniform.buffer.is_same(storage.buffer)
                        && uniform.offset < storage.offset + storage.size
                        && storage.offset < uniform.offset + uniform.size
                    {
                        return Err(ComputeEncodeError::Bindings {
                            source: B::OverlappingUniformStorage {
                                uniform: entry.binding,
                                storage: storage.binding,
                            },
                        });
                    }
                }
            }
        }
        self.record_dispatch(pipeline, workgroups, &views, Some(group))
    }
    fn record_dispatch(
        &mut self,
        pipeline: &ComputePipeline,
        workgroups: [u32; 3],
        bindings: &[StorageBufferBinding<'_>],
        group: Option<&crate::BindGroup>,
    ) -> Result<(), ComputeEncodeError> {
        use ComputeEncodeError as E;
        crate::bindings::check_group(pipeline.binding_layout(), group)
            .map_err(|source| E::Bindings { source })?;
        if !pipeline.belongs_to(&self.device) {
            return Err(E::ForeignPipeline {
                label: pipeline.label().to_owned(),
            });
        }
        if workgroups
            .iter()
            .any(|&count| count > crate::profile::PREVIEW_LIMITS.max_workgroup_count)
        {
            return Err(E::InvalidWorkgroups { counts: workgroups });
        }
        let dimensions =
            std::array::from_fn(|axis| workgroups[axis] * pipeline.workgroup_size()[axis]);
        // Bound axes make each multiplication above safe. A zero axis must
        // dominate before multiplying other dimensions, which may overflow.
        let count = if dimensions.contains(&0) {
            Some(0)
        } else {
            product(dimensions)
        };
        let invocation_count = count
            .filter(|&count| count <= crate::profile::PREVIEW_LIMITS.max_compute_invocations)
            .ok_or(E::TooManyInvocations {
                workgroups,
                workgroup_size: pipeline.workgroup_size(),
            })?;
        let mut seen = 0_u8;
        for binding in bindings {
            let slot = binding.binding;
            if slot >= crate::profile::PREVIEW_LIMITS.binding_slots {
                return Err(E::InvalidBinding { binding: slot });
            }
            if seen & (1 << slot) != 0 {
                return Err(E::DuplicateBinding { binding: slot });
            }
            seen |= 1 << slot;
            if !binding.buffer.belongs_to(&self.device) {
                return Err(E::ForeignBuffer {
                    binding: slot,
                    label: binding.buffer.label().to_owned(),
                });
            }
            if !binding.buffer.usages().contains(&BufferUsage::Storage) {
                return Err(E::MissingStorageUsage {
                    binding: slot,
                    label: binding.buffer.label().to_owned(),
                });
            }
            if !binding.offset.is_multiple_of(16) || !binding.size.is_multiple_of(16) {
                return Err(E::UnalignedRange {
                    binding: slot,
                    offset: binding.offset,
                    size: binding.size,
                });
            }
            if binding
                .offset
                .checked_add(binding.size)
                .is_none_or(|end| end > binding.buffer.size())
            {
                return Err(E::OutOfBounds {
                    binding: slot,
                    offset: binding.offset,
                    size: binding.size,
                    buffer_size: binding.buffer.size(),
                });
            }
        }
        for slot in 0..8 {
            if !pipeline.uses(slot) {
                continue;
            }
            let binding = bindings
                .iter()
                .find(|b| b.binding == slot)
                .ok_or(E::MissingBinding { binding: slot })?;
            let required_bytes = u64::from(invocation_count) * 16;
            if binding.size < required_bytes {
                return Err(E::InsufficientElements {
                    binding: slot,
                    required_bytes,
                    size: binding.size,
                });
            }
        }
        for (index, first) in bindings.iter().enumerate() {
            for second in &bindings[index + 1..] {
                if (pipeline.writes(first.binding) || pipeline.writes(second.binding))
                    && first.buffer.is_same(second.buffer)
                    && first.size != 0
                    && second.size != 0
                    && first.offset < second.offset + second.size
                    && second.offset < first.offset + first.size
                {
                    return Err(E::OverlappingBindings {
                        first: first.binding,
                        second: second.binding,
                    });
                }
            }
        }
        let mut retained = Vec::new();
        retained
            .try_reserve_exact(bindings.len())
            .map_err(|source| E::AllocationFailed { source })?;
        for binding in bindings {
            retained.push(ComputeBufferBinding {
                binding: binding.binding,
                buffer: binding.buffer.clone(),
                offset: binding.offset,
                size: binding.size,
            });
        }
        self.commands
            .try_reserve(1)
            .map_err(|source| E::AllocationFailed { source })?;
        self.commands.push(Command::Dispatch(ComputeDispatch {
            pipeline: pipeline.clone(),
            workgroups,
            dimensions,
            invocation_count,
            bindings: retained,
            bind_group: group.cloned(),
        }));
        Ok(())
    }
}

fn product(values: [u32; 3]) -> Option<u32> {
    values.into_iter().try_fold(1_u32, u32::checked_mul)
}

/// Failed dispatch commits no output. Shader messages retain invocation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeExecutionError {
    StoragePoisoned {
        binding: u32,
        label: String,
    },
    AllocationFailed,
    ShaderFailed {
        invocation: [u32; 3],
        message: String,
    },
}
impl fmt::Display for ComputeExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoragePoisoned { binding, label } => write!(
                f,
                "compute slot {binding} buffer {label:?} storage is poisoned"
            ),
            Self::AllocationFailed => f.write_str("cannot reserve staged compute outputs"),
            Self::ShaderFailed {
                invocation,
                message,
            } => write!(f, "compute invocation {invocation:?} failed: {message}"),
        }
    }
}
impl Error for ComputeExecutionError {}

/// Caller holds the device execution gate. Lock each used allocation only once.
pub(crate) fn execute(dispatch: &ComputeDispatch) -> Result<(), ComputeExecutionError> {
    if dispatch.invocation_count == 0 {
        return Ok(());
    }
    let pipeline = &dispatch.pipeline;
    let uniforms = if let Some(group) = dispatch.bind_group() {
        group
            .snapshot_uniforms(pipeline.shader().required_uniforms())
            .map_err(|error| match error {
                crate::bindings::SnapshotError::Allocation => {
                    ComputeExecutionError::AllocationFailed
                }
                crate::bindings::SnapshotError::Poison { binding, label } => {
                    ComputeExecutionError::StoragePoisoned { binding, label }
                }
            })?
    } else {
        Vec::new()
    };
    // At most eight unique allocations; fixed arrays avoid allocation for locks.
    let mut buffers: [Option<&ComputeBufferBinding>; 8] = [None; 8];
    let mut slot_indices = [0_usize; 8];
    let mut unique = 0;
    for binding in dispatch
        .bindings
        .iter()
        .filter(|b| pipeline.uses(b.binding))
    {
        let index = (0..unique)
            .find(|&i| buffers[i].is_some_and(|b| b.buffer.is_same(&binding.buffer)))
            .unwrap_or(unique);
        if index == unique {
            buffers[index] = Some(binding);
            unique += 1;
        }
        slot_indices[binding.binding as usize] = index;
    }
    let mut locks = [const { None }; 8];
    for (index, binding) in buffers.iter().enumerate() {
        if let Some(binding) = binding {
            locks[index] = Some(binding.buffer.lock_bytes().map_err(|_| {
                ComputeExecutionError::StoragePoisoned {
                    binding: binding.binding,
                    label: binding.buffer.label().to_owned(),
                }
            })?);
        }
    }
    let mut views = [None; 8];
    for binding in &dispatch.bindings {
        views[binding.binding as usize] = Some(binding);
    }
    let mut staged: [Vec<u8>; 8] = std::array::from_fn(|_| Vec::new());
    let byte_count = dispatch.invocation_count as usize * 16;
    for &slot in pipeline.output_bindings() {
        staged[slot as usize]
            .try_reserve_exact(byte_count)
            .map_err(|_| ComputeExecutionError::AllocationFailed)?;
    }
    for linear in 0..dispatch.invocation_count {
        let global = coordinates(linear, dispatch.dimensions);
        let local = std::array::from_fn(|axis| global[axis] % pipeline.workgroup_size()[axis]);
        let group = std::array::from_fn(|axis| global[axis] / pipeline.workgroup_size()[axis]);
        let inputs = [
            vector(global, linear),
            vector(local, flatten(local, pipeline.workgroup_size())),
            vector(group, flatten(group, dispatch.workgroups)),
            vector(dispatch.workgroups, 0),
        ];
        let mut outputs = [[0.0; 4]; 8];
        pipeline
            .shader()
            .evaluate_bound(
                &inputs,
                &mut outputs,
                |binding, _| Err(crate::ShaderExecutionError::MissingTextureBinding { binding }),
                |binding| {
                    let slot = binding as usize;
                    // The immutable recorded description proves presence and bounds.
                    let view = views[slot]
                        .ok_or(crate::ShaderExecutionError::MissingStorageBinding { binding })?;
                    let bytes = locks[slot_indices[slot]]
                        .as_ref()
                        .ok_or(crate::ShaderExecutionError::MissingStorageBinding { binding })?;
                    let start = view.offset as usize + linear as usize * 16;
                    Ok(std::array::from_fn(|lane| {
                        let start = start + lane * 4;
                        f32::from_le_bytes([
                            bytes[start],
                            bytes[start + 1],
                            bytes[start + 2],
                            bytes[start + 3],
                        ])
                    }))
                },
                |binding, element| crate::bindings::load_uniform(&uniforms, binding, element),
            )
            .map_err(|error| ComputeExecutionError::ShaderFailed {
                invocation: global,
                message: error.to_string(),
            })?;
        for (output, &slot) in outputs.iter().zip(pipeline.output_bindings()) {
            for lane in output {
                staged[slot as usize].extend_from_slice(&lane.to_le_bytes());
            }
        }
    }
    // All locks and staging exist; validated disjoint output ranges allow an
    // infallible commit. No error may occur after the first byte is committed.
    for binding in &dispatch.bindings {
        if pipeline.writes(binding.binding) {
            if let Some(bytes) = locks[slot_indices[binding.binding as usize]].as_mut() {
                let start = binding.offset as usize;
                bytes[start..start + byte_count].copy_from_slice(&staged[binding.binding as usize]);
            }
        }
    }
    Ok(())
}

fn coordinates(linear: u32, dimensions: [u32; 3]) -> [u32; 3] {
    [
        linear % dimensions[0],
        (linear / dimensions[0]) % dimensions[1],
        linear / dimensions[0] / dimensions[1],
    ]
}
fn flatten(id: [u32; 3], dimensions: [u32; 3]) -> u32 {
    id[0] + dimensions[0] * (id[1] + dimensions[1] * id[2])
}
fn vector(id: [u32; 3], linear: u32) -> [f32; 4] {
    [id[0] as f32, id[1] as f32, id[2] as f32, linear as f32]
}
