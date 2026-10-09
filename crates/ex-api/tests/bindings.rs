use ex::*;
use ShaderInstruction as Op;

#[test]
fn maximum_uniform_element_and_all_eight_sparse_slots_execute() {
    let d = device();
    let entries: Vec<_> = (0..8)
        .map(|slot| {
            (
                slot,
                ShaderVisibility::All,
                if slot == 1 {
                    BindingType::StorageBuffer { read_only: false }
                } else {
                    BindingType::UniformBuffer { min_size: 4096 }
                },
            )
        })
        .collect();
    let l = layout(&d, &entries);
    let b = buffer(&d, 4112);
    let out = buffer(&d, 16);
    write(&b, 4096, &[[9.; 4]]);
    let entries: Vec<_> = (0..8)
        .map(|slot| {
            if slot == 1 {
                storage(slot, &out, 0, 16)
            } else {
                u(slot, &b, 16, 4096)
            }
        })
        .collect();
    let g = group(&d, &l, &entries);
    let p = compute(
        &d,
        &l,
        &[Op::LoadUniform {
            binding: 7,
            element: 255,
        }],
        0,
    );
    let mut e = d.create_command_encoder();
    e.dispatch_with_bind_group(&p, [1; 3], &g).unwrap();
    let c = e.finish();
    assert_eq!(c.resource_uses().unwrap().len(), 2);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(read(&out)[0], [9.; 4]);
}
fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn buffer(d: &Device, size: u64) -> Buffer {
    d.create_buffer(BufferDescriptor {
        label: "bytes",
        size,
        usages: &[
            BufferUsage::Uniform,
            BufferUsage::Storage,
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
            BufferUsage::Vertex,
            BufferUsage::Index,
        ],
        host_access: HostAccess::ReadWrite,
    })
    .unwrap()
}
fn write(b: &Buffer, offset: u64, vectors: &[[f32; 4]]) {
    b.write(
        offset,
        &vectors
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
fn read(b: &Buffer) -> Vec<[f32; 4]> {
    let mut bytes = vec![0; b.size() as usize];
    b.read(0, &mut bytes).unwrap();
    bytes
        .as_chunks::<16>()
        .0
        .iter()
        .map(|r| {
            std::array::from_fn(|l| f32::from_le_bytes(r[l * 4..l * 4 + 4].try_into().unwrap()))
        })
        .collect()
}
fn layout(d: &Device, entries: &[(u32, ShaderVisibility, BindingType)]) -> BindGroupLayout {
    d.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "interface",
        entries: &entries
            .iter()
            .map(|&(binding, visibility, ty)| BindGroupLayoutEntry {
                binding,
                visibility,
                ty,
            })
            .collect::<Vec<_>>(),
    })
    .unwrap()
}
fn view(b: &Buffer, offset: u64, size: u64) -> BufferBinding {
    BufferBinding {
        buffer: b.clone(),
        offset,
        size,
    }
}
fn u(slot: u32, b: &Buffer, offset: u64, size: u64) -> BindGroupEntry {
    BindGroupEntry {
        binding: slot,
        resource: BindingResource::UniformBuffer(view(b, offset, size)),
    }
}
fn storage(slot: u32, b: &Buffer, offset: u64, size: u64) -> BindGroupEntry {
    BindGroupEntry {
        binding: slot,
        resource: BindingResource::StorageBuffer(view(b, offset, size)),
    }
}
fn group(d: &Device, l: &BindGroupLayout, entries: &[BindGroupEntry]) -> BindGroup {
    d.create_bind_group(BindGroupDescriptor {
        label: "resources",
        layout: l,
        entries,
    })
    .unwrap()
}
fn shader(d: &Device, stage: ShaderStage, ops: &[Op], output: u32) -> Shader {
    d.create_shader(ShaderDescriptor {
        label: "s",
        stage,
        input_count: if stage == ShaderStage::Compute {
            4
        } else if stage == ShaderStage::Vertex {
            1
        } else {
            0
        },
        instructions: ops,
        outputs: &[output],
    })
    .unwrap()
}
fn compute(d: &Device, l: &BindGroupLayout, ops: &[Op], output: u32) -> ComputePipeline {
    let s = shader(d, ShaderStage::Compute, ops, output);
    d.create_compute_pipeline_with_bindings(
        ComputePipelineDescriptor {
            label: "c",
            shader: &s,
            workgroup_size: [1; 3],
            output_bindings: &[1],
        },
        l,
    )
    .unwrap()
}
fn compute_layout(d: &Device, minimum: u64) -> BindGroupLayout {
    layout(
        d,
        &[
            (
                0,
                ShaderVisibility::All,
                BindingType::UniformBuffer { min_size: minimum },
            ),
            (
                1,
                ShaderVisibility::Compute,
                BindingType::StorageBuffer { read_only: false },
            ),
        ],
    )
}
fn target(d: &Device) -> Framebuffer {
    d.create_framebuffer(FramebufferDescriptor {
        label: "target",
        width: 16,
        height: 8,
        format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn graphics(d: &Device, l: &BindGroupLayout) -> GraphicsPipeline {
    let v = shader(
        d,
        ShaderStage::Vertex,
        &[
            Op::Input { location: 0 },
            Op::LoadUniform {
                binding: 0,
                element: 0,
            },
            Op::Add { left: 0, right: 1 },
        ],
        2,
    );
    let f = shader(
        d,
        ShaderStage::Fragment,
        &[Op::LoadUniform {
            binding: 0,
            element: 1,
        }],
        0,
    );
    d.create_graphics_pipeline_with_bindings(
        GraphicsPipelineDescriptor {
            label: "g",
            vertex_shader: &v,
            fragment_shader: &f,
            vertex_layout: VertexBufferLayout {
                stride: 16,
                attributes: &[VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x4,
                }],
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        },
        GraphicsPipelineOptions::default(),
        None,
        l,
    )
    .unwrap()
}
fn vertices(d: &Device) -> Buffer {
    let b = buffer(d, 96);
    write(
        &b,
        0,
        &[
            [-0.25, -0.75, 0., 1.],
            [0.25, -0.75, 0., 1.],
            [0.25, 0.75, 0., 1.],
            [-0.25, -0.75, 0., 1.],
            [0.25, 0.75, 0., 1.],
            [-0.25, 0.75, 0., 1.],
        ],
    );
    b
}

#[test]
fn layouts_own_sorted_entries_and_compare_structure_on_one_device() {
    let d = device();
    let mut entries = vec![
        BindGroupLayoutEntry {
            binding: 7,
            visibility: ShaderVisibility::All,
            ty: BindingType::UniformBuffer { min_size: 16 },
        },
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderVisibility::Compute,
            ty: BindingType::StorageBuffer { read_only: true },
        },
    ];
    let a = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "a",
            entries: &entries,
        })
        .unwrap();
    entries.reverse();
    let b = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "b",
            entries: &entries,
        })
        .unwrap();
    assert!(a.is_compatible(&b));
    assert!(!a.is_same(&b));
    assert!(a.is_same(&a.clone()));
    assert_eq!(a.entries()[0].binding, 0);
    entries.clear();
    assert_eq!(a.entries().len(), 2);
    assert!(a.belongs_to(&d));
    assert!(a.device().is_same(&d));
    assert_eq!(a.label(), "a");
    let foreign = device()
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "a",
            entries: a.entries(),
        })
        .unwrap();
    assert!(!a.is_compatible(&foreign));
    for (slot, size) in [(8, 16), (0, 0), (0, 15), (0, 17), (0, 4112)] {
        assert!(d
            .create_bind_group_layout(BindGroupLayoutDescriptor {
                label: "bad",
                entries: &[BindGroupLayoutEntry {
                    binding: slot,
                    visibility: ShaderVisibility::All,
                    ty: BindingType::UniformBuffer { min_size: size }
                }]
            })
            .is_err());
    }
    assert!(d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "duplicate",
            entries: &[a.entries()[0]; 2]
        })
        .is_err());
    let empty = layout(&d, &[]);
    assert!(group(&d, &empty, &[]).entries().is_empty());
}

#[test]
fn visibility_covers_exact_supported_stages() {
    for (visibility, expected) in [
        (ShaderVisibility::Vertex, [true, false, false]),
        (ShaderVisibility::Fragment, [false, true, false]),
        (ShaderVisibility::Graphics, [true, true, false]),
        (ShaderVisibility::Compute, [false, false, true]),
        (ShaderVisibility::All, [true; 3]),
    ] {
        for (stage, want) in [
            ShaderStage::Vertex,
            ShaderStage::Fragment,
            ShaderStage::Compute,
        ]
        .into_iter()
        .zip(expected)
        {
            assert_eq!(visibility.contains(stage), want);
        }
    }
}

#[test]
fn groups_validate_exact_slots_devices_types_permissions_and_ranges() {
    let d = device();
    let l = layout(
        &d,
        &[(
            0,
            ShaderVisibility::All,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let b = buffer(&d, 8192);
    let create = |entries: &[BindGroupEntry]| {
        d.create_bind_group(BindGroupDescriptor {
            label: "g",
            layout: &l,
            entries,
        })
    };
    for entries in [
        vec![],
        vec![u(1, &b, 0, 32)],
        vec![u(0, &b, 0, 32); 2],
        vec![storage(0, &b, 0, 32)],
    ] {
        assert!(create(&entries).is_err());
    }
    for (offset, size) in [
        (1, 32),
        (0, 0),
        (0, 17),
        (0, 16),
        (8192, 32),
        (u64::MAX - 15, 32),
        (0, 4112),
    ] {
        assert!(create(&[u(0, &b, offset, size)]).is_err());
    }
    assert!(create(&[u(0, &buffer(&device(), 32), 0, 32)]).is_err());
    let wrong = d
        .create_buffer(BufferDescriptor {
            label: "wrong",
            size: 32,
            usages: &[BufferUsage::Storage],
            host_access: HostAccess::ReadWrite,
        })
        .unwrap();
    assert!(matches!(
        create(&[u(0, &wrong, 0, 32)]),
        Err(BindingCreateError::InvalidDescriptor(
            BindingValidationError::MissingUsage {
                usage: BufferUsage::Uniform,
                ..
            }
        ))
    ));
    let g = create(&[u(0, &b, 16, 32)]).unwrap();
    assert!(g.belongs_to(&d));
    assert!(g.device().is_same(&d));
    assert_eq!(g.label(), "g");
    assert!(g.is_same(&g.clone()));
    assert!(g.layout().is_same(&l));
    let error = device()
        .create_bind_group(BindGroupDescriptor {
            label: "foreign",
            layout: &l,
            entries: &[],
        })
        .unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn uniform_instructions_validate_bounds_in_all_stages_and_need_bound_execution() {
    let d = device();
    for stage in [
        ShaderStage::Vertex,
        ShaderStage::Fragment,
        ShaderStage::Compute,
    ] {
        let s = shader(
            &d,
            stage,
            &[Op::LoadUniform {
                binding: 7,
                element: 255,
            }],
            0,
        );
        assert!(matches!(
            s.evaluate(&vec![[0.; 4]; s.input_count() as usize]),
            Err(ShaderExecutionError::MissingUniformBinding {
                binding: 7,
                element: 255
            })
        ));
        for (binding, element) in [(8, 0), (0, 256)] {
            assert!(matches!(
                d.create_shader(ShaderDescriptor {
                    label: "bad",
                    stage,
                    input_count: s.input_count(),
                    instructions: &[Op::LoadUniform { binding, element }],
                    outputs: &[0]
                }),
                Err(ShaderCreateError::InvalidDescriptor(
                    ShaderValidationError::InvalidUniformLoad { .. }
                ))
            ));
        }
    }
}

#[test]
fn pipeline_interfaces_check_uniform_minima_stages_types_and_writable_outputs() {
    let d = device();
    let s = shader(
        &d,
        ShaderStage::Compute,
        &[Op::LoadUniform {
            binding: 0,
            element: 1,
        }],
        0,
    );
    let desc = ComputePipelineDescriptor {
        label: "p",
        shader: &s,
        workgroup_size: [1; 3],
        output_bindings: &[1],
    };
    assert!(matches!(
        d.create_compute_pipeline(desc),
        Err(ComputePipelineCreateError::Bindings {
            source: BindingValidationError::LayoutRequired
        })
    ));
    for uniform in [
        (
            ShaderVisibility::Compute,
            BindingType::UniformBuffer { min_size: 16 },
        ),
        (
            ShaderVisibility::Vertex,
            BindingType::UniformBuffer { min_size: 32 },
        ),
        (ShaderVisibility::All, BindingType::SampledTexture),
    ] {
        let l = layout(
            &d,
            &[
                (0, uniform.0, uniform.1),
                (
                    1,
                    ShaderVisibility::Compute,
                    BindingType::StorageBuffer { read_only: false },
                ),
            ],
        );
        assert!(d.create_compute_pipeline_with_bindings(desc, &l).is_err());
    }
    let l = layout(
        &d,
        &[
            (
                0,
                ShaderVisibility::Compute,
                BindingType::UniformBuffer { min_size: 32 },
            ),
            (
                1,
                ShaderVisibility::Compute,
                BindingType::StorageBuffer { read_only: true },
            ),
        ],
    );
    assert!(matches!(
        d.create_compute_pipeline_with_bindings(desc, &l),
        Err(ComputePipelineCreateError::Bindings {
            source: BindingValidationError::ReadOnlyOutput { binding: 1 }
        })
    ));
    assert!(d
        .create_compute_pipeline_with_bindings(desc, &compute_layout(&device(), 32))
        .is_err());
    let p = d
        .create_compute_pipeline_with_bindings(desc, &compute_layout(&d, 32))
        .unwrap();
    assert!(p.binding_layout().is_some());
    let collision = shader(
        &d,
        ShaderStage::Compute,
        &[Op::LoadUniform {
            binding: 1,
            element: 0,
        }],
        0,
    );
    assert!(d
        .create_compute_pipeline_with_bindings(
            ComputePipelineDescriptor {
                shader: &collision,
                ..desc
            },
            &compute_layout(&d, 32)
        )
        .is_err());
}

#[test]
fn compute_groups_use_relative_views_late_parameters_and_retained_resources() {
    let d = device();
    let l = compute_layout(&d, 32);
    let b = buffer(&d, 64);
    let out = buffer(&d, 32);
    let g = group(&d, &l, &[u(0, &b, 16, 32), storage(1, &out, 0, 32)]);
    let p = compute(
        &d,
        &l,
        &[
            Op::LoadUniform {
                binding: 0,
                element: 1,
            },
            Op::Input { location: 0 },
            Op::Add { left: 0, right: 1 },
        ],
        2,
    );
    let mut e = d.create_command_encoder();
    assert!(matches!(
        e.dispatch(&p, [1; 3], &[]),
        Err(ComputeEncodeError::Bindings {
            source: BindingValidationError::MissingGroup
        })
    ));
    e.dispatch_with_bind_group(&p, [2, 1, 1], &g).unwrap();
    let commands = e.finish();
    if let Command::Dispatch(dispatch) = &commands.commands()[0] {
        assert!(dispatch.bind_group().unwrap().is_same(&g));
    }
    drop(g);
    drop(l);
    drop(p);
    for value in [2., 7.] {
        write(&b, 32, &[[value; 4]]);
        d.default_queue().submit(&commands).unwrap();
        assert_eq!(
            read(&out),
            vec![
                [value, value, value, value],
                [value + 1., value, value, value + 1.]
            ]
        );
    }
}

#[test]
fn compute_aliases_reject_uniform_writers_but_accept_disjoint_and_read_only_views() {
    let d = device();
    let l = compute_layout(&d, 16);
    let b = buffer(&d, 48);
    write(&b, 0, &[[3.; 4], [5.; 4], [9.; 4]]);
    let p = compute(
        &d,
        &l,
        &[Op::LoadUniform {
            binding: 0,
            element: 0,
        }],
        0,
    );
    let bad = group(&d, &l, &[u(0, &b, 0, 16), storage(1, &b, 0, 16)]);
    let mut e = d.create_command_encoder();
    assert!(matches!(
        e.dispatch_with_bind_group(&p, [1; 3], &bad),
        Err(ComputeEncodeError::Bindings {
            source: BindingValidationError::OverlappingUniformStorage { .. }
        })
    ));
    assert!(e.commands().is_empty());
    let good = group(&d, &l, &[u(0, &b, 0, 16), storage(1, &b, 16, 16)]);
    e.dispatch_with_bind_group(&p, [1; 3], &good).unwrap();
    let commands = e.finish();
    assert_eq!(
        commands.resource_uses().unwrap()[0].access,
        ResourceAccess::StorageReadWrite
    );
    d.default_queue().submit(&commands).unwrap();
    assert_eq!(read(&b)[1], [3.; 4]);
    let l = layout(
        &d,
        &[
            (
                0,
                ShaderVisibility::Compute,
                BindingType::UniformBuffer { min_size: 16 },
            ),
            (
                1,
                ShaderVisibility::Compute,
                BindingType::StorageBuffer { read_only: false },
            ),
            (
                2,
                ShaderVisibility::Compute,
                BindingType::StorageBuffer { read_only: true },
            ),
        ],
    );
    let out = buffer(&d, 16);
    let g = group(
        &d,
        &l,
        &[
            u(0, &b, 0, 16),
            storage(1, &out, 0, 16),
            storage(2, &b, 0, 16),
        ],
    );
    let p = compute(
        &d,
        &l,
        &[
            Op::LoadUniform {
                binding: 0,
                element: 0,
            },
            Op::LoadStorage { binding: 2 },
            Op::Add { left: 0, right: 1 },
        ],
        2,
    );
    let mut e = d.create_command_encoder();
    e.dispatch_with_bind_group(&p, [1; 3], &g).unwrap();
    let c = e.finish();
    assert_eq!(
        c.resource_uses().unwrap()[1].access,
        ResourceAccess::BufferRead
    );
    d.default_queue().submit(&c).unwrap();
    assert_eq!(read(&out)[0], [6.; 4]);
}

#[test]
fn graphics_captures_per_draw_groups_for_vertex_and_fragment_parameters() {
    let d = device();
    let l = layout(
        &d,
        &[(
            0,
            ShaderVisibility::Graphics,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let p = graphics(&d, &l);
    let v = vertices(&d);
    let t = target(&d);
    let a = buffer(&d, 32);
    let b = buffer(&d, 32);
    let ga = group(&d, &l, &[u(0, &a, 0, 32)]);
    let gb = group(&d, &l, &[u(0, &b, 0, 32)]);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([0; 4])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..6),
        Err(RenderEncodeError::Bindings {
            source: BindingValidationError::MissingGroup
        })
    ));
    pass.set_bind_group(&ga).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.set_bind_group(&gb).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    if let Command::RenderPass(r) = &c.commands()[0] {
        assert!(r.draws()[0].bind_group().unwrap().is_same(&ga));
        assert!(r.draws()[1].bind_group().unwrap().is_same(&gb));
    }
    write(&a, 0, &[[-0.5, 0., 0., 0.], [1., 0., 0., 1.]]);
    write(&b, 0, &[[0.5, 0., 0., 0.], [0., 1., 0., 1.]]);
    drop(ga);
    drop(gb);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(t.pixel(4, 4).unwrap(), [255, 0, 0, 255]);
    assert_eq!(t.pixel(12, 4).unwrap(), [0, 255, 0, 255]);
    write(&a, 16, &[[0., 0., 1., 1.]]);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(t.pixel(4, 4).unwrap(), [0, 0, 255, 255]);
}

#[test]
fn group_compatibility_and_failed_setters_preserve_prior_recording_state() {
    let d = device();
    let l = layout(
        &d,
        &[(
            0,
            ShaderVisibility::Graphics,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let l2 = layout(
        &d,
        &[(
            0,
            ShaderVisibility::Graphics,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let bad = compute_layout(&d, 32);
    let p = graphics(&d, &l);
    let b = buffer(&d, 32);
    let v = vertices(&d);
    let t = target(&d);
    let g = group(&d, &l2, &[u(0, &b, 0, 32)]);
    let wrong = group(&d, &bad, &[u(0, &b, 0, 32), storage(1, &b, 0, 32)]);
    let f = device();
    let fl = layout(&f, &[]);
    let foreign = group(&f, &fl, &[]);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_bind_group(&g).unwrap();
    assert!(pass.set_bind_group(&foreign).is_err());
    pass.draw_programmable(0..0).unwrap();
    pass.set_bind_group(&wrong).unwrap();
    assert!(pass.draw_programmable(0..0).is_err());
    pass.set_bind_group(&g).unwrap();
    pass.draw_programmable(0..0).unwrap();
    pass.finish().unwrap();
    if let Command::RenderPass(r) = &e.commands()[0] {
        assert_eq!(r.draws().len(), 2);
    }
}

#[test]
fn nonfinite_loaded_parameters_roll_back_scope_and_dispatch_but_unused_elements_are_allowed() {
    let d = device();
    let l = compute_layout(&d, 32);
    let b = buffer(&d, 32);
    let out = buffer(&d, 16);
    let p = compute(
        &d,
        &l,
        &[Op::LoadUniform {
            binding: 0,
            element: 0,
        }],
        0,
    );
    let g = group(&d, &l, &[u(0, &b, 0, 32), storage(1, &out, 0, 16)]);
    write(&b, 0, &[[2.; 4], [f32::NAN; 4]]);
    let mut e = d.create_command_encoder();
    e.dispatch_with_bind_group(&p, [1; 3], &g).unwrap();
    let c = e.finish();
    d.default_queue().submit(&c).unwrap();
    assert_eq!(read(&out)[0], [2.; 4]);
    write(&b, 0, &[[f32::INFINITY; 4]]);
    assert!(matches!(
        d.default_queue().submit(&c),
        Err(QueueSubmitError::ComputeFailed { .. })
    ));
    assert_eq!(read(&out)[0], [2.; 4]);
    let l = layout(
        &d,
        &[(
            0,
            ShaderVisibility::Graphics,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let p = graphics(&d, &l);
    let v = vertices(&d);
    let t = target(&d);
    let g = group(&d, &l, &[u(0, &b, 0, 32)]);
    write(&b, 0, &[[0.; 4], [1.; 4]]);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([7; 4])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_bind_group(&g).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    d.default_queue().submit(&c).unwrap();
    let old = t.pixel(8, 4).unwrap();
    write(&b, 16, &[[f32::NAN; 4]]);
    assert!(matches!(
        d.default_queue().submit(&c),
        Err(QueueSubmitError::RenderFailed {
            source: RenderExecutionError::NonFiniteShaderResult {
                stage: ShaderStage::Fragment,
                ..
            },
            ..
        })
    ));
    assert_eq!(t.pixel(8, 4).unwrap(), old);
    assert_eq!(t.pixel(0, 0).unwrap(), [7; 4]);
}

#[test]
fn copied_parameters_require_explicit_uniform_dependency_and_allow_completed_reuse() {
    let d = device();
    let l = compute_layout(&d, 16);
    let src = buffer(&d, 16);
    let params = buffer(&d, 16);
    let out = buffer(&d, 16);
    let g = group(&d, &l, &[u(0, &params, 0, 16), storage(1, &out, 0, 16)]);
    let p = compute(
        &d,
        &l,
        &[Op::LoadUniform {
            binding: 0,
            element: 0,
        }],
        0,
    );
    write(&src, 0, &[[8.; 4]]);
    for barrier in [false, true] {
        let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
        e.transition_resources(&[
            ResourceTransition {
                resource: Resource::Buffer(src.clone()),
                before: None,
                after: ResourceAccess::TransferRead,
            },
            ResourceTransition {
                resource: Resource::Buffer(params.clone()),
                before: None,
                after: ResourceAccess::TransferWrite,
            },
            ResourceTransition {
                resource: Resource::Buffer(out.clone()),
                before: None,
                after: ResourceAccess::StorageWrite,
            },
        ])
        .unwrap();
        e.copy_buffer(&src, 0, &params, 0, 16).unwrap();
        if barrier {
            e.transition_resources(&[ResourceTransition {
                resource: Resource::Buffer(params.clone()),
                before: Some(ResourceAccess::TransferWrite),
                after: ResourceAccess::UniformRead,
            }])
            .unwrap();
        }
        e.dispatch_with_bind_group(&p, [1; 3], &g).unwrap();
        let c = e.finish();
        assert!(c
            .resource_uses()
            .unwrap()
            .iter()
            .any(|u| u.access == ResourceAccess::UniformRead));
        if barrier {
            d.default_queue().submit(&c).unwrap();
            assert_eq!(read(&out)[0], [8.; 4]);
        } else {
            assert!(matches!(
                d.default_queue().submit(&c),
                Err(QueueSubmitError::DependencyFailed { .. })
            ));
            assert_eq!(read(&params)[0], [0.; 4]);
        }
    }
}

#[test]
fn zero_operations_and_unused_layout_slots_do_not_read_parameters() {
    let d = device();
    let l = layout(
        &d,
        &[
            (
                0,
                ShaderVisibility::All,
                BindingType::UniformBuffer { min_size: 16 },
            ),
            (
                1,
                ShaderVisibility::Compute,
                BindingType::StorageBuffer { read_only: false },
            ),
            (
                7,
                ShaderVisibility::All,
                BindingType::UniformBuffer { min_size: 16 },
            ),
        ],
    );
    let b = buffer(&d, 16);
    let out = buffer(&d, 16);
    let unused = buffer(&d, 16);
    let g = group(
        &d,
        &l,
        &[
            u(0, &b, 0, 16),
            storage(1, &out, 0, 16),
            u(7, &unused, 0, 16),
        ],
    );
    let p = compute(
        &d,
        &l,
        &[Op::LoadUniform {
            binding: 0,
            element: 0,
        }],
        0,
    );
    write(&b, 0, &[[f32::NAN; 4]]);
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.dispatch_with_bind_group(&p, [0, 1, 1], &g).unwrap();
    let c = e.finish();
    assert!(c.resource_uses().unwrap().is_empty());
    d.default_queue().submit(&c).unwrap();
    let p = compute(&d, &l, &[Op::Constant([4.; 4])], 0);
    let mut e = d.create_command_encoder();
    e.dispatch_with_bind_group(&p, [1; 3], &g).unwrap();
    let c = e.finish();
    assert_eq!(c.resource_uses().unwrap().len(), 1);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(read(&out)[0], [4.; 4]);
}

#[test]
fn indexed_graphics_uniform_vertex_alias_is_combined_read_only() {
    let d = device();
    let l = layout(
        &d,
        &[(
            0,
            ShaderVisibility::Graphics,
            BindingType::UniformBuffer { min_size: 32 },
        )],
    );
    let p = graphics(&d, &l);
    let b = buffer(&d, 160);
    let v = vertices(&d);
    let mut bytes = [0; 96];
    v.read(0, &mut bytes).unwrap();
    b.write(0, &bytes).unwrap();
    write(&b, 96, &[[0.; 4], [1.; 4]]);
    b.write(128, &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]).unwrap();
    let g = group(&d, &l, &[u(0, &b, 96, 32)]);
    let t = target(&d);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&b, 0).unwrap();
    pass.set_index_buffer(&b, 128, IndexFormat::Uint16).unwrap();
    pass.set_bind_group(&g).unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    assert_eq!(
        c.resource_uses().unwrap()[1].access,
        ResourceAccess::BufferRead
    );
    d.default_queue().submit(&c).unwrap();
    assert_eq!(t.pixel(8, 4).unwrap(), [255; 4]);
}

#[test]
fn paired_texture_groups_drive_fragment_sampling_and_check_foreign_samplers() {
    let d = device();
    let l = layout(
        &d,
        &[
            (
                0,
                ShaderVisibility::Graphics,
                BindingType::UniformBuffer { min_size: 16 },
            ),
            (2, ShaderVisibility::Fragment, BindingType::SampledTexture),
        ],
    );
    let params = buffer(&d, 16);
    write(&params, 0, &[[0.; 4]]);
    let tex = d
        .create_texture(TextureDescriptor {
            label: "tex",
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::Write,
        })
        .unwrap();
    tex.write_pixels(&[[12, 34, 56, 255]]).unwrap();
    let desc = SamplerDescriptor {
        label: "s",
        filter: FilterMode::Nearest,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
    };
    let sampler = d.create_sampler(desc).unwrap();
    let entry = |s: Sampler| BindGroupEntry {
        binding: 2,
        resource: BindingResource::SampledTexture {
            view: tex.view(),
            sampler: s,
        },
    };
    assert!(d
        .create_bind_group(BindGroupDescriptor {
            label: "bad",
            layout: &l,
            entries: &[
                u(0, &params, 0, 16),
                entry(device().create_sampler(desc).unwrap())
            ]
        })
        .is_err());
    let g = group(&d, &l, &[u(0, &params, 0, 16), entry(sampler)]);
    let v = shader(&d, ShaderStage::Vertex, &[Op::Input { location: 0 }], 0);
    let f = shader(
        &d,
        ShaderStage::Fragment,
        &[
            Op::LoadUniform {
                binding: 0,
                element: 0,
            },
            Op::SampleTexture {
                binding: 2,
                coordinates: 0,
            },
        ],
        1,
    );
    let p = d
        .create_graphics_pipeline_with_bindings(
            GraphicsPipelineDescriptor {
                label: "sample",
                vertex_shader: &v,
                fragment_shader: &f,
                vertex_layout: VertexBufferLayout {
                    stride: 16,
                    attributes: &[VertexAttribute {
                        location: 0,
                        offset: 0,
                        format: VertexFormat::Float32x4,
                    }],
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            },
            GraphicsPipelineOptions::default(),
            None,
            &l,
        )
        .unwrap();
    let vb = vertices(&d);
    let t = target(&d);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vb, 0).unwrap();
    pass.set_bind_group(&g).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    assert_eq!(
        c.resource_uses().unwrap().last().unwrap().access,
        ResourceAccess::SampledRead
    );
    d.default_queue().submit(&c).unwrap();
    assert_eq!(t.pixel(8, 4).unwrap(), [12, 34, 56, 255]);
}
