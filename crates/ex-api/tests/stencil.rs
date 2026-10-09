use ex::*;
fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn target(d: &Device) -> Framebuffer {
    d.create_framebuffer(FramebufferDescriptor {
        label: "c",
        width: 4,
        height: 4,
        format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn stencil(d: &Device) -> StencilBuffer {
    d.create_stencil_buffer(StencilBufferDescriptor {
        label: "s",
        width: 4,
        height: 4,
    })
    .unwrap()
}
fn depth(d: &Device) -> DepthBuffer {
    d.create_depth_buffer(DepthBufferDescriptor {
        label: "z",
        width: 4,
        height: 4,
    })
    .unwrap()
}
fn vertices(d: &Device, data: &[[f32; 4]]) -> Buffer {
    let bytes: Vec<_> = data
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
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
    vertices(
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
fn descriptor<'a>(v: &'a Shader, f: &'a Shader) -> GraphicsPipelineDescriptor<'a> {
    GraphicsPipelineDescriptor {
        label: "p",
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
    state: StencilState,
    options: GraphicsPipelineOptions,
    fail: bool,
) -> GraphicsPipeline {
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
        vec![ShaderInstruction::Constant([1., 0., 0., 0.5])]
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
    d.create_graphics_pipeline_with_stencil(descriptor(&v, &f), options, state)
        .unwrap()
}
fn draw(pass: &mut RenderPass<'_>, p: &GraphicsPipeline, v: &Buffer, count: u32) {
    pass.set_pipeline(p).unwrap();
    pass.set_vertex_buffer(v, 0).unwrap();
    pass.draw_programmable(0..count).unwrap();
}
fn render(
    d: &Device,
    t: &Framebuffer,
    s: &StencilBuffer,
    state: StencilState,
    initial: u8,
) -> ([u8; 4], u8) {
    let v = quad(d, 0.);
    let p = pipeline(d, state, GraphicsPipelineOptions::default(), false);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            t,
            Some([7; 4]),
            RenderAttachments {
                stencil: Some(s),
                clear_stencil: Some(initial),
                ..RenderAttachments::default()
            },
        )
        .unwrap();
    draw(&mut pass, &p, &v, 6);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    let mut values = [0; 16];
    s.read_stencils(&mut values).unwrap();
    assert!(values.iter().all(|v| *v == values[0]));
    (t.pixel(0, 0).unwrap(), values[0])
}
fn face(
    compare: CompareFunction,
    fail_op: StencilOperation,
    depth_fail_op: StencilOperation,
    pass_op: StencilOperation,
) -> StencilFaceState {
    StencilFaceState {
        compare,
        fail_op,
        depth_fail_op,
        pass_op,
    }
}
fn state(f: StencilFaceState) -> StencilState {
    StencilState {
        front: f,
        back: f,
        ..StencilState::replace(91)
    }
}
fn invisible() -> BlendState {
    let mut b = BlendState::additive();
    b.write_mask = [false; 4];
    b
}

#[test]
fn storage_owns_metadata_initializes_zero_and_checks_readback_and_extent() {
    let d = device();
    let mut label = String::from("owned");
    let s = d
        .create_stencil_buffer(StencilBufferDescriptor {
            label: &label,
            width: 4,
            height: 4,
        })
        .unwrap();
    label.clear();
    assert_eq!(s.label(), "owned");
    assert_eq!((s.width(), s.height()), (4, 4));
    assert!(s.belongs_to(&d));
    assert!(s.device().is_same(&d));
    assert!(s.is_same(&s.clone()));
    assert!(!s.is_same(&stencil(&d)));
    let mut values = [9; 16];
    s.read_stencils(&mut values).unwrap();
    assert_eq!(values, [0; 16]);
    let mut short = [9; 15];
    assert!(matches!(
        s.read_stencils(&mut short),
        Err(StencilBufferAccessError::WrongPixelCount {
            expected: 16,
            actual: 15
        })
    ));
    assert_eq!(short, [9; 15]);
    assert!(matches!(
        s.stencil(0, 4),
        Err(StencilBufferAccessError::OutOfBounds { .. })
    ));
    for (width, height) in [(0, 1), (1, 0), (16385, 1), (1, 16385)] {
        assert!(d
            .create_stencil_buffer(StencilBufferDescriptor {
                label: "bad",
                width,
                height
            })
            .is_err());
    }
    assert!(StencilBufferDescriptor {
        label: "max",
        width: 16384,
        height: 16384
    }
    .validate()
    .is_ok());
}

#[test]
fn attachment_validation_and_empty_direct_indexed_draws_require_stencil() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let z = depth(&d);
    let other = stencil(&device());
    let small = d
        .create_stencil_buffer(StencilBufferDescriptor {
            label: "small",
            width: 1,
            height: 4,
        })
        .unwrap();
    let mut e = d.create_command_encoder();
    assert!(matches!(
        e.begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                stencil: Some(&other),
                ..RenderAttachments::default()
            }
        ),
        Err(RenderEncodeError::ForeignStencilBuffer { .. })
    ));
    assert!(matches!(
        e.begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                stencil: Some(&small),
                ..RenderAttachments::default()
            }
        ),
        Err(RenderEncodeError::StencilExtentMismatch {
            stencil_width: 1,
            ..
        })
    ));
    assert!(matches!(
        e.begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                clear_stencil: Some(0),
                ..RenderAttachments::default()
            }
        ),
        Err(RenderEncodeError::MissingStencilBuffer)
    ));
    assert!(matches!(
        e.begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                clear_depth: Some(0.5),
                ..RenderAttachments::default()
            }
        ),
        Err(RenderEncodeError::MissingDepthBuffer)
    ));
    assert!(matches!(
        e.begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                depth: Some(&z),
                clear_depth: Some(f32::NAN),
                ..RenderAttachments::default()
            }
        ),
        Err(RenderEncodeError::InvalidDepthClear)
    ));
    let p = pipeline(
        &d,
        StencilState::replace(1),
        GraphicsPipelineOptions::default(),
        false,
    );
    let v = quad(&d, 0.);
    let i = d
        .create_buffer(BufferDescriptor {
            label: "empty",
            size: 2,
            usages: &[BufferUsage::Index],
            host_access: HostAccess::None,
        })
        .unwrap();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingStencilBuffer)
    ));
    assert!(matches!(
        pass.draw_indexed(0..0, 0),
        Err(RenderEncodeError::MissingStencilBuffer)
    ));
    drop(pass);
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                stencil: Some(&s),
                clear_stencil: Some(255),
                ..RenderAttachments::default()
            },
        )
        .unwrap();
    draw(&mut pass, &p, &v, 0);
    pass.finish().unwrap();
    let Command::RenderPass(p) = &e.commands()[0] else {
        panic!()
    };
    assert!(p.stencil_buffer().unwrap().is_same(&s));
    assert_eq!(p.clear_stencil(), Some(255));
    assert_eq!(s.stencil(0, 0).unwrap(), 0);
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(s.stencil(0, 0).unwrap(), 255);
}

#[test]
fn comparison_uses_masked_reference_against_stored_value_in_correct_order() {
    use CompareFunction::*;
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    for (compare, expected) in [
        (Never, [false, false, false]),
        (Less, [true, false, false]),
        (Equal, [false, true, false]),
        (LessEqual, [true, true, false]),
        (Greater, [false, false, true]),
        (NotEqual, [true, false, true]),
        (GreaterEqual, [false, true, true]),
        (Always, [true, true, true]),
    ] {
        for (reference, passed) in [9, 10, 11].into_iter().zip(expected) {
            let mut st = StencilState::test_equal(reference);
            st.front.compare = compare;
            st.back.compare = compare;
            let (c, value) = render(&d, &t, &s, st, 10);
            assert_eq!(
                c,
                if passed { [255, 0, 0, 128] } else { [7; 4] },
                "{compare:?} {reference}"
            );
            assert_eq!(value, 10);
        }
    }
    let mut st = StencilState::test_equal(0xA5);
    st.read_mask = 0x0F;
    assert_eq!(render(&d, &t, &s, st, 0x35).0, [255, 0, 0, 128]);
    st.read_mask = 0;
    assert_eq!(render(&d, &t, &s, st, 0x99).0, [255, 0, 0, 128]);
}

#[test]
fn every_operation_handles_zero_maximum_and_middle_values() {
    use StencilOperation::*;
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    for (operation, expected) in [
        (Keep, [0, 17, 255]),
        (Zero, [0, 0, 0]),
        (Replace, [91, 91, 91]),
        (IncrementClamp, [1, 18, 255]),
        (DecrementClamp, [0, 16, 254]),
        (Invert, [255, 238, 0]),
        (IncrementWrap, [1, 18, 0]),
        (DecrementWrap, [255, 16, 254]),
    ] {
        for (initial, expected) in [0, 17, 255].into_iter().zip(expected) {
            let st = state(face(CompareFunction::Always, Keep, Keep, operation));
            assert_eq!(
                render(&d, &t, &s, st, initial).1,
                expected,
                "{operation:?} {initial}"
            );
        }
    }
}

#[test]
fn write_masks_preserve_other_bits_and_operations_use_full_old_byte() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let mut st = StencilState::replace(0x3C);
    st.write_mask = 0x0F;
    st.read_mask = 0;
    assert_eq!(render(&d, &t, &s, st, 0xA5).1, 0xAC);
    st.front.pass_op = StencilOperation::IncrementWrap;
    st.back = st.front;
    st.write_mask = 0xF0;
    assert_eq!(render(&d, &t, &s, st, 0x0F).1, 0x1F);
    st.write_mask = 0;
    assert_eq!(render(&d, &t, &s, st, 0xD5).1, 0xD5);
    st = StencilState::test_equal(1);
    assert_eq!(render(&d, &t, &s, st, 0).0, [7; 4]);
}

#[test]
fn stencil_fail_depth_fail_and_success_choose_distinct_operations_and_skip_bad_shading() {
    use StencilOperation::*;
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let z = depth(&d);
    let v = quad(&d, 0.);
    for (compare, depth_clear, bad, expected, c) in [
        (CompareFunction::Never, 1., true, 8, [7; 4]),
        (CompareFunction::Always, 0., true, 6, [7; 4]),
        (CompareFunction::Always, 1., false, 91, [255, 0, 0, 128]),
    ] {
        let st = state(face(compare, IncrementClamp, DecrementClamp, Replace));
        let p = pipeline(
            &d,
            st,
            GraphicsPipelineOptions {
                depth: Some(DepthState {
                    compare: CompareFunction::Less,
                    write_enabled: true,
                }),
                blend: None,
            },
            bad,
        );
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass_with_attachments(
                &t,
                Some([7; 4]),
                RenderAttachments {
                    depth: Some(&z),
                    clear_depth: Some(depth_clear),
                    stencil: Some(&s),
                    clear_stencil: Some(7),
                },
            )
            .unwrap();
        draw(&mut pass, &p, &v, 6);
        pass.finish().unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        assert_eq!(s.stencil(0, 0).unwrap(), expected);
        assert_eq!(t.pixel(0, 0).unwrap(), c);
        assert_eq!(z.depth(0, 0).unwrap(), if bad { depth_clear } else { 0.5 });
    }
}

#[test]
fn front_back_rules_survive_winding_normalization_and_clipped_fan() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let mut st = StencilState::replace(13);
    st.back.pass_op = StencilOperation::Invert;
    let p = pipeline(&d, st, GraphicsPipelineOptions::default(), false);
    for reverse in [false, true] {
        let mut data = [[-2., 2., 0., 1.], [2., 2., 0., 1.], [-2., -2., 0., 1.]];
        if reverse {
            data.swap(1, 2);
        }
        let v = vertices(&d, &data);
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass_with_attachments(
                &t,
                None,
                RenderAttachments {
                    stencil: Some(&s),
                    clear_stencil: Some(0),
                    ..RenderAttachments::default()
                },
            )
            .unwrap();
        draw(&mut pass, &p, &v, 3);
        pass.finish().unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(
                    s.stencil(x, y).unwrap(),
                    if x + y < 3 {
                        if reverse {
                            255
                        } else {
                            13
                        }
                    } else {
                        0
                    }
                );
            }
        }
    }
}

#[test]
fn mark_then_test_preserves_live_stencil_across_scopes_and_resubmission() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let triangle = vertices(
        &d,
        &[[-1., 1., 0., 1.], [1., 1., 0., 1.], [-1., -1., 0., 1.]],
    );
    let q = quad(&d, 0.);
    let mark = pipeline(
        &d,
        StencilState::replace(3),
        GraphicsPipelineOptions {
            depth: None,
            blend: Some(invisible()),
        },
        false,
    );
    let test = pipeline(
        &d,
        StencilState::test_equal(3),
        GraphicsPipelineOptions::default(),
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            Some([7; 4]),
            RenderAttachments {
                stencil: Some(&s),
                clear_stencil: Some(0),
                ..RenderAttachments::default()
            },
        )
        .unwrap();
    draw(&mut pass, &mark, &triangle, 3);
    pass.finish().unwrap();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                stencil: Some(&s),
                ..RenderAttachments::default()
            },
        )
        .unwrap();
    draw(&mut pass, &test, &q, 6);
    pass.finish().unwrap();
    let c = e.finish();
    drop(mark);
    drop(test);
    drop(triangle);
    drop(q);
    for _ in 0..2 {
        d.default_queue().submit(&c).unwrap();
        assert_eq!(t.pixel(0, 0).unwrap(), [255, 0, 0, 128]);
        assert_eq!(t.pixel(3, 3).unwrap(), [7; 4]);
        assert_eq!(s.stencil(0, 0).unwrap(), 3);
    }
}

#[test]
fn indexed_live_textures_blending_and_depth_work_with_stencil() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let z = depth(&d);
    let v = quad(&d, -0.5);
    let vs = d
        .create_shader(ShaderDescriptor {
            label: "v",
            stage: ShaderStage::Vertex,
            input_count: 1,
            instructions: &[ShaderInstruction::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let fs = d
        .create_shader(ShaderDescriptor {
            label: "f",
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
        .create_graphics_pipeline_with_stencil(
            descriptor(&vs, &fs),
            GraphicsPipelineOptions {
                depth: Some(DepthState {
                    compare: CompareFunction::Less,
                    write_enabled: true,
                }),
                blend: Some(BlendState::alpha_blending()),
            },
            StencilState::replace(8),
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
    let sampler = d
        .create_sampler(SamplerDescriptor {
            label: "sampler",
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
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            Some([0, 0, 255, 255]),
            RenderAttachments {
                depth: Some(&z),
                clear_depth: Some(1.),
                stencil: Some(&s),
                clear_stencil: Some(0),
            },
        )
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
    pass.set_texture(0, &tex.view(), &sampler).unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    let c = e.finish();
    drop(p);
    drop(v);
    drop(i);
    drop(sampler);
    drop(vs);
    drop(fs);
    for (rgba, expected) in [
        ([255, 0, 0, 128], [128, 0, 127, 255]),
        ([0, 255, 0, 128], [0, 128, 127, 255]),
    ] {
        tex.write_pixels(&[rgba]).unwrap();
        d.default_queue().submit(&c).unwrap();
        assert_eq!(t.pixel(0, 0).unwrap(), expected);
        assert_eq!(z.depth(0, 0).unwrap(), 0.25);
        assert_eq!(s.stencil(0, 0).unwrap(), 8);
    }
}

#[test]
fn late_failure_rolls_back_colour_depth_stencil_and_clears_with_prefix_preserved() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let z = depth(&d);
    let v = quad(&d, 0.);
    let mut st = StencilState::replace(9);
    st.front.compare = CompareFunction::Never;
    st.back = st.front;
    st.front.fail_op = StencilOperation::Invert;
    st.back = st.front;
    let fail_update = pipeline(&d, st, GraphicsPipelineOptions::default(), false);
    let bad = pipeline(
        &d,
        StencilState::replace(5),
        GraphicsPipelineOptions {
            depth: Some(DepthState {
                compare: CompareFunction::Less,
                write_enabled: true,
            }),
            blend: Some(BlendState::alpha_blending()),
        },
        true,
    );
    let mut e = d.create_command_encoder();
    e.begin_render_pass_with_attachments(
        &t,
        Some([8; 4]),
        RenderAttachments {
            depth: Some(&z),
            clear_depth: Some(0.8),
            stencil: Some(&s),
            clear_stencil: Some(8),
        },
    )
    .unwrap()
    .finish()
    .unwrap();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            Some([9; 4]),
            RenderAttachments {
                depth: Some(&z),
                clear_depth: Some(1.),
                stencil: Some(&s),
                clear_stencil: Some(9),
            },
        )
        .unwrap();
    draw(&mut pass, &fail_update, &v, 6);
    let far = quad(&d, 0.5);
    let good = pipeline(
        &d,
        StencilState::replace(1),
        GraphicsPipelineOptions {
            depth: Some(DepthState {
                compare: CompareFunction::Less,
                write_enabled: true,
            }),
            blend: Some(BlendState::alpha_blending()),
        },
        false,
    );
    draw(&mut pass, &good, &far, 6);
    draw(&mut pass, &bad, &v, 6);
    pass.finish().unwrap();
    e.begin_render_pass_with_attachments(
        &t,
        Some([10; 4]),
        RenderAttachments {
            depth: Some(&z),
            clear_depth: Some(0.),
            stencil: Some(&s),
            clear_stencil: Some(10),
        },
    )
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
    assert_eq!(s.stencil(0, 0).unwrap(), 8);
}

#[test]
fn pipeline_state_is_copied_validated_and_prior_creators_disable_stencil() {
    let d = device();
    let st = StencilState::replace(12);
    let p = pipeline(&d, st, GraphicsPipelineOptions::default(), false);
    assert_eq!(p.stencil_state(), Some(st));
    let desc = descriptor(p.vertex_shader(), p.fragment_shader());
    for old in [
        d.create_graphics_pipeline(desc).unwrap(),
        d.create_graphics_pipeline_with_options(desc, GraphicsPipelineOptions::default())
            .unwrap(),
        d.create_graphics_pipeline_with_depth(
            desc,
            DepthState {
                compare: CompareFunction::Always,
                write_enabled: false,
            },
        )
        .unwrap(),
    ] {
        assert_eq!(old.stencil_state(), None);
    }
    assert!(matches!(
        device().create_graphics_pipeline_with_stencil(
            desc,
            GraphicsPipelineOptions::default(),
            st
        ),
        Err(GraphicsPipelineCreateError::ForeignShader { .. })
    ));
}

#[test]
fn disabled_stencil_and_legacy_draws_preserve_attached_values_but_clears_apply() {
    let d = device();
    let t = target(&d);
    let s = stencil(&d);
    let p = pipeline(
        &d,
        StencilState::replace(12),
        GraphicsPipelineOptions::default(),
        false,
    );
    let data: Vec<_> = [[-1_f32, 1.], [1., 1.], [-1., -1.]]
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    let v = d
        .create_buffer(BufferDescriptor {
            label: "legacy",
            size: 24,
            usages: &[BufferUsage::Vertex],
            host_access: HostAccess::Write,
        })
        .unwrap();
    v.write(0, &data).unwrap();
    let off = d
        .create_graphics_pipeline(descriptor(p.vertex_shader(), p.fragment_shader()))
        .unwrap();
    let q = quad(&d, 0.);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_attachments(
            &t,
            None,
            RenderAttachments {
                stencil: Some(&s),
                clear_stencil: Some(42),
                ..RenderAttachments::default()
            },
        )
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw(0..3, [1; 4]).unwrap();
    draw(&mut pass, &off, &q, 6);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(s.stencil(0, 0).unwrap(), 42);
}
