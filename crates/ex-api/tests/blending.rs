use ex::*;

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn target(d: &Device) -> Framebuffer {
    d.create_framebuffer(FramebufferDescriptor {
        label: "colour",
        width: 4,
        height: 4,
        format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn records(d: &Device, values: &[[f32; 4]]) -> Buffer {
    let bytes: Vec<_> = values
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let b = d
        .create_buffer(BufferDescriptor {
            label: "v",
            size: bytes.len() as u64,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    b.write(0, &bytes).unwrap();
    b
}
fn quad(d: &Device, z: f32) -> Buffer {
    records(
        d,
        &[
            [-1., 1., z, 1.],
            [1., 1., z, 1.],
            [1., -1., z, 1.],
            [1., -1., z, 1.],
            [-1., -1., z, 1.],
            [-1., 1., z, 1.],
        ],
    )
}
fn shaders(d: &Device, rgba: [f32; 4], fail: bool) -> (Shader, Shader) {
    let v = d
        .create_shader(ShaderDescriptor {
            label: "position",
            stage: ShaderStage::Vertex,
            input_count: 1,
            instructions: &[ShaderInstruction::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let ops = if fail {
        vec![
            ShaderInstruction::Constant([f32::MAX; 4]),
            ShaderInstruction::Multiply { left: 0, right: 0 },
        ]
    } else {
        vec![ShaderInstruction::Constant(rgba)]
    };
    let f = d
        .create_shader(ShaderDescriptor {
            label: "colour",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &ops,
            outputs: &[if fail { 1 } else { 0 }],
        })
        .unwrap();
    (v, f)
}
fn descriptor<'a>(v: &'a Shader, f: &'a Shader) -> GraphicsPipelineDescriptor<'a> {
    GraphicsPipelineDescriptor {
        label: "blend",
        vertex_shader: v,
        fragment_shader: f,
        vertex_layout: VertexBufferLayout {
            stride: 16,
            attributes: &[VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x4,
            }],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    }
}
fn pipeline(
    d: &Device,
    rgba: [f32; 4],
    blend: Option<BlendState>,
    depth: Option<DepthState>,
    fail: bool,
) -> GraphicsPipeline {
    let (v, f) = shaders(d, rgba, fail);
    d.create_graphics_pipeline_with_options(
        descriptor(&v, &f),
        GraphicsPipelineOptions { depth, blend },
    )
    .unwrap()
}
fn draw(p: &mut RenderPass<'_>, pipeline: &GraphicsPipeline, v: &Buffer) {
    p.set_pipeline(pipeline).unwrap();
    p.set_vertex_buffer(v, 0).unwrap();
    p.draw_programmable(0..6).unwrap();
}
fn render(
    d: &Device,
    t: &Framebuffer,
    rgba: [f32; 4],
    blend: Option<BlendState>,
    clear: [u8; 4],
) -> [u8; 4] {
    let v = quad(d, 0.);
    let pipeline = pipeline(d, rgba, blend, None, false);
    let mut e = d.create_command_encoder();
    let mut p = e.begin_render_pass(t, Some(clear)).unwrap();
    draw(&mut p, &pipeline, &v);
    p.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    let mut pixels = [[0; 4]; 16];
    t.read_pixels(&mut pixels).unwrap();
    assert!(pixels.iter().all(|v| *v == pixels[0]));
    pixels[0]
}
fn equation(
    source_factor: BlendFactor,
    destination_factor: BlendFactor,
    operation: BlendOperation,
) -> BlendState {
    let component = BlendComponent {
        source_factor,
        destination_factor,
        operation,
    };
    BlendState {
        colour: component,
        alpha: component,
        write_mask: [true; 4],
    }
}
fn depth_state(write_enabled: bool) -> DepthState {
    DepthState {
        compare: CompareFunction::Less,
        write_enabled,
    }
}

#[test]
fn options_preserve_creation_validation_and_retained_immutable_state() {
    let d = device();
    let (v, f) = shaders(&d, [1.; 4], false);
    let desc = descriptor(&v, &f);
    let options = GraphicsPipelineOptions::default();
    assert_eq!(
        options,
        GraphicsPipelineOptions {
            depth: None,
            blend: None
        }
    );
    for p in [
        d.create_graphics_pipeline(desc).unwrap(),
        d.create_graphics_pipeline_with_options(desc, options)
            .unwrap(),
        d.create_graphics_pipeline_with_depth(desc, depth_state(true))
            .unwrap(),
    ] {
        assert_eq!(p.blend_state(), None);
    }
    let mut blend = BlendState::alpha_blending();
    let p = d
        .create_graphics_pipeline_with_options(
            desc,
            GraphicsPipelineOptions {
                depth: Some(depth_state(false)),
                blend: Some(blend),
            },
        )
        .unwrap();
    blend.write_mask = [false; 4];
    assert_eq!(blend.write_mask, [false; 4]);
    assert_eq!(p.blend_state(), Some(BlendState::alpha_blending()));
    assert_eq!(p.depth_state(), Some(depth_state(false)));
    assert!(p.is_same(&p.clone()));
    let other = device();
    assert!(matches!(
        other.create_graphics_pipeline_with_options(desc, options),
        Err(GraphicsPipelineCreateError::ForeignShader { .. })
    ));
    let bad = GraphicsPipelineDescriptor {
        vertex_layout: VertexBufferLayout {
            stride: 0,
            attributes: desc.vertex_layout.attributes,
        },
        ..desc
    };
    assert!(matches!(
        d.create_graphics_pipeline_with_options(bad, options),
        Err(GraphicsPipelineCreateError::InvalidStride { .. })
    ));
    drop(v);
    drop(f);
    assert_eq!(p.fragment_shader().evaluate(&[]).unwrap(), vec![[1.; 4]]);
}

#[test]
fn every_factor_scales_source_and_destination_including_alpha() {
    use BlendFactor::*;
    let d = device();
    let t = target(&d);
    // Independent rational calculations for source (.5,.25,.75,.5), destination bytes.
    for (factor, source, destination) in [
        (Zero, [0, 0, 0, 0], [0, 0, 0, 0]),
        (One, [128, 64, 191, 128], [64, 128, 192, 64]),
        (SrcColour, [64, 16, 143, 64], [32, 32, 144, 32]),
        (OneMinusSrcColour, [64, 48, 48, 64], [32, 96, 48, 32]),
        (DstColour, [32, 32, 144, 32], [16, 64, 145, 16]),
        (OneMinusDstColour, [96, 32, 47, 96], [48, 64, 47, 48]),
        (SrcAlpha, [64, 32, 96, 64], [32, 64, 96, 32]),
        (OneMinusSrcAlpha, [64, 32, 96, 64], [32, 64, 96, 32]),
        (DstAlpha, [32, 16, 48, 32], [16, 32, 48, 16]),
        (OneMinusDstAlpha, [96, 48, 143, 96], [48, 96, 144, 48]),
        (SrcAlphaSaturate, [64, 32, 96, 128], [32, 64, 96, 64]),
    ] {
        let rgba = [0.5, 0.25, 0.75, 0.5];
        let clear = [64, 128, 192, 64];
        assert_eq!(
            render(
                &d,
                &t,
                rgba,
                Some(equation(factor, Zero, BlendOperation::Add)),
                clear
            ),
            source,
            "source {factor:?}"
        );
        assert_eq!(
            render(
                &d,
                &t,
                rgba,
                Some(equation(Zero, factor, BlendOperation::Add)),
                clear
            ),
            destination,
            "destination {factor:?}"
        );
    }
    // Saturation chooses the destination limit, rather than source alpha, here.
    assert_eq!(
        render(
            &d,
            &t,
            [1., 1., 1., 0.75],
            Some(equation(SrcAlphaSaturate, Zero, BlendOperation::Add)),
            [0, 0, 0, 204]
        ),
        [51, 51, 51, 191]
    );
}

#[test]
fn operations_and_independent_alpha_equation() {
    use BlendFactor::*;
    use BlendOperation::*;
    let d = device();
    let t = target(&d);
    for (op, expected) in [
        (Add, [255, 192, 255, 192]),
        (Subtract, [127, 0, 0, 64]),
        (ReverseSubtract, [0, 64, 65, 0]),
        (Min, [64, 64, 128, 64]),
        (Max, [191, 128, 192, 128]),
    ] {
        let factor = if matches!(op, Min | Max) { Zero } else { One };
        assert_eq!(
            render(
                &d,
                &t,
                [0.75, 0.25, 0.5, 0.5],
                Some(equation(factor, factor, op)),
                [64, 128, 192, 64]
            ),
            expected,
            "{op:?}"
        );
    }
    let state = BlendState {
        colour: BlendState::additive().colour,
        alpha: BlendComponent {
            source_factor: Zero,
            destination_factor: One,
            operation: ReverseSubtract,
        },
        write_mask: [true; 4],
    };
    assert_eq!(
        render(&d, &t, [0.5; 4], Some(state), [64; 4]),
        [192, 192, 192, 64]
    );
}

#[test]
fn straight_and_premultiplied_presets_accumulate_alpha_without_squaring_it() {
    let d = device();
    let t = target(&d);
    for clear in [[0, 0, 255, 255], [0, 0, 0, 0], [0, 0, 64, 64]] {
        let a = render(
            &d,
            &t,
            [1., 0., 0., 0.5],
            Some(BlendState::alpha_blending()),
            clear,
        );
        let b = render(
            &d,
            &t,
            [0.5, 0., 0., 0.5],
            Some(BlendState::premultiplied_alpha()),
            clear,
        );
        assert_eq!(a, b);
        assert_eq!(
            a,
            [
                128,
                0,
                (f64::from(clear[2]) * 0.5).round() as u8,
                (127.5 + f64::from(clear[3]) * 0.5).round() as u8
            ]
        );
    }
    assert_eq!(
        render(
            &d,
            &t,
            [1., 0., 0., 0.],
            Some(BlendState::alpha_blending()),
            [13, 29, 71, 173]
        ),
        [13, 29, 71, 173]
    );
}

#[test]
fn all_channel_masks_preserve_disabled_bytes_and_never_mask_clear() {
    let d = device();
    let t = target(&d);
    for bits in 0..16 {
        let mask = std::array::from_fn(|i| bits & (1 << i) != 0);
        let mut b = BlendState::additive();
        b.write_mask = mask;
        let actual = render(&d, &t, [0.5; 4], Some(b), [13, 29, 71, 173]);
        let blended = [141, 157, 199, 255];
        let expected = std::array::from_fn(|i| {
            if mask[i] {
                blended[i]
            } else {
                [13, 29, 71, 173][i]
            }
        });
        assert_eq!(actual, expected);
    }
}

#[test]
fn source_clamping_and_per_fragment_quantization() {
    let d = device();
    let t = target(&d);
    // A half-byte tie must round upward without a normalize/denormalize drift.
    assert_eq!(
        render(
            &d,
            &t,
            [0., 0., 1., 0.5],
            Some(BlendState::alpha_blending()),
            [16, 24, 40, 255]
        ),
        [8, 12, 148, 255]
    );
    assert_eq!(
        render(
            &d,
            &t,
            [-1., 2., f32::MAX, 2.],
            Some(BlendState::alpha_blending()),
            [64; 4]
        ),
        [0, 255, 255, 255]
    );
    assert_eq!(
        render(
            &d,
            &t,
            [1., 1., 1., -1.],
            Some(BlendState::alpha_blending()),
            [64; 4]
        ),
        [64; 4]
    );
    let v = quad(&d, 0.);
    let p = pipeline(
        &d,
        [0.5; 4],
        Some(BlendState::alpha_blending()),
        None,
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([0; 4])).unwrap();
    draw(&mut pass, &p, &v);
    draw(&mut pass, &p, &v);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [96, 96, 96, 192]);
    // The original disabled blend conversion remains unchanged.
    assert_eq!(
        render(&d, &t, [-1., 2., 0.5, 0.], None, [64; 4]),
        [0, 255, 128, 0]
    );
}

#[test]
fn transparent_draw_order_matters_while_additive_example_is_commutative() {
    let d = device();
    let t = target(&d);
    let v = quad(&d, 0.);
    for additive in [false, true] {
        let b = if additive {
            BlendState::additive()
        } else {
            BlendState::alpha_blending()
        };
        let red = pipeline(&d, [0.5, 0., 0., 0.5], Some(b), None, false);
        let blue = pipeline(&d, [0., 0., 0.5, 0.5], Some(b), None, false);
        for reverse in [false, true] {
            let mut e = d.create_command_encoder();
            let mut pass = e.begin_render_pass(&t, Some([0; 4])).unwrap();
            for p in if reverse {
                [&blue, &red]
            } else {
                [&red, &blue]
            } {
                draw(&mut pass, p, &v);
            }
            pass.finish().unwrap();
            d.default_queue().submit(&e.finish()).unwrap();
            let expected = if additive {
                [128, 0, 128, 255]
            } else if reverse {
                [64, 0, 32, 192]
            } else {
                [32, 0, 64, 192]
            };
            assert_eq!(t.pixel(0, 0).unwrap(), expected);
        }
    }
}

#[test]
fn depth_rejection_write_control_and_zero_alpha_do_not_imply_discard() {
    let d = device();
    let t = target(&d);
    let z = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "z",
            width: 4,
            height: 4,
        })
        .unwrap();
    let near = quad(&d, -0.5);
    let far = quad(&d, 0.5);
    let invisible = pipeline(
        &d,
        [1., 0., 0., 0.],
        Some(BlendState::alpha_blending()),
        Some(depth_state(true)),
        false,
    );
    let blue = pipeline(
        &d,
        [0., 0., 1., 1.],
        Some(BlendState::alpha_blending()),
        Some(depth_state(false)),
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([7; 4]), &z, Some(1.))
        .unwrap();
    draw(&mut pass, &invisible, &near);
    draw(&mut pass, &blue, &far);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [7; 4]);
    assert_eq!(z.depth(0, 0).unwrap(), 0.25);
    let mut masked = BlendState::alpha_blending();
    masked.write_mask = [false; 4];
    let p = pipeline(&d, [1.; 4], Some(masked), Some(depth_state(true)), false);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([9; 4]), &z, Some(1.))
        .unwrap();
    draw(&mut pass, &p, &near);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [9; 4]);
    assert_eq!(z.depth(0, 0).unwrap(), 0.25);
    let transparent = pipeline(
        &d,
        [1., 0., 0., 0.5],
        Some(BlendState::alpha_blending()),
        Some(depth_state(false)),
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([0, 0, 255, 255]), &z, Some(1.))
        .unwrap();
    draw(&mut pass, &transparent, &near);
    draw(&mut pass, &blue, &far);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(z.depth(0, 0).unwrap(), 1.);
    assert_eq!(t.pixel(0, 0).unwrap(), [0, 0, 255, 255]);
}

#[test]
fn indexed_sampled_blending_retains_live_texture_and_pipeline_resources() {
    let d = device();
    let t = target(&d);
    let v = quad(&d, 0.);
    let (vs, _) = shaders(&d, [1.; 4], false);
    let fs = d
        .create_shader(ShaderDescriptor {
            label: "sample",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &[
                ShaderInstruction::Constant([0.5; 4]),
                ShaderInstruction::SampleTexture {
                    binding: 0,
                    coordinates: 0,
                },
            ],
            outputs: &[1],
        })
        .unwrap();
    let p = d
        .create_graphics_pipeline_with_options(
            descriptor(&vs, &fs),
            GraphicsPipelineOptions {
                depth: None,
                blend: Some(BlendState::alpha_blending()),
            },
        )
        .unwrap();
    let tex = d
        .create_texture(TextureDescriptor {
            label: "live",
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::Write,
        })
        .unwrap();
    let s = d
        .create_sampler(SamplerDescriptor {
            label: "s",
            filter: FilterMode::Nearest,
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
        })
        .unwrap();
    let i = d
        .create_buffer(BufferDescriptor {
            label: "i",
            size: 12,
            usages: &[BufferUsage::Index],
            host_access: HostAccess::Write,
        })
        .unwrap();
    i.write(0, &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]).unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([0, 0, 255, 255])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
    pass.set_texture(0, &tex.view(), &s).unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    drop(p);
    drop(vs);
    drop(fs);
    drop(v);
    drop(i);
    drop(s);
    for (source, expected) in [
        ([255, 0, 0, 128], [128, 0, 127, 255]),
        ([0, 255, 0, 128], [0, 128, 127, 255]),
    ] {
        tex.write_pixels(&[source]).unwrap();
        d.default_queue().submit(&c).unwrap();
        assert_eq!(t.pixel(0, 0).unwrap(), expected);
    }
}

#[test]
fn interpolated_alpha_controls_fragment_blending() {
    let d = device();
    let t = target(&d);
    let v = records(
        &d,
        &[
            [-1., 1., 0., 1.],
            [1., 0., 0., 0.],
            [1., 1., 0., 1.],
            [1., 0., 0., 1.],
            [-1., -1., 0., 1.],
            [1., 0., 0., 0.],
        ],
    );
    let vs = d
        .create_shader(ShaderDescriptor {
            label: "vary",
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
            label: "vary",
            stage: ShaderStage::Fragment,
            input_count: 1,
            instructions: &[ShaderInstruction::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let p = d
        .create_graphics_pipeline_with_options(
            GraphicsPipelineDescriptor {
                vertex_layout: VertexBufferLayout {
                    stride: 32,
                    attributes: &[
                        VertexAttribute {
                            location: 0,
                            offset: 0,
                            format: VertexFormat::Float32x4,
                        },
                        VertexAttribute {
                            location: 1,
                            offset: 16,
                            format: VertexFormat::Float32x4,
                        },
                    ],
                },
                ..descriptor(&vs, &fs)
            },
            GraphicsPipelineOptions {
                depth: None,
                blend: Some(BlendState::alpha_blending()),
            },
        )
        .unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([0, 0, 255, 255])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [32, 0, 223, 255]);
}

#[test]
fn late_fragment_failure_rolls_back_blended_colour_and_depth_even_with_mask_off() {
    let d = device();
    let t = target(&d);
    let z = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "z",
            width: 4,
            height: 4,
        })
        .unwrap();
    let far = quad(&d, 0.5);
    let near = quad(&d, -0.5);
    let good = pipeline(
        &d,
        [1., 0., 0., 0.5],
        Some(BlendState::alpha_blending()),
        Some(depth_state(true)),
        false,
    );
    let mut mask = BlendState::additive();
    mask.write_mask = [false; 4];
    let bad = pipeline(&d, [1.; 4], Some(mask), Some(depth_state(true)), true);
    let mut e = d.create_command_encoder();
    e.begin_render_pass_with_depth(&t, Some([8; 4]), &z, Some(0.8))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([9; 4]), &z, Some(1.))
        .unwrap();
    draw(&mut pass, &good, &far);
    draw(&mut pass, &bad, &near);
    pass.finish().unwrap();
    e.begin_render_pass_with_depth(&t, Some([10; 4]), &z, Some(0.))
        .unwrap()
        .finish()
        .unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::RenderFailed {
            command_index: 1,
            source: RenderExecutionError::NonFiniteShaderResult { .. }
        })
    ));
    assert_eq!(t.pixel(0, 0).unwrap(), [8; 4]);
    assert_eq!(z.depth(0, 0).unwrap(), 0.8);
}

#[test]
fn legacy_fixed_draw_ignores_bound_blend_pipeline() {
    let d = device();
    let t = target(&d);
    let p = pipeline(
        &d,
        [0.; 4],
        Some(BlendState::alpha_blending()),
        Some(depth_state(true)),
        false,
    );
    let bytes: Vec<_> = [[-1_f32, 1.], [1., 1.], [-1., -1.]]
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let v = d
        .create_buffer(BufferDescriptor {
            label: "legacy",
            size: 24,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    v.write(0, &bytes).unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, Some([7; 4])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw(0..3, [64, 128, 192, 0]).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [64, 128, 192, 0]);
}
