use super::{BufferDescriptor, BufferDescriptorError, BufferUsage, HostAccess};
use crate::{CopyBufferRole, Device};
use std::collections::TryReserveError;
use std::error::Error;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex, MutexGuard};

/// Failure to create reference buffer storage.
#[derive(Debug)]
pub enum BufferCreateError {
    /// The request violates a backend-independent descriptor rule.
    InvalidDescriptor(BufferDescriptorError),
    /// The byte count exceeds this process's contiguous-vector capacity.
    SizeTooLarge { size: u64 },
    /// Reserving bytes or owned metadata failed. Small handle allocations still
    /// use Rust's ordinary allocation-failure behavior.
    AllocationFailed { size: u64, source: TryReserveError },
}

impl fmt::Display for BufferCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDescriptor(source) => write!(f, "cannot create buffer: {source}"),
            Self::SizeTooLarge { size } => write!(
                f,
                "cannot create a {size}-byte buffer: size exceeds this process's contiguous storage limit"
            ),
            Self::AllocationFailed { size, source } => write!(
                f,
                "cannot reserve storage or metadata for a {size}-byte buffer: {source}"
            ),
        }
    }
}

impl Error for BufferCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDescriptor(source) => Some(source),
            Self::AllocationFailed { source, .. } => Some(source),
            Self::SizeTooLarge { .. } => None,
        }
    }
}

/// A rejected CPU access. The error owns its label and can outlive the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferAccessError {
    /// The requested host permissions do not permit reads.
    ReadNotAllowed { label: String, access: HostAccess },
    /// The requested host permissions do not permit writes.
    WriteNotAllowed { label: String, access: HostAccess },
    /// The whole range does not fit, including offset-plus-length overflow.
    OutOfBounds {
        label: String,
        offset: u64,
        length: u64,
        buffer_size: u64,
    },
    /// A panic interrupted an internal storage operation; contents are not trusted.
    StoragePoisoned { label: String },
}

impl fmt::Display for BufferAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadNotAllowed { label, access } => write!(
                f,
                "buffer {label:?} cannot be read by the CPU with {access:?} host access; request Read or ReadWrite"
            ),
            Self::WriteNotAllowed { label, access } => write!(
                f,
                "buffer {label:?} cannot be written by the CPU with {access:?} host access; request Write or ReadWrite"
            ),
            Self::OutOfBounds { label, offset, length, buffer_size } => write!(
                f,
                "buffer {label:?}: {length} bytes at offset {offset} do not fit in {buffer_size} bytes"
            ),
            Self::StoragePoisoned { label } => write!(
                f,
                "buffer {label:?} storage is unavailable after an interrupted internal operation"
            ),
        }
    }
}

impl Error for BufferAccessError {}

struct BufferState {
    device: Device,
    label: String,
    size: u64,
    usages: Vec<BufferUsage>,
    host_access: HostAccess,
    bytes: Mutex<Vec<u8>>,
}

/// Owned, zero-initialized reference buffer storage associated with one device.
///
/// Clones retain the same allocation. CPU operations copy bytes and serialize
/// through the device execution gate and private storage mutex. No mapped slices
/// or hardware GPU work are exposed. The final handle releases storage and device.
#[derive(Clone)]
pub struct Buffer {
    state: Arc<BufferState>,
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("label", &self.label())
            .field("size", &self.size())
            .field("usages", &self.usages())
            .field("host_access", &self.host_access())
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Allocates zero-initialized bytes and copies the descriptor's metadata.
    ///
    /// The reference target uses system memory even for [`HostAccess::None`],
    /// but public CPU access always enforces the requested permissions.
    /// No queue command is recorded or submitted.
    pub fn create_buffer(
        &self,
        descriptor: BufferDescriptor<'_>,
    ) -> Result<Buffer, BufferCreateError> {
        descriptor
            .validate()
            .map_err(BufferCreateError::InvalidDescriptor)?;
        let size = usize::try_from(descriptor.size)
            .ok()
            .filter(|&size| size <= isize::MAX as usize)
            .ok_or(BufferCreateError::SizeTooLarge {
                size: descriptor.size,
            })?;

        let allocation_error = |source| BufferCreateError::AllocationFailed {
            size: descriptor.size,
            source,
        };
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(allocation_error)?;
        bytes.resize(size, 0);

        let mut label = String::new();
        label
            .try_reserve_exact(descriptor.label.len())
            .map_err(allocation_error)?;
        label.push_str(descriptor.label);
        let mut usages = Vec::new();
        usages
            .try_reserve_exact(descriptor.usages.len())
            .map_err(allocation_error)?;
        usages.extend_from_slice(descriptor.usages);

        Ok(Buffer {
            state: Arc::new(BufferState {
                device: self.clone(),
                label,
                size: descriptor.size,
                usages,
                host_access: descriptor.host_access,
                bytes: Mutex::new(bytes),
            }),
        })
    }
}

impl Buffer {
    /// Borrows the owned debug label; an empty label is allowed.
    pub fn label(&self) -> &str {
        &self.state.label
    }

    /// The allocated byte count, not an element count or vector capacity.
    pub fn size(&self) -> u64 {
        self.state.size
    }

    /// Borrows the device-side usage roles, preserving the request's order.
    /// Duplicate roles still have no extra meaning.
    pub fn usages(&self) -> &[BufferUsage] {
        &self.state.usages
    }

    /// The requested permissions for explicit CPU copies.
    pub fn host_access(&self) -> HostAccess {
        self.state.host_access
    }

    /// Retains the original creating device, rather than creating a new one.
    pub fn device(&self) -> Device {
        self.state.device.clone()
    }

    /// Checks the logical ownership domain, independent of adapter metadata.
    pub fn belongs_to(&self, device: &Device) -> bool {
        self.state.device.is_same(device)
    }

    /// Checks whether two handles retain the same allocation.
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Copies CPU bytes into the buffer at a byte offset.
    ///
    /// Requires Write or ReadWrite host access, independent of usage roles.
    /// Permission is checked before bounds, including for empty copies.
    /// A failed operation leaves the buffer unchanged. This call is synchronous.
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<(), BufferAccessError> {
        if !matches!(
            self.host_access(),
            HostAccess::Write | HostAccess::ReadWrite
        ) {
            return Err(BufferAccessError::WriteNotAllowed {
                label: self.label().to_owned(),
                access: self.host_access(),
            });
        }
        let range = self.checked_range(offset, data.len())?;
        let _execution = self.state.device.lock_execution();
        let mut bytes = self.state.bytes.lock().map_err(|_| self.poisoned_error())?;
        bytes[range].copy_from_slice(data);
        Ok(())
    }

    /// Copies buffer bytes into caller-owned CPU storage.
    ///
    /// Requires Read or ReadWrite host access. A failed operation leaves the
    /// destination unchanged. This copies synchronously and exposes no mapping.
    pub fn read(&self, offset: u64, destination: &mut [u8]) -> Result<(), BufferAccessError> {
        if !matches!(self.host_access(), HostAccess::Read | HostAccess::ReadWrite) {
            return Err(BufferAccessError::ReadNotAllowed {
                label: self.label().to_owned(),
                access: self.host_access(),
            });
        }
        let range = self.checked_range(offset, destination.len())?;
        let _execution = self.state.device.lock_execution();
        let bytes = self.state.bytes.lock().map_err(|_| self.poisoned_error())?;
        destination.copy_from_slice(&bytes[range]);
        Ok(())
    }

    /// Reference executor only: caller holds this device's execution gate, and
    /// uses immutable ranges/roles validated by CommandEncoder. Do not call the
    /// public read/write methods here: they acquire the same non-reentrant gate.
    pub(crate) fn copy_to(
        &self,
        source_offset: u64,
        destination: &Self,
        destination_offset: u64,
        size: u64,
    ) -> Result<(), CopyBufferRole> {
        if size == 0 {
            return Ok(());
        }
        // Creation bounds size by isize::MAX; recording checked addition/ranges.
        // Storage never resizes and descriptions cannot be publicly fabricated.
        let source_range = source_offset as usize..(source_offset + size) as usize;
        let destination_start = destination_offset as usize;
        if self.is_same(destination) {
            let mut bytes = self
                .state
                .bytes
                .lock()
                .map_err(|_| CopyBufferRole::Source)?;
            bytes.copy_within(source_range, destination_start);
        } else {
            let source = self
                .state
                .bytes
                .lock()
                .map_err(|_| CopyBufferRole::Source)?;
            let mut destination = destination
                .state
                .bytes
                .lock()
                .map_err(|_| CopyBufferRole::Destination)?;
            let destination_range = destination_start..destination_start + size as usize;
            destination[destination_range].copy_from_slice(&source[source_range]);
        }
        Ok(())
    }

    fn checked_range(&self, offset: u64, length: usize) -> Result<Range<usize>, BufferAccessError> {
        // Rust's supported pointer widths fit in u64; a slice length does too.
        let length = length as u64;
        let end = offset.checked_add(length).filter(|&end| end <= self.size());
        match end {
            // Creation established size <= isize::MAX. These offsets therefore
            // fit usize, and remain valid because storage is never resized.
            Some(end) => Ok(offset as usize..end as usize),
            None => Err(BufferAccessError::OutOfBounds {
                label: self.label().to_owned(),
                offset,
                length,
                buffer_size: self.size(),
            }),
        }
    }

    /// Reference executor only: device gate held and immutable range validated.
    pub(crate) fn with_bytes<R>(
        &self,
        range: Range<usize>,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Result<R, ()> {
        let bytes = self.state.bytes.lock().map_err(|_| ())?;
        Ok(f(&bytes[range]))
    }

    /// Executor only: caller holds the device gate and locks each allocation once.
    pub(crate) fn lock_bytes(&self) -> Result<MutexGuard<'_, Vec<u8>>, ()> {
        self.state.bytes.lock().map_err(|_| ())
    }

    fn poisoned_error(&self) -> BufferAccessError {
        BufferAccessError::StoragePoisoned {
            label: self.label().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Instance;

    #[test]
    fn uniform_poison_preserves_render_compute_outputs_and_skips_empty_sources() {
        use crate::*;
        let d = Instance::new().default_adapter().create_device();
        let parameters = d
            .create_buffer(BufferDescriptor {
                label: "parameters",
                size: 16,
                usages: &[BufferUsage::Uniform],
                host_access: HostAccess::ReadWrite,
            })
            .unwrap();
        let state = Arc::downgrade(&parameters.state);
        let out = d
            .create_buffer(BufferDescriptor {
                label: "output",
                size: 16,
                usages: &[BufferUsage::Storage],
                host_access: HostAccess::Read,
            })
            .unwrap();
        let layout = d
            .create_bind_group_layout(BindGroupLayoutDescriptor {
                label: "resources",
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderVisibility::All,
                        ty: BindingType::UniformBuffer { min_size: 16 },
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderVisibility::Compute,
                        ty: BindingType::StorageBuffer { read_only: false },
                    },
                ],
            })
            .unwrap();
        let group = d
            .create_bind_group(BindGroupDescriptor {
                label: "g",
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::UniformBuffer(BufferBinding {
                            buffer: parameters.clone(),
                            offset: 0,
                            size: 16,
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::StorageBuffer(BufferBinding {
                            buffer: out.clone(),
                            offset: 0,
                            size: 16,
                        }),
                    },
                ],
            })
            .unwrap();
        let make_shader = |stage, input_count, op| {
            d.create_shader(ShaderDescriptor {
                label: "s",
                stage,
                input_count,
                instructions: &[op],
                outputs: &[0],
            })
            .unwrap()
        };
        let kernel = make_shader(
            ShaderStage::Compute,
            4,
            ShaderInstruction::LoadUniform {
                binding: 0,
                element: 0,
            },
        );
        let compute = d
            .create_compute_pipeline_with_bindings(
                ComputePipelineDescriptor {
                    label: "c",
                    shader: &kernel,
                    workgroup_size: [1; 3],
                    output_bindings: &[1],
                },
                &layout,
            )
            .unwrap();
        let vertex = make_shader(
            ShaderStage::Vertex,
            0,
            ShaderInstruction::Constant([0., 0., 0., 1.]),
        );
        let fragment = make_shader(
            ShaderStage::Fragment,
            0,
            ShaderInstruction::LoadUniform {
                binding: 0,
                element: 0,
            },
        );
        let graphics = d
            .create_graphics_pipeline_with_bindings(
                GraphicsPipelineDescriptor {
                    label: "g",
                    vertex_shader: &vertex,
                    fragment_shader: &fragment,
                    vertex_layout: VertexBufferLayout {
                        stride: 4,
                        attributes: &[],
                    },
                    colour_format: FramebufferFormat::Rgba8Unorm,
                },
                GraphicsPipelineOptions::default(),
                None,
                &layout,
            )
            .unwrap();
        let vertices = d
            .create_buffer(BufferDescriptor {
                label: "v",
                size: 12,
                usages: &[BufferUsage::Vertex],
                host_access: HostAccess::None,
            })
            .unwrap();
        let target = d
            .create_framebuffer(FramebufferDescriptor {
                label: "t",
                width: 1,
                height: 1,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        poison(&parameters);
        let mut retained = Vec::new();
        for count in [3, 0] {
            let mut e = d.create_command_encoder();
            let mut pass = e.begin_render_pass(&target, Some([7; 4])).unwrap();
            pass.set_pipeline(&graphics).unwrap();
            pass.set_vertex_buffer(&vertices, 0).unwrap();
            pass.set_bind_group(&group).unwrap();
            pass.draw_programmable(0..count).unwrap();
            pass.finish().unwrap();
            let commands = e.finish();
            commands.resource_uses().unwrap();
            if count == 3 {
                assert!(matches!(
                    d.default_queue().submit(&commands),
                    Err(QueueSubmitError::RenderFailed {
                        source: RenderExecutionError::UniformStoragePoisoned { binding: 0, .. },
                        ..
                    })
                ));
                assert_eq!(target.pixel(0, 0).unwrap(), [0; 4]);
            } else {
                d.default_queue().submit(&commands).unwrap();
                assert_eq!(target.pixel(0, 0).unwrap(), [7; 4]);
            }
            retained.push(commands);
        }
        for count in [1, 0] {
            let mut e = d.create_command_encoder();
            e.dispatch_with_bind_group(&compute, [count, 1, 1], &group)
                .unwrap();
            let commands = e.finish();
            if count == 1 {
                assert!(matches!(
                    d.default_queue().submit(&commands),
                    Err(QueueSubmitError::ComputeFailed {
                        source: ComputeExecutionError::StoragePoisoned { binding: 0, .. },
                        ..
                    })
                ));
            } else {
                d.default_queue().submit(&commands).unwrap();
            }
            let mut bytes = [9; 16];
            out.read(0, &mut bytes).unwrap();
            assert_eq!(bytes, [0; 16]);
            retained.push(commands);
        }
        drop(parameters);
        drop(group);
        assert!(state.upgrade().is_some());
        drop(retained);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn transition_commands_retain_storage_until_finished_work_drops() {
        let b = buffer();
        let state = Arc::downgrade(&b.state);
        let mut e = b
            .device()
            .create_command_encoder_with_synchronization(crate::SynchronizationMode::Explicit);
        e.transition_resources(&[crate::ResourceTransition {
            resource: crate::Resource::Buffer(b.clone()),
            before: None,
            after: crate::ResourceAccess::StorageRead,
        }])
        .unwrap();
        drop(b);
        let commands = e.finish();
        commands.validate_dependencies().unwrap();
        assert!(state.upgrade().is_some());
        drop(commands);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn dependency_analysis_skips_poison_and_precedes_all_execution() {
        let d = Instance::new().default_adapter().create_device();
        let a = transfer(&d, "source");
        let bad = transfer(&d, "poisoned");
        let out = transfer(&d, "out");
        a.write(0, &[1; 4]).unwrap();
        poison(&bad);
        let mut e =
            d.create_command_encoder_with_synchronization(crate::SynchronizationMode::Explicit);
        e.copy_buffer(&a, 0, &out, 0, 4).unwrap();
        e.copy_buffer(&bad, 0, &out, 0, 4).unwrap();
        let commands = e.finish();
        assert_eq!(commands.resource_uses().unwrap().len(), 4);
        assert!(matches!(
            d.default_queue().submit(&commands),
            Err(crate::QueueSubmitError::DependencyFailed { .. })
        ));
        let mut bytes = [9; 4];
        out.read(0, &mut bytes).unwrap();
        assert_eq!(bytes, [0; 4]);
        let mut e =
            d.create_command_encoder_with_synchronization(crate::SynchronizationMode::Explicit);
        e.transition_resources(&[
            crate::ResourceTransition {
                resource: crate::Resource::Buffer(bad.clone()),
                before: None,
                after: crate::ResourceAccess::TransferRead,
            },
            crate::ResourceTransition {
                resource: crate::Resource::Buffer(out.clone()),
                before: None,
                after: crate::ResourceAccess::TransferWrite,
            },
        ])
        .unwrap();
        e.copy_buffer(&bad, 0, &out, 0, 4).unwrap();
        let commands = e.finish();
        commands.validate_dependencies().unwrap();
        assert!(matches!(
            d.default_queue().submit(&commands),
            Err(crate::QueueSubmitError::StoragePoisoned { .. })
        ));
        let mut e =
            d.create_command_encoder_with_synchronization(crate::SynchronizationMode::Explicit);
        e.copy_buffer(&bad, 4, &out, 4, 0).unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
    }

    fn buffer() -> Buffer {
        Instance::new()
            .default_adapter()
            .create_device()
            .create_buffer(BufferDescriptor {
                label: "lifecycle test",
                size: 4,
                usages: &[BufferUsage::Storage],
                host_access: HostAccess::ReadWrite,
            })
            .unwrap()
    }

    #[test]
    fn storage_is_released_after_the_final_buffer_handle_drops() {
        let buffer = buffer();
        let state = Arc::downgrade(&buffer.state);
        let retained = buffer.clone();
        drop(buffer);
        assert!(state.upgrade().is_some());
        drop(retained);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn interrupted_storage_is_reported_without_changing_the_read_destination() {
        let buffer = buffer();
        let alias = buffer.clone();
        let interrupted = std::panic::catch_unwind(move || {
            let _guard = alias.state.bytes.lock().unwrap();
            panic!("simulate an interrupted internal storage operation");
        });
        assert!(interrupted.is_err());
        let mut destination = [99; 4];
        assert!(matches!(
            buffer.read(0, &mut destination),
            Err(BufferAccessError::StoragePoisoned { .. })
        ));
        assert_eq!(destination, [99; 4]);
        assert!(matches!(
            buffer.write(0, &[1]),
            Err(BufferAccessError::StoragePoisoned { .. })
        ));
    }

    #[test]
    fn recorded_copies_retain_storage_until_finished_work_drops() {
        let device = Instance::new().default_adapter().create_device();
        let request = BufferDescriptor {
            label: "recorded allocation",
            size: 4,
            usages: &[
                BufferUsage::TransferSource,
                BufferUsage::TransferDestination,
            ],
            host_access: HostAccess::None,
        };
        let source = device.create_buffer(request).unwrap();
        let destination = device.create_buffer(request).unwrap();
        let source_state = Arc::downgrade(&source.state);
        let destination_state = Arc::downgrade(&destination.state);
        let mut encoder = device.create_command_encoder();
        encoder.copy_buffer(&source, 0, &destination, 0, 4).unwrap();
        drop(source);
        drop(destination);
        drop(device);
        assert!(source_state.upgrade().is_some());
        assert!(destination_state.upgrade().is_some());
        let commands = encoder.finish();
        assert!(source_state.upgrade().is_some());
        assert!(destination_state.upgrade().is_some());
        drop(commands);
        assert!(source_state.upgrade().is_none());
        assert!(destination_state.upgrade().is_none());
    }

    #[test]
    fn rejected_copy_does_not_retain_buffers() {
        let buffer = buffer(); // Storage role, so recording must reject it.
        let state = Arc::downgrade(&buffer.state);
        let mut encoder = buffer.device().create_command_encoder();
        let error = encoder.copy_buffer(&buffer, 0, &buffer, 0, 1).unwrap_err();
        drop(buffer);
        assert!(state.upgrade().is_none());
        assert!(encoder.commands().is_empty());
        assert!(error.to_string().contains("TransferSource"));
    }

    fn transfer(device: &Device, label: &str) -> Buffer {
        device
            .create_buffer(BufferDescriptor {
                label,
                size: 4,
                usages: &[
                    BufferUsage::TransferSource,
                    BufferUsage::TransferDestination,
                ],
                host_access: HostAccess::ReadWrite,
            })
            .unwrap()
    }

    fn poison(buffer: &Buffer) {
        assert!(std::panic::catch_unwind(|| {
            let _storage = buffer.state.bytes.lock().unwrap();
            panic!("simulate interrupted internal execution");
        })
        .is_err());
    }

    #[test]
    fn submission_failure_preserves_the_completed_prefix_and_skips_later_work() {
        use crate::QueueSubmitError;
        for role in [CopyBufferRole::Source, CopyBufferRole::Destination] {
            let device = Instance::new().default_adapter().create_device();
            let source = transfer(&device, "good source");
            let completed = transfer(&device, "completed");
            let bad = transfer(&device, "poisoned");
            let failed_destination = transfer(&device, "failed destination");
            let later = transfer(&device, "later");
            source.write(0, &[7; 4]).unwrap();
            let mut encoder = device.create_command_encoder();
            encoder.copy_buffer(&source, 0, &completed, 0, 4).unwrap();
            match role {
                CopyBufferRole::Source => encoder
                    .copy_buffer(&bad, 0, &failed_destination, 0, 4)
                    .unwrap(),
                CopyBufferRole::Destination => encoder.copy_buffer(&source, 0, &bad, 0, 4).unwrap(),
            }
            encoder.copy_buffer(&source, 0, &later, 0, 4).unwrap();
            let commands = encoder.finish();
            poison(&bad);
            let error = device.default_queue().submit(&commands).unwrap_err();
            assert_eq!(
                error,
                QueueSubmitError::StoragePoisoned {
                    command_index: 1,
                    role,
                    label: "poisoned".to_owned(),
                }
            );
            assert!(error.to_string().contains("earlier commands completed"));
            assert!(std::error::Error::source(&error).is_none());
            let mut bytes = [99; 4];
            completed.read(0, &mut bytes).unwrap();
            assert_eq!(bytes, [7; 4]);
            later.read(0, &mut bytes).unwrap();
            assert_eq!(bytes, [0; 4]);
            failed_destination.read(0, &mut bytes).unwrap();
            assert_eq!(bytes, [0; 4]);
            let poisoned_bytes = bad.state.bytes.lock().unwrap_err().into_inner();
            assert_eq!(*poisoned_bytes, [0; 4]);
            drop(poisoned_bytes);
            // A recoverable error does not make unrelated storage unavailable.
            source.write(0, &[8; 4]).unwrap();
            assert_eq!(commands.commands().len(), 3);
        }
    }

    #[test]
    fn empty_copies_skip_poisoned_storage_but_nonempty_aliases_report_source() {
        use crate::QueueSubmitError;
        let device = Instance::new().default_adapter().create_device();
        let buffer = transfer(&device, "alias");
        let mut empty = device.create_command_encoder();
        empty.copy_buffer(&buffer, 4, &buffer, 4, 0).unwrap();
        let mut nonempty = device.create_command_encoder();
        nonempty
            .copy_buffer(&buffer, 0, &buffer.clone(), 2, 2)
            .unwrap();
        poison(&buffer);
        device.default_queue().submit(&empty.finish()).unwrap();
        assert_eq!(
            device.default_queue().submit(&nonempty.finish()),
            Err(QueueSubmitError::StoragePoisoned {
                command_index: 0,
                role: CopyBufferRole::Source,
                label: "alias".to_owned(),
            })
        );
    }

    #[test]
    fn completed_submission_does_not_leave_queue_owned_resource_references() {
        let device = Instance::new().default_adapter().create_device();
        let queue = device.default_queue();
        let source = transfer(&device, "source");
        let destination = transfer(&device, "destination");
        let source_state = Arc::downgrade(&source.state);
        let destination_state = Arc::downgrade(&destination.state);
        let mut encoder = device.create_command_encoder();
        encoder.copy_buffer(&source, 0, &destination, 0, 4).unwrap();
        let commands = encoder.finish();
        drop(source);
        drop(destination);
        queue.submit(&commands).unwrap();
        assert!(source_state.upgrade().is_some());
        assert!(destination_state.upgrade().is_some());
        drop(commands);
        assert!(source_state.upgrade().is_none());
        assert!(destination_state.upgrade().is_none());
    }

    #[test]
    fn poisoned_vertex_fetch_rejects_the_pass_before_clear_and_empty_fetch_skips_storage() {
        use crate::{
            FramebufferDescriptor, FramebufferFormat, QueueSubmitError, RenderExecutionError,
        };
        let device = Instance::new().default_adapter().create_device();
        let buffer = device
            .create_buffer(BufferDescriptor {
                label: "poisoned vertices",
                size: 24,
                usages: &[BufferUsage::Vertex],
                host_access: HostAccess::None,
            })
            .unwrap();
        let target = device
            .create_framebuffer(FramebufferDescriptor {
                label: "target",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_render_pass(&target, Some([7; 4])).unwrap();
        pass.set_vertex_buffer(&buffer, 0).unwrap();
        pass.draw(0..3, [8; 4]).unwrap();
        pass.finish().unwrap();
        poison(&buffer);
        assert_eq!(
            device.default_queue().submit(&encoder.finish()),
            Err(QueueSubmitError::RenderFailed {
                command_index: 0,
                source: RenderExecutionError::VertexStoragePoisoned {
                    label: "poisoned vertices".to_owned()
                },
            })
        );
        assert_eq!(target.pixel(0, 0).unwrap(), [0; 4]);
        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_render_pass(&target, Some([9; 4])).unwrap();
        pass.set_vertex_buffer(&buffer, 24).unwrap();
        pass.draw(0..0, [8; 4]).unwrap();
        pass.finish().unwrap();
        device.default_queue().submit(&encoder.finish()).unwrap();
        assert_eq!(target.pixel(0, 0).unwrap(), [9; 4]);
    }

    fn indexed_resources() -> (
        Device,
        crate::Framebuffer,
        crate::GraphicsPipeline,
        Buffer,
        Buffer,
    ) {
        use crate::{
            FramebufferDescriptor, FramebufferFormat, GraphicsPipelineDescriptor, ShaderDescriptor,
            ShaderInstruction as Op, ShaderStage, VertexAttribute, VertexBufferLayout,
            VertexFormat,
        };
        let device = Instance::new().default_adapter().create_device();
        let target = device
            .create_framebuffer(FramebufferDescriptor {
                label: "indexed target",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let vertex = device
            .create_shader(ShaderDescriptor {
                label: "v",
                stage: ShaderStage::Vertex,
                input_count: 1,
                instructions: &[Op::Input { location: 0 }],
                outputs: &[0],
            })
            .unwrap();
        let fragment = device
            .create_shader(ShaderDescriptor {
                label: "f",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[Op::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(GraphicsPipelineDescriptor {
                label: "indexed",
                vertex_shader: &vertex,
                fragment_shader: &fragment,
                vertex_layout: VertexBufferLayout {
                    stride: 8,
                    attributes: &[VertexAttribute {
                        location: 0,
                        offset: 0,
                        format: VertexFormat::Float32x2,
                    }],
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let vertices = device
            .create_buffer(BufferDescriptor {
                label: "indexed vertices",
                size: 24,
                usages: &[BufferUsage::Vertex],
                host_access: HostAccess::None,
            })
            .unwrap();
        let indices = device
            .create_buffer(BufferDescriptor {
                label: "indices",
                size: 6,
                usages: &[BufferUsage::Index],
                host_access: HostAccess::Write,
            })
            .unwrap();
        indices.write(0, &[0, 0, 1, 0, 2, 0]).unwrap();
        (device, target, pipeline, vertices, indices)
    }

    #[test]
    fn indexed_poison_preserves_target_and_empty_draws_skip_both_sources() {
        use crate::{IndexFormat, QueueSubmitError, RenderExecutionError};
        for poison_index in [true, false] {
            let (device, target, pipeline, vertices, indices) = indexed_resources();
            let mut encoder = device.create_command_encoder();
            let mut pass = encoder.begin_render_pass(&target, Some([7; 4])).unwrap();
            pass.set_pipeline(&pipeline).unwrap();
            pass.set_vertex_buffer(&vertices, 0).unwrap();
            pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)
                .unwrap();
            pass.draw_indexed(0..3, 0).unwrap();
            pass.finish().unwrap();
            if poison_index {
                poison(&indices);
            } else {
                poison(&vertices);
            }
            let expected = if poison_index {
                RenderExecutionError::IndexStoragePoisoned {
                    label: "indices".to_owned(),
                }
            } else {
                RenderExecutionError::VertexStoragePoisoned {
                    label: "indexed vertices".to_owned(),
                }
            };
            assert_eq!(
                device.default_queue().submit(&encoder.finish()),
                Err(QueueSubmitError::RenderFailed {
                    command_index: 0,
                    source: expected
                })
            );
            assert_eq!(target.pixel(0, 0).unwrap(), [0; 4]);
            if poison_index {
                poison(&vertices);
            } else {
                poison(&indices);
            }
            let mut encoder = device.create_command_encoder();
            let mut pass = encoder.begin_render_pass(&target, Some([9; 4])).unwrap();
            pass.set_pipeline(&pipeline).unwrap();
            pass.set_vertex_buffer(&vertices, 24).unwrap();
            pass.set_index_buffer(&indices, 6, IndexFormat::Uint16)
                .unwrap();
            pass.draw_indexed(0..0, i32::MIN).unwrap();
            pass.finish().unwrap();
            device.default_queue().submit(&encoder.finish()).unwrap();
            assert_eq!(target.pixel(0, 0).unwrap(), [9; 4]);
        }
    }

    #[test]
    fn indexed_commands_release_both_buffer_allocations_after_last_owner_drops() {
        let (device, target, pipeline, vertices, indices) = indexed_resources();
        let vertex_state = Arc::downgrade(&vertices.state);
        let index_state = Arc::downgrade(&indices.state);
        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_render_pass(&target, None).unwrap();
        pass.set_pipeline(&pipeline).unwrap();
        pass.set_vertex_buffer(&vertices, 0).unwrap();
        pass.set_index_buffer(&indices, 0, crate::IndexFormat::Uint16)
            .unwrap();
        pass.draw_indexed(0..3, 0).unwrap();
        pass.finish().unwrap();
        let commands = encoder.finish();
        drop(vertices);
        drop(indices);
        drop(pipeline);
        device.default_queue().submit(&commands).unwrap();
        assert!(vertex_state.upgrade().is_some());
        assert!(index_state.upgrade().is_some());
        drop(commands);
        assert!(vertex_state.upgrade().is_none());
        assert!(index_state.upgrade().is_none());
    }

    #[test]
    fn compute_poison_checks_all_outputs_before_commit_but_skips_empty_and_unused() {
        use crate::{
            ComputeExecutionError, ComputePipelineDescriptor, QueueSubmitError, ShaderDescriptor,
            ShaderInstruction, ShaderStage, StorageBufferBinding,
        };
        let device = Instance::new().default_adapter().create_device();
        let descriptor = BufferDescriptor {
            label: "compute storage",
            size: 16,
            usages: &[BufferUsage::Storage],
            host_access: HostAccess::ReadWrite,
        };
        let first = device.create_buffer(descriptor).unwrap();
        let poisoned = device.create_buffer(descriptor).unwrap();
        let kernel = device
            .create_shader(ShaderDescriptor {
                label: "outputs",
                stage: ShaderStage::Compute,
                input_count: 4,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0, 0],
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(ComputePipelineDescriptor {
                label: "two",
                shader: &kernel,
                workgroup_size: [1, 1, 1],
                output_bindings: &[0, 1],
            })
            .unwrap();
        let alias = poisoned.clone();
        assert!(std::panic::catch_unwind(move || {
            let _guard = alias.state.bytes.lock().unwrap();
            panic!("interrupted compute output");
        })
        .is_err());
        let bindings = [
            StorageBufferBinding {
                binding: 0,
                buffer: &first,
                offset: 0,
                size: 16,
            },
            StorageBufferBinding {
                binding: 1,
                buffer: &poisoned,
                offset: 0,
                size: 16,
            },
        ];
        let mut encoder = device.create_command_encoder();
        encoder.dispatch(&pipeline, [1, 1, 1], &bindings).unwrap();
        assert!(matches!(
            device.default_queue().submit(&encoder.finish()),
            Err(QueueSubmitError::ComputeFailed {
                source: ComputeExecutionError::StoragePoisoned { binding: 1, .. },
                ..
            })
        ));
        let mut bytes = [99; 16];
        first.read(0, &mut bytes).unwrap();
        assert_eq!(bytes, [0; 16]);
        let mut encoder = device.create_command_encoder();
        encoder.dispatch(&pipeline, [0, 1, 1], &bindings).unwrap();
        device.default_queue().submit(&encoder.finish()).unwrap();
        let one = device
            .create_shader(ShaderDescriptor {
                label: "one",
                stage: ShaderStage::Compute,
                input_count: 4,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let one = device
            .create_compute_pipeline(ComputePipelineDescriptor {
                label: "one",
                shader: &one,
                workgroup_size: [1, 1, 1],
                output_bindings: &[0],
            })
            .unwrap();
        let mut encoder = device.create_command_encoder();
        encoder.dispatch(&one, [1, 1, 1], &bindings).unwrap();
        device.default_queue().submit(&encoder.finish()).unwrap();
        first.read(0, &mut bytes).unwrap();
        assert_eq!(bytes, [1.0_f32.to_le_bytes(); 4].concat().as_slice());
    }

    #[test]
    fn compute_commands_retain_and_release_buffer_storage() {
        use crate::{
            ComputePipelineDescriptor, ShaderDescriptor, ShaderInstruction, ShaderStage,
            StorageBufferBinding,
        };
        let device = Instance::new().default_adapter().create_device();
        let buffer = device
            .create_buffer(BufferDescriptor {
                label: "compute retained",
                size: 16,
                usages: &[BufferUsage::Storage],
                host_access: HostAccess::None,
            })
            .unwrap();
        let weak = Arc::downgrade(&buffer.state);
        let kernel = device
            .create_shader(ShaderDescriptor {
                label: "one",
                stage: ShaderStage::Compute,
                input_count: 4,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(ComputePipelineDescriptor {
                label: "one",
                shader: &kernel,
                workgroup_size: [1, 1, 1],
                output_bindings: &[0],
            })
            .unwrap();
        let mut encoder = device.create_command_encoder();
        encoder
            .dispatch(
                &pipeline,
                [1, 1, 1],
                &[StorageBufferBinding {
                    binding: 0,
                    buffer: &buffer,
                    offset: 0,
                    size: 16,
                }],
            )
            .unwrap();
        let commands = encoder.finish();
        drop(buffer);
        drop(pipeline);
        drop(kernel);
        drop(device);
        assert!(weak.upgrade().is_some());
        commands.device().default_queue().submit(&commands).unwrap();
        drop(commands);
        assert!(weak.upgrade().is_none());
    }
}
