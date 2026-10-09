use ex::*;
use ResourceAccess as A;

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn image(d: &Device, width: u32, height: u32, host_access: HostAccess) -> Texture {
    d.create_texture(TextureDescriptor {
        label: "image",
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        host_access,
    })
    .unwrap()
}
fn buffer(d: &Device, size: u64, usages: &[BufferUsage], host_access: HostAccess) -> Buffer {
    d.create_buffer(BufferDescriptor {
        label: "rows",
        size,
        usages,
        host_access,
    })
    .unwrap()
}
fn view(t: &Texture, x: u32, y: u32, width: u32, height: u32) -> TextureView {
    t.create_view(TextureViewDescriptor {
        x,
        y,
        width,
        height,
    })
    .unwrap()
}
fn transition(resource: Resource, before: Option<A>, after: A) -> ResourceTransition {
    ResourceTransition {
        resource,
        before,
        after,
    }
}
fn pixels(t: &Texture) -> Vec<[u8; 4]> {
    let mut result = vec![[0; 4]; (t.width() * t.height()) as usize];
    t.read_pixels(&mut result).unwrap();
    result
}

#[test]
fn padded_subrectangle_roundtrip_preserves_all_other_pixels_and_bytes() {
    let d = device();
    let texture = image(&d, 4, 4, HostAccess::Read);
    let region = view(&texture, 1, 1, 2, 2);
    let source = buffer(&d, 24, &[BufferUsage::TransferSource], HostAccess::Write);
    let destination = buffer(
        &d,
        40,
        &[BufferUsage::TransferDestination],
        HostAccess::ReadWrite,
    );
    let mut upload = [55; 24];
    upload[4..12].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    upload[16..24].copy_from_slice(&[9, 10, 11, 12, 13, 14, 15, 16]);
    source.write(0, &upload).unwrap();
    destination.write(0, &[99; 40]).unwrap();
    let mut e = d.create_command_encoder();
    e.copy_buffer_to_texture(
        &source,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: 12,
        },
        &region,
    )
    .unwrap();
    e.copy_texture_to_buffer(
        &region,
        &destination,
        TextureBufferLayout {
            offset: 8,
            bytes_per_row: 16,
        },
    )
    .unwrap();
    let c = e.finish();
    if let Command::CopyBufferToTexture(copy) = &c.commands()[0] {
        assert!(copy.buffer().is_same(&source));
        assert_eq!(copy.layout().offset, 4);
        assert_eq!(copy.texture().region(), region.region());
    } else {
        panic!("wrong command");
    }
    assert_eq!(pixels(&texture), vec![[0; 4]; 16]);
    d.default_queue().submit(&c).unwrap();
    let mut expected = vec![[0; 4]; 16];
    expected[5] = [1, 2, 3, 4];
    expected[6] = [5, 6, 7, 8];
    expected[9] = [9, 10, 11, 12];
    expected[10] = [13, 14, 15, 16];
    assert_eq!(pixels(&texture), expected);
    let mut bytes = [0; 40];
    destination.read(0, &mut bytes).unwrap();
    let mut expected = [99; 40];
    expected[8..16].copy_from_slice(&upload[4..12]);
    expected[24..32].copy_from_slice(&upload[16..24]);
    assert_eq!(bytes, expected);
    // Submission reads updated bytes, rather than a recording-time snapshot.
    source.write(4, &[21; 8]).unwrap();
    d.default_queue().submit(&c).unwrap();
    assert_eq!(texture.as_framebuffer().pixel(1, 1).unwrap(), [21; 4]);
}

#[test]
fn exact_final_row_and_one_row_layouts_do_not_require_trailing_padding() {
    let d = device();
    let t = image(&d, 2, 1, HostAccess::Read);
    let b = buffer(&d, 12, &[BufferUsage::TransferSource], HostAccess::Write);
    b.write(4, &[42; 8]).unwrap();
    let mut e = d.create_command_encoder();
    // Very large aligned pitch is valid for one row, never converted to usize.
    e.copy_buffer_to_texture(
        &b,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: u64::MAX - 3,
        },
        &t.view(),
    )
    .unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&t), vec![[42; 4]; 2]);
}

#[test]
fn layout_alignment_stride_bounds_and_overflow_fail_without_appending() {
    let d = device();
    let t = image(&d, 2, 2, HostAccess::Read);
    let b = buffer(
        &d,
        24,
        &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        HostAccess::ReadWrite,
    );
    let mut e = d.create_command_encoder();
    for layout in [
        TextureBufferLayout {
            offset: 1,
            bytes_per_row: 8,
        },
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 10,
        },
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 4,
        },
        TextureBufferLayout {
            offset: 20,
            bytes_per_row: 8,
        },
        TextureBufferLayout {
            offset: u64::MAX - 3,
            bytes_per_row: 8,
        },
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: u64::MAX - 3,
        },
    ] {
        assert!(matches!(
            e.copy_buffer_to_texture(&b, layout, &t.view()),
            Err(TextureTransferEncodeError::InvalidBufferLayout { .. })
        ));
        assert!(matches!(
            e.copy_texture_to_buffer(&t.view(), &b, layout),
            Err(TextureTransferEncodeError::InvalidBufferLayout { .. })
        ));
    }
    assert!(e.commands().is_empty());
    assert_eq!(pixels(&t), vec![[0; 4]; 4]);
}

#[test]
fn transfer_devices_usages_and_image_extent_checks_are_structural() {
    let d = device();
    let foreign = device();
    let t = image(&d, 2, 2, HostAccess::None);
    let b = buffer(&d, 16, &[BufferUsage::Vertex], HostAccess::None);
    let wrong = buffer(
        &foreign,
        16,
        &[BufferUsage::TransferSource],
        HostAccess::None,
    );
    let foreign_image = image(&foreign, 2, 2, HostAccess::None);
    let small = image(&d, 1, 1, HostAccess::None);
    let mut e = d.create_command_encoder();
    let layout = TextureBufferLayout {
        offset: 0,
        bytes_per_row: 8,
    };
    assert!(matches!(
        e.copy_buffer_to_texture(&wrong, layout, &t.view()),
        Err(TextureTransferEncodeError::ForeignResource {
            role: CopyBufferRole::Source,
            kind: ResourceKind::Buffer,
            ..
        })
    ));
    assert!(matches!(
        e.copy_texture_to_buffer(&foreign_image.view(), &b, layout),
        Err(TextureTransferEncodeError::ForeignResource {
            role: CopyBufferRole::Source,
            kind: ResourceKind::Texture,
            ..
        })
    ));
    assert!(matches!(
        e.copy_buffer_to_texture(&b, layout, &t.view()),
        Err(TextureTransferEncodeError::MissingBufferUsage {
            required: BufferUsage::TransferSource,
            ..
        })
    ));
    assert!(matches!(
        e.copy_texture_to_buffer(&t.view(), &b, layout),
        Err(TextureTransferEncodeError::MissingBufferUsage {
            required: BufferUsage::TransferDestination,
            ..
        })
    ));
    assert!(matches!(
        e.copy_texture(&t.view(), &small.view()),
        Err(TextureTransferEncodeError::ExtentMismatch { .. })
    ));
    assert!(matches!(
        e.copy_texture(&t.view(), &foreign_image.view()),
        Err(TextureTransferEncodeError::ForeignResource {
            role: CopyBufferRole::Destination,
            ..
        })
    ));
    assert!(e.commands().is_empty());
}

#[test]
fn image_copy_offsets_and_disjoint_self_copy_use_live_pixels() {
    let d = device();
    let t = image(&d, 4, 3, HostAccess::ReadWrite);
    let out = image(&d, 5, 4, HostAccess::Read);
    let input: Vec<_> = (0..12).map(|n| [n; 4]).collect();
    t.write_pixels(&input).unwrap();
    let mut e = d.create_command_encoder();
    let a = view(&t, 0, 0, 2, 2);
    let b = view(&t, 2, 1, 2, 2);
    e.copy_texture(&a, &b).unwrap();
    e.copy_texture(&b, &view(&out, 1, 1, 2, 2)).unwrap();
    let c = e.finish();
    if let Command::CopyTexture(copy) = &c.commands()[0] {
        assert_eq!(copy.source().region(), a.region());
        assert_eq!(copy.destination().region(), b.region());
    } else {
        panic!("wrong command")
    }
    d.default_queue().submit(&c).unwrap();
    let mut expected = input;
    expected[6] = [0; 4];
    expected[7] = [1; 4];
    expected[10] = [4; 4];
    expected[11] = [5; 4];
    assert_eq!(pixels(&t), expected);
    let mut expected = vec![[0; 4]; 20];
    expected[6] = [0; 4];
    expected[7] = [1; 4];
    expected[11] = [4; 4];
    expected[12] = [5; 4];
    assert_eq!(pixels(&out), expected);
}

#[test]
fn overlapping_rectangles_fail_through_framebuffer_aliases() {
    let d = device();
    let t = image(&d, 4, 4, HostAccess::Read);
    let alias = t.as_framebuffer().as_texture();
    let mut e = d.create_command_encoder();
    for destination in [view(&alias, 0, 0, 2, 2), view(&alias, 1, 1, 2, 2)] {
        assert!(matches!(
            e.copy_texture(&view(&t, 0, 0, 2, 2), &destination),
            Err(TextureTransferEncodeError::OverlappingCopy { .. })
        ));
    }
    assert!(e.commands().is_empty());
}

#[test]
fn attachment_aliases_preserve_identity_host_permissions_and_contents() {
    let d = device();
    for access in [
        HostAccess::None,
        HostAccess::Write,
        HostAccess::Read,
        HostAccess::ReadWrite,
    ] {
        let t = image(&d, 2, 2, access);
        let f = t.as_framebuffer();
        let alias = f.as_texture();
        assert!(t.is_same(&alias));
        assert!(f.is_same(&alias.as_framebuffer()));
        assert!(Resource::Texture(t.clone()).is_same(&Resource::Framebuffer(f.clone())));
        assert!(Resource::Framebuffer(f.clone()).is_same(&Resource::Texture(t.clone())));
        assert_eq!(alias.host_access(), access);
        let mut e = d.create_command_encoder();
        e.begin_render_pass(&f, Some([31; 4]))
            .unwrap()
            .finish()
            .unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        let mut destination = [[99; 4]; 4];
        if matches!(access, HostAccess::Read | HostAccess::ReadWrite) {
            f.read_pixels(&mut destination).unwrap();
            assert_eq!(destination, [[31; 4]; 4]);
            assert_eq!(pixels(&t), destination);
        } else {
            assert!(matches!(
                f.read_pixels(&mut destination),
                Err(FramebufferAccessError::ReadForbidden { .. })
            ));
            assert!(matches!(
                f.pixel(0, 0),
                Err(FramebufferAccessError::ReadForbidden { .. })
            ));
            assert_eq!(destination, [[99; 4]; 4]);
        }
    }
    let f = d
        .create_framebuffer(FramebufferDescriptor {
            label: "standalone",
            width: 1,
            height: 1,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    assert_eq!(f.as_texture().host_access(), HostAccess::Read);
    assert_eq!(pixels(&f.as_texture()), vec![[0; 4]]);
    assert!(matches!(
        f.as_texture().write_pixels(&[[9; 4]]),
        Err(TextureAccessError::WriteForbidden { .. })
    ));
}

fn sampling(d: &Device, layout: Option<&BindGroupLayout>) -> GraphicsPipeline {
    let vs = d
        .create_shader(ShaderDescriptor {
            label: "positions",
            stage: ShaderStage::Vertex,
            input_count: 2,
            instructions: &[
                ShaderInstruction::Input { location: 0 },
                ShaderInstruction::Input { location: 1 },
            ],
            outputs: &[0, 1],
        })
        .unwrap();
    let fs = d
        .create_shader(ShaderDescriptor {
            label: "sample",
            stage: ShaderStage::Fragment,
            input_count: 1,
            instructions: &[
                ShaderInstruction::Input { location: 0 },
                ShaderInstruction::SampleTexture {
                    binding: 0,
                    coordinates: 0,
                },
            ],
            outputs: &[1],
        })
        .unwrap();
    let desc = GraphicsPipelineDescriptor {
        label: "sample",
        vertex_shader: &vs,
        fragment_shader: &fs,
        vertex_layout: VertexBufferLayout {
            stride: 16,
            attributes: &[
                VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    location: 1,
                    offset: 8,
                    format: VertexFormat::Float32x2,
                },
            ],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    };
    match layout {
        Some(l) => d
            .create_graphics_pipeline_with_bindings(
                desc,
                GraphicsPipelineOptions::default(),
                None,
                l,
            )
            .unwrap(),
        None => d.create_graphics_pipeline(desc).unwrap(),
    }
}
fn sampler(d: &Device) -> Sampler {
    d.create_sampler(SamplerDescriptor {
        label: "nearest",
        filter: FilterMode::Nearest,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
    })
    .unwrap()
}
fn quad(d: &Device) -> Buffer {
    let data: Vec<_> = [
        [-1.0_f32, 1.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [1.0, -1.0, 1.0, 1.0],
        [-1.0, 1.0, 0.0, 0.0],
        [1.0, -1.0, 1.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0],
    ]
    .into_iter()
    .flatten()
    .flat_map(f32::to_le_bytes)
    .collect();
    let b = buffer(d, 96, &[BufferUsage::Vertex], HostAccess::Write);
    b.write(0, &data).unwrap();
    b
}

#[test]
fn rendering_then_sampling_uses_shared_image_in_a_later_pass() {
    let d = device();
    let source = image(&d, 2, 2, HostAccess::None);
    let target = image(&d, 4, 4, HostAccess::Read);
    let p = sampling(&d, None);
    let v = quad(&d);
    let s = sampler(&d);
    let mut e = d.create_command_encoder();
    e.begin_render_pass(&source.as_framebuffer(), Some([64, 128, 255, 255]))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = e
        .begin_render_pass(&target.as_framebuffer(), Some([0; 4]))
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &source.view(), &s).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&target), vec![[64, 128, 255, 255]; 16]);
}

#[test]
fn feedback_rejection_covers_groups_legacy_indexed_empty_and_disjoint_views() {
    let d = device();
    let t = image(&d, 4, 4, HostAccess::Read);
    let v = quad(&d);
    let s = sampler(&d);
    let i = buffer(&d, 12, &[BufferUsage::Index], HostAccess::Write);
    i.write(0, &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]).unwrap();
    let layout = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "material",
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderVisibility::Fragment,
                ty: BindingType::SampledTexture,
            }],
        })
        .unwrap();
    let group = d
        .create_bind_group(BindGroupDescriptor {
            label: "feedback",
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::SampledTexture {
                    view: view(&t, 3, 3, 1, 1),
                    sampler: s.clone(),
                },
            }],
        })
        .unwrap();
    for grouped in [false, true] {
        let p = sampling(&d, if grouped { Some(&layout) } else { None });
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass(&t.as_framebuffer(), Some([77; 4]))
            .unwrap();
        pass.set_pipeline(&p).unwrap();
        pass.set_vertex_buffer(&v, 0).unwrap();
        pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
        if grouped {
            pass.set_bind_group(&group).unwrap();
        } else {
            pass.set_texture(0, &t.view(), &s).unwrap();
        }
        for empty in [false, true] {
            assert!(matches!(
                pass.draw_programmable(0..if empty { 0 } else { 6 }),
                Err(RenderEncodeError::AttachmentFeedback { binding: 0, .. })
            ));
            assert!(matches!(
                pass.draw_indexed(0..if empty { 0 } else { 6 }, 0),
                Err(RenderEncodeError::AttachmentFeedback { binding: 0, .. })
            ));
        }
        // Failed draw did not append anything. Clear still records and succeeds.
        pass.finish().unwrap();
        if let Command::RenderPass(scope) = &e.commands()[0] {
            assert!(scope.draws().is_empty());
        } else {
            panic!("wrong command")
        }
        d.default_queue().submit(&e.finish()).unwrap();
        assert_eq!(pixels(&t), vec![[77; 4]; 16]);
    }
}

#[test]
fn explicit_alias_dependencies_validate_upload_render_readback_and_reuse() {
    let d = device();
    let t = image(&d, 2, 2, HostAccess::None);
    let f = t.as_framebuffer();
    let upload = buffer(&d, 16, &[BufferUsage::TransferSource], HostAccess::Write);
    upload.write(0, &[6; 16]).unwrap();
    let read = buffer(
        &d,
        16,
        &[BufferUsage::TransferDestination],
        HostAccess::Read,
    );
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[
        transition(Resource::Buffer(upload.clone()), None, A::TransferRead),
        transition(Resource::Framebuffer(f.clone()), None, A::TransferWrite),
        transition(Resource::Buffer(read.clone()), None, A::TransferWrite),
    ])
    .unwrap();
    e.copy_buffer_to_texture(
        &upload,
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 8,
        },
        &t.view(),
    )
    .unwrap();
    e.transition_resources(&[transition(
        Resource::Texture(t.clone()),
        Some(A::TransferWrite),
        A::ColourReadWrite,
    )])
    .unwrap();
    e.begin_render_pass(&f, Some([9; 4]))
        .unwrap()
        .finish()
        .unwrap();
    e.transition_resources(&[transition(
        Resource::Framebuffer(f),
        Some(A::ColourReadWrite),
        A::TransferRead,
    )])
    .unwrap();
    e.copy_texture_to_buffer(
        &t.view(),
        &read,
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 8,
        },
    )
    .unwrap();
    let c = e.finish();
    let uses = c.resource_uses().unwrap();
    assert_eq!(uses.len(), 5);
    assert_eq!(
        uses.iter().map(|u| u.access).collect::<Vec<_>>(),
        vec![
            A::TransferRead,
            A::TransferWrite,
            A::ColourReadWrite,
            A::TransferRead,
            A::TransferWrite
        ]
    );
    for _ in 0..2 {
        c.validate_dependencies().unwrap();
        d.default_queue().submit(&c).unwrap();
    }
    let mut bytes = [0; 16];
    read.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [9; 16]);
}

#[test]
fn missing_image_barrier_preflight_preserves_all_commands_and_rejects_duplicate_aliases() {
    let d = device();
    let t = image(&d, 2, 2, HostAccess::Read);
    let f = t.as_framebuffer();
    let b = buffer(
        &d,
        16,
        &[BufferUsage::TransferDestination],
        HostAccess::Read,
    );
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    assert!(matches!(
        e.transition_resources(&[
            transition(Resource::Texture(t.clone()), None, A::ColourReadWrite),
            transition(Resource::Framebuffer(f.clone()), None, A::SampledRead)
        ]),
        Err(DependencyEncodeError::DuplicateResource { .. })
    ));
    assert!(e.commands().is_empty());
    e.transition_resources(&[
        transition(Resource::Texture(t.clone()), None, A::ColourReadWrite),
        transition(Resource::Buffer(b.clone()), None, A::TransferWrite),
    ])
    .unwrap();
    e.begin_render_pass(&f, Some([19; 4]))
        .unwrap()
        .finish()
        .unwrap();
    e.copy_texture_to_buffer(
        &t.view(),
        &b,
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 8,
        },
    )
    .unwrap();
    let c = e.finish();
    assert!(matches!(
        d.default_queue().submit(&c),
        Err(QueueSubmitError::DependencyFailed {
            source: DependencyValidationError::MissingDependency {
                hazard: HazardKind::ReadAfterWrite,
                command_index: 2,
                previous_command_index: 1,
                ..
            }
        })
    ));
    assert_eq!(pixels(&t), vec![[0; 4]; 4]);
}

#[test]
fn disjoint_self_copy_is_one_whole_image_read_write_use() {
    let d = device();
    let t = image(&d, 4, 1, HostAccess::ReadWrite);
    t.write_pixels(&[[1; 4], [2; 4], [3; 4], [4; 4]]).unwrap();
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[transition(
        Resource::Framebuffer(t.as_framebuffer()),
        None,
        A::TransferReadWrite,
    )])
    .unwrap();
    e.copy_texture(&view(&t, 0, 0, 2, 1), &view(&t, 2, 0, 2, 1))
        .unwrap();
    let c = e.finish();
    let uses = c.resource_uses().unwrap();
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].access, A::TransferReadWrite);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(pixels(&t), vec![[1; 4], [2; 4], [1; 4], [2; 4]]);
}

#[test]
fn failed_render_preserves_shared_colour_depth_stencil_and_skips_readback() {
    let d = device();
    let t = image(&d, 2, 2, HostAccess::ReadWrite);
    t.write_pixels(&[[23; 4]; 4]).unwrap();
    let depth = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "depth",
            width: 2,
            height: 2,
        })
        .unwrap();
    let stencil = d
        .create_stencil_buffer(StencilBufferDescriptor {
            label: "stencil",
            width: 2,
            height: 2,
        })
        .unwrap();
    let bad = buffer(&d, 24, &[BufferUsage::Vertex], HostAccess::Write);
    bad.write(
        0,
        &[f32::NAN, 0., 0., 0., 0., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let readback = buffer(
        &d,
        16,
        &[BufferUsage::TransferDestination],
        HostAccess::ReadWrite,
    );
    readback.write(0, &[99; 16]).unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t.as_framebuffer(),
            Some([77; 4]),
            RenderAttachments {
                depth: Some(&depth),
                clear_depth: Some(0.5),
                stencil: Some(&stencil),
                clear_stencil: Some(8),
            },
        )
        .unwrap();
    pass.set_vertex_buffer(&bad, 0).unwrap();
    pass.draw(0..3, [255; 4]).unwrap();
    pass.finish().unwrap();
    e.copy_texture_to_buffer(
        &t.view(),
        &readback,
        TextureBufferLayout {
            offset: 0,
            bytes_per_row: 8,
        },
    )
    .unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::RenderFailed {
            command_index: 0,
            source: RenderExecutionError::NonFiniteVertex { .. }
        })
    ));
    assert_eq!(pixels(&t), vec![[23; 4]; 4]);
    assert_eq!(depth.depth(0, 0).unwrap(), 1.0);
    assert_eq!(stencil.stencil(0, 0).unwrap(), 0);
    let mut bytes = [0; 16];
    readback.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [99; 16]);
    let mut e = d.create_command_encoder();
    e.begin_render_pass_with_attachments(
        &t.as_framebuffer(),
        Some([42; 4]),
        RenderAttachments {
            depth: Some(&depth),
            clear_depth: Some(0.25),
            stencil: Some(&stencil),
            clear_stencil: Some(3),
        },
    )
    .unwrap()
    .finish()
    .unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&t), vec![[42; 4]; 4]);
    assert_eq!(depth.depth(0, 0).unwrap(), 0.25);
    assert_eq!(stencil.stencil(0, 0).unwrap(), 3);
}

#[test]
fn fixed_draws_and_unused_texture_slots_allow_attachment_aliases() {
    let d = device();
    let t = image(&d, 2, 2, HostAccess::Read);
    let p = sampling(&d, None);
    let v = quad(&d);
    let s = sampler(&d);
    let other = image(&d, 1, 1, HostAccess::ReadWrite);
    other.write_pixels(&[[255; 4]]).unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t.as_framebuffer(), None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &other.view(), &s).unwrap();
    pass.set_texture(7, &t.view(), &s).unwrap();
    pass.draw_programmable(0..6).unwrap();
    // A fixed draw ignores pipeline texture requirements and all texture slots.
    pass.set_texture(0, &t.view(), &s).unwrap();
    pass.draw(0..0, [0; 4]).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&t), vec![[255; 4]; 4]);
}

#[test]
fn render_to_sample_barrier_is_required_across_different_handle_kinds() {
    let d = device();
    let source = image(&d, 2, 2, HostAccess::Read);
    let destination = image(&d, 2, 2, HostAccess::Read);
    let p = sampling(&d, None);
    let v = quad(&d);
    let s = sampler(&d);
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[
        transition(
            Resource::Framebuffer(source.as_framebuffer()),
            None,
            A::ColourReadWrite,
        ),
        transition(
            Resource::Texture(destination.clone()),
            None,
            A::ColourReadWrite,
        ),
        transition(Resource::Buffer(v.clone()), None, A::VertexRead),
    ])
    .unwrap();
    e.begin_render_pass(&source.as_framebuffer(), Some([7; 4]))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = e
        .begin_render_pass(&destination.as_framebuffer(), Some([8; 4]))
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &source.view(), &s).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::DependencyFailed {
            source: DependencyValidationError::MissingDependency {
                hazard: HazardKind::ReadAfterWrite,
                command_index: 2,
                previous_command_index: 1,
                ..
            }
        })
    ));
    assert_eq!(pixels(&source), vec![[0; 4]; 4]);
    assert_eq!(pixels(&destination), vec![[0; 4]; 4]);
}
