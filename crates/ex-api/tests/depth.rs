use ex::*;

#[test]
fn sampled_fragments_and_depth_work_together_with_live_textures() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let near = quad(&d, -0.5);
    let far = quad(&d, 0.5);
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
        .create_graphics_pipeline_with_depth(
            GraphicsPipelineDescriptor {
                label: "textured depth",
                vertex_shader: &vs,
                fragment_shader: &fs,
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
            state(CompareFunction::Less, true),
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
    let blue = pipeline(
        &d,
        Some(state(CompareFunction::Less, true)),
        [0., 0., 1., 1.],
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, None, &z, Some(1.))
        .unwrap();
    pass.set_texture(0, &tex.view(), &s).unwrap();
    draw(&mut pass, &p, &near);
    draw(&mut pass, &blue, &far);
    pass.finish().unwrap();
    let c = e.finish();
    for rgba in [[255, 0, 0, 255], [0, 255, 0, 255]] {
        tex.write_pixels(&[rgba]).unwrap();
        d.default_queue().submit(&c).unwrap();
        assert_eq!(t.pixel(0, 0).unwrap(), rgba);
        assert_eq!(read(&z), [0.25; 16]);
    }
}

#[test]
fn clip_depth_endpoints_map_to_zero_and_one_and_outside_geometry_is_clipped() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let p = pipeline(
        &d,
        Some(state(CompareFunction::Always, true)),
        [1.; 4],
        false,
    );
    for clip_z in [-2., -1., 1., 2.] {
        let v = quad(&d, clip_z);
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass_with_depth(&t, Some([7; 4]), &z, Some(0.5))
            .unwrap();
        draw(&mut pass, &p, &v);
        pass.finish().unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        let inside = (-1.0..=1.0).contains(&clip_z);
        assert_eq!(
            t.pixel(0, 0).unwrap(),
            if inside { [255; 4] } else { [7; 4] }
        );
        assert_eq!(
            read(&z),
            [if inside { clip_z * 0.5 + 0.5 } else { 0.5 }; 16]
        );
    }
}
fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn colour(d: &Device) -> Framebuffer {
    d.create_framebuffer(FramebufferDescriptor {
        label: "colour",
        width: 4,
        height: 4,
        format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn depth(d: &Device) -> DepthBuffer {
    d.create_depth_buffer(DepthBufferDescriptor {
        label: "depth",
        width: 4,
        height: 4,
    })
    .unwrap()
}
fn pipeline(d: &Device, state: Option<DepthState>, rgba: [f32; 4], fail: bool) -> GraphicsPipeline {
    use ShaderInstruction as Op;
    let v = d
        .create_shader(ShaderDescriptor {
            label: "position",
            stage: ShaderStage::Vertex,
            input_count: 1,
            instructions: &[Op::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    let ops = if fail {
        vec![
            Op::Constant([f32::MAX; 4]),
            Op::Multiply { left: 0, right: 0 },
        ]
    } else {
        vec![Op::Constant(rgba)]
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
    let desc = GraphicsPipelineDescriptor {
        label: "depth pipeline",
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
    };
    if let Some(s) = state {
        d.create_graphics_pipeline_with_depth(desc, s).unwrap()
    } else {
        d.create_graphics_pipeline(desc).unwrap()
    }
}
fn state(compare: CompareFunction, write_enabled: bool) -> DepthState {
    DepthState {
        compare,
        write_enabled,
    }
}
fn records(d: &Device, data: &[[f32; 4]]) -> Buffer {
    let bytes: Vec<_> = data
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let b = d
        .create_buffer(BufferDescriptor {
            label: "positions",
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
fn draw(pass: &mut RenderPass<'_>, p: &GraphicsPipeline, v: &Buffer) {
    pass.set_pipeline(p).unwrap();
    pass.set_vertex_buffer(v, 0).unwrap();
    pass.draw_programmable(0..6).unwrap();
}
fn read(z: &DepthBuffer) -> [f32; 16] {
    let mut v = [0.; 16];
    z.read_depths(&mut v).unwrap();
    v
}

#[test]
fn depth_storage_initialization_metadata_identity_and_readback_errors() {
    let d = device();
    let mut label = String::from("owned");
    let z = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: &label,
            width: 4,
            height: 4,
        })
        .unwrap();
    label.clear();
    assert_eq!(z.label(), "owned");
    assert_eq!((z.width(), z.height()), (4, 4));
    assert!(z.device().is_same(&d));
    assert!(z.belongs_to(&d));
    assert!(z.is_same(&z.clone()));
    assert!(!z.is_same(&depth(&d)));
    assert_eq!(read(&z), [1.; 16]);
    let mut values = [9.; 15];
    assert!(matches!(
        z.read_depths(&mut values),
        Err(DepthBufferAccessError::WrongPixelCount {
            expected: 16,
            actual: 15
        })
    ));
    assert_eq!(values, [9.; 15]);
    assert!(matches!(
        z.depth(4, 0),
        Err(DepthBufferAccessError::OutOfBounds { .. })
    ));
    for (width, height) in [(0, 4), (4, 0), (16385, 4), (4, 16385)] {
        assert!(d
            .create_depth_buffer(DepthBufferDescriptor {
                label: "bad",
                width,
                height
            })
            .is_err());
    }
    assert!(DepthBufferDescriptor {
        label: "max",
        width: 16384,
        height: 16384
    }
    .validate()
    .is_ok());
}
#[test]
fn depth_recording_checks_ownership_extents_clears_and_missing_attachments() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let p = pipeline(&d, Some(state(CompareFunction::Less, true)), [1.; 4], false);
    let v = quad(&d, 0.);
    assert_eq!(p.depth_state(), Some(state(CompareFunction::Less, true)));
    assert_eq!(pipeline(&d, None, [1.; 4], false).depth_state(), None);
    let mut e = d.create_command_encoder();
    let foreign = depth(&device());
    assert!(matches!(
        e.begin_render_pass_with_depth(&t, None, &foreign, None),
        Err(RenderEncodeError::ForeignDepthBuffer { .. })
    ));
    let foreign_colour = colour(&device());
    assert!(matches!(
        e.begin_render_pass_with_depth(&foreign_colour, None, &z, None),
        Err(RenderEncodeError::ForeignFramebuffer { .. })
    ));
    let small = d
        .create_depth_buffer(DepthBufferDescriptor {
            label: "small",
            width: 2,
            height: 4,
        })
        .unwrap();
    assert!(matches!(
        e.begin_render_pass_with_depth(&t, None, &small, None),
        Err(RenderEncodeError::DepthExtentMismatch { depth_width: 2, .. })
    ));
    for clear in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        assert!(matches!(
            e.begin_render_pass_with_depth(&t, None, &z, Some(clear)),
            Err(RenderEncodeError::InvalidDepthClear)
        ));
    }
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingDepthBuffer)
    ));
    drop(pass);
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([7; 4]), &z, Some(0.))
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw_programmable(0..0).unwrap();
    pass.finish().unwrap();
    let Command::RenderPass(pass) = &e.commands()[0] else {
        panic!()
    };
    assert!(pass.depth_buffer().unwrap().is_same(&z));
    assert_eq!(pass.clear_depth(), Some(0.));
    assert_eq!(z.depth(0, 0).unwrap(), 1.);
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(read(&z), [0.; 16]);
}
#[test]
fn nearer_surface_wins_in_both_draw_orders() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let near = quad(&d, -0.5);
    let far = quad(&d, 0.5);
    let a = pipeline(
        &d,
        Some(state(CompareFunction::Less, true)),
        [1., 0., 0., 1.],
        false,
    );
    let b = pipeline(
        &d,
        Some(state(CompareFunction::Less, true)),
        [0., 0., 1., 1.],
        false,
    );
    for reverse in [false, true] {
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass_with_depth(&t, Some([0; 4]), &z, Some(1.))
            .unwrap();
        for (p, v) in if reverse {
            [(&b, &far), (&a, &near)]
        } else {
            [(&a, &near), (&b, &far)]
        } {
            draw(&mut pass, p, v);
        }
        pass.finish().unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        assert_eq!(t.pixel(0, 0).unwrap(), [255, 0, 0, 255]);
        assert_eq!(read(&z), [0.25; 16]);
    }
}
#[test]
fn all_comparisons_include_exact_equal_depth_and_write_control() {
    use CompareFunction::*;
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
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
        let p = pipeline(&d, Some(state(compare, true)), [1.; 4], false);
        for (incoming, passes) in [0.25, 0.5, 0.75].into_iter().zip(expected) {
            let v = quad(&d, incoming * 2. - 1.);
            let mut e = d.create_command_encoder();
            let mut pass = e
                .begin_render_pass_with_depth(&t, Some([7; 4]), &z, Some(0.5))
                .unwrap();
            draw(&mut pass, &p, &v);
            pass.finish().unwrap();
            d.default_queue().submit(&e.finish()).unwrap();
            assert_eq!(
                t.pixel(0, 0).unwrap(),
                if passes { [255; 4] } else { [7; 4] }
            );
            assert_eq!(read(&z), [if passes { incoming } else { 0.5 }; 16]);
        }
    }
}
#[test]
fn disabled_writes_still_test_and_depth_persists_between_scopes() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let a = pipeline(
        &d,
        Some(state(CompareFunction::Less, false)),
        [1., 0., 0., 1.],
        false,
    );
    let b = pipeline(
        &d,
        Some(state(CompareFunction::Less, false)),
        [0., 0., 1., 1.],
        false,
    );
    let near = quad(&d, -0.5);
    let far = quad(&d, 0.5);
    for clear in [1., 0.5] {
        let mut e = d.create_command_encoder();
        let mut pass = e
            .begin_render_pass_with_depth(&t, Some([0; 4]), &z, Some(clear))
            .unwrap();
        draw(&mut pass, &a, &near);
        draw(&mut pass, &b, &far);
        pass.finish().unwrap();
        d.default_queue().submit(&e.finish()).unwrap();
        assert_eq!(read(&z), [clear; 16]);
        assert_eq!(
            t.pixel(0, 0).unwrap(),
            if clear == 1. {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            }
        );
    }
    let a = pipeline(
        &d,
        Some(state(CompareFunction::Less, true)),
        [1., 0., 0., 1.],
        false,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, None, &z, Some(1.))
        .unwrap();
    draw(&mut pass, &a, &near);
    pass.finish().unwrap();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([7; 4]), &z, None)
        .unwrap();
    draw(&mut pass, &b, &far);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(0, 0).unwrap(), [7; 4]);
    assert_eq!(read(&z), [0.25; 16]);
}
#[test]
fn projected_depth_is_linear_in_screen_space_not_a_perspective_varying() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let p = pipeline(
        &d,
        Some(state(CompareFunction::Always, true)),
        [1.; 4],
        false,
    );
    let v = records(
        &d,
        &[[-1., 1., -0.5, 1.], [2., 2., 1., 2.], [-4., -4., 0., 4.]],
    );
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([0; 4]), &z, Some(1.))
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            let u = (x as f64 + 0.5) / 4.;
            let v = (y as f64 + 0.5) / 4.;
            if u + v < 1. {
                let expected = ((1. - u - v) * 0.25 + u * 0.75 + v * 0.5) as f32;
                assert_eq!(z.depth(x, y).unwrap(), expected);
            } else {
                assert_eq!(z.depth(x, y).unwrap(), 1.);
            }
        }
    }
}
#[test]
fn indexed_depth_and_depth_disabled_pipeline_mix_without_touching_depth() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let p = pipeline(
        &d,
        Some(state(CompareFunction::Less, true)),
        [1., 0., 0., 1.],
        false,
    );
    let v = quad(&d, -0.5);
    let i = d
        .create_buffer(BufferDescriptor {
            label: "indices",
            size: 12,
            usages: &[BufferUsage::Index],
            host_access: HostAccess::Write,
        })
        .unwrap();
    i.write(0, &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0]).unwrap();
    let off = pipeline(&d, None, [0., 0., 1., 1.], false);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, None, &z, Some(1.))
        .unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    draw(&mut pass, &off, &v);
    pass.finish().unwrap();
    drop(p);
    drop(off);
    drop(v);
    drop(i);
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(read(&z), [0.25; 16]);
    assert_eq!(t.pixel(0, 0).unwrap(), [0, 0, 255, 255]);
}
#[test]
fn occluded_fragments_skip_bad_shader_but_visible_failures_roll_back_both_attachments() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let good = pipeline(&d, Some(state(CompareFunction::Less, true)), [1.; 4], false);
    let bad = pipeline(&d, Some(state(CompareFunction::Less, true)), [1.; 4], true);
    let near = quad(&d, -0.5);
    let far = quad(&d, 0.5);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, Some([7; 4]), &z, Some(1.))
        .unwrap();
    draw(&mut pass, &good, &near);
    draw(&mut pass, &bad, &far);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(read(&z), [0.25; 16]);
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
    assert_eq!(read(&z), [0.8; 16]);
}
#[test]
fn reverse_depth_clear_and_greater_comparison_choose_larger_values() {
    let d = device();
    let t = colour(&d);
    let z = depth(&d);
    let a = pipeline(
        &d,
        Some(state(CompareFunction::Greater, true)),
        [1., 0., 0., 1.],
        false,
    );
    let b = pipeline(
        &d,
        Some(state(CompareFunction::Greater, true)),
        [0., 0., 1., 1.],
        false,
    );
    let low = quad(&d, -0.5);
    let high = quad(&d, 0.5);
    let mut e = d.create_command_encoder();
    let mut pass = e
        .begin_render_pass_with_depth(&t, None, &z, Some(0.))
        .unwrap();
    draw(&mut pass, &a, &high);
    draw(&mut pass, &b, &low);
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(read(&z), [0.75; 16]);
    assert_eq!(t.pixel(0, 0).unwrap(), [255, 0, 0, 255]);
}
