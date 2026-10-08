use ex::{
    Buffer, BufferDescriptor, BufferUsage, Device, Framebuffer, FramebufferDescriptor,
    FramebufferFormat, GraphicsPipeline, GraphicsPipelineCreateError as PipelineError,
    GraphicsPipelineDescriptor, HostAccess, Instance, QueueSubmitError, RenderEncodeError,
    RenderExecutionError, Shader, ShaderDescriptor, ShaderInstruction as Op, ShaderStage,
    VertexAttribute, VertexBufferLayout, VertexFormat,
};

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn target(device: &Device, size: u32) -> Framebuffer {
    device
        .create_framebuffer(FramebufferDescriptor {
            label: "programmable",
            width: size,
            height: size,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap()
}
fn shader(device: &Device, stage: ShaderStage, inputs: u32, ops: &[Op], outputs: &[u32]) -> Shader {
    device
        .create_shader(ShaderDescriptor {
            label: "test shader",
            stage,
            input_count: inputs,
            instructions: ops,
            outputs,
        })
        .unwrap()
}
fn stages(device: &Device) -> (Shader, Shader) {
    (
        shader(
            device,
            ShaderStage::Vertex,
            2,
            &[Op::Input { location: 0 }, Op::Input { location: 1 }],
            &[0, 1],
        ),
        shader(
            device,
            ShaderStage::Fragment,
            1,
            &[Op::Input { location: 0 }],
            &[0],
        ),
    )
}
fn attributes() -> [VertexAttribute; 2] {
    // Deliberately out of location order: locations select shader inputs.
    [
        VertexAttribute {
            location: 1,
            offset: 16,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            location: 0,
            offset: 0,
            format: VertexFormat::Float32x4,
        },
    ]
}
fn pipeline(device: &Device, vertex: &Shader, fragment: &Shader) -> GraphicsPipeline {
    device
        .create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "test pipeline",
            vertex_shader: vertex,
            fragment_shader: fragment,
            vertex_layout: VertexBufferLayout {
                stride: 32,
                attributes: &attributes(),
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap()
}
fn data() -> [[f32; 8]; 3] {
    [
        [-1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
    ]
}
fn bytes(data: &[[f32; 8]]) -> Vec<u8> {
    data.iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect()
}
fn buffer(device: &Device, bytes: &[u8]) -> Buffer {
    let buffer = device
        .create_buffer(BufferDescriptor {
            label: "vertices",
            size: bytes.len() as u64,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    buffer.write(0, bytes).unwrap();
    buffer
}
fn draw(
    device: &Device,
    target: &Framebuffer,
    pipeline: &GraphicsPipeline,
    buffer: &Buffer,
) -> Result<(), QueueSubmitError> {
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(target, Some([0; 4])).unwrap();
    pass.set_pipeline(pipeline).unwrap();
    pass.set_vertex_buffer(buffer, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish())
}

#[test]
fn shaders_control_positions_and_fragment_colours() {
    let device = device();
    let target = target(&device, 8);
    let vertex = shader(
        &device,
        ShaderStage::Vertex,
        2,
        &[
            Op::Input { location: 0 },
            Op::Input { location: 1 },
            Op::Constant([0.5, 0.5, 1.0, 1.0]),
            Op::Multiply { left: 0, right: 2 },
        ],
        &[3, 1],
    );
    let fragment = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[
            Op::Input { location: 0 },
            Op::Swizzle {
                value: 0,
                components: [2, 1, 0, 3],
            },
        ],
        &[1],
    );
    draw(
        &device,
        &target,
        &pipeline(&device, &vertex, &fragment),
        &buffer(&device, &bytes(&data())),
    )
    .unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), [0; 4]);
    assert_eq!(target.pixel(2, 2).unwrap(), [32, 32, 191, 255]);
    assert_eq!(target.pixel(3, 2).unwrap(), [32, 96, 128, 255]);
}

#[test]
fn barycentric_colours_perspective_division_and_winding() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let ordinary = target(&device, 4);
    draw(&device, &ordinary, &p, &buffer(&device, &bytes(&data()))).unwrap();
    assert_eq!(ordinary.pixel(0, 0).unwrap(), [191, 32, 32, 255]);
    assert_eq!(ordinary.pixel(1, 0).unwrap(), [128, 96, 32, 255]);
    let mut perspective = data();
    perspective[1][0] = 2.0;
    perspective[1][1] = 2.0;
    perspective[1][3] = 2.0;
    perspective[2][0] = -4.0;
    perspective[2][1] = -4.0;
    perspective[2][3] = 4.0;
    let projected = target(&device, 4);
    draw(
        &device,
        &projected,
        &p,
        &buffer(&device, &bytes(&perspective)),
    )
    .unwrap();
    assert_eq!(projected.pixel(0, 0).unwrap(), [227, 19, 9, 255]);
    perspective.swap(1, 2);
    let reverse = target(&device, 4);
    draw(
        &device,
        &reverse,
        &p,
        &buffer(&device, &bytes(&perspective)),
    )
    .unwrap();
    let mut a = [[0; 4]; 16];
    let mut b = a;
    projected.read_pixels(&mut a).unwrap();
    reverse.read_pixels(&mut b).unwrap();
    assert_eq!(a, b);
}

#[test]
fn clipping_preserves_varyings_and_clips_z_without_depth_testing() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let mut enlarged = data();
    enlarged[1][0] = 3.0;
    enlarged[2][1] = -3.0;
    let image = target(&device, 4);
    draw(&device, &image, &p, &buffer(&device, &bytes(&enlarged))).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [223, 16, 16, 255]);
    assert_eq!(image.pixel(3, 3).unwrap(), [32, 112, 112, 255]);
    let mut depth_clip = data();
    depth_clip[0][2] = -2.0;
    draw(&device, &image, &p, &buffer(&device, &bytes(&depth_clip))).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [0; 4]);
    assert_eq!(image.pixel(2, 0).unwrap(), [64, 159, 32, 255]);
    for vertex in &mut depth_clip {
        vertex[2] = 2.0;
    }
    draw(&device, &image, &p, &buffer(&device, &bytes(&depth_clip))).unwrap();
    let mut pixels = [[9; 4]; 16];
    image.read_pixels(&mut pixels).unwrap();
    assert_eq!(pixels, [[0; 4]; 16]);
}

#[test]
fn fragment_clamps_rounds_and_overwrites_alpha_without_blending() {
    let device = device();
    let (v, _) = stages(&device);
    let f = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[Op::Constant([-1.0, 2.0, 0.5, 0.0])],
        &[0],
    );
    let image = target(&device, 4);
    draw(
        &device,
        &image,
        &pipeline(&device, &v, &f),
        &buffer(&device, &bytes(&data())),
    )
    .unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [0, 255, 128, 0]);
}

#[test]
fn attribute_formats_expand_components_and_ignore_padding() {
    let device = device();
    let (v, f) = stages(&device);
    for (format, expected) in [
        (VertexFormat::Float32, [255, 0, 0, 255]),
        (VertexFormat::Float32x2, [255, 128, 0, 255]),
        (VertexFormat::Float32x3, [255, 128, 64, 255]),
        (VertexFormat::Float32x4, [255, 128, 64, 0]),
    ] {
        let attrs = [
            VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x4,
            },
            VertexAttribute {
                location: 1,
                offset: 16,
                format,
            },
        ];
        let p = device
            .create_graphics_pipeline(GraphicsPipelineDescriptor {
                label: "formats",
                vertex_shader: &v,
                fragment_shader: &f,
                vertex_layout: VertexBufferLayout {
                    stride: 36,
                    attributes: &attrs,
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        let mut storage = Vec::new();
        for vertex in data() {
            storage.extend(vertex[..4].iter().flat_map(|n| n.to_le_bytes()));
            storage.extend(
                [1.0_f32, 0.5, 0.25, 0.0]
                    .iter()
                    .flat_map(|n| n.to_le_bytes()),
            );
            storage.extend_from_slice(&f32::NAN.to_le_bytes()); // unused padding is not input
        }
        let image = target(&device, 4);
        draw(&device, &image, &p, &buffer(&device, &storage)).unwrap();
        assert_eq!(image.pixel(0, 0).unwrap(), expected);
        assert_eq!(
            format.size(),
            match format {
                VertexFormat::Float32 => 4,
                VertexFormat::Float32x2 => 8,
                VertexFormat::Float32x3 => 12,
                VertexFormat::Float32x4 => 16,
            }
        );
    }
}

#[test]
fn pipeline_owns_metadata_retains_stages_and_shares_identity() {
    let device = device();
    let (v, f) = stages(&device);
    let p = {
        let label = String::from("owned pipeline");
        let mut attrs = attributes();
        let p = device
            .create_graphics_pipeline(GraphicsPipelineDescriptor {
                label: &label,
                vertex_shader: &v,
                fragment_shader: &f,
                vertex_layout: VertexBufferLayout {
                    stride: 32,
                    attributes: &attrs,
                },
                colour_format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap();
        attrs[0].offset = u64::MAX;
        assert_ne!(p.vertex_attributes(), attrs);
        p
    };
    assert_eq!(p.label(), "owned pipeline");
    assert_eq!(p.vertex_stride(), 32);
    assert_eq!(p.vertex_attributes(), attributes());
    assert_eq!(p.colour_format(), FramebufferFormat::Rgba8Unorm);
    assert!(p.vertex_shader().is_same(&v));
    assert!(p.fragment_shader().is_same(&f));
    assert!(p.device().is_same(&device));
    assert!(p.belongs_to(&device));
    assert!(!p.belongs_to(&device.adapter().create_device()));
    assert!(p.is_same(&p.clone()));
    assert!(!p.is_same(&pipeline(&device, &v, &f)));
    drop(v);
    drop(f);
    let image = target(&device, 4);
    draw(&device, &image, &p, &buffer(&device, &bytes(&data()))).unwrap();
}

#[test]
fn pipeline_rejects_foreign_stages_and_incompatible_interfaces() {
    let device = device();
    let (v, f) = stages(&device);
    let attrs = attributes();
    let desc = GraphicsPipelineDescriptor {
        label: "bad",
        vertex_shader: &v,
        fragment_shader: &f,
        vertex_layout: VertexBufferLayout {
            stride: 32,
            attributes: &attrs,
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    };
    let (other_v, other_f) = stages(&device.adapter().create_device());
    assert!(matches!(
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            vertex_shader: &other_v,
            ..desc
        }),
        Err(PipelineError::ForeignShader {
            stage: ShaderStage::Vertex,
            ..
        })
    ));
    assert!(matches!(
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            fragment_shader: &other_f,
            ..desc
        }),
        Err(PipelineError::ForeignShader {
            stage: ShaderStage::Fragment,
            ..
        })
    ));
    assert!(matches!(
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            vertex_shader: &f,
            ..desc
        }),
        Err(PipelineError::WrongShaderStage {
            expected: ShaderStage::Vertex,
            actual: ShaderStage::Fragment
        })
    ));
    assert!(matches!(
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            fragment_shader: &v,
            ..desc
        }),
        Err(PipelineError::WrongShaderStage {
            expected: ShaderStage::Fragment,
            actual: ShaderStage::Vertex
        })
    ));
    let extra = shader(
        &device,
        ShaderStage::Fragment,
        2,
        &[Op::Input { location: 0 }],
        &[0],
    );
    assert!(matches!(
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            fragment_shader: &extra,
            ..desc
        }),
        Err(PipelineError::StageInterfaceMismatch {
            vertex_varyings: 1,
            fragment_inputs: 2
        })
    ));
}

#[test]
fn pipeline_rejects_layout_errors_and_checked_attribute_overflow() {
    let device = device();
    let (v, f) = stages(&device);
    let check = |stride, attributes: &[VertexAttribute]| {
        device.create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "layout",
            vertex_shader: &v,
            fragment_shader: &f,
            vertex_layout: VertexBufferLayout { stride, attributes },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
    };
    for stride in [0, 1, 31] {
        assert!(matches!(
            check(stride, &attributes()),
            Err(PipelineError::InvalidStride { .. })
        ));
    }
    assert!(matches!(
        check(32, &attributes()[..1]),
        Err(PipelineError::AttributeCountMismatch {
            expected: 2,
            actual: 1
        })
    ));
    let mut attrs = attributes();
    attrs[0].location = 2;
    assert!(matches!(
        check(32, &attrs),
        Err(PipelineError::InvalidAttributeLocation {
            location: 2,
            input_count: 2
        })
    ));
    attrs = attributes();
    attrs[0].location = 0;
    assert!(matches!(
        check(32, &attrs),
        Err(PipelineError::DuplicateAttributeLocation { location: 0 })
    ));
    attrs = attributes();
    attrs[0].offset = 3;
    assert!(matches!(
        check(32, &attrs),
        Err(PipelineError::UnalignedAttributeOffset { offset: 3, .. })
    ));
    attrs[0].offset = 20;
    assert!(matches!(
        check(32, &attrs),
        Err(PipelineError::AttributeOutOfBounds { .. })
    ));
    attrs[0].offset = u64::MAX - 3;
    assert!(matches!(
        check(u64::MAX - 3, &attrs),
        Err(PipelineError::AttributeOutOfBounds { .. })
    ));
    attrs = attributes();
    attrs[0].offset = 0;
    assert!(check(16, &attrs).is_ok()); // alias reads allowed
}

#[test]
fn recording_checks_state_ranges_and_preserves_prior_bindings() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    let source = buffer(&device, &bytes(&data()));
    let other = device.adapter().create_device();
    let (ov, of) = stages(&other);
    let foreign = pipeline(&other, &ov, &of);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..3),
        Err(RenderEncodeError::MissingPipeline)
    ));
    pass.set_pipeline(&p).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..3),
        Err(RenderEncodeError::MissingVertexBuffer)
    ));
    pass.set_vertex_buffer(&source, 0).unwrap();
    assert!(matches!(
        pass.set_pipeline(&foreign),
        Err(RenderEncodeError::ForeignPipeline { .. })
    ));
    assert!(matches!(
        pass.draw_programmable(std::ops::Range { start: 3, end: 0 }),
        Err(RenderEncodeError::InvalidVertexRange { .. })
    ));
    assert!(matches!(
        pass.draw_programmable(0..4),
        Err(RenderEncodeError::IncompleteTriangle { .. })
    ));
    assert!(matches!(
        pass.draw_programmable(3..6),
        Err(RenderEncodeError::VertexRangeOutOfBounds { .. })
    ));
    pass.draw_programmable(3..3).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    let ex::Command::RenderPass(recorded) = &encoder.commands()[0] else {
        panic!("render expected")
    };
    assert_eq!(recorded.draws().len(), 2);
    assert!(recorded.draws()[1].pipeline().unwrap().is_same(&p));
    assert_eq!(recorded.draws()[1].colour(), [0; 4]);
    assert_eq!(image.pixel(0, 0).unwrap(), [0; 4]);
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [191, 32, 32, 255]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingPipeline)
    ));
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&source, 0).unwrap();
    let huge = device
        .create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "huge stride",
            vertex_shader: &v,
            fragment_shader: &f,
            vertex_layout: VertexBufferLayout {
                stride: u64::MAX - 3,
                attributes: &attributes(),
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap();
    pass.set_pipeline(&huge).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..3),
        Err(RenderEncodeError::VertexRangeOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.draw_programmable(2..2),
        Err(RenderEncodeError::VertexRangeOutOfBounds { .. })
    ));
}

#[test]
fn nonzero_binding_and_first_vertex_use_pipeline_stride() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    let mut storage = vec![0; 4 + 32 * 3];
    storage.extend(bytes(&data()));
    let source = buffer(&device, &storage);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&source, 4).unwrap();
    pass.draw_programmable(3..6).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [191, 32, 32, 255]);
}

#[test]
fn fragment_failure_rolls_back_clear_and_prior_draws_but_keeps_prior_commands() {
    let device = device();
    let (v, _) = stages(&device);
    let f = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[
            Op::Input { location: 0 },
            Op::Constant([4.0, 1.0, 1.0, 1.0]),
            Op::Multiply { left: 0, right: 1 },
        ],
        &[2],
    );
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    let mut vertices = data();
    vertices[0][4] = 0.0;
    vertices[1][4] = f32::MAX;
    vertices[2][4] = f32::MAX;
    let source = buffer(&device, &bytes(&vertices));
    let mut encoder = device.create_command_encoder();
    encoder
        .begin_render_pass(&image, Some([7; 4]))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = encoder.begin_render_pass(&image, Some([99; 4])).unwrap();
    let legacy = buffer(
        &device,
        &[-1.0_f32, 1.0, 1.0, 1.0, -1.0, -1.0]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    pass.set_vertex_buffer(&legacy, 0).unwrap();
    pass.draw(0..3, [22; 4]).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&source, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    encoder
        .begin_render_pass(&image, Some([88; 4]))
        .unwrap()
        .finish()
        .unwrap();
    let error = device
        .default_queue()
        .submit(&encoder.finish())
        .unwrap_err();
    assert!(matches!(
        error,
        QueueSubmitError::RenderFailed {
            command_index: 1,
            source: RenderExecutionError::NonFiniteShaderResult {
                stage: ShaderStage::Fragment,
                invocation: 1,
                instruction: 2,
                ..
            }
        }
    ));
    let mut pixels = [[0; 4]; 16];
    image.read_pixels(&mut pixels).unwrap();
    assert_eq!(pixels, [[7; 4]; 16]);
}

#[test]
fn invalid_vertex_numbers_clip_w_and_vertex_arithmetic_preserve_target() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    for w in [0.0, -1.0] {
        let mut vertices = data();
        vertices[1][3] = w;
        assert!(matches!(
            draw(&device, &image, &p, &buffer(&device, &bytes(&vertices))),
            Err(QueueSubmitError::RenderFailed {
                source: RenderExecutionError::InvalidClipW { vertex_index: 1 },
                ..
            })
        ));
        assert_eq!(image.pixel(0, 0).unwrap(), [0; 4]);
    }
    let mut vertices = data();
    vertices[2][4] = f32::NAN;
    assert!(matches!(
        draw(&device, &image, &p, &buffer(&device, &bytes(&vertices))),
        Err(QueueSubmitError::RenderFailed {
            source: RenderExecutionError::NonFiniteShaderInput {
                stage: ShaderStage::Vertex,
                invocation: 2,
                location: 1,
                ..
            },
            ..
        })
    ));
    let overflowing = shader(
        &device,
        ShaderStage::Vertex,
        2,
        &[
            Op::Input { location: 0 },
            Op::Input { location: 1 },
            Op::Constant([4.0; 4]),
            Op::Multiply { left: 1, right: 2 },
        ],
        &[0, 3],
    );
    vertices = data();
    vertices[0][4] = f32::MAX;
    assert!(matches!(
        draw(
            &device,
            &image,
            &pipeline(&device, &overflowing, &f),
            &buffer(&device, &bytes(&vertices))
        ),
        Err(QueueSubmitError::RenderFailed {
            source: RenderExecutionError::NonFiniteShaderResult {
                stage: ShaderStage::Vertex,
                invocation: 0,
                instruction: 3,
                ..
            },
            ..
        })
    ));
}

#[test]
fn copy_upload_reuse_and_dropped_handles_retain_live_programmable_resources() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    let upload = device
        .create_buffer(BufferDescriptor {
            label: "upload",
            size: 96,
            usages: &[BufferUsage::TransferSource],
            host_access: HostAccess::Write,
        })
        .unwrap();
    upload.write(0, &bytes(&data())).unwrap();
    let source = device
        .create_buffer(BufferDescriptor {
            label: "device vertices",
            size: 96,
            usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
            host_access: HostAccess::None,
        })
        .unwrap();
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &source, 0, 96).unwrap();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&source, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    let commands = encoder.finish();
    drop(v);
    drop(f);
    drop(p);
    drop(source);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [191, 32, 32, 255]);
    let mut blue = data();
    for vertex in &mut blue {
        vertex[4..].copy_from_slice(&[0.0, 0.0, 1.0, 1.0]);
    }
    upload.write(0, &bytes(&blue)).unwrap();
    drop(upload);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [0, 0, 255, 255]);
}

#[test]
fn mixed_draw_order_and_pipeline_changes_are_retained_per_draw() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    let solid = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[Op::Constant([0.0, 1.0, 0.0, 1.0])],
        &[0],
    );
    let green = pipeline(&device, &v, &solid);
    let source = buffer(&device, &bytes(&data()));
    let fixed = buffer(
        &device,
        &[-1.0_f32, 1.0, 1.0, 1.0, -1.0, -1.0]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&source, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.set_pipeline(&green).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.set_vertex_buffer(&fixed, 0).unwrap();
    pass.draw(0..3, [44; 4]).unwrap();
    pass.finish().unwrap();
    let ex::Command::RenderPass(recorded) = &encoder.commands()[0] else {
        panic!("render expected")
    };
    assert!(recorded.draws()[0].pipeline().unwrap().is_same(&p));
    assert!(recorded.draws()[1].pipeline().unwrap().is_same(&green));
    assert!(recorded.draws()[2].pipeline().is_none());
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [44; 4]);
}

#[test]
fn multiple_varyings_and_extreme_finite_geometry_execute() {
    let device = device();
    let v = shader(
        &device,
        ShaderStage::Vertex,
        2,
        &[Op::Input { location: 0 }, Op::Input { location: 1 }],
        &[0, 1, 1],
    );
    let f = shader(
        &device,
        ShaderStage::Fragment,
        2,
        &[
            Op::Input { location: 0 },
            Op::Input { location: 1 },
            Op::Add { left: 0, right: 1 },
        ],
        &[2],
    );
    let image = target(&device, 4);
    draw(
        &device,
        &image,
        &pipeline(&device, &v, &f),
        &buffer(&device, &bytes(&data())),
    )
    .unwrap();
    assert_eq!(image.pixel(0, 0).unwrap(), [255, 64, 64, 255]);
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let vertices = [
        [
            0.0,
            f32::MAX,
            0.0,
            1.0,
            f32::MAX,
            f32::MAX,
            f32::MAX,
            f32::MAX,
        ],
        [
            -f32::MAX,
            -f32::MAX,
            0.0,
            1.0,
            f32::MAX,
            f32::MAX,
            f32::MAX,
            f32::MAX,
        ],
        [
            f32::MAX,
            -f32::MAX,
            0.0,
            1.0,
            f32::MAX,
            f32::MAX,
            f32::MAX,
            f32::MAX,
        ],
    ];
    draw(&device, &image, &p, &buffer(&device, &bytes(&vertices))).unwrap();
    let mut pixels = [[0; 4]; 16];
    image.read_pixels(&mut pixels).unwrap();
    assert_eq!(pixels, [[255; 4]; 16]);
}

#[test]
fn shared_edges_have_single_ownership_and_degenerate_triangles_write_nothing() {
    let device = device();
    let (v, _) = stages(&device);
    let red = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[Op::Constant([1.0, 0.0, 0.0, 1.0])],
        &[0],
    );
    let green = shader(
        &device,
        ShaderStage::Fragment,
        1,
        &[Op::Constant([0.0, 1.0, 0.0, 1.0])],
        &[0],
    );
    let red = pipeline(&device, &v, &red);
    let green = pipeline(&device, &v, &green);
    let first = buffer(&device, &bytes(&data()));
    let mut second = data();
    second[0][0] = 1.0;
    second[1][1] = -1.0;
    let second = buffer(&device, &bytes(&second));
    let image = target(&device, 4);
    let mut previous = [[0; 4]; 16];
    for reverse in [false, true] {
        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_render_pass(&image, Some([0; 4])).unwrap();
        let mut draws = [(&red, &first), (&green, &second)];
        if reverse {
            draws.reverse();
        }
        for (p, b) in draws {
            pass.set_pipeline(p).unwrap();
            pass.set_vertex_buffer(b, 0).unwrap();
            pass.draw_programmable(0..3).unwrap();
        }
        pass.finish().unwrap();
        device.default_queue().submit(&encoder.finish()).unwrap();
        let mut pixels = [[0; 4]; 16];
        image.read_pixels(&mut pixels).unwrap();
        assert_eq!(pixels.iter().filter(|&&p| p == [255, 0, 0, 255]).count(), 6);
        assert_eq!(
            pixels.iter().filter(|&&p| p == [0, 255, 0, 255]).count(),
            10
        );
        if reverse {
            assert_eq!(pixels, previous);
        } else {
            previous = pixels;
        }
    }
    let mut flat = data();
    flat[1] = flat[0];
    let flat = buffer(&device, &bytes(&flat));
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&image, None).unwrap();
    pass.set_pipeline(&red).unwrap();
    pass.set_vertex_buffer(&flat, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.draw_programmable(3..3).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    let mut pixels = [[0; 4]; 16];
    image.read_pixels(&mut pixels).unwrap();
    assert_eq!(pixels, previous);
}

#[test]
fn perspective_interpolation_handles_smallest_and_large_positive_clip_scales() {
    let device = device();
    let (v, f) = stages(&device);
    let p = pipeline(&device, &v, &f);
    let image = target(&device, 4);
    for scale in [
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-20,
        1.0,
        1e20,
        f32::MAX / 4.0,
    ] {
        let mut vertices = data();
        for (vertex, factor) in vertices.iter_mut().zip([1.0, 2.0, 4.0]) {
            for coordinate in &mut vertex[..4] {
                *coordinate *= scale * factor;
            }
        }
        draw(&device, &image, &p, &buffer(&device, &bytes(&vertices))).unwrap();
        assert_eq!(image.pixel(0, 0).unwrap(), [227, 19, 9, 255]);
        assert_eq!(image.pixel(3, 3).unwrap(), [0; 4]);
    }
}
