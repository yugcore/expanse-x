//! Adapter-parameterized public cases; no reference-name or private-lock checks.
use ex::*;
use ResourceAccess as A;
use ShaderInstruction as Op;

fn buffer(d: &Device, size: u64, usages: &[BufferUsage], access: HostAccess) -> Buffer {
    d.create_buffer(BufferDescriptor {
        label: "profile buffer",
        size,
        usages,
        host_access: access,
    })
    .unwrap()
}
fn image(d: &Device, width: u32, height: u32, access: HostAccess) -> Texture {
    d.create_texture(TextureDescriptor {
        label: "profile image",
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        host_access: access,
    })
    .unwrap()
}
fn bytes(b: &Buffer) -> Vec<u8> {
    let mut out = vec![0; b.size() as usize];
    b.read(0, &mut out).unwrap();
    out
}
fn pixels(t: &Texture) -> Vec<[u8; 4]> {
    let mut out = vec![[0; 4]; (t.width() * t.height()) as usize];
    t.read_pixels(&mut out).unwrap();
    out
}
fn write_vectors(b: &Buffer, values: &[[f32; 4]]) {
    b.write(
        0,
        &values
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
fn shader(d: &Device, stage: ShaderStage, input_count: u32, ops: &[Op], outputs: &[u32]) -> Shader {
    d.create_shader(ShaderDescriptor {
        label: "profile shader",
        stage,
        input_count,
        instructions: ops,
        outputs,
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
fn rectangle(t: &Texture, x: u32, y: u32, width: u32, height: u32) -> TextureView {
    t.create_view(TextureViewDescriptor {
        x,
        y,
        width,
        height,
    })
    .unwrap()
}

pub(super) fn limits(adapter: &Adapter) {
    assert_eq!(adapter.core_profile(), CoreProfile::Preview1);
    let d = adapter.create_device();
    let l = d.core_profile().limits();
    assert_eq!(d.clone().core_profile(), adapter.core_profile());
    assert_eq!(
        d.default_queue().device().core_profile(),
        adapter.core_profile()
    );
    assert_eq!(l.max_buffer_size, isize::MAX as u64);
    assert_eq!(
        (
            l.max_image_extent,
            l.max_shader_inputs,
            l.max_shader_instructions,
            l.max_vertex_outputs
        ),
        (16384, 16, 256, 17)
    );
    assert_eq!(
        (
            l.binding_slots,
            l.max_uniform_buffer_size,
            l.max_workgroup_size,
            l.max_workgroup_count,
            l.max_compute_invocations
        ),
        (8, 4096, 1024, 65535, 1048576)
    );
    assert!(matches!(
        d.create_buffer(BufferDescriptor {
            label: "too large",
            size: l.max_buffer_size + 1,
            usages: &[BufferUsage::Storage],
            host_access: HostAccess::None
        }),
        Err(BufferCreateError::SizeTooLarge { .. })
    ));
    let extent = l.max_image_extent + 1;
    assert!(matches!(
        d.create_texture(TextureDescriptor {
            label: "too wide",
            width: extent,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::None,
        }),
        Err(TextureCreateError::InvalidDescriptor(
            TextureDescriptorError::ExtentTooLarge { maximum, .. }
        )) if maximum == l.max_image_extent
    ));
    assert!(d
        .create_framebuffer(FramebufferDescriptor {
            label: "too wide",
            width: extent,
            height: 1,
            format: FramebufferFormat::Rgba8Unorm
        })
        .is_err());
    assert!(d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "too wide",
            width: extent,
            height: 1
        })
        .is_err());
    assert!(d
        .create_stencil_buffer(StencilBufferDescriptor {
            label: "too wide",
            width: extent,
            height: 1
        })
        .is_err());
    let ops = vec![Op::Constant([0.; 4]); l.max_shader_instructions as usize];
    let outputs = vec![0; l.max_vertex_outputs as usize];
    let desc = ShaderDescriptor {
        label: "limits",
        stage: ShaderStage::Vertex,
        input_count: l.max_shader_inputs,
        instructions: &ops,
        outputs: &outputs,
    };
    let valid = d.create_shader(desc).unwrap();
    assert_eq!(
        valid
            .evaluate(&vec![[1.; 4]; l.max_shader_inputs as usize])
            .unwrap()
            .len(),
        l.max_vertex_outputs as usize
    );
    assert!(matches!(
        d.create_shader(ShaderDescriptor {
            input_count: l.max_shader_inputs + 1,
            ..desc
        }),
        Err(ShaderCreateError::InvalidDescriptor(
            ShaderValidationError::TooManyInputs { maximum, .. }
        )) if maximum == l.max_shader_inputs
    ));
    let too_many = vec![Op::Constant([0.; 4]); l.max_shader_instructions as usize + 1];
    assert!(matches!(
        d.create_shader(ShaderDescriptor {
            instructions: &too_many,
            ..desc
        }),
        Err(ShaderCreateError::InvalidDescriptor(
            ShaderValidationError::TooManyInstructions { maximum, .. }
        )) if maximum == l.max_shader_instructions as usize
    ));
    let binding = BindGroupLayoutEntry {
        binding: l.binding_slots - 1,
        visibility: ShaderVisibility::All,
        ty: BindingType::UniformBuffer {
            min_size: l.max_uniform_buffer_size,
        },
    };
    d.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "last slot",
        entries: &[binding],
    })
    .unwrap();
    assert!(matches!(
        d.create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "bad slot",
            entries: &[BindGroupLayoutEntry {
                binding: l.binding_slots,
                ..binding
            }]
        }),
        Err(BindingCreateError::InvalidDescriptor(
            BindingValidationError::InvalidBinding { .. }
        ))
    ));
    assert!(matches!(
        d.create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "bad minimum",
            entries: &[BindGroupLayoutEntry {
                ty: BindingType::UniformBuffer {
                    min_size: l.max_uniform_buffer_size + 16
                },
                ..binding
            }]
        }),
        Err(BindingCreateError::InvalidDescriptor(
            BindingValidationError::InvalidMinimum { .. }
        ))
    ));
    let kernel = shader(&d, ShaderStage::Compute, 4, &[Op::Constant([1.; 4])], &[0]);
    let desc = ComputePipelineDescriptor {
        label: "local limits",
        shader: &kernel,
        workgroup_size: [l.max_workgroup_size, 1, 1],
        output_bindings: &[0],
    };
    let p = d.create_compute_pipeline(desc).unwrap();
    assert!(matches!(
        d.create_compute_pipeline(ComputePipelineDescriptor {
            workgroup_size: [l.max_workgroup_size + 1, 1, 1],
            ..desc
        }),
        Err(ComputePipelineCreateError::InvalidWorkgroupSize { .. })
    ));
    let out = buffer(&d, 16, &[BufferUsage::Storage], HostAccess::None);
    let bindings = [StorageBufferBinding {
        binding: 0,
        buffer: &out,
        offset: 0,
        size: 16,
    }];
    let mut e = d.create_command_encoder();
    assert!(matches!(
        e.dispatch(&p, [l.max_workgroup_count + 1, 0, 0], &bindings),
        Err(ComputeEncodeError::InvalidWorkgroups { .. })
    ));
    assert!(matches!(
        e.dispatch(
            &p,
            [l.max_compute_invocations / l.max_workgroup_size + 1, 1, 1],
            &bindings
        ),
        Err(ComputeEncodeError::TooManyInvocations { .. })
    ));
    assert!(e.commands().is_empty());
    e.dispatch(
        &p,
        [l.max_workgroup_count, 0, l.max_workgroup_count],
        &bindings,
    )
    .unwrap();
    d.default_queue().submit(&e.finish()).unwrap(); // Zero dominates otherwise huge dimensions.
}

pub(super) fn shaders(adapter: &Adapter) {
    let d = adapter.create_device();
    let s = shader(
        &d,
        ShaderStage::Vertex,
        1,
        &[
            Op::Input { location: 0 },
            Op::Constant([2., 3., 4., 5.]),
            Op::Multiply { left: 0, right: 1 },
            Op::Swizzle {
                value: 2,
                components: [3, 2, 1, 0],
            },
            Op::Add { left: 2, right: 3 },
            Op::Dot4 { left: 0, right: 1 },
        ],
        &[4, 5],
    );
    assert_eq!(
        s.evaluate(&[[1., 2., 3., 4.]]).unwrap(),
        vec![[22., 18., 18., 22.], [40.; 4]]
    );
    assert!(matches!(
        s.evaluate(&[[f32::NAN; 4]]),
        Err(ShaderExecutionError::NonFiniteInput { .. })
    ));
    let bad = shader(
        &d,
        ShaderStage::Fragment,
        0,
        &[
            Op::Constant([f32::MAX; 4]),
            Op::Multiply { left: 0, right: 0 },
        ],
        &[1],
    );
    assert!(matches!(
        bad.evaluate(&[]),
        Err(ShaderExecutionError::NonFiniteResult { .. })
    ));
    assert!(matches!(
        d.create_shader(ShaderDescriptor {
            label: "wrong stage",
            stage: ShaderStage::Vertex,
            input_count: 0,
            instructions: &[
                Op::Constant([0.; 4]),
                Op::SampleTexture {
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
    assert!(matches!(
        d.create_shader(ShaderDescriptor {
            label: "wrong stage",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &[Op::LoadStorage { binding: 0 }],
            outputs: &[0]
        }),
        Err(ShaderCreateError::InvalidDescriptor(
            ShaderValidationError::StorageLoadingStage { .. }
        ))
    ));
}

pub(super) fn copies(adapter: &Adapter) {
    let d = adapter.create_device();
    let source = buffer(&d, 12, &[BufferUsage::TransferSource], HostAccess::Write);
    let middle = buffer(
        &d,
        12,
        &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        HostAccess::None,
    );
    let read = buffer(
        &d,
        12,
        &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        HostAccess::ReadWrite,
    );
    source
        .write(0, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])
        .unwrap();
    read.write(0, &[99; 12]).unwrap();
    assert!(matches!(
        middle.write(0, &[1]),
        Err(BufferAccessError::WriteNotAllowed { .. })
    ));
    let mut e = d.create_command_encoder();
    e.copy_buffer(&source, 1, &middle, 2, 6).unwrap();
    e.copy_buffer(&middle, 2, &read, 1, 6).unwrap();
    e.copy_buffer(&read, 1, &read, 7, 5).unwrap();
    e.copy_buffer(&source, 12, &read, 12, 0).unwrap();
    let c = e.finish();
    assert_eq!(bytes(&read), vec![99; 12]);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(bytes(&read), vec![99, 2, 3, 4, 5, 6, 7, 2, 3, 4, 5, 6]);
    source.write(1, &[8; 6]).unwrap();
    d.default_queue().submit(&c).unwrap();
    assert_eq!(bytes(&read), vec![99, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8]);
}

pub(super) fn dependencies(adapter: &Adapter) {
    let d = adapter.create_device();
    let source = buffer(&d, 4, &[BufferUsage::TransferSource], HostAccess::Write);
    let middle = buffer(
        &d,
        4,
        &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        HostAccess::ReadWrite,
    );
    let out = buffer(&d, 4, &[BufferUsage::TransferDestination], HostAccess::Read);
    source.write(0, &[7; 4]).unwrap();
    middle.write(0, &[19; 4]).unwrap();
    for barrier in [false, true] {
        let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
        e.transition_resources(&[
            transition(Resource::Buffer(source.clone()), None, A::TransferRead),
            transition(Resource::Buffer(middle.clone()), None, A::TransferReadWrite),
            transition(Resource::Buffer(out.clone()), None, A::TransferWrite),
        ])
        .unwrap();
        e.copy_buffer(&source, 0, &middle, 0, 4).unwrap();
        if barrier {
            e.transition_resources(&[transition(
                Resource::Buffer(middle.clone()),
                Some(A::TransferReadWrite),
                A::TransferRead,
            )])
            .unwrap();
        }
        e.copy_buffer(&middle, 0, &out, 0, 4).unwrap();
        let c = e.finish();
        if barrier {
            c.validate_dependencies().unwrap();
            d.default_queue().submit(&c).unwrap();
            assert_eq!(bytes(&out), vec![7; 4]);
        } else {
            assert!(matches!(
                d.default_queue().submit(&c),
                Err(QueueSubmitError::DependencyFailed {
                    source: DependencyValidationError::MissingDependency {
                        command_index: 2,
                        previous_command_index: 1,
                        hazard: HazardKind::ReadAfterWrite,
                        ..
                    }
                })
            ));
            assert_eq!(bytes(&middle), vec![19; 4]);
            assert_eq!(bytes(&out), vec![0; 4]);
        }
    }
    assert_eq!(
        d.default_queue()
            .submit(&adapter.create_device().create_command_encoder().finish()),
        Err(QueueSubmitError::ForeignCommandBuffer)
    );
}

fn quad(d: &Device, z: f32) -> Buffer {
    let b = buffer(d, 64, &[BufferUsage::Vertex], HostAccess::Write);
    write_vectors(
        &b,
        &[
            [-1., 1., z, 1.],
            [1., 1., z, 1.],
            [1., -1., z, 1.],
            [-1., -1., z, 1.],
        ],
    );
    b
}
fn indices(d: &Device) -> Buffer {
    let b = buffer(d, 12, &[BufferUsage::Index], HostAccess::Write);
    b.write(0, &[0, 0, 1, 0, 2, 0, 0, 0, 2, 0, 3, 0]).unwrap();
    b
}
fn position_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        stride: 16,
        attributes: &[VertexAttribute {
            location: 0,
            offset: 0,
            format: VertexFormat::Float32x4,
        }],
    }
}

pub(super) fn images(adapter: &Adapter) {
    let d = adapter.create_device();
    let input = image(&d, 4, 2, HostAccess::None);
    let atlas = image(&d, 4, 2, HostAccess::None);
    let upload = buffer(&d, 24, &[BufferUsage::TransferSource], HostAccess::Write);
    let mut source = [77; 24];
    source[4..12].copy_from_slice(&[32, 64, 128, 255, 96, 128, 192, 255]);
    source[16..24].copy_from_slice(&[160, 192, 224, 255, 224, 240, 248, 255]);
    upload.write(0, &source).unwrap();
    let a = rectangle(&input, 0, 0, 2, 2);
    let b = rectangle(&input, 2, 0, 2, 2);
    let view = rectangle(&atlas, 1, 0, 2, 2);
    let layout = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "sample + parameters",
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderVisibility::Fragment,
                    ty: BindingType::SampledTexture,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderVisibility::Graphics,
                    ty: BindingType::UniformBuffer { min_size: 32 },
                },
            ],
        })
        .unwrap();
    let parameters = buffer(&d, 32, &[BufferUsage::Uniform], HostAccess::Write);
    write_vectors(&parameters, &[[1.; 4], [0.5, 0.5, 0.5, 1.]]);
    let sampler = d
        .create_sampler(SamplerDescriptor {
            label: "linear repeat",
            filter: FilterMode::Linear,
            address_u: AddressMode::Repeat,
            address_v: AddressMode::Repeat,
        })
        .unwrap();
    // Constant UV lies halfway between all four texel centres: exact mean.
    let vs = shader(
        &d,
        ShaderStage::Vertex,
        1,
        &[
            Op::Input { location: 0 },
            Op::LoadUniform {
                binding: 1,
                element: 0,
            },
            Op::Multiply { left: 0, right: 1 },
            Op::Constant([0.5, 0.5, 0., 0.]),
        ],
        &[2, 3],
    );
    let fs = shader(
        &d,
        ShaderStage::Fragment,
        1,
        &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
            Op::LoadUniform {
                binding: 1,
                element: 1,
            },
            Op::Multiply { left: 1, right: 2 },
        ],
        &[3],
    );
    let p = d
        .create_graphics_pipeline_with_bindings(
            GraphicsPipelineDescriptor {
                label: "sample mean",
                vertex_shader: &vs,
                fragment_shader: &fs,
                vertex_layout: position_layout(),
                colour_format: FramebufferFormat::Rgba8Unorm,
            },
            GraphicsPipelineOptions::default(),
            None,
            &layout,
        )
        .unwrap();
    let group = |view: TextureView| {
        d.create_bind_group(BindGroupDescriptor {
            label: "material",
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::SampledTexture {
                        view,
                        sampler: sampler.clone(),
                    },
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::UniformBuffer(BufferBinding {
                        buffer: parameters.clone(),
                        offset: 0,
                        size: 32,
                    }),
                },
            ],
        })
        .unwrap()
    };
    let offscreen = image(&d, 2, 2, HostAccess::None);
    let final_image = image(&d, 2, 2, HostAccess::None);
    let vertices = quad(&d, 0.25);
    let index = indices(&d);
    let first = group(view.clone());
    let second = group(offscreen.view());
    let read = buffer(
        &d,
        28,
        &[BufferUsage::TransferDestination],
        HostAccess::ReadWrite,
    );
    read.write(0, &[99; 28]).unwrap();
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[
        transition(Resource::Buffer(upload.clone()), None, A::TransferRead),
        transition(Resource::Texture(input.clone()), None, A::TransferWrite),
        transition(Resource::Texture(atlas.clone()), None, A::TransferWrite),
        transition(
            Resource::Framebuffer(offscreen.as_framebuffer()),
            None,
            A::ColourReadWrite,
        ),
        transition(
            Resource::Texture(final_image.clone()),
            None,
            A::ColourReadWrite,
        ),
        transition(Resource::Buffer(vertices.clone()), None, A::VertexRead),
        transition(Resource::Buffer(index.clone()), None, A::IndexRead),
        transition(Resource::Buffer(parameters.clone()), None, A::UniformRead),
        transition(Resource::Buffer(read.clone()), None, A::TransferWrite),
    ])
    .unwrap();
    e.copy_buffer_to_texture(
        &upload,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: 12,
        },
        &a,
    )
    .unwrap();
    e.transition_resources(&[transition(
        Resource::Texture(input.clone()),
        Some(A::TransferWrite),
        A::TransferReadWrite,
    )])
    .unwrap();
    e.copy_texture(&a, &b).unwrap();
    e.transition_resources(&[transition(
        Resource::Framebuffer(input.as_framebuffer()),
        Some(A::TransferReadWrite),
        A::TransferRead,
    )])
    .unwrap();
    e.copy_texture(&b, &view).unwrap();
    e.transition_resources(&[transition(
        Resource::Texture(atlas.clone()),
        Some(A::TransferWrite),
        A::SampledRead,
    )])
    .unwrap();
    for (target, group) in [(&offscreen, &first), (&final_image, &second)] {
        let mut pass = e
            .begin_render_pass(&target.as_framebuffer(), Some([0; 4]))
            .unwrap();
        pass.set_pipeline(&p).unwrap();
        pass.set_vertex_buffer(&vertices, 0).unwrap();
        pass.set_index_buffer(&index, 0, IndexFormat::Uint16)
            .unwrap();
        pass.set_bind_group(group).unwrap();
        pass.draw_indexed(0..6, 0).unwrap();
        pass.finish().unwrap();
        e.transition_resources(&[transition(
            Resource::Texture(target.clone()),
            Some(A::ColourReadWrite),
            if target.is_same(&offscreen) {
                A::SampledRead
            } else {
                A::TransferRead
            },
        )])
        .unwrap();
    }
    e.copy_texture_to_buffer(
        &final_image.view(),
        &read,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: 12,
        },
    )
    .unwrap();
    let c = e.finish();
    c.validate_dependencies().unwrap();
    d.default_queue().submit(&c).unwrap();
    // Mean RGB is (128,156,198); two passes each multiply by 0.5 with unorm rounding.
    let mut expected = vec![99; 28];
    for range in [4..12, 16..24] {
        for pixel in expected[range].as_chunks_mut::<4>().0 {
            *pixel = [32, 39, 50, 255];
        }
    }
    assert_eq!(bytes(&read), expected);
    assert!(Resource::Texture(offscreen.clone())
        .is_same(&Resource::Framebuffer(offscreen.as_framebuffer())));
    assert!(matches!(
        offscreen.as_framebuffer().pixel(0, 0),
        Err(FramebufferAccessError::ReadForbidden { .. })
    ));
    let mut feedback = d.create_command_encoder();
    let mut pass = feedback
        .begin_render_pass(&offscreen.as_framebuffer(), None)
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_bind_group(&second).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::AttachmentFeedback { binding: 0, .. })
    ));
}

fn graphics(
    d: &Device,
    colour: [f32; 4],
    options: GraphicsPipelineOptions,
    stencil: Option<StencilState>,
    fail: bool,
) -> GraphicsPipeline {
    let vs = shader(
        d,
        ShaderStage::Vertex,
        1,
        &[Op::Input { location: 0 }],
        &[0],
    );
    let ops = if fail {
        vec![
            Op::Constant([f32::MAX; 4]),
            Op::Multiply { left: 0, right: 0 },
        ]
    } else {
        vec![Op::Constant(colour)]
    };
    let fs = shader(
        d,
        ShaderStage::Fragment,
        0,
        &ops,
        &[if fail { 1 } else { 0 }],
    );
    let desc = GraphicsPipelineDescriptor {
        label: "attachments",
        vertex_shader: &vs,
        fragment_shader: &fs,
        vertex_layout: position_layout(),
        colour_format: FramebufferFormat::Rgba8Unorm,
    };
    match stencil {
        Some(s) => d
            .create_graphics_pipeline_with_stencil(desc, options, s)
            .unwrap(),
        None => d
            .create_graphics_pipeline_with_options(desc, options)
            .unwrap(),
    }
}
pub(super) fn attachments(adapter: &Adapter) {
    let d = adapter.create_device();
    let colour = image(&d, 4, 4, HostAccess::Read);
    let target = colour.as_framebuffer();
    let depth = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "depth",
            width: 4,
            height: 4,
        })
        .unwrap();
    let stencil = d
        .create_stencil_buffer(StencilBufferDescriptor {
            label: "stencil",
            width: 4,
            height: 4,
        })
        .unwrap();
    let vertices = quad(&d, -0.5);
    let index = indices(&d);
    let options = GraphicsPipelineOptions {
        depth: Some(DepthState {
            compare: CompareFunction::Less,
            write_enabled: true,
        }),
        blend: Some(BlendState::alpha_blending()),
    };
    let p = graphics(
        &d,
        [1., 0., 0., 0.5],
        options,
        Some(StencilState::replace(3)),
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &target,
            Some([0, 0, 255, 255]),
            RenderAttachments {
                depth: Some(&depth),
                clear_depth: Some(1.),
                stencil: Some(&stencil),
                clear_stencil: Some(0),
            },
        )
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_index_buffer(&index, 0, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&colour), vec![[128, 0, 128, 255]; 16]);
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(depth.depth(x, y).unwrap(), 0.25);
            assert_eq!(stencil.stencil(x, y).unwrap(), 3);
        }
    }
    let bad = graphics(&d, [0.; 4], GraphicsPipelineOptions::default(), None, true);
    let later = image(&d, 1, 1, HostAccess::Read);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &target,
            Some([7; 4]),
            RenderAttachments {
                depth: Some(&depth),
                clear_depth: Some(0.75),
                stencil: Some(&stencil),
                clear_stencil: Some(8),
            },
        )
        .unwrap();
    pass.set_pipeline(&bad).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_index_buffer(&index, 0, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    e.begin_render_pass(&later.as_framebuffer(), Some([99; 4]))
        .unwrap()
        .finish()
        .unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::RenderFailed {
            command_index: 0,
            source: RenderExecutionError::NonFiniteShaderResult { .. }
        })
    ));
    assert_eq!(pixels(&colour), vec![[128, 0, 128, 255]; 16]);
    assert_eq!(depth.depth(0, 0).unwrap(), 0.25);
    assert_eq!(stencil.stencil(0, 0).unwrap(), 3);
    assert_eq!(pixels(&later), vec![[0; 4]]);
}

pub(super) fn compute(adapter: &Adapter) {
    let d = adapter.create_device();
    let layout = d
        .create_bind_group_layout(BindGroupLayoutDescriptor {
            label: "compute interface",
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderVisibility::Compute,
                    ty: BindingType::StorageBuffer { read_only: false },
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderVisibility::Compute,
                    ty: BindingType::UniformBuffer { min_size: 16 },
                },
            ],
        })
        .unwrap();
    let storage = buffer(&d, 80, &[BufferUsage::Storage], HostAccess::ReadWrite);
    let values = [
        [1., 2., 3., 4.],
        [2., 3., 4., 5.],
        [3., 4., 5., 6.],
        [4., 5., 6., 7.],
        [99.; 4],
    ];
    write_vectors(&storage, &values);
    let parameters = buffer(&d, 16, &[BufferUsage::Uniform], HostAccess::Write);
    write_vectors(&parameters, &[[2., 3., 4., 1.]]);
    let group = d
        .create_bind_group(BindGroupDescriptor {
            label: "compute values",
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::StorageBuffer(BufferBinding {
                        buffer: storage.clone(),
                        offset: 0,
                        size: 80,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::UniformBuffer(BufferBinding {
                        buffer: parameters.clone(),
                        offset: 0,
                        size: 16,
                    }),
                },
            ],
        })
        .unwrap();
    let s = shader(
        &d,
        ShaderStage::Compute,
        4,
        &[
            Op::LoadStorage { binding: 0 },
            Op::LoadUniform {
                binding: 1,
                element: 0,
            },
            Op::Multiply { left: 0, right: 1 },
        ],
        &[2],
    );
    let p = d
        .create_compute_pipeline_with_bindings(
            ComputePipelineDescriptor {
                label: "scale vectors",
                shader: &s,
                workgroup_size: [2, 1, 1],
                output_bindings: &[0],
            },
            &layout,
        )
        .unwrap();
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[
        transition(Resource::Buffer(storage.clone()), None, A::StorageReadWrite),
        transition(Resource::Buffer(parameters.clone()), None, A::UniformRead),
    ])
    .unwrap();
    e.dispatch_with_bind_group(&p, [2, 1, 1], &group).unwrap();
    let c = e.finish();
    d.default_queue().submit(&c).unwrap();
    let expected = [
        [2., 6., 12., 4.],
        [4., 9., 16., 5.],
        [6., 12., 20., 6.],
        [8., 15., 24., 7.],
        [99.; 4],
    ];
    assert_eq!(
        bytes(&storage),
        expected
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>()
    );
    let snapshot = bytes(&storage);
    write_vectors(&parameters, &[[f32::NAN; 4]]);
    assert!(matches!(
        d.default_queue().submit(&c),
        Err(QueueSubmitError::ComputeFailed {
            command_index: 1,
            ..
        })
    ));
    assert_eq!(bytes(&storage), snapshot);
    write_vectors(&parameters, &[[1.; 4]]);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(bytes(&storage), snapshot);
}

pub(super) fn lifetime(adapter: &Adapter) {
    let (queue, commands, readback) = {
        let d = adapter.clone().create_device();
        let source = buffer(&d, 4, &[BufferUsage::TransferSource], HostAccess::Write);
        source.write(0, &[11, 22, 33, 44]).unwrap();
        let read = buffer(&d, 4, &[BufferUsage::TransferDestination], HostAccess::Read);
        let mut e = d.create_command_encoder();
        e.copy_buffer(&source, 0, &read, 0, 4).unwrap();
        (d.default_queue(), e.finish(), read)
    };
    queue.submit(&commands).unwrap();
    assert_eq!(bytes(&readback), vec![11, 22, 33, 44]);
    if let Command::CopyBuffer(copy) = &commands.commands()[0] {
        copy.source().write(0, &[55; 4]).unwrap();
    } else {
        panic!("recorded copy missing")
    }
    queue.clone().submit(&commands).unwrap();
    assert_eq!(bytes(&readback), vec![55; 4]);
}

pub(super) fn compute_ids(adapter: &Adapter) {
    let d = adapter.create_device();
    let outputs: Vec<_> = (0..8)
        .map(|_| buffer(&d, 512, &[BufferUsage::Storage], HostAccess::Read))
        .collect();
    let bindings: Vec<_> = outputs
        .iter()
        .enumerate()
        .map(|(slot, b)| StorageBufferBinding {
            binding: slot as u32,
            buffer: b,
            offset: 0,
            size: 512,
        })
        .collect();
    let s = shader(
        &d,
        ShaderStage::Compute,
        4,
        &[
            Op::Input { location: 0 },
            Op::Input { location: 1 },
            Op::Input { location: 2 },
            Op::Input { location: 3 },
            Op::Constant([1.; 4]),
            Op::Add { left: 0, right: 2 },
            Op::Swizzle {
                value: 1,
                components: [2, 1, 0, 3],
            },
            Op::Dot4 { left: 0, right: 4 },
        ],
        &[0, 1, 2, 3, 4, 5, 6, 7],
    );
    let p = d
        .create_compute_pipeline(ComputePipelineDescriptor {
            label: "all compute IDs",
            shader: &s,
            workgroup_size: [2, 2, 2],
            output_bindings: &[0, 1, 2, 3, 4, 5, 6, 7],
        })
        .unwrap();
    let mut e = d.create_command_encoder();
    e.dispatch(&p, [2, 1, 2], &bindings).unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    let actual: Vec<_> = outputs.iter().map(bytes).collect();
    for z in 0..4 {
        for y in 0..2 {
            for x in 0..4 {
                let linear = x + 4 * (y + 2 * z);
                let local = x % 2 + 2 * (y + 2 * (z % 2));
                let group = x / 2 + 2 * (z / 2);
                let global = [x as f32, y as f32, z as f32, linear as f32];
                let group_id = [(x / 2) as f32, 0., (z / 2) as f32, group as f32];
                let expected = [
                    global,
                    [(x % 2) as f32, y as f32, (z % 2) as f32, local as f32],
                    group_id,
                    [2., 1., 2., 0.],
                    [1.; 4],
                    std::array::from_fn(|lane| global[lane] + group_id[lane]),
                    [(z % 2) as f32, y as f32, (x % 2) as f32, local as f32],
                    [(x + y + z + linear) as f32; 4],
                ];
                for slot in 0..8 {
                    let expected: Vec<_> = expected[slot]
                        .into_iter()
                        .flat_map(f32::to_le_bytes)
                        .collect();
                    assert_eq!(&actual[slot][linear * 16..linear * 16 + 16], expected);
                }
            }
        }
    }
    // Early invocations have finite results; x=2 overflows later. All eight
    // outputs must still retain the previous successful dispatch's bytes.
    let bad = shader(
        &d,
        ShaderStage::Compute,
        4,
        &[
            Op::Input { location: 0 },
            Op::Constant([f32::MAX; 4]),
            Op::Multiply { left: 0, right: 1 },
        ],
        &[2; 8],
    );
    let p = d
        .create_compute_pipeline(ComputePipelineDescriptor {
            label: "late numeric failure",
            shader: &bad,
            workgroup_size: [2, 2, 2],
            output_bindings: &[0, 1, 2, 3, 4, 5, 6, 7],
        })
        .unwrap();
    let mut e = d.create_command_encoder();
    e.dispatch(&p, [2, 1, 2], &bindings).unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::ComputeFailed {
            command_index: 0,
            ..
        })
    ));
    assert_eq!(outputs.iter().map(bytes).collect::<Vec<_>>(), actual);
}
