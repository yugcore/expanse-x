use ex::*;

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn shader(device: &Device, instructions: &[ShaderInstruction], outputs: &[u32]) -> Shader {
    device
        .create_shader(ShaderDescriptor {
            label: "kernel",
            stage: ShaderStage::Compute,
            input_count: 4,
            instructions,
            outputs,
        })
        .unwrap()
}
fn pipeline(device: &Device, shader: &Shader, local: [u32; 3], outputs: &[u32]) -> ComputePipeline {
    device
        .create_compute_pipeline(ComputePipelineDescriptor {
            label: "compute",
            shader,
            workgroup_size: local,
            output_bindings: outputs,
        })
        .unwrap()
}
fn buffer(device: &Device, elements: u64) -> Buffer {
    device
        .create_buffer(BufferDescriptor {
            label: "storage",
            size: elements * 16,
            usages: &[
                BufferUsage::Storage,
                BufferUsage::TransferSource,
                BufferUsage::TransferDestination,
                BufferUsage::Vertex,
            ],
            host_access: HostAccess::ReadWrite,
        })
        .unwrap()
}
fn binding(slot: u32, buffer: &Buffer) -> StorageBufferBinding<'_> {
    StorageBufferBinding {
        binding: slot,
        buffer,
        offset: 0,
        size: buffer.size(),
    }
}
fn write(buffer: &Buffer, data: &[[f32; 4]]) {
    buffer
        .write(
            0,
            &data
                .iter()
                .flatten()
                .flat_map(|n| n.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
}
fn read(buffer: &Buffer) -> Vec<[f32; 4]> {
    let mut bytes = vec![0; buffer.size() as usize];
    buffer.read(0, &mut bytes).unwrap();
    bytes
        .as_chunks::<16>()
        .0
        .iter()
        .map(|bytes| {
            std::array::from_fn(|lane| {
                f32::from_le_bytes(bytes[lane * 4..lane * 4 + 4].try_into().unwrap())
            })
        })
        .collect()
}

#[test]
fn compute_shader_interfaces_storage_and_standalone_evaluation() {
    let device = device();
    for stage in [ShaderStage::Vertex, ShaderStage::Fragment] {
        assert!(matches!(
            device.create_shader(ShaderDescriptor {
                label: "bad",
                stage,
                input_count: 0,
                instructions: &[ShaderInstruction::LoadStorage { binding: 0 }],
                outputs: &[0]
            }),
            Err(ShaderCreateError::InvalidDescriptor(
                ShaderValidationError::StorageLoadingStage { .. }
            ))
        ));
    }
    for count in [0, 3, 5] {
        assert!(matches!(
            device.create_shader(ShaderDescriptor {
                label: "bad",
                stage: ShaderStage::Compute,
                input_count: count,
                instructions: &[ShaderInstruction::Constant([0.0; 4])],
                outputs: &[0]
            }),
            Err(ShaderCreateError::InvalidDescriptor(
                ShaderValidationError::ComputeInputCount { .. }
            ))
        ));
    }
    assert!(matches!(
        device.create_shader(ShaderDescriptor {
            label: "bad",
            stage: ShaderStage::Compute,
            input_count: 4,
            instructions: &[ShaderInstruction::LoadStorage { binding: 8 }],
            outputs: &[0]
        }),
        Err(ShaderCreateError::InvalidDescriptor(
            ShaderValidationError::InvalidStorageBinding { .. }
        ))
    ));
    for outputs in [vec![], vec![0; 9]] {
        assert!(matches!(
            device.create_shader(ShaderDescriptor {
                label: "bad",
                stage: ShaderStage::Compute,
                input_count: 4,
                instructions: &[ShaderInstruction::Constant([0.0; 4])],
                outputs: &outputs
            }),
            Err(ShaderCreateError::InvalidDescriptor(
                ShaderValidationError::InvalidOutputCount { .. }
            ))
        ));
    }
    assert!(matches!(
        device.create_shader(ShaderDescriptor {
            label: "bad",
            stage: ShaderStage::Compute,
            input_count: 4,
            instructions: &[
                ShaderInstruction::Input { location: 0 },
                ShaderInstruction::SampleTexture {
                    binding: 0,
                    coordinates: 0
                }
            ],
            outputs: &[1]
        }),
        Err(ShaderCreateError::InvalidDescriptor(
            ShaderValidationError::TextureSamplingStage { .. }
        ))
    ));
    let load = shader(
        &device,
        &[ShaderInstruction::LoadStorage { binding: 7 }],
        &[0],
    );
    assert!(matches!(
        load.evaluate(&[[0.0; 4]; 4]),
        Err(ShaderExecutionError::MissingStorageBinding { binding: 7 })
    ));
    let identity = shader(&device, &[ShaderInstruction::Input { location: 0 }], &[0]);
    assert_eq!(identity.evaluate(&[[1.0; 4]; 4]).unwrap(), vec![[1.0; 4]]);
}

#[test]
fn pipeline_validation_owned_metadata_and_identity() {
    let device = device();
    let other = self::device();
    let kernel = shader(&device, &[ShaderInstruction::Constant([1.0; 4])], &[0, 0]);
    let label = String::from("owned kernel");
    let descriptor = ComputePipelineDescriptor {
        label: &label,
        shader: &kernel,
        workgroup_size: [2, 2, 2],
        output_bindings: &[1, 7],
    };
    assert!(matches!(
        other.create_compute_pipeline(descriptor),
        Err(ComputePipelineCreateError::ForeignShader { .. })
    ));
    for local in [[0, 1, 1], [1025, 1, 1], [33, 32, 1], [1024, 1024, 1024]] {
        assert!(matches!(
            device.create_compute_pipeline(ComputePipelineDescriptor {
                workgroup_size: local,
                ..descriptor
            }),
            Err(ComputePipelineCreateError::InvalidWorkgroupSize { .. })
        ));
    }
    for slots in [&[0][..], &[0, 8], &[3, 3]] {
        assert!(device
            .create_compute_pipeline(ComputePipelineDescriptor {
                output_bindings: slots,
                ..descriptor
            })
            .is_err());
    }
    let fragment = device
        .create_shader(ShaderDescriptor {
            label: "fragment",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &[ShaderInstruction::Constant([1.0; 4])],
            outputs: &[0],
        })
        .unwrap();
    assert!(matches!(
        device.create_compute_pipeline(ComputePipelineDescriptor {
            shader: &fragment,
            output_bindings: &[0],
            ..descriptor
        }),
        Err(ComputePipelineCreateError::WrongShaderStage { .. })
    ));
    let p = device.create_compute_pipeline(descriptor).unwrap();
    drop(label);
    assert_eq!(p.label(), "owned kernel");
    assert!(p.shader().is_same(&kernel));
    assert_eq!(p.workgroup_size(), [2, 2, 2]);
    assert_eq!(p.output_bindings(), &[1, 7]);
    assert!(p.is_same(&p.clone()));
    assert!(p.belongs_to(&device));
    assert!(p.device().is_same(&device));
    assert!(!p.is_same(&pipeline(&device, &kernel, [2, 2, 2], &[1, 7])));
    pipeline(&device, &kernel, [1024, 1, 1], &[0, 1]);
}

#[test]
fn all_four_builtin_vectors_use_global_x_fastest_elements() {
    let device = device();
    let kernel = shader(
        &device,
        &(0..4)
            .map(|location| ShaderInstruction::Input { location })
            .collect::<Vec<_>>(),
        &[0, 1, 2, 3],
    );
    let p = pipeline(&device, &kernel, [2, 3, 2], &[0, 2, 5, 7]);
    let outputs: Vec<_> = (0..4).map(|_| buffer(&device, 96)).collect();
    let bindings: Vec<_> = [0, 2, 5, 7]
        .into_iter()
        .zip(&outputs)
        .map(|(slot, b)| binding(slot, b))
        .collect();
    let mut encoder = device.create_command_encoder();
    encoder.dispatch(&p, [2, 2, 2], &bindings).unwrap();
    match &encoder.commands()[0] {
        Command::Dispatch(d) => {
            assert!(d.pipeline().is_same(&p));
            assert_eq!(d.workgroups(), [2, 2, 2]);
            assert_eq!(d.invocation_count(), 96);
            assert_eq!(d.bindings()[0].binding(), 0);
            assert!(d.bindings()[0].buffer().is_same(&outputs[0]));
            assert_eq!(d.bindings()[0].offset(), 0);
            assert_eq!(d.bindings()[0].size(), 1536);
        }
        _ => panic!("expected dispatch"),
    }
    assert_eq!(read(&outputs[0]), vec![[0.0; 4]; 96]);
    device.default_queue().submit(&encoder.finish()).unwrap();
    let result: Vec<_> = outputs.iter().map(read).collect();
    for z in 0..4 {
        for y in 0..6 {
            for x in 0..4 {
                let index = x + 4 * (y + 6 * z);
                let local = [x % 2, y % 3, z % 2];
                let group = [x / 2, y / 3, z / 2];
                assert_eq!(
                    result[0][index],
                    [x as f32, y as f32, z as f32, index as f32]
                );
                assert_eq!(
                    result[1][index],
                    [
                        local[0] as f32,
                        local[1] as f32,
                        local[2] as f32,
                        (local[0] + 2 * (local[1] + 3 * local[2])) as f32
                    ]
                );
                assert_eq!(
                    result[2][index],
                    [
                        group[0] as f32,
                        group[1] as f32,
                        group[2] as f32,
                        (group[0] + 2 * (group[1] + 2 * group[2])) as f32
                    ]
                );
                assert_eq!(result[3][index], [2.0, 2.0, 2.0, 0.0]);
            }
        }
    }
}

#[test]
fn dispatch_validates_ownership_ranges_slots_and_counts_without_appending() {
    let device = device();
    let other = self::device();
    let kernel = shader(
        &device,
        &[ShaderInstruction::LoadStorage { binding: 1 }],
        &[0],
    );
    let p = pipeline(&device, &kernel, [2, 1, 1], &[0]);
    let data = buffer(&device, 2);
    let foreign = buffer(&other, 2);
    let wrong_usage = device
        .create_buffer(BufferDescriptor {
            label: "wrong",
            size: 32,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::None,
        })
        .unwrap();
    let mut encoder = device.create_command_encoder();
    assert!(matches!(
        other.create_command_encoder().dispatch(&p, [1, 1, 1], &[]),
        Err(ComputeEncodeError::ForeignPipeline { .. })
    ));
    assert!(matches!(
        encoder.dispatch(&p, [65536, 1, 1], &[]),
        Err(ComputeEncodeError::InvalidWorkgroups { .. })
    ));
    assert!(matches!(
        encoder.dispatch(&p, [65535, 65535, 65535], &[]),
        Err(ComputeEncodeError::TooManyInvocations { .. })
    ));
    let valid = binding(0, &data);
    for (bindings, kind) in [
        (vec![binding(8, &data)], "slot"),
        (vec![valid, valid], "duplicate"),
        (vec![binding(0, &foreign)], "foreign"),
        (vec![binding(0, &wrong_usage)], "usage"),
        (
            vec![StorageBufferBinding { offset: 1, ..valid }],
            "alignment",
        ),
        (
            vec![StorageBufferBinding { size: 17, ..valid }],
            "alignment",
        ),
        (
            vec![StorageBufferBinding {
                offset: u64::MAX - 15,
                size: 32,
                ..valid
            }],
            "bounds",
        ),
        (
            vec![StorageBufferBinding {
                offset: 32,
                size: 16,
                ..valid
            }],
            "bounds",
        ),
        (
            vec![
                StorageBufferBinding { size: 16, ..valid },
                binding(1, &data),
            ],
            "short",
        ),
        (vec![valid], "missing"),
        (vec![valid, binding(1, &data)], "overlap"),
    ] {
        let error = encoder.dispatch(&p, [1, 1, 1], &bindings).unwrap_err();
        assert!(
            match kind {
                "slot" => matches!(error, ComputeEncodeError::InvalidBinding { .. }),
                "duplicate" => matches!(error, ComputeEncodeError::DuplicateBinding { .. }),
                "foreign" => matches!(error, ComputeEncodeError::ForeignBuffer { .. }),
                "usage" => matches!(error, ComputeEncodeError::MissingStorageUsage { .. }),
                "alignment" => matches!(error, ComputeEncodeError::UnalignedRange { .. }),
                "bounds" => matches!(error, ComputeEncodeError::OutOfBounds { .. }),
                "short" => matches!(error, ComputeEncodeError::InsufficientElements { .. }),
                "missing" => matches!(error, ComputeEncodeError::MissingBinding { .. }),
                "overlap" => matches!(error, ComputeEncodeError::OverlappingBindings { .. }),
                _ => false,
            },
            "{kind}: {error}"
        );
        assert!(encoder.commands().is_empty());
    }
    assert_eq!(read(&data), vec![[0.0; 4]; 2]);
}

#[test]
fn zero_dispatch_checks_contract_but_does_not_run_bad_shader() {
    let device = device();
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::Constant([f32::MAX; 4]),
            ShaderInstruction::Add { left: 0, right: 0 },
        ],
        &[1],
    );
    let p = pipeline(&device, &kernel, [1024, 1, 1], &[0]);
    let data = buffer(&device, 1);
    let mut encoder = device.create_command_encoder();
    assert!(matches!(
        encoder.dispatch(&p, [65535, 65535, 0], &[]),
        Err(ComputeEncodeError::MissingBinding { .. })
    ));
    encoder
        .dispatch(
            &p,
            [65535, 65535, 0],
            &[StorageBufferBinding {
                binding: 0,
                buffer: &data,
                offset: 16,
                size: 0,
            }],
        )
        .unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(read(&data), vec![[0.0; 4]]);
}

#[test]
fn in_place_live_reuse_and_multiple_dispatch_ordering_preserve_tail() {
    let device = device();
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::LoadStorage { binding: 3 },
            ShaderInstruction::Constant([2.0; 4]),
            ShaderInstruction::Multiply { left: 0, right: 1 },
        ],
        &[2],
    );
    let p = pipeline(&device, &kernel, [2, 1, 1], &[3]);
    let data = buffer(&device, 3);
    write(&data, &[[1.0, 2.0, 3.0, 4.0], [5.0; 4], [99.0; 4]]);
    let mut encoder = device.create_command_encoder();
    encoder
        .dispatch(&p, [1, 1, 1], &[binding(3, &data)])
        .unwrap();
    encoder
        .dispatch(&p, [1, 1, 1], &[binding(3, &data)])
        .unwrap();
    let commands = encoder.finish();
    drop(p);
    drop(kernel);
    assert_eq!(read(&data)[0], [1.0, 2.0, 3.0, 4.0]);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(
        read(&data),
        vec![[4.0, 8.0, 12.0, 16.0], [20.0; 4], [99.0; 4]]
    );
    write(&data, &[[3.0; 4], [4.0; 4], [88.0; 4]]);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(read(&data), vec![[12.0; 4], [16.0; 4], [88.0; 4]]);
    assert!(matches!(
        self::device().default_queue().submit(&commands),
        Err(QueueSubmitError::ForeignCommandBuffer)
    ));
}

#[test]
fn disjoint_shared_views_and_read_only_aliases_do_not_double_lock() {
    let device = device();
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::LoadStorage { binding: 0 },
            ShaderInstruction::LoadStorage { binding: 1 },
            ShaderInstruction::Add { left: 0, right: 1 },
        ],
        &[2],
    );
    let p = pipeline(&device, &kernel, [1, 1, 1], &[2]);
    let data = buffer(&device, 4);
    write(&data, &[[2.0; 4], [3.0; 4], [9.0; 4], [99.0; 4]]);
    let view = |binding, offset| StorageBufferBinding {
        binding,
        buffer: &data,
        offset,
        size: 32,
    };
    let mut encoder = device.create_command_encoder();
    encoder
        .dispatch(&p, [2, 1, 1], &[view(0, 0), view(1, 0), view(2, 32)])
        .unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(read(&data), vec![[2.0; 4], [3.0; 4], [4.0; 4], [6.0; 4]]);
}

#[test]
fn nonfinite_late_invocation_rolls_back_all_outputs_keeps_prefix_skips_suffix() {
    let device = device();
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::LoadStorage { binding: 0 },
            ShaderInstruction::Constant([2.0; 4]),
            ShaderInstruction::Multiply { left: 0, right: 1 },
        ],
        &[0, 2],
    );
    let p = pipeline(&device, &kernel, [1, 1, 1], &[1, 2]);
    let source = buffer(&device, 2);
    let first = buffer(&device, 2);
    let second = buffer(&device, 2);
    let suffix = buffer(&device, 2);
    let prefix = buffer(&device, 2);
    write(&prefix, &[[7.0; 4]; 2]);
    write(&first, &[[8.0; 4]; 2]);
    write(&second, &[[9.0; 4]; 2]);
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&prefix, 0, &first, 0, 32).unwrap();
    encoder
        .dispatch(
            &p,
            [2, 1, 1],
            &[binding(0, &source), binding(1, &first), binding(2, &second)],
        )
        .unwrap();
    encoder.copy_buffer(&prefix, 0, &suffix, 0, 32).unwrap();
    let commands = encoder.finish();
    for bad in [f32::MAX, f32::NAN, f32::INFINITY] {
        write(&source, &[[1.0; 4], [bad; 4]]);
        let error = device.default_queue().submit(&commands).unwrap_err();
        assert!(matches!(
            &error,
            QueueSubmitError::ComputeFailed {
                command_index: 1,
                source: ComputeExecutionError::ShaderFailed {
                    invocation: [1, 0, 0],
                    ..
                }
            }
        ));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(read(&first), vec![[7.0; 4]; 2]);
        assert_eq!(read(&second), vec![[9.0; 4]; 2]);
        assert_eq!(read(&suffix), vec![[0.0; 4]; 2]);
    }
    write(&source, &[[1.0; 4], [3.0; 4]]);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(read(&first), vec![[1.0; 4], [3.0; 4]]);
    assert_eq!(read(&second), vec![[2.0; 4], [6.0; 4]]);
    assert_eq!(read(&suffix), vec![[7.0; 4]; 2]);
}

#[test]
fn device_only_storage_copy_compute_copy_readback_and_retained_handles() {
    let device = device();
    let input = buffer(&device, 2);
    let readback = buffer(&device, 2);
    let storage = device
        .create_buffer(BufferDescriptor {
            label: "device only",
            size: 32,
            usages: &[
                BufferUsage::Storage,
                BufferUsage::TransferSource,
                BufferUsage::TransferDestination,
            ],
            host_access: HostAccess::None,
        })
        .unwrap();
    write(&input, &[[4.0; 4], [8.0; 4]]);
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::LoadStorage { binding: 0 },
            ShaderInstruction::Constant([0.5; 4]),
            ShaderInstruction::Multiply { left: 0, right: 1 },
        ],
        &[2],
    );
    let p = pipeline(&device, &kernel, [2, 1, 1], &[0]);
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&input, 0, &storage, 0, 32).unwrap();
    encoder
        .dispatch(&p, [1, 1, 1], &[binding(0, &storage)])
        .unwrap();
    encoder.copy_buffer(&storage, 0, &readback, 0, 32).unwrap();
    let commands = encoder.finish();
    drop(storage);
    drop(input);
    drop(p);
    drop(kernel);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(read(&readback), vec![[2.0; 4], [4.0; 4]]);
}

#[test]
fn eight_output_slots_offsets_and_unused_bytes() {
    let device = device();
    let kernel = shader(
        &device,
        &(0..8)
            .map(|i| ShaderInstruction::Constant([i as f32; 4]))
            .collect::<Vec<_>>(),
        &[0, 1, 2, 3, 4, 5, 6, 7],
    );
    let p = pipeline(&device, &kernel, [1, 1, 1], &[7, 6, 5, 4, 3, 2, 1, 0]);
    let data = buffer(&device, 10);
    write(&data, &[[99.0; 4]; 10]);
    let views: Vec<_> = (0..8)
        .map(|binding| StorageBufferBinding {
            binding,
            buffer: &data,
            offset: (binding as u64 + 1) * 16,
            size: 16,
        })
        .collect();
    let mut encoder = device.create_command_encoder();
    encoder.dispatch(&p, [1, 1, 1], &views).unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    let values = read(&data);
    assert_eq!(values[0], [99.0; 4]);
    assert_eq!(values[9], [99.0; 4]);
    for slot in 0..8 {
        assert_eq!(values[slot + 1], [(7 - slot) as f32; 4]);
    }
}

#[test]
fn computed_vertices_are_visible_to_following_programmable_draw() {
    let device = device();
    let vertices = buffer(&device, 3);
    write(
        &vertices,
        &[
            [0.0, 0.8, 0.0, 1.0],
            [-0.8, -0.8, 0.0, 1.0],
            [0.8, -0.8, 0.0, 1.0],
        ],
    );
    let kernel = shader(
        &device,
        &[
            ShaderInstruction::LoadStorage { binding: 0 },
            ShaderInstruction::Constant([0.5, 0.5, 1.0, 1.0]),
            ShaderInstruction::Multiply { left: 0, right: 1 },
        ],
        &[2],
    );
    let compute = pipeline(&device, &kernel, [1, 1, 1], &[0]);
    let vertex = device
        .create_shader(ShaderDescriptor {
            label: "position",
            stage: ShaderStage::Vertex,
            input_count: 1,
            instructions: &[ShaderInstruction::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let fragment = device
        .create_shader(ShaderDescriptor {
            label: "cyan",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &[ShaderInstruction::Constant([0.0, 1.0, 1.0, 1.0])],
            outputs: &[0],
        })
        .unwrap();
    let graphics = device
        .create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "computed vertices",
            vertex_shader: &vertex,
            fragment_shader: &fragment,
            vertex_layout: VertexBufferLayout {
                stride: 16,
                attributes: &[VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x4,
                }],
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    let target = device
        .create_framebuffer(FramebufferDescriptor {
            label: "compute draw",
            width: 16,
            height: 16,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    let mut encoder = device.create_command_encoder();
    encoder
        .dispatch(&compute, [3, 1, 1], &[binding(0, &vertices)])
        .unwrap();
    let mut pass = encoder.begin_render_pass(&target, Some([0; 4])).unwrap();
    pass.set_pipeline(&graphics).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(8, 8).unwrap(), [0, 255, 255, 255]);
    assert_eq!(target.pixel(2, 13).unwrap(), [0; 4]);
    assert_eq!(read(&vertices)[0], [0.0, 0.4, 0.0, 1.0]);
}

#[test]
fn invocation_limit_is_checked_without_large_execution() {
    let device = device();
    let kernel = shader(&device, &[ShaderInstruction::Input { location: 0 }], &[0]);
    let p = pipeline(&device, &kernel, [1024, 1, 1], &[0]);
    let data = buffer(&device, 1_048_576);
    let mut encoder = device.create_command_encoder();
    encoder
        .dispatch(&p, [1024, 1, 1], &[binding(0, &data)])
        .unwrap();
    assert!(matches!(
        encoder.dispatch(&p, [1025, 1, 1], &[binding(0, &data)]),
        Err(ComputeEncodeError::TooManyInvocations { .. })
    ));
    match &encoder.commands()[0] {
        Command::Dispatch(d) => assert_eq!(d.invocation_count(), 1_048_576),
        _ => panic!("expected dispatch"),
    }
}
