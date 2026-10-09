use ex::*;
use ResourceAccess as A;

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn buffer(d: &Device) -> Buffer {
    d.create_buffer(BufferDescriptor {
        label: "same label",
        size: 64,
        usages: &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
            BufferUsage::Vertex,
            BufferUsage::Index,
            BufferUsage::Storage,
        ],
        host_access: HostAccess::ReadWrite,
    })
    .unwrap()
}
fn encoder(d: &Device) -> CommandEncoder {
    d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit)
}
fn t(b: &Buffer, before: Option<ResourceAccess>, after: ResourceAccess) -> ResourceTransition {
    ResourceTransition {
        resource: Resource::Buffer(b.clone()),
        before,
        after,
    }
}
fn declare(e: &mut CommandEncoder, buffers: &[&Buffer]) {
    e.transition_resources(
        &buffers
            .iter()
            .map(|b| t(b, None, A::TransferReadWrite))
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
fn read(b: &Buffer) -> [u8; 64] {
    let mut data = [0; 64];
    b.read(0, &mut data).unwrap();
    data
}
fn kernel(d: &Device, ops: &[ShaderInstruction], output: u32) -> ComputePipeline {
    let s = d
        .create_shader(ShaderDescriptor {
            label: "k",
            stage: ShaderStage::Compute,
            input_count: 4,
            instructions: ops,
            outputs: &[output],
        })
        .unwrap();
    d.create_compute_pipeline(ComputePipelineDescriptor {
        label: "p",
        shader: &s,
        workgroup_size: [1; 3],
        output_bindings: &[1],
    })
    .unwrap()
}
fn bind(slot: u32, b: &Buffer, offset: u64) -> StorageBufferBinding<'_> {
    StorageBufferBinding {
        binding: slot,
        buffer: b,
        offset,
        size: 16,
    }
}

#[test]
fn modes_and_empty_batches_preserve_legacy_behavior() {
    let d = device();
    let mut e = d.create_command_encoder();
    assert_eq!(SynchronizationMode::default(), SynchronizationMode::Ordered);
    assert_eq!(e.synchronization_mode(), SynchronizationMode::Ordered);
    assert!(matches!(
        e.transition_resources(&[]),
        Err(DependencyEncodeError::ExplicitModeRequired)
    ));
    let a = buffer(&d);
    let b = buffer(&d);
    e.copy_buffer(&a, 0, &b, 0, 4).unwrap();
    e.copy_buffer(&b, 0, &a, 0, 4).unwrap();
    let c = e.finish();
    c.validate_dependencies().unwrap();
    d.default_queue().submit(&c).unwrap();
    let mut e = encoder(&d);
    e.transition_resources(&[]).unwrap();
    assert!(e.commands().is_empty());
    assert_eq!(
        e.finish().synchronization_mode(),
        SynchronizationMode::Explicit
    );
}

#[test]
fn batches_validate_devices_roles_and_identity_atomically() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    let foreign = buffer(&device());
    let mut e = encoder(&d);
    for bad in [
        t(&foreign, None, A::TransferRead),
        t(&b, None, A::SampledRead),
        t(&b, Some(A::ColourReadWrite), A::TransferRead),
    ] {
        assert!(e
            .transition_resources(&[t(&a, None, A::TransferRead), bad])
            .is_err());
        assert!(e.commands().is_empty());
    }
    assert!(matches!(
        e.transition_resources(&[
            t(&a, None, A::TransferRead),
            t(&a.clone(), None, A::TransferWrite)
        ]),
        Err(DependencyEncodeError::DuplicateResource { .. })
    ));
    // Distinct allocations with identical labels are independent.
    declare(&mut e, &[&a, &b]);
    let r = Resource::Buffer(a.clone());
    assert_eq!(r.kind(), ResourceKind::Buffer);
    assert_eq!(r.label(), "same label");
    assert!(r.belongs_to(&d));
    assert!(r.device().is_same(&d));
    assert!(r.is_same(&Resource::Buffer(a.clone())));
    assert!(!r.is_same(&Resource::Buffer(b)));
    if let Command::ResourceTransitions(batch) = &e.commands()[0] {
        assert_eq!(batch.transitions().len(), 2);
    } else {
        panic!("expected transitions");
    }
}

#[test]
fn buffer_roles_require_creation_permissions() {
    let d = device();
    let roles = [
        A::TransferRead,
        A::TransferWrite,
        A::TransferReadWrite,
        A::VertexRead,
        A::IndexRead,
        A::UniformRead,
        A::StorageRead,
        A::StorageWrite,
        A::StorageReadWrite,
        A::BufferRead,
    ];
    for usage in [
        BufferUsage::TransferSource,
        BufferUsage::TransferDestination,
        BufferUsage::Vertex,
        BufferUsage::Index,
        BufferUsage::Storage,
        BufferUsage::Uniform,
    ] {
        let b = d
            .create_buffer(BufferDescriptor {
                label: "one role",
                size: 16,
                usages: &[usage],
                host_access: HostAccess::None,
            })
            .unwrap();
        for role in roles {
            let expected = match role {
                A::TransferRead => usage == BufferUsage::TransferSource,
                A::TransferWrite => usage == BufferUsage::TransferDestination,
                A::TransferReadWrite => false,
                A::VertexRead => usage == BufferUsage::Vertex,
                A::IndexRead => usage == BufferUsage::Index,
                A::UniformRead => usage == BufferUsage::Uniform,
                A::StorageRead | A::StorageWrite | A::StorageReadWrite => {
                    usage == BufferUsage::Storage
                }
                A::BufferRead => usage != BufferUsage::TransferDestination,
                _ => unreachable!(),
            };
            assert_eq!(
                encoder(&d)
                    .transition_resources(&[t(&b, None, role)])
                    .is_ok(),
                expected
            );
        }
    }
    for a in roles {
        assert!(a.is_read() || a.is_write());
    }
    assert!(!A::StorageWrite.is_read());
    assert!(!A::BufferRead.is_write());
}

#[test]
fn missing_declarations_reject_submission_before_any_execution() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    a.write(0, &[7; 64]).unwrap();
    let mut e = encoder(&d);
    e.copy_buffer(&a, 0, &b, 0, 64).unwrap();
    let c = e.finish();
    assert!(matches!(
        c.validate_dependencies(),
        Err(DependencyValidationError::MissingTransition {
            command_index: 0,
            required: A::TransferRead,
            ..
        })
    ));
    let error = d.default_queue().submit(&c).unwrap_err();
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(read(&b), [0; 64]);
    assert_eq!(
        device().default_queue().submit(&c),
        Err(QueueSubmitError::ForeignCommandBuffer)
    );
}

#[test]
fn raw_war_waw_are_reported_with_prior_command_context() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    let c = buffer(&d);
    for (first, second, hazard) in [
        ((&a, &b), (&b, &c), HazardKind::ReadAfterWrite),
        ((&b, &a), (&c, &b), HazardKind::WriteAfterRead),
        ((&a, &b), (&c, &b), HazardKind::WriteAfterWrite),
    ] {
        let mut e = encoder(&d);
        declare(&mut e, &[&a, &b, &c]);
        e.copy_buffer(first.0, 0, first.1, 0, 4).unwrap();
        e.copy_buffer(second.0, 0, second.1, 0, 4).unwrap();
        assert!(
            matches!(e.finish().validate_dependencies(), Err(DependencyValidationError::MissingDependency {
            command_index: 2, previous_command_index: 1, hazard: h, .. }) if h == hazard)
        );
    }
}

#[test]
fn dependency_preflight_preserves_prefix_and_valid_barriers_allow_reuse() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    let c = buffer(&d);
    a.write(0, &[7; 64]).unwrap();
    b.write(0, &[9; 64]).unwrap();
    for fix in [false, true] {
        let mut e = encoder(&d);
        declare(&mut e, &[&a, &b, &c]);
        e.copy_buffer(&a, 0, &b, 0, 64).unwrap();
        e.transition_resources(&[t(
            if fix { &b } else { &a },
            Some(A::TransferReadWrite),
            A::TransferReadWrite,
        )])
        .unwrap();
        e.copy_buffer(&b, 0, &c, 0, 64).unwrap();
        let commands = e.finish();
        if !fix {
            assert!(d.default_queue().submit(&commands).is_err());
            assert_eq!(read(&b), [9; 64]);
            assert_eq!(read(&c), [0; 64]);
        } else {
            for byte in [7, 3] {
                a.write(0, &[byte; 64]).unwrap();
                d.default_queue().submit(&commands).unwrap();
                assert_eq!(read(&c), [byte; 64]);
            }
        }
    }
}

#[test]
fn transition_state_and_role_errors_are_distinct() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    for (first, second, actual) in [
        (Some(A::TransferRead), None, None),
        (None, None, Some(A::TransferRead)),
        (None, Some(A::StorageRead), Some(A::TransferRead)),
    ] {
        let mut e = encoder(&d);
        e.transition_resources(&[t(&a, first, A::TransferRead)])
            .unwrap();
        if first.is_none() {
            e.transition_resources(&[t(&a, second, A::TransferRead)])
                .unwrap();
        }
        assert!(
            matches!(e.finish().validate_dependencies(), Err(DependencyValidationError::TransitionMismatch { actual: v, .. }) if v == actual)
        );
    }
    let mut e = encoder(&d);
    e.transition_resources(&[t(&a, None, A::StorageRead), t(&b, None, A::TransferWrite)])
        .unwrap();
    e.copy_buffer(&a, 0, &b, 0, 4).unwrap();
    assert!(matches!(
        e.finish().validate_dependencies(),
        Err(DependencyValidationError::AccessMismatch {
            declared: A::StorageRead,
            required: A::TransferRead,
            ..
        })
    ));
}

#[test]
fn repeated_reads_and_disjoint_alias_copies_follow_whole_resource_contract() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    let c = buffer(&d);
    let mut e = encoder(&d);
    e.transition_resources(&[
        t(&a, None, A::BufferRead),
        t(&b, None, A::TransferWrite),
        t(&c, None, A::TransferWrite),
    ])
    .unwrap();
    e.copy_buffer(&a, 0, &b, 0, 4).unwrap();
    e.copy_buffer(&a, 4, &c, 0, 4).unwrap();
    e.finish().validate_dependencies().unwrap();
    a.write(0, &[1, 2, 3, 4]).unwrap();
    let mut e = encoder(&d);
    declare(&mut e, &[&a]);
    e.copy_buffer(&a, 0, &a, 4, 4).unwrap();
    let commands = e.finish();
    assert_eq!(
        commands.resource_uses().unwrap()[0].access,
        A::TransferReadWrite
    );
    d.default_queue().submit(&commands).unwrap();
    assert_eq!(&read(&a)[4..8], &[1, 2, 3, 4]);
    let mut e = encoder(&d);
    declare(&mut e, &[&a, &b, &c]);
    e.copy_buffer(&b, 0, &a, 0, 4).unwrap();
    e.copy_buffer(&c, 0, &a, 32, 4).unwrap();
    assert!(matches!(
        e.finish().validate_dependencies(),
        Err(DependencyValidationError::MissingDependency {
            hazard: HazardKind::WriteAfterWrite,
            ..
        })
    ));
}

#[test]
fn compute_uses_only_required_slots_and_combines_disjoint_allocation_views() {
    let d = device();
    let b = buffer(&d);
    let unused = buffer(&d);
    let p = kernel(&d, &[ShaderInstruction::LoadStorage { binding: 0 }], 0);
    let mut e = encoder(&d);
    e.transition_resources(&[t(&b, None, A::StorageReadWrite)])
        .unwrap();
    e.dispatch(
        &p,
        [1; 3],
        &[bind(0, &b, 0), bind(1, &b, 16), bind(7, &unused, 0)],
    )
    .unwrap();
    let commands = e.finish();
    let uses = commands.resource_uses().unwrap();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].access, A::StorageReadWrite);
    commands.validate_dependencies().unwrap();
    d.default_queue().submit(&commands).unwrap();
    let out = buffer(&d);
    let mut e = encoder(&d);
    e.dispatch(&p, [1; 3], &[bind(0, &b, 0), bind(1, &out, 0)])
        .unwrap();
    assert_eq!(
        e.finish()
            .resource_uses()
            .unwrap()
            .iter()
            .map(|u| u.access)
            .collect::<Vec<_>>(),
        [A::StorageRead, A::StorageWrite]
    );
}

#[test]
fn empty_operations_have_no_resource_dependencies() {
    let d = device();
    let b = buffer(&d);
    let p = kernel(&d, &[ShaderInstruction::Constant([1.; 4])], 0);
    let mut e = encoder(&d);
    e.copy_buffer(&b, 64, &b, 64, 0).unwrap();
    e.dispatch(&p, [0, 1, 1], &[bind(1, &b, 0)]).unwrap();
    let c = e.finish();
    assert!(c.resource_uses().unwrap().is_empty());
    d.default_queue().submit(&c).unwrap();
}

fn attachments(d: &Device) -> (Framebuffer, DepthBuffer, StencilBuffer) {
    (
        d.create_framebuffer(FramebufferDescriptor {
            label: "colour",
            width: 4,
            height: 4,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap(),
        d.create_depth_buffer(DepthBufferDescriptor {
            label: "depth",
            width: 4,
            height: 4,
        })
        .unwrap(),
        d.create_stencil_buffer(StencilBufferDescriptor {
            label: "stencil",
            width: 4,
            height: 4,
        })
        .unwrap(),
    )
}

fn graphics(d: &Device, textured: bool) -> GraphicsPipeline {
    let v = d
        .create_shader(ShaderDescriptor {
            label: "position",
            stage: ShaderStage::Vertex,
            input_count: 1,
            instructions: &[ShaderInstruction::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let ops = if textured {
        vec![
            ShaderInstruction::Constant([0.0; 4]),
            ShaderInstruction::SampleTexture {
                binding: 7,
                coordinates: 0,
            },
        ]
    } else {
        vec![ShaderInstruction::Constant([1.; 4])]
    };
    let f = d
        .create_shader(ShaderDescriptor {
            label: "colour",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &ops,
            outputs: &[if textured { 1 } else { 0 }],
        })
        .unwrap();
    d.create_graphics_pipeline(GraphicsPipelineDescriptor {
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
    })
    .unwrap()
}

#[test]
fn all_attachment_kinds_validate_roles_and_empty_scopes_still_use_them() {
    let d = device();
    let (c, z, s) = attachments(&d);
    let rs = [
        (Resource::Framebuffer(c.clone()), A::ColourReadWrite),
        (Resource::DepthBuffer(z.clone()), A::DepthReadWrite),
        (Resource::StencilBuffer(s.clone()), A::StencilReadWrite),
    ];
    let mut e = encoder(&d);
    for (r, a) in &rs {
        assert!(matches!(
            e.transition_resources(&[ResourceTransition {
                resource: r.clone(),
                before: None,
                after: A::StorageRead
            }]),
            Err(DependencyEncodeError::InvalidAccess { .. })
        ));
        e.transition_resources(&[ResourceTransition {
            resource: r.clone(),
            before: None,
            after: *a,
        }])
        .unwrap();
    }
    for _ in 0..2 {
        e.begin_render_pass_with_attachments(
            &c,
            None,
            RenderAttachments {
                depth: Some(&z),
                stencil: Some(&s),
                ..RenderAttachments::default()
            },
        )
        .unwrap()
        .finish()
        .unwrap();
    }
    let commands = e.finish();
    let uses = commands.resource_uses().unwrap();
    assert_eq!(uses.len(), 6);
    assert!(matches!(
        commands.validate_dependencies(),
        Err(DependencyValidationError::MissingDependency {
            kind: ResourceKind::Framebuffer,
            hazard: HazardKind::WriteAfterWrite,
            ..
        })
    ));
    let mut e = encoder(&d);
    e.transition_resources(
        &rs.iter()
            .map(|(r, a)| ResourceTransition {
                resource: r.clone(),
                before: None,
                after: *a,
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    e.begin_render_pass_with_attachments(
        &c,
        Some([9; 4]),
        RenderAttachments {
            depth: Some(&z),
            clear_depth: Some(0.25),
            stencil: Some(&s),
            clear_stencil: Some(7),
        },
    )
    .unwrap()
    .finish()
    .unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(c.pixel(0, 0).unwrap(), [9; 4]);
    assert_eq!(z.depth(0, 0).unwrap(), 0.25);
    assert_eq!(s.stencil(0, 0).unwrap(), 7);
}

#[test]
fn indexed_vertex_alias_is_one_read_and_empty_draw_ignores_sources() {
    let d = device();
    let (c, _, _) = attachments(&d);
    let b = buffer(&d);
    let p = graphics(&d, false);
    for count in [0, 3] {
        let mut e = encoder(&d);
        let mut pass = e.begin_render_pass(&c, None).unwrap();
        pass.set_pipeline(&p).unwrap();
        pass.set_vertex_buffer(&b, 0).unwrap();
        pass.set_index_buffer(&b, 48, IndexFormat::Uint16).unwrap();
        pass.draw_indexed(0..count, 0).unwrap();
        pass.finish().unwrap();
        let uses = e.finish().resource_uses().unwrap();
        assert_eq!(uses.len(), if count == 0 { 1 } else { 2 });
        if count != 0 {
            assert_eq!(uses[1].access, A::BufferRead);
        }
    }
}

#[test]
fn used_texture_views_are_tracked_by_allocation_and_unused_slots_are_ignored() {
    let d = device();
    let (c, _, _) = attachments(&d);
    let b = buffer(&d);
    let tex = d
        .create_texture(TextureDescriptor {
            label: "used",
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::None,
        })
        .unwrap();
    let unused = d
        .create_texture(TextureDescriptor {
            label: "unused",
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::None,
        })
        .unwrap();
    let sampler = d
        .create_sampler(SamplerDescriptor {
            label: "s",
            filter: FilterMode::Nearest,
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
        })
        .unwrap();
    let p = graphics(&d, true);
    let mut e = encoder(&d);
    let r = Resource::Texture(tex.clone());
    assert!(matches!(
        e.transition_resources(&[ResourceTransition {
            resource: r.clone(),
            before: None,
            after: A::TransferWrite
        }]),
        Err(DependencyEncodeError::InvalidAccess { .. })
    ));
    e.transition_resources(&[
        ResourceTransition {
            resource: Resource::Framebuffer(c.clone()),
            before: None,
            after: A::ColourReadWrite,
        },
        t(&b, None, A::VertexRead),
        ResourceTransition {
            resource: r,
            before: None,
            after: A::SampledRead,
        },
    ])
    .unwrap();
    let mut pass = e.begin_render_pass(&c, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&b, 0).unwrap();
    pass.set_texture(6, &unused.view(), &sampler).unwrap();
    for _ in 0..2 {
        pass.set_texture(7, &tex.view(), &sampler).unwrap();
        pass.draw_programmable(0..3).unwrap();
    }
    pass.finish().unwrap();
    let commands = e.finish();
    let uses = commands.resource_uses().unwrap();
    assert_eq!(uses.len(), 3);
    assert_eq!(uses[2].access, A::SampledRead);
    assert!(uses[2].resource.is_same(&Resource::Texture(tex)));
    commands.validate_dependencies().unwrap();
    // Inspection handles remain usable after finished work is dropped.
    drop(commands);
    assert!(uses[2].resource.belongs_to(&d));
}

#[test]
fn accepted_dependencies_preserve_runtime_failure_boundaries() {
    let d = device();
    let a = buffer(&d);
    let b = buffer(&d);
    let out = buffer(&d);
    a.write(0, &[3; 64]).unwrap();
    out.write(0, &[9; 64]).unwrap();
    let p = kernel(
        &d,
        &[
            ShaderInstruction::Constant([f32::MAX; 4]),
            ShaderInstruction::Multiply { left: 0, right: 0 },
        ],
        1,
    );
    let mut e = encoder(&d);
    e.transition_resources(&[
        t(&a, None, A::TransferRead),
        t(&b, None, A::TransferWrite),
        t(&out, None, A::StorageWrite),
    ])
    .unwrap();
    e.copy_buffer(&a, 0, &b, 0, 64).unwrap();
    e.dispatch(&p, [1; 3], &[bind(1, &out, 0)]).unwrap();
    let commands = e.finish();
    commands.validate_dependencies().unwrap();
    assert!(matches!(
        d.default_queue().submit(&commands),
        Err(QueueSubmitError::ComputeFailed {
            command_index: 2,
            ..
        })
    ));
    assert_eq!(read(&b), [3; 64]);
    assert_eq!(read(&out), [9; 64]);
}
