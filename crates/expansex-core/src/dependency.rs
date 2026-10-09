//! Whole-resource access roles and checked explicit command-buffer dependencies.
use crate::{
    Buffer, BufferUsage, Command, CommandBuffer, CommandEncoder, DepthBuffer, Device, Framebuffer,
    StencilBuffer, Texture,
};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;

/// Synchronization contract selected when creating an encoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SynchronizationMode {
    /// Commands complete in order; writes are visible to following commands.
    #[default]
    Ordered,
    /// Declare resource roles and resolve cross-command write hazards explicitly.
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Buffer,
    Texture,
    Framebuffer,
    DepthBuffer,
    StencilBuffer,
}

/// Retained mutable storage identity. Equal labels do not imply equal resources.
#[derive(Debug, Clone)]
pub enum Resource {
    Buffer(Buffer),
    Texture(Texture),
    Framebuffer(Framebuffer),
    DepthBuffer(DepthBuffer),
    StencilBuffer(StencilBuffer),
}
impl Resource {
    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::Buffer(_) => ResourceKind::Buffer,
            Self::Texture(_) => ResourceKind::Texture,
            Self::Framebuffer(_) => ResourceKind::Framebuffer,
            Self::DepthBuffer(_) => ResourceKind::DepthBuffer,
            Self::StencilBuffer(_) => ResourceKind::StencilBuffer,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Buffer(r) => r.label(),
            Self::Texture(r) => r.label(),
            Self::Framebuffer(r) => r.label(),
            Self::DepthBuffer(r) => r.label(),
            Self::StencilBuffer(r) => r.label(),
        }
    }
    pub fn device(&self) -> Device {
        match self {
            Self::Buffer(r) => r.device(),
            Self::Texture(r) => r.device(),
            Self::Framebuffer(r) => r.device(),
            Self::DepthBuffer(r) => r.device(),
            Self::StencilBuffer(r) => r.device(),
        }
    }
    pub fn belongs_to(&self, device: &Device) -> bool {
        match self {
            Self::Buffer(r) => r.belongs_to(device),
            Self::Texture(r) => r.belongs_to(device),
            Self::Framebuffer(r) => r.belongs_to(device),
            Self::DepthBuffer(r) => r.belongs_to(device),
            Self::StencilBuffer(r) => r.belongs_to(device),
        }
    }
    pub fn is_same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Buffer(a), Self::Buffer(b)) => a.is_same(b),
            (Self::Texture(a), Self::Texture(b)) => a.is_same(b),
            (Self::Framebuffer(a), Self::Framebuffer(b)) => a.is_same(b),
            (Self::DepthBuffer(a), Self::DepthBuffer(b)) => a.is_same(b),
            (Self::StencilBuffer(a), Self::StencilBuffer(b)) => a.is_same(b),
            _ => false,
        }
    }
    fn permits(&self, access: ResourceAccess) -> bool {
        use ResourceAccess as A;
        match self {
            Self::Buffer(buffer) => {
                let has = |role| buffer.usages().contains(&role);
                match access {
                    A::TransferRead => has(BufferUsage::TransferSource),
                    A::TransferWrite => has(BufferUsage::TransferDestination),
                    A::TransferReadWrite => {
                        has(BufferUsage::TransferSource) && has(BufferUsage::TransferDestination)
                    }
                    A::VertexRead => has(BufferUsage::Vertex),
                    A::IndexRead => has(BufferUsage::Index),
                    A::UniformRead => has(BufferUsage::Uniform),
                    A::StorageRead | A::StorageWrite | A::StorageReadWrite => {
                        has(BufferUsage::Storage)
                    }
                    A::BufferRead => [
                        BufferUsage::TransferSource,
                        BufferUsage::Vertex,
                        BufferUsage::Index,
                        BufferUsage::Storage,
                        BufferUsage::Uniform,
                    ]
                    .into_iter()
                    .any(has),
                    _ => false,
                }
            }
            Self::Texture(_) => access == A::SampledRead,
            Self::Framebuffer(_) => access == A::ColourReadWrite,
            Self::DepthBuffer(_) => access == A::DepthReadWrite,
            Self::StencilBuffer(_) => access == A::StencilReadWrite,
        }
    }
}

/// Whole-resource access role, independent of CPU HostAccess permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAccess {
    TransferRead,
    TransferWrite,
    TransferReadWrite,
    VertexRead,
    IndexRead,
    UniformRead,
    StorageRead,
    StorageWrite,
    StorageReadWrite,
    /// Any supported read-only buffer role, including combined vertex/index use.
    BufferRead,
    SampledRead,
    ColourReadWrite,
    DepthReadWrite,
    StencilReadWrite,
}
impl ResourceAccess {
    pub fn is_read(&self) -> bool {
        !matches!(self, Self::TransferWrite | Self::StorageWrite)
    }
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Self::TransferWrite
                | Self::TransferReadWrite
                | Self::StorageWrite
                | Self::StorageReadWrite
                | Self::ColourReadWrite
                | Self::DepthReadWrite
                | Self::StencilReadWrite
        )
    }
    fn allows(self, required: Self) -> bool {
        self == required
            || match self {
                Self::BufferRead => matches!(
                    required,
                    Self::TransferRead
                        | Self::VertexRead
                        | Self::IndexRead
                        | Self::StorageRead
                        | Self::UniformRead
                ),
                Self::StorageReadWrite => {
                    matches!(required, Self::StorageRead | Self::StorageWrite)
                }
                Self::TransferReadWrite => {
                    matches!(required, Self::TransferRead | Self::TransferWrite)
                }
                _ => false,
            }
    }
}

/// One dependency point and access-role change; None introduces local state.
#[derive(Debug, Clone)]
pub struct ResourceTransition {
    pub resource: Resource,
    pub before: Option<ResourceAccess>,
    pub after: ResourceAccess,
}

/// Immutable recorded transition batch retaining each referenced allocation.
#[derive(Debug)]
pub struct ResourceTransitions {
    transitions: Vec<ResourceTransition>,
}
impl ResourceTransitions {
    pub fn transitions(&self) -> &[ResourceTransition] {
        &self.transitions
    }
}

#[derive(Debug)]
pub enum DependencyEncodeError {
    ExplicitModeRequired,
    ForeignResource {
        kind: ResourceKind,
        label: String,
    },
    InvalidAccess {
        kind: ResourceKind,
        label: String,
        access: ResourceAccess,
    },
    DuplicateResource {
        kind: ResourceKind,
        label: String,
    },
    AllocationFailed {
        source: TryReserveError,
    },
}
impl fmt::Display for DependencyEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitModeRequired => f.write_str("resource transitions require an Explicit encoder"),
            Self::ForeignResource { kind, label } => write!(f, "transition {kind:?} {label:?} belongs to another device"),
            Self::InvalidAccess { kind, label, access } => write!(f, "transition {kind:?} {label:?} cannot use {access:?}; check resource kind and buffer usages"),
            Self::DuplicateResource { kind, label } => write!(f, "transition batch repeats {kind:?} {label:?}; record separate dependency points"),
            Self::AllocationFailed { source } => write!(f, "cannot reserve recorded resource transitions: {source}"),
        }
    }
}
impl Error for DependencyEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AllocationFailed { source } => Some(source),
            _ => None,
        }
    }
}

impl CommandEncoder {
    /// Record a full dependency for each resource, preserving contents.
    /// State and hazards are checked by validate_dependencies/submit after finish.
    ///
    /// ```
    /// use expansex_core::*;
    /// let d = Instance::new().default_adapter().create_device();
    /// let b = d.create_buffer(BufferDescriptor {
    ///     label: "transfer", size: 8,
    ///     usages: &[BufferUsage::TransferSource, BufferUsage::TransferDestination],
    ///     host_access: HostAccess::ReadWrite,
    /// })?;
    /// b.write(0, &[1, 2, 3, 4])?;
    /// let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    /// e.transition_resources(&[ResourceTransition {
    ///     resource: Resource::Buffer(b.clone()), before: None,
    ///     after: ResourceAccess::TransferReadWrite,
    /// }])?;
    /// e.copy_buffer(&b, 0, &b, 4, 4)?;
    /// let commands = e.finish();
    /// commands.validate_dependencies()?;
    /// d.default_queue().submit(&commands)?;
    /// let mut bytes = [0; 4]; b.read(4, &mut bytes)?;
    /// assert_eq!(bytes, [1, 2, 3, 4]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn transition_resources(
        &mut self,
        transitions: &[ResourceTransition],
    ) -> Result<(), DependencyEncodeError> {
        if self.synchronization_mode() != SynchronizationMode::Explicit {
            return Err(DependencyEncodeError::ExplicitModeRequired);
        }
        for (index, transition) in transitions.iter().enumerate() {
            let resource = &transition.resource;
            if !resource.belongs_to(&self.device) {
                return Err(DependencyEncodeError::ForeignResource {
                    kind: resource.kind(),
                    label: resource.label().to_owned(),
                });
            }
            for access in transition.before.into_iter().chain([transition.after]) {
                if !resource.permits(access) {
                    return Err(DependencyEncodeError::InvalidAccess {
                        kind: resource.kind(),
                        label: resource.label().to_owned(),
                        access,
                    });
                }
            }
            if transitions[..index]
                .iter()
                .any(|prior| resource.is_same(&prior.resource))
            {
                return Err(DependencyEncodeError::DuplicateResource {
                    kind: resource.kind(),
                    label: resource.label().to_owned(),
                });
            }
        }
        if transitions.is_empty() {
            return Ok(());
        }
        let mut retained = Vec::new();
        retained
            .try_reserve_exact(transitions.len())
            .map_err(|source| DependencyEncodeError::AllocationFailed { source })?;
        retained.extend_from_slice(transitions);
        self.commands
            .try_reserve(1)
            .map_err(|source| DependencyEncodeError::AllocationFailed { source })?;
        self.commands
            .push(Command::ResourceTransitions(ResourceTransitions {
                transitions: retained,
            }));
        Ok(())
    }
}

/// Actual combined access to one allocation within one command.
#[derive(Debug, Clone)]
pub struct ResourceUse {
    pub command_index: usize,
    pub resource: Resource,
    pub access: ResourceAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    ReadAfterWrite,
    WriteAfterRead,
    WriteAfterWrite,
}

/// Full preflight errors: no command in the submission has executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyValidationError {
    AllocationFailed,
    MissingTransition {
        command_index: usize,
        kind: ResourceKind,
        label: String,
        required: ResourceAccess,
    },
    AccessMismatch {
        command_index: usize,
        kind: ResourceKind,
        label: String,
        declared: ResourceAccess,
        required: ResourceAccess,
    },
    TransitionMismatch {
        command_index: usize,
        kind: ResourceKind,
        label: String,
        before: Option<ResourceAccess>,
        actual: Option<ResourceAccess>,
    },
    MissingDependency {
        command_index: usize,
        previous_command_index: usize,
        kind: ResourceKind,
        label: String,
        hazard: HazardKind,
        previous: ResourceAccess,
        required: ResourceAccess,
    },
}
impl fmt::Display for DependencyValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => f.write_str("cannot reserve dependency analysis state or resource uses"),
            Self::MissingTransition { command_index, kind, label, required } => write!(f, "command {command_index} uses undeclared {kind:?} {label:?} as {required:?}; add a transition from None before first use"),
            Self::AccessMismatch { command_index, kind, label, declared, required } => write!(f, "command {command_index} needs {required:?} for {kind:?} {label:?}, declared {declared:?}; transition from the declared role"),
            Self::TransitionMismatch { command_index, kind, label, before, actual } => write!(f, "transition command {command_index} expects {kind:?} {label:?} in {before:?}, actual local state is {actual:?}"),
            Self::MissingDependency { command_index, previous_command_index, kind, label, hazard, previous, required } => write!(f, "{hazard:?} on {kind:?} {label:?}: command {previous_command_index} uses {previous:?}, command {command_index} needs {required:?}; add a resource transition between them"),
        }
    }
}
impl Error for DependencyValidationError {}

impl CommandBuffer {
    /// Inspect combined whole-resource uses without reading bytes or checking poison.
    /// Returned resource handles retain storage independently of this command buffer.
    pub fn resource_uses(&self) -> Result<Vec<ResourceUse>, DependencyValidationError> {
        use ResourceAccess as A;
        let mut uses = Vec::new();
        for (command_index, command) in self.commands().iter().enumerate() {
            let mut add = |resource, access| add_use(&mut uses, command_index, resource, access);
            match command {
                Command::ResourceTransitions(_) => {}
                Command::CopyBuffer(copy) if copy.size() != 0 => {
                    add(Resource::Buffer(copy.source().clone()), A::TransferRead)?;
                    add(
                        Resource::Buffer(copy.destination().clone()),
                        A::TransferWrite,
                    )?;
                }
                Command::Dispatch(dispatch) if dispatch.invocation_count() != 0 => {
                    let pipeline = dispatch.pipeline();
                    for binding in dispatch.bindings() {
                        let access = match (
                            pipeline.reads(binding.binding()),
                            pipeline.writes(binding.binding()),
                        ) {
                            (true, true) => A::StorageReadWrite,
                            (true, false) => A::StorageRead,
                            (false, true) => A::StorageWrite,
                            (false, false) => continue,
                        };
                        add(Resource::Buffer(binding.buffer().clone()), access)?;
                    }
                    if let Some(group) = dispatch.bind_group() {
                        add_uniform_uses(&mut add, group, pipeline.shader().required_uniforms())?;
                    }
                }
                Command::RenderPass(pass) => {
                    add(
                        Resource::Framebuffer(pass.target().clone()),
                        A::ColourReadWrite,
                    )?;
                    if let Some(depth) = pass.depth_buffer() {
                        add(Resource::DepthBuffer(depth.clone()), A::DepthReadWrite)?;
                    }
                    if let Some(stencil) = pass.stencil_buffer() {
                        add(
                            Resource::StencilBuffer(stencil.clone()),
                            A::StencilReadWrite,
                        )?;
                    }
                    for draw in pass.draws() {
                        let empty = draw.indexed().map_or_else(
                            || draw.vertices().is_empty(),
                            |indexed| indexed.indices().is_empty(),
                        );
                        if empty {
                            continue;
                        }
                        add(
                            Resource::Buffer(draw.vertex_buffer().clone()),
                            A::VertexRead,
                        )?;
                        if let Some(indexed) = draw.indexed() {
                            add(Resource::Buffer(indexed.buffer().clone()), A::IndexRead)?;
                        }
                        if let (Some(group), Some(pipeline)) = (draw.bind_group(), draw.pipeline())
                        {
                            add_uniform_uses(
                                &mut add,
                                group,
                                pipeline.vertex_shader().required_uniforms()
                                    | pipeline.fragment_shader().required_uniforms(),
                            )?;
                        }
                        for slot in 0..8 {
                            if let Some(binding) = draw.texture(slot) {
                                add(
                                    Resource::Texture(binding.view().texture().clone()),
                                    A::SampledRead,
                                )?;
                            }
                        }
                    }
                }
                Command::CopyBuffer(_) | Command::Dispatch(_) => {}
            }
        }
        Ok(uses)
    }

    /// Check the explicit plan before execution; Ordered needs no declarations.
    /// Local transition state is rebuilt on every call and reusable submission.
    pub fn validate_dependencies(&self) -> Result<(), DependencyValidationError> {
        if self.synchronization_mode() == SynchronizationMode::Ordered {
            return Ok(());
        }
        let uses = self.resource_uses()?;
        let mut next_use = 0;
        let mut states: Vec<State> = Vec::new();
        for (command_index, command) in self.commands().iter().enumerate() {
            if let Command::ResourceTransitions(batch) = command {
                for transition in batch.transitions() {
                    let position = states
                        .iter()
                        .position(|state| state.resource.is_same(&transition.resource));
                    let actual = position.map(|index| states[index].access);
                    if actual != transition.before {
                        return Err(DependencyValidationError::TransitionMismatch {
                            command_index,
                            kind: transition.resource.kind(),
                            label: transition.resource.label().to_owned(),
                            before: transition.before,
                            actual,
                        });
                    }
                    if let Some(index) = position {
                        states[index].access = transition.after;
                        states[index].last = None;
                    } else {
                        states
                            .try_reserve(1)
                            .map_err(|_| DependencyValidationError::AllocationFailed)?;
                        states.push(State {
                            resource: transition.resource.clone(),
                            access: transition.after,
                            last: None,
                        });
                    }
                }
            }
            while let Some(usage) = uses
                .get(next_use)
                .filter(|usage| usage.command_index == command_index)
            {
                let state = states
                    .iter_mut()
                    .find(|state| state.resource.is_same(&usage.resource))
                    .ok_or_else(|| DependencyValidationError::MissingTransition {
                        command_index,
                        kind: usage.resource.kind(),
                        label: usage.resource.label().to_owned(),
                        required: usage.access,
                    })?;
                if let Some((previous_command_index, previous)) = state.last {
                    let hazard = match (previous.is_write(), usage.access.is_write()) {
                        (true, true) => Some(HazardKind::WriteAfterWrite),
                        (true, false) => Some(HazardKind::ReadAfterWrite),
                        (false, true) => Some(HazardKind::WriteAfterRead),
                        (false, false) => None,
                    };
                    if let Some(hazard) = hazard {
                        return Err(DependencyValidationError::MissingDependency {
                            command_index,
                            previous_command_index,
                            kind: usage.resource.kind(),
                            label: usage.resource.label().to_owned(),
                            hazard,
                            previous,
                            required: usage.access,
                        });
                    }
                }
                if !state.access.allows(usage.access) {
                    return Err(DependencyValidationError::AccessMismatch {
                        command_index,
                        kind: usage.resource.kind(),
                        label: usage.resource.label().to_owned(),
                        declared: state.access,
                        required: usage.access,
                    });
                }
                state.last = Some((command_index, usage.access));
                next_use += 1;
            }
        }
        Ok(())
    }
}

struct State {
    resource: Resource,
    access: ResourceAccess,
    last: Option<(usize, ResourceAccess)>,
}

fn add_use(
    uses: &mut Vec<ResourceUse>,
    command_index: usize,
    resource: Resource,
    access: ResourceAccess,
) -> Result<(), DependencyValidationError> {
    if let Some(previous) = uses
        .iter_mut()
        .rev()
        .take_while(|usage| usage.command_index == command_index)
        .find(|usage| usage.resource.is_same(&resource))
    {
        if previous.access != access {
            // Copies combine transfer roles. Draws combine read-only sources.
            // Compute combines storage and uniform reads; a writer makes its
            // whole-allocation summary StorageReadWrite even for disjoint views.
            previous.access = if !previous.access.is_write() && !access.is_write() {
                ResourceAccess::BufferRead
            } else if matches!(
                previous.access,
                ResourceAccess::TransferRead
                    | ResourceAccess::TransferWrite
                    | ResourceAccess::TransferReadWrite
            ) {
                ResourceAccess::TransferReadWrite
            } else {
                ResourceAccess::StorageReadWrite
            };
        }
        return Ok(());
    }
    uses.try_reserve(1)
        .map_err(|_| DependencyValidationError::AllocationFailed)?;
    uses.push(ResourceUse {
        command_index,
        resource,
        access,
    });
    Ok(())
}

fn add_uniform_uses(
    add: &mut impl FnMut(Resource, ResourceAccess) -> Result<(), DependencyValidationError>,
    group: &crate::BindGroup,
    mask: u8,
) -> Result<(), DependencyValidationError> {
    for entry in group
        .entries()
        .iter()
        .filter(|e| mask & (1 << e.binding) != 0)
    {
        if let crate::BindingResource::UniformBuffer(view) = &entry.resource {
            add(
                Resource::Buffer(view.buffer.clone()),
                ResourceAccess::UniformRead,
            )?;
        }
    }
    Ok(())
}
