use std::sync::{Arc, Mutex, MutexGuard};

/// An EX target identified during discovery, independent of other graphics APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendKind {
    /// EX's CPU/software reference target, with copies and programmable triangles.
    Reference,
}

/// Immutable discovery metadata, not a claim of supported graphics operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterInfo {
    /// Human-readable name identifying this target as software.
    pub name: String,
    /// The EX implementation selected by this adapter.
    pub backend: BackendKind,
}

/// Discovers EX targets. Currently exposes one built-in software adapter.
///
/// Creating an instance neither detects installed GPUs nor starts execution.
#[derive(Debug)]
pub struct Instance {
    adapters: [Adapter; 1],
}

impl Instance {
    /// Constructs discovery for the always-available reference target.
    pub fn new() -> Self {
        Self {
            adapters: [Adapter {
                info: Arc::new(AdapterInfo {
                    name: String::from("EX Reference (software)"),
                    backend: BackendKind::Reference,
                }),
            }],
        }
    }

    /// Borrows the discovered targets. Clone an entry to retain it independently.
    pub fn adapters(&self) -> &[Adapter] {
        &self.adapters
    }

    /// Returns an owned handle to the built-in reference adapter.
    ///
    /// This target is always present, so selection cannot fail in this version.
    pub fn default_adapter(&self) -> Adapter {
        self.adapters[0].clone()
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new()
    }
}

/// An owned handle to discovered target metadata, usable after its instance drops.
///
/// Cloning shares metadata; it does not create a logical device.
#[derive(Debug, Clone)]
pub struct Adapter {
    info: Arc<AdapterInfo>,
}

impl Adapter {
    /// Supported core contract, separate from this adapter's backend identity.
    pub fn core_profile(&self) -> crate::CoreProfile {
        crate::backend::core_profile(self.info.backend)
    }

    /// Borrows target metadata for the lifetime of this handle.
    pub fn info(&self) -> &AdapterInfo {
        &self.info
    }

    /// Creates a fresh logical ownership domain with one default queue.
    ///
    /// This only allocates shared CPU state, with no driver calls or execution.
    /// Every call creates a distinct device, even on the same adapter.
    pub fn create_device(&self) -> Device {
        Device {
            state: Arc::new(DeviceState {
                adapter: self.clone(),
                execution: Mutex::new(()),
                backend: crate::backend::create(self.info.backend),
            }),
        }
    }
}

/// Shared identity for one logical device and its single default queue.
///
/// There are no back references: state owns adapter metadata, and handles own
/// state. Future resources can retain this domain without exposing its pointer.
#[derive(Debug)]
struct DeviceState {
    adapter: Adapter,
    execution: Mutex<()>,
    backend: Box<dyn crate::backend::ExecutionBackend>,
}

/// A logical EX device. Creates reference resources, shaders, pipelines, and
/// encoders; its synchronous queue executes copies and fixed/programmed triangles.
///
/// Cloning retains the same device; use [`Adapter::create_device`] for a new one.
#[derive(Debug, Clone)]
pub struct Device {
    state: Arc<DeviceState>,
}

impl Device {
    /// The supported contract of this device's selected executor.
    pub fn core_profile(&self) -> crate::CoreProfile {
        self.state.backend.core_profile()
    }
    // Queue policy has already checked device identity and explicit dependencies.
    // The runtime owns ordering, gate lifetime, error indices, and early stop.
    pub(crate) fn execute_commands(
        &self,
        commands: &crate::CommandBuffer,
    ) -> Result<(), crate::QueueSubmitError> {
        let _execution = self.lock_execution();
        for (command_index, command) in commands.commands().iter().enumerate() {
            self.state
                .backend
                .execute(command)
                .map_err(|error| error.at_command(command_index))?;
        }
        Ok(())
    }

    /// Returns an owned handle to this device's selected target.
    pub fn adapter(&self) -> Adapter {
        self.state.adapter.clone()
    }

    /// Returns a handle to this device's single default endpoint.
    ///
    /// Repeated calls share the endpoint and execution gate. No worker is started.
    pub fn default_queue(&self) -> Queue {
        Queue {
            state: Arc::clone(&self.state),
        }
    }

    /// Tests logical device identity, rather than equal adapter metadata.
    pub fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Acquire before any storage lock. This gate contains no resource data;
    /// recovering a poisoned guard preserves serialization. Storage poisoning
    /// is still reported separately and never recovered by byte operations.
    pub(crate) fn lock_execution(&self) -> MutexGuard<'_, ()> {
        self.state
            .execution
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

/// An owned handle to a device's single default queue endpoint.
///
/// Retains the owning device state after the original device handle is dropped.
/// Submission executes recorded copies synchronously on the reference target.
#[derive(Debug, Clone)]
pub struct Queue {
    state: Arc<DeviceState>,
}

impl Queue {
    /// Returns a handle to the original owning device, not a newly created device.
    pub fn device(&self) -> Device {
        Device {
            state: Arc::clone(&self.state),
        }
    }

    /// Checks whether this queue belongs to the specified logical device.
    pub fn belongs_to(&self, device: &Device) -> bool {
        Arc::ptr_eq(&self.state, &device.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_executor_obeys_preflight_gate_order_and_failure_stop() {
        use crate::backend::{ExecutionBackend, ExecutionError};
        use crate::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Weak;
        #[derive(Debug, Default)]
        struct Trace {
            calls: AtomicUsize,
            gated_calls: AtomicUsize,
        }
        #[derive(Debug)]
        struct TracedExecutor {
            owner: Weak<DeviceState>,
            trace: Arc<Trace>,
        }
        impl ExecutionBackend for TracedExecutor {
            fn core_profile(&self) -> CoreProfile {
                CoreProfile::Preview1
            }
            fn execute(&self, command: &Command) -> Result<(), ExecutionError> {
                let owner = self.owner.upgrade().unwrap();
                assert!(matches!(
                    owner.execution.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                self.trace.gated_calls.fetch_add(1, Ordering::SeqCst);
                let index = self.trace.calls.fetch_add(1, Ordering::SeqCst);
                if index == 1 {
                    return Err(ExecutionError::BufferCopyPoisoned {
                        role: CopyBufferRole::Destination,
                        label: "trace".into(),
                    });
                }
                crate::reference::ReferenceBackend.execute(command)
            }
        }
        let trace = Arc::new(Trace::default());
        let adapter = Instance::new().default_adapter();
        let device = Device {
            state: Arc::new_cyclic(|owner| DeviceState {
                adapter: adapter.clone(),
                execution: Mutex::new(()),
                backend: Box::new(TracedExecutor {
                    owner: owner.clone(),
                    trace: trace.clone(),
                }),
            }),
        };
        assert_eq!(device.core_profile(), adapter.core_profile());
        let make = || {
            device
                .create_buffer(BufferDescriptor {
                    label: "trace",
                    size: 4,
                    usages: &[
                        BufferUsage::TransferSource,
                        BufferUsage::TransferDestination,
                    ],
                    host_access: HostAccess::ReadWrite,
                })
                .unwrap()
        };
        let source = make();
        source.write(0, &[7; 4]).unwrap();
        let prefix = make();
        let failed = make();
        let later = make();
        let queue = device.default_queue();
        let foreign = adapter.create_device().create_command_encoder().finish();
        assert_eq!(
            queue.submit(&foreign),
            Err(QueueSubmitError::ForeignCommandBuffer)
        );
        let mut invalid =
            device.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
        invalid.copy_buffer(&source, 0, &prefix, 0, 4).unwrap();
        assert!(matches!(
            queue.submit(&invalid.finish()),
            Err(QueueSubmitError::DependencyFailed { .. })
        ));
        assert_eq!(trace.calls.load(Ordering::SeqCst), 0);
        let mut encoder = device.create_command_encoder();
        for destination in [&prefix, &failed, &later] {
            encoder.copy_buffer(&source, 0, destination, 0, 4).unwrap();
        }
        assert_eq!(
            queue.clone().submit(&encoder.finish()),
            Err(QueueSubmitError::StoragePoisoned {
                command_index: 1,
                role: CopyBufferRole::Destination,
                label: "trace".into(),
            })
        );
        assert_eq!(trace.calls.load(Ordering::SeqCst), 2);
        assert_eq!(trace.gated_calls.load(Ordering::SeqCst), 2);
        for (destination, expected) in [(&prefix, [7; 4]), (&failed, [0; 4]), (&later, [0; 4])] {
            let mut result = [99; 4];
            destination.read(0, &mut result).unwrap();
            assert_eq!(result, expected);
        }
        let weak = Arc::downgrade(&device.state);
        drop(foreign);
        drop(source);
        drop(prefix);
        drop(failed);
        drop(later);
        drop(queue);
        drop(device);
        assert!(weak.upgrade().is_none()); // Weak fixture back-reference forms no ownership cycle.
    }

    #[test]
    fn execution_gate_blocks_cpu_access_and_queue_clones_but_not_other_devices() {
        use crate::{
            BufferDescriptor, BufferUsage, FramebufferDescriptor, FramebufferFormat, HostAccess,
        };
        use std::sync::mpsc;
        use std::time::Duration;
        let device = Instance::new().default_adapter().create_device();
        let buffer = device
            .create_buffer(BufferDescriptor {
                label: "gated",
                size: 4,
                usages: &[
                    BufferUsage::TransferSource,
                    BufferUsage::TransferDestination,
                ],
                host_access: HostAccess::ReadWrite,
            })
            .unwrap();
        let other = device.adapter().create_device();
        let target = device
            .create_framebuffer(FramebufferDescriptor {
                label: "gated framebuffer",
                width: 2,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let queue = device.default_queue().clone();
        let commands = device.create_command_encoder().finish();
        let gate = device.lock_execution();
        let (done, received) = mpsc::channel();
        let write_done = done.clone();
        let read_done = done.clone();
        let pixel_done = done.clone();
        let image_done = done.clone();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                buffer.write(0, &[7; 4]).unwrap();
                write_done.send(()).unwrap();
            });
            scope.spawn(|| {
                buffer.read(0, &mut [0; 4]).unwrap();
                read_done.send(()).unwrap();
            });
            scope.spawn(|| {
                queue.submit(&commands).unwrap();
                done.send(()).unwrap();
            });
            scope.spawn(|| {
                target.pixel(0, 0).unwrap();
                pixel_done.send(()).unwrap();
            });
            scope.spawn(|| {
                target.read_pixels(&mut [[0; 4]; 4]).unwrap();
                image_done.send(()).unwrap();
            });
            assert!(matches!(
                received.recv_timeout(Duration::from_millis(50)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            // Device gates are independent; this succeeds while the first is held.
            other
                .default_queue()
                .submit(&other.create_command_encoder().finish())
                .unwrap();
            drop(gate);
            for _ in 0..5 {
                received.recv_timeout(Duration::from_secs(2)).unwrap();
            }
        });
    }

    #[test]
    fn recovering_a_poisoned_empty_gate_preserves_usable_storage() {
        use crate::{BufferDescriptor, BufferUsage, HostAccess};
        let device = Instance::new().default_adapter().create_device();
        let buffer = device
            .create_buffer(BufferDescriptor {
                label: "usable after gate poison",
                size: 4,
                usages: &[BufferUsage::Storage],
                host_access: HostAccess::ReadWrite,
            })
            .unwrap();
        assert!(std::panic::catch_unwind(|| {
            let _gate = device.lock_execution();
            panic!("simulate panic without touching resource storage");
        })
        .is_err());
        assert!(device.state.execution.is_poisoned());
        device
            .default_queue()
            .submit(&device.create_command_encoder().finish())
            .unwrap();
        buffer.write(0, &[7; 4]).unwrap();
        let mut bytes = [0; 4];
        buffer.read(0, &mut bytes).unwrap();
        assert_eq!(bytes, [7; 4]);
    }

    #[test]
    fn empty_recorded_work_retains_device_until_it_drops() {
        let device = Instance::new().default_adapter().create_device();
        let state = Arc::downgrade(&device.state);
        let encoder = device.create_command_encoder();
        drop(device);
        assert!(state.upgrade().is_some());
        let commands = encoder.finish();
        assert!(state.upgrade().is_some());
        drop(commands);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn the_last_queue_handle_releases_device_state_and_adapter_metadata() {
        let instance = Instance::new();
        let adapter = instance.default_adapter();
        let metadata = Arc::downgrade(&adapter.info);
        let device = adapter.create_device();
        let state = Arc::downgrade(&device.state);
        let queue = device.default_queue();
        let retained_queue = queue.clone();

        drop(instance);
        drop(adapter);
        drop(device);
        drop(queue);
        assert!(state.upgrade().is_some());
        assert!(metadata.upgrade().is_some());

        drop(retained_queue);
        assert!(state.upgrade().is_none());
        assert!(metadata.upgrade().is_none());
    }

    #[test]
    fn shaders_retain_device_until_the_final_shader_clone_drops() {
        use crate::{ShaderDescriptor, ShaderInstruction, ShaderStage};
        let device = Instance::new().default_adapter().create_device();
        let state = Arc::downgrade(&device.state);
        let shader = device
            .create_shader(ShaderDescriptor {
                label: "device lifetime",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant([1.0; 4])],
                outputs: &[0],
            })
            .unwrap();
        let retained = shader.clone();
        drop(device);
        drop(shader);
        assert!(state.upgrade().is_some());
        drop(retained);
        assert!(state.upgrade().is_none());
    }

    #[test]
    fn buffers_retain_device_state_until_the_final_buffer_handle_drops() {
        use crate::{BufferDescriptor, BufferUsage, HostAccess};

        let device = Instance::new().default_adapter().create_device();
        let state = Arc::downgrade(&device.state);
        let buffer = device
            .create_buffer(BufferDescriptor {
                label: "retained device",
                size: 4,
                usages: &[BufferUsage::Storage],
                host_access: HostAccess::None,
            })
            .unwrap();
        let retained = buffer.clone();
        drop(device);
        drop(buffer);
        assert!(state.upgrade().is_some());
        drop(retained);
        assert!(state.upgrade().is_none());
    }
}
