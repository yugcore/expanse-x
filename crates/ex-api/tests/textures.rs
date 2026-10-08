use ex::*;

#[test]
fn texture_uvs_use_perspective_correct_interpolation() {
    let d = device();
    let t = target(&d);
    let p = sample_pipeline(&d);
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&TEXELS).unwrap();
    let s = sampler(
        &d,
        FilterMode::Linear,
        AddressMode::ClampToEdge,
        AddressMode::ClampToEdge,
    );
    let records = [
        [-1.0_f32, 1.0, 0.0, 1.0, 0.0, 0.0],
        [2.0, 2.0, 0.0, 2.0, 1.0, 0.0],
        [4.0, -4.0, 0.0, 4.0, 1.0, 1.0],
        [-2.0, -2.0, 0.0, 2.0, 0.0, 1.0],
    ];
    let data: Vec<_> = [0, 1, 2, 2, 3, 0]
        .into_iter()
        .flat_map(|i| records[i].into_iter().flat_map(f32::to_le_bytes))
        .collect();
    let v = buffer(&d, &data, BufferUsage::Vertex);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &tex.view(), &s).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            let u = (f64::from(x) + 0.5) / 4.0;
            let v = (f64::from(y) + 0.5) / 4.0;
            let (nu, nv, den) = if u >= v {
                (
                    (u - v) / 2.0 + v / 4.0,
                    v / 4.0,
                    1.0 - u + (u - v) / 2.0 + v / 4.0,
                )
            } else {
                (
                    u / 4.0,
                    u / 4.0 + (v - u) / 2.0,
                    u / 4.0 + (v - u) / 2.0 + 1.0 - v,
                )
            };
            let expected = tex
                .view()
                .sample(&s, [(nu / den) as f32, (nv / den) as f32])
                .unwrap()
                .map(|v| (v * 255.0).round() as u8);
            assert_eq!(t.pixel(x, y).unwrap(), expected);
        }
    }
}

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn texture(d: &Device, access: HostAccess) -> Texture {
    d.create_texture(TextureDescriptor {
        label: "texels",
        width: 2,
        height: 2,
        format: TextureFormat::Rgba8Unorm,
        host_access: access,
    })
    .unwrap()
}
const TEXELS: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 255, 255],
];
fn sampler(d: &Device, filter: FilterMode, u: AddressMode, v: AddressMode) -> Sampler {
    d.create_sampler(SamplerDescriptor {
        label: "sampling",
        filter,
        address_u: u,
        address_v: v,
    })
    .unwrap()
}
fn close(actual: [f32; 4], expected: [f32; 4]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 1e-6, "{actual:?} != {expected:?}");
    }
}
fn pipeline(d: &Device, fragment_ops: &[ShaderInstruction], outputs: &[u32]) -> GraphicsPipeline {
    use ShaderInstruction as Op;
    let vs = d
        .create_shader(ShaderDescriptor {
            label: "UV vertex",
            stage: ShaderStage::Vertex,
            input_count: 2,
            instructions: &[Op::Input { location: 0 }, Op::Input { location: 1 }],
            outputs: &[0, 1],
        })
        .unwrap();
    let fs = d
        .create_shader(ShaderDescriptor {
            label: "sample",
            stage: ShaderStage::Fragment,
            input_count: 1,
            instructions: fragment_ops,
            outputs,
        })
        .unwrap();
    d.create_graphics_pipeline(GraphicsPipelineDescriptor {
        label: "textured",
        vertex_shader: &vs,
        fragment_shader: &fs,
        vertex_layout: VertexBufferLayout {
            stride: 24,
            attributes: &[
                VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x4,
                },
                VertexAttribute {
                    location: 1,
                    offset: 16,
                    format: VertexFormat::Float32x2,
                },
            ],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn sample_pipeline(d: &Device) -> GraphicsPipeline {
    pipeline(
        d,
        &[
            ShaderInstruction::Input { location: 0 },
            ShaderInstruction::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
        ],
        &[1],
    )
}
fn buffer(d: &Device, bytes: &[u8], usage: BufferUsage) -> Buffer {
    let b = d
        .create_buffer(BufferDescriptor {
            label: "geometry",
            size: bytes.len() as u64,
            usages: &[usage],
            host_access: HostAccess::Write,
        })
        .unwrap();
    b.write(0, bytes).unwrap();
    b
}
fn vertices(d: &Device, expanded: bool) -> Buffer {
    let records = [
        [-1.0_f32, 1.0, 0.0, 1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0, 1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0, 1.0, 1.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0, 0.0, 1.0],
    ];
    let indices: &[usize] = if expanded {
        &[0, 1, 2, 2, 3, 0]
    } else {
        &[0, 1, 2, 3]
    };
    let bytes: Vec<_> = indices
        .iter()
        .flat_map(|&i| records[i].into_iter().flat_map(f32::to_le_bytes))
        .collect();
    buffer(d, &bytes, BufferUsage::Vertex)
}
fn target(d: &Device) -> Framebuffer {
    d.create_framebuffer(FramebufferDescriptor {
        label: "output",
        width: 4,
        height: 4,
        format: FramebufferFormat::Rgba8Unorm,
    })
    .unwrap()
}
fn pixels(t: &Framebuffer) -> [[u8; 4]; 16] {
    let mut p = [[0; 4]; 16];
    t.read_pixels(&mut p).unwrap();
    p
}
fn commands(
    d: &Device,
    t: &Framebuffer,
    p: &GraphicsPipeline,
    view: &TextureView,
    s: &Sampler,
    indexed: bool,
) -> CommandBuffer {
    let vertices = vertices(d, !indexed);
    let indices = buffer(d, &[0, 0, 1, 0, 2, 0, 2, 0, 3, 0, 0, 0], BufferUsage::Index);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(t, Some([9; 4])).unwrap();
    pass.set_pipeline(p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_texture(0, view, s).unwrap();
    if indexed {
        pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)
            .unwrap();
        pass.draw_indexed(0..6, 0).unwrap();
    } else {
        pass.draw_programmable(0..6).unwrap();
    }
    pass.finish().unwrap();
    e.finish()
}

#[test]
fn texture_creation_owns_metadata_and_cpu_permissions_are_independent() {
    let d = device();
    let mut label = String::from("owned");
    let t = d
        .create_texture(TextureDescriptor {
            label: &label,
            width: 2,
            height: 2,
            format: TextureFormat::Rgba8Unorm,
            host_access: HostAccess::ReadWrite,
        })
        .unwrap();
    label.clear();
    assert_eq!(t.label(), "owned");
    assert_eq!(
        (t.width(), t.height(), t.format(), t.host_access()),
        (2, 2, TextureFormat::Rgba8Unorm, HostAccess::ReadWrite)
    );
    assert!(t.device().is_same(&d));
    assert!(t.belongs_to(&d));
    assert!(t.is_same(&t.clone()));
    assert!(!t.is_same(&texture(&d, HostAccess::ReadWrite)));
    let mut p = [[7; 4]; 4];
    t.read_pixels(&mut p).unwrap();
    assert_eq!(p, [[0; 4]; 4]);
    assert!(matches!(
        t.write_pixels(&TEXELS[..3]),
        Err(TextureAccessError::WrongPixelCount {
            expected: 4,
            actual: 3
        })
    ));
    t.write_pixels(&TEXELS).unwrap();
    t.clone().read_pixels(&mut p).unwrap();
    assert_eq!(p, TEXELS);
    assert!(matches!(
        t.read_pixels(&mut p[..3]),
        Err(TextureAccessError::WrongPixelCount { .. })
    ));
    for access in [HostAccess::None, HostAccess::Read, HostAccess::Write] {
        let t = texture(&d, access);
        assert_eq!(t.write_pixels(&TEXELS).is_ok(), access == HostAccess::Write);
        assert_eq!(t.read_pixels(&mut p).is_ok(), access == HostAccess::Read);
        let s = sampler(
            &d,
            FilterMode::Nearest,
            AddressMode::ClampToEdge,
            AddressMode::ClampToEdge,
        );
        assert!(t.view().sample(&s, [0.5; 2]).is_ok());
    }
    for (w, h) in [(0, 2), (2, 0), (16385, 2), (2, 16385)] {
        assert!(d
            .create_texture(TextureDescriptor {
                label: "bad",
                width: w,
                height: h,
                format: TextureFormat::Rgba8Unorm,
                host_access: HostAccess::None
            })
            .is_err());
    }
    assert!(TextureDescriptor {
        label: "limit",
        width: 16384,
        height: 16384,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::None
    }
    .validate()
    .is_ok());
}

#[test]
fn views_check_bounds_without_overflow_and_address_only_their_rectangle() {
    let d = device();
    let t = texture(&d, HostAccess::Write);
    t.write_pixels(&TEXELS).unwrap();
    for r in [
        TextureViewDescriptor {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        },
        TextureViewDescriptor {
            x: 2,
            y: 0,
            width: 1,
            height: 1,
        },
        TextureViewDescriptor {
            x: u32::MAX,
            y: 0,
            width: 2,
            height: 1,
        },
        TextureViewDescriptor {
            x: 0,
            y: 1,
            width: 1,
            height: u32::MAX,
        },
    ] {
        assert!(matches!(
            t.create_view(r),
            Err(TextureAccessError::InvalidView { .. })
        ));
    }
    let r = TextureViewDescriptor {
        x: 1,
        y: 0,
        width: 1,
        height: 2,
    };
    let v = t.create_view(r).unwrap();
    assert_eq!(v.region(), r);
    assert!(v.texture().is_same(&t));
    for filter in [FilterMode::Nearest, FilterMode::Linear] {
        let s = sampler(&d, filter, AddressMode::Repeat, AddressMode::ClampToEdge);
        close(v.sample(&s, [-30.0, 0.25]).unwrap(), [0.0, 1.0, 0.0, 1.0]);
        close(v.sample(&s, [30.0, 0.75]).unwrap(), [1.0; 4]);
    }
}

#[test]
fn nearest_sampling_centres_edges_repeat_and_extreme_coordinates() {
    let d = device();
    let t = texture(&d, HostAccess::Write);
    t.write_pixels(&TEXELS).unwrap();
    let v = t.view();
    let clamp = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::ClampToEdge,
        AddressMode::ClampToEdge,
    );
    let repeat = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    close(
        v.sample(&repeat, [-f32::from_bits(1), 0.25]).unwrap(),
        [0.0, 1.0, 0.0, 1.0],
    );
    for (uv, expected) in [
        ([0.25, 0.25], [1., 0., 0., 1.]),
        ([0.75, 0.25], [0., 1., 0., 1.]),
        ([0.25, 0.75], [0., 0., 1., 1.]),
        ([1., 1.], [1.; 4]),
        ([-f32::MAX, f32::MAX], [0., 0., 1., 1.]),
    ] {
        close(v.sample(&clamp, uv).unwrap(), expected);
    }
    close(v.sample(&repeat, [1., 1.]).unwrap(), [1., 0., 0., 1.]);
    close(v.sample(&repeat, [-0.25, -0.25]).unwrap(), [1.; 4]);
    close(
        v.sample(&repeat, [f32::MAX, -f32::MAX]).unwrap(),
        [1., 0., 0., 1.],
    );
    for uv in [[f32::NAN, 0.], [0., f32::INFINITY], [f32::NEG_INFINITY, 0.]] {
        assert_eq!(
            v.sample(&repeat, uv),
            Err(TextureSampleError::NonFiniteCoordinates)
        );
    }
    let foreign = sampler(
        &device(),
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    assert!(matches!(
        v.sample(&foreign, [0.; 2]),
        Err(TextureSampleError::ForeignSampler { .. })
    ));
    assert_eq!(clamp.label(), "sampling");
    assert!(clamp.device().is_same(&d));
    assert!(clamp.is_same(&clamp.clone()));
    assert!(!clamp.is_same(&repeat));
}

#[test]
fn linear_sampling_interpolates_four_texels_and_wraps_each_neighbour() {
    let d = device();
    let t = texture(&d, HostAccess::Write);
    t.write_pixels(&TEXELS).unwrap();
    let v = t.view();
    let clamp = sampler(
        &d,
        FilterMode::Linear,
        AddressMode::ClampToEdge,
        AddressMode::ClampToEdge,
    );
    let repeat = sampler(
        &d,
        FilterMode::Linear,
        AddressMode::Repeat,
        AddressMode::ClampToEdge,
    );
    close(v.sample(&clamp, [0.5; 2]).unwrap(), [0.5, 0.5, 0.5, 1.]);
    close(v.sample(&clamp, [0.25; 2]).unwrap(), [1., 0., 0., 1.]);
    close(v.sample(&clamp, [0., 0.25]).unwrap(), [1., 0., 0., 1.]);
    close(v.sample(&repeat, [0., 0.25]).unwrap(), [0.5, 0.5, 0., 1.]);
    close(
        v.sample(&repeat, [-0.125, 0.25]).unwrap(),
        [0.25, 0.75, 0., 1.],
    );
    close(
        v.sample(&clamp, [-f32::MAX, f32::MAX]).unwrap(),
        [0., 0., 1., 1.],
    );
    let repeat_both = sampler(
        &d,
        FilterMode::Linear,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    close(
        v.sample(&repeat_both, [1., 1.]).unwrap(),
        [0.5, 0.5, 0.5, 1.],
    );
    t.write_pixels(&[[64, 128, 192, 32]; 4]).unwrap();
    close(
        v.sample(&clamp, [0.5; 2]).unwrap(),
        [64. / 255., 128. / 255., 192. / 255., 32. / 255.],
    );
}

#[test]
fn sample_instructions_validate_stage_slots_and_registers_and_standalone_errors() {
    use ShaderInstruction as Op;
    let d = device();
    let mut desc = ShaderDescriptor {
        label: "sample",
        stage: ShaderStage::Fragment,
        input_count: 1,
        instructions: &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 7,
                coordinates: 0,
            },
        ],
        outputs: &[1],
    };
    let s = d.create_shader(desc).unwrap();
    assert!(matches!(
        s.evaluate(&[[0.; 4]]),
        Err(ShaderExecutionError::MissingTextureBinding { binding: 7 })
    ));
    desc.stage = ShaderStage::Vertex;
    assert!(matches!(
        desc.validate(),
        Err(ShaderValidationError::TextureSamplingStage { instruction: 1, .. })
    ));
    desc.stage = ShaderStage::Fragment;
    desc.instructions = &[
        Op::Input { location: 0 },
        Op::SampleTexture {
            binding: 8,
            coordinates: 0,
        },
    ];
    assert!(matches!(
        desc.validate(),
        Err(ShaderValidationError::InvalidTextureBinding { binding: 8, .. })
    ));
    desc.instructions = &[
        Op::Input { location: 0 },
        Op::SampleTexture {
            binding: 0,
            coordinates: 1,
        },
    ];
    assert!(matches!(
        desc.validate(),
        Err(ShaderValidationError::InvalidOperand {
            instruction: 1,
            operand: 1
        })
    ));
}

#[test]
fn recording_requires_used_slots_and_failed_binds_keep_previous_state() {
    let d = device();
    let t = target(&d);
    let p = sample_pipeline(&d);
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&TEXELS).unwrap();
    let view = tex.view();
    let s = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::ClampToEdge,
        AddressMode::ClampToEdge,
    );
    let v = vertices(&d, true);
    let other = device();
    let foreign_tex = texture(&other, HostAccess::None);
    let foreign_sampler = sampler(
        &other,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingTextureBinding { binding: 0 })
    ));
    pass.set_texture(0, &view, &s).unwrap();
    assert!(matches!(
        pass.set_texture(8, &view, &s),
        Err(RenderEncodeError::InvalidTextureSlot { binding: 8 })
    ));
    assert!(matches!(
        pass.set_texture(0, &foreign_tex.view(), &s),
        Err(RenderEncodeError::ForeignTexture { .. })
    ));
    assert!(matches!(
        pass.set_texture(0, &view, &foreign_sampler),
        Err(RenderEncodeError::ForeignSampler { .. })
    ));
    pass.draw_programmable(0..6).unwrap();
    pass.draw(0..0, [7; 4]).unwrap();
    pass.finish().unwrap();
    let Command::RenderPass(pass) = &e.commands()[0] else {
        panic!()
    };
    assert!(pass.draws()[0]
        .texture(0)
        .unwrap()
        .view()
        .texture()
        .is_same(&tex));
    assert!(pass.draws()[0].texture(0).unwrap().sampler().is_same(&s));
    assert!(pass.draws()[0].texture(u32::MAX).is_none());
    assert!(pass.draws()[1].texture(0).is_none());
    d.default_queue().submit(&e.finish()).unwrap();
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingTextureBinding { binding: 0 })
    ));
    let i = buffer(&d, &[0; 6], BufferUsage::Index);
    pass.set_index_buffer(&i, 0, IndexFormat::Uint16).unwrap();
    assert!(matches!(
        pass.draw_indexed(0..0, 0),
        Err(RenderEncodeError::MissingTextureBinding { binding: 0 })
    ));
}

#[test]
fn indexed_and_direct_quads_sample_exact_interpolated_uvs() {
    let d = device();
    let t = target(&d);
    let p = sample_pipeline(&d);
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&TEXELS).unwrap();
    let view = tex.view();
    for filter in [FilterMode::Nearest, FilterMode::Linear] {
        let s = sampler(
            &d,
            filter,
            AddressMode::ClampToEdge,
            AddressMode::ClampToEdge,
        );
        for indexed in [false, true] {
            let c = commands(&d, &t, &p, &view, &s, indexed);
            assert!(c.commands().len() == 1);
            d.default_queue().submit(&c).unwrap();
            for y in 0..4 {
                for x in 0..4 {
                    let expected = view
                        .sample(&s, [(x as f32 + 0.5) / 4., (y as f32 + 0.5) / 4.])
                        .unwrap()
                        .map(|v| (v * 255.).round() as u8);
                    assert_eq!(t.pixel(x, y).unwrap(), expected);
                }
            }
        }
    }
}

#[test]
fn commands_retain_resources_and_resample_live_uploads() {
    let d = device();
    let t = target(&d);
    let p = sample_pipeline(&d);
    let tex = texture(&d, HostAccess::Write);
    let view = tex.view();
    let s = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    let c = commands(&d, &t, &p, &view, &s, true);
    assert_eq!(pixels(&t), [[0; 4]; 16]);
    drop(p);
    drop(view);
    drop(s);
    tex.write_pixels(&[[25, 50, 75, 100]; 4]).unwrap();
    d.default_queue().submit(&c).unwrap();
    assert_eq!(pixels(&t), [[25, 50, 75, 100]; 16]);
    tex.write_pixels(&[[80, 90, 100, 110]; 4]).unwrap();
    drop(tex);
    d.default_queue().submit(&c).unwrap();
    assert_eq!(pixels(&t), [[80, 90, 100, 110]; 16]);
}

#[test]
fn aliased_views_multiple_slots_and_unused_results_require_all_bindings() {
    use ShaderInstruction as Op;
    let d = device();
    let t = target(&d);
    let p = pipeline(
        &d,
        &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
            Op::SampleTexture {
                binding: 7,
                coordinates: 0,
            },
            Op::Add { left: 1, right: 2 },
        ],
        &[3],
    );
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&[[40, 50, 60, 70]; 4]).unwrap();
    let v = vertices(&d, true);
    let s = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &tex.view(), &s).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..6),
        Err(RenderEncodeError::MissingTextureBinding { binding: 7 })
    ));
    pass.set_texture(7, &tex.view(), &s).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(pixels(&t), [[80, 100, 120, 140]; 16]);
    let p = pipeline(
        &d,
        &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 7,
                coordinates: 0,
            },
            Op::Constant([1.; 4]),
        ],
        &[2],
    );
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    assert!(matches!(
        pass.draw_programmable(0..0),
        Err(RenderEncodeError::MissingTextureBinding { binding: 7 })
    ));
}

#[test]
fn per_draw_rebinding_retains_old_view_and_sampler() {
    let d = device();
    let t = target(&d);
    let p = sample_pipeline(&d);
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&TEXELS).unwrap();
    let a = tex
        .create_view(TextureViewDescriptor {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        })
        .unwrap();
    let b = tex
        .create_view(TextureViewDescriptor {
            x: 1,
            y: 1,
            width: 1,
            height: 1,
        })
        .unwrap();
    let nearest = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    let linear = sampler(
        &d,
        FilterMode::Linear,
        AddressMode::ClampToEdge,
        AddressMode::ClampToEdge,
    );
    let v = vertices(&d, true);
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass(&t, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &a, &nearest).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.set_texture(0, &b, &linear).unwrap();
    pass.draw_programmable(3..6).unwrap();
    pass.finish().unwrap();
    let Command::RenderPass(pass) = &e.commands()[0] else {
        panic!()
    };
    assert_eq!(
        pass.draws()[0].texture(0).unwrap().view().region(),
        a.region()
    );
    assert!(pass.draws()[0]
        .texture(0)
        .unwrap()
        .sampler()
        .is_same(&nearest));
    assert_eq!(
        pass.draws()[1].texture(0).unwrap().view().region(),
        b.region()
    );
    drop(a);
    drop(b);
    drop(tex);
    drop(nearest);
    drop(linear);
    d.default_queue().submit(&e.finish()).unwrap();
    assert_eq!(t.pixel(3, 0).unwrap(), TEXELS[0]);
    assert_eq!(t.pixel(0, 3).unwrap(), TEXELS[3]);
}

#[test]
fn sampled_fragment_failure_rolls_back_scope_and_preserves_completed_prefix() {
    use ShaderInstruction as Op;
    let d = device();
    let t = target(&d);
    let tex = texture(&d, HostAccess::Write);
    tex.write_pixels(&TEXELS).unwrap();
    let s = sampler(
        &d,
        FilterMode::Nearest,
        AddressMode::Repeat,
        AddressMode::Repeat,
    );
    let good = sample_pipeline(&d);
    let bad = pipeline(
        &d,
        &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
            Op::Constant([f32::MAX; 4]),
            Op::Multiply { left: 1, right: 2 },
            Op::Multiply { left: 3, right: 2 },
        ],
        &[4],
    );
    let v = vertices(&d, true);
    let mut e = d.create_command_encoder();
    e.begin_render_pass(&t, Some([7; 4]))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = e.begin_render_pass(&t, Some([9; 4])).unwrap();
    pass.set_vertex_buffer(&v, 0).unwrap();
    pass.set_texture(0, &tex.view(), &s).unwrap();
    pass.set_pipeline(&good).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.set_pipeline(&bad).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    e.begin_render_pass(&t, Some([11; 4]))
        .unwrap()
        .finish()
        .unwrap();
    assert!(matches!(
        d.default_queue().submit(&e.finish()),
        Err(QueueSubmitError::RenderFailed {
            command_index: 1,
            source: RenderExecutionError::NonFiniteShaderResult {
                stage: ShaderStage::Fragment,
                ..
            }
        })
    ));
    assert_eq!(pixels(&t), [[7; 4]; 16]);
}
