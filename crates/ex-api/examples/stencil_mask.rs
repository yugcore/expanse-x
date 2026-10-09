use ex::*;
use std::path::PathBuf;
#[path = "triangle/png.rs"]
mod png;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase4-stencil-mask.png"));
    let d = Instance::new().default_adapter().create_device();
    let target = d.create_framebuffer(FramebufferDescriptor {
        label: "masked checkerboard",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let depth = d.create_depth_buffer(DepthBufferDescriptor {
        label: "z",
        width: 512,
        height: 384,
    })?;
    let stencil = d.create_stencil_buffer(StencilBufferDescriptor {
        label: "diamond mask",
        width: 512,
        height: 384,
    })?;
    let points = [
        [256., 32.],
        [464., 192.],
        [256., 352.],
        [256., 32.],
        [256., 352.],
        [48., 192.],
        [0., 0.],
        [512., 0.],
        [512., 384.],
        [512., 384.],
        [0., 384.],
        [0., 0.],
    ];
    let mut values = Vec::new();
    for [x, y] in points {
        values.extend_from_slice(&[
            x / 512. * 2. - 1.,
            1. - y / 384. * 2.,
            0_f32,
            1.,
            x / 512. * 4.,
            y / 384. * 4.,
            0.,
            0.,
        ]);
    }
    let bytes: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let vertices = d.create_buffer(BufferDescriptor {
        label: "mask and fill",
        size: bytes.len() as u64,
        usages: &[BufferUsage::Vertex],
        host_access: HostAccess::Write,
    })?;
    vertices.write(0, &bytes)?;
    let vs = d.create_shader(ShaderDescriptor {
        label: "positions and UV",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[
            ShaderInstruction::Input { location: 0 },
            ShaderInstruction::Input { location: 1 },
        ],
        outputs: &[0, 1],
    })?;
    let mark = d.create_shader(ShaderDescriptor {
        label: "invisible marker",
        stage: ShaderStage::Fragment,
        input_count: 1,
        instructions: &[ShaderInstruction::Constant([0., 0., 0., 1.])],
        outputs: &[0],
    })?;
    let sample = d.create_shader(ShaderDescriptor {
        label: "checkerboard",
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
    })?;
    let layout = VertexBufferLayout {
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
    };
    let descriptor = |fragment_shader| GraphicsPipelineDescriptor {
        label: "stencil",
        vertex_shader: &vs,
        fragment_shader,
        vertex_layout: layout,
        colour_format: FramebufferFormat::Rgba8Unorm,
    };
    let mut hidden = BlendState::additive();
    hidden.write_mask = [false; 4];
    let mask_pipeline = d.create_graphics_pipeline_with_stencil(
        descriptor(&mark),
        GraphicsPipelineOptions {
            depth: None,
            blend: Some(hidden),
        },
        StencilState::replace(1),
    )?;
    let fill_pipeline = d.create_graphics_pipeline_with_stencil(
        descriptor(&sample),
        GraphicsPipelineOptions {
            depth: Some(DepthState {
                compare: CompareFunction::Less,
                write_enabled: true,
            }),
            blend: Some(BlendState::alpha_blending()),
        },
        StencilState::test_equal(1),
    )?;
    let texture = d.create_texture(TextureDescriptor {
        label: "alpha checker",
        width: 4,
        height: 4,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::Write,
    })?;
    let texels: Vec<_> = (0..16)
        .map(|i| {
            if (i % 4 + i / 4) % 2 == 0 {
                [64, 192, 255, 192]
            } else {
                [255, 128, 32, 128]
            }
        })
        .collect();
    texture.write_pixels(&texels)?;
    let sampler = d.create_sampler(SamplerDescriptor {
        label: "repeat",
        filter: FilterMode::Nearest,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
    })?;
    let mut e = d.create_command_encoder();
    let mut pass = e.begin_render_pass_with_attachments(
        &target,
        Some([16, 24, 40, 255]),
        RenderAttachments {
            depth: Some(&depth),
            clear_depth: Some(1.),
            stencil: Some(&stencil),
            clear_stencil: Some(0),
        },
    )?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.set_pipeline(&mask_pipeline)?;
    pass.draw_programmable(0..6)?;
    pass.set_pipeline(&fill_pipeline)?;
    pass.set_texture(0, &texture.view(), &sampler)?;
    pass.draw_programmable(6..12)?;
    pass.finish()?;
    d.default_queue().submit(&e.finish())?;
    let mut pixels = vec![[0; 4]; 512 * 384];
    target.read_pixels(&mut pixels)?;
    let mut depths = vec![0.; 512 * 384];
    depth.read_depths(&mut depths)?;
    let mut masks = vec![0; 512 * 384];
    stencil.read_stencils(&mut masks)?;
    let mut covered = 0;
    for ((p, z), s) in pixels.iter().zip(&depths).zip(&masks) {
        match *s {
            0 => {
                assert_eq!(*p, [16, 24, 40, 255]);
                assert_eq!(*z, 1.);
            }
            1 => {
                assert!(*p == [52, 150, 202, 255] || *p == [136, 76, 36, 255]);
                assert_eq!(*z, 0.5);
                covered += 1;
            }
            _ => panic!("unexpected stencil value"),
        }
    }
    assert!(covered > 0);
    png::write_rgba(&output, 512, 384, &pixels)?;
    println!(
        "EX rendered a blended checkerboard through a diamond stencil mask: {covered} pixels."
    );
    println!(
        "Colour, depth and stencil readback verified; saved {}",
        output.display()
    );
    Ok(())
}
