use ex::*;
use std::path::PathBuf;
#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase4-blended-triangles.png"));
    let d = Instance::new().default_adapter().create_device();
    let target = d.create_framebuffer(FramebufferDescriptor {
        label: "blend comparison",
        width: 768,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let depth = d.create_depth_buffer(DepthBufferDescriptor {
        label: "transparent depth",
        width: 768,
        height: 384,
    })?;
    let vs = d.create_shader(ShaderDescriptor {
        label: "position",
        stage: ShaderStage::Vertex,
        input_count: 1,
        instructions: &[ShaderInstruction::Input { location: 0 }],
        outputs: &[0],
    })?;
    let replace = BlendState {
        colour: BlendComponent {
            source_factor: BlendFactor::One,
            destination_factor: BlendFactor::Zero,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            source_factor: BlendFactor::Zero,
            destination_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        write_mask: [true; 4],
    };
    // Identical triangles in three panels, drawn far to near. Depth is tested,
    // but transparent surfaces do not update depth; background stays opaque.
    let mut positions = Vec::new();
    for panel in 0..3 {
        for (points, z) in [
            ([[16., 24.], [240., 24.], [128., 360.]], 0.5),
            ([[64., 96.], [224., 96.], [144., 336.]], 0.),
            ([[32., 48.], [128., 48.], [80., 336.]], -0.5),
        ] {
            for [x, y] in points {
                positions.push([
                    (x + panel as f32 * 256.) / 768. * 2. - 1.,
                    1. - y / 384. * 2.,
                    z,
                    1.,
                ]);
            }
        }
    }
    let bytes: Vec<_> = positions
        .iter()
        .flat_map(|p| p.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    let vertices = d.create_buffer(BufferDescriptor {
        label: "three panels",
        size: bytes.len() as u64,
        usages: &[BufferUsage::Vertex],
        host_access: HostAccess::Write,
    })?;
    vertices.write(0, &bytes)?;
    let mut pipelines = Vec::new();
    for blend in [
        replace,
        BlendState::alpha_blending(),
        BlendState::additive(),
    ] {
        for rgba in [[0., 0., 1., 0.5], [0., 1., 0., 0.5], [1., 0., 0., 0.5]] {
            let fs = d.create_shader(ShaderDescriptor {
                label: "half alpha",
                stage: ShaderStage::Fragment,
                input_count: 0,
                instructions: &[ShaderInstruction::Constant(rgba)],
                outputs: &[0],
            })?;
            pipelines.push(d.create_graphics_pipeline_with_options(
                GraphicsPipelineDescriptor {
                    label: "composite",
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
                GraphicsPipelineOptions {
                    depth: Some(DepthState {
                        compare: CompareFunction::Less,
                        write_enabled: false,
                    }),
                    blend: Some(blend),
                },
            )?);
        }
    }
    let mut e = d.create_command_encoder();
    let mut pass =
        e.begin_render_pass_with_depth(&target, Some([16, 24, 40, 255]), &depth, Some(1.))?;
    pass.set_vertex_buffer(&vertices, 0)?;
    for (index, pipeline) in pipelines.iter().enumerate() {
        pass.set_pipeline(pipeline)?;
        pass.draw_programmable(index as u32 * 3..index as u32 * 3 + 3)?;
    }
    pass.finish()?;
    d.default_queue().submit(&e.finish())?;
    let mut pixels = vec![[0; 4]; 768 * 384];
    target.read_pixels(&mut pixels)?;
    let mut depths = vec![0.; 768 * 384];
    depth.read_depths(&mut depths)?;
    assert!(depths.iter().all(|&z| z == 1.));
    assert!(pixels.iter().all(|p| p[3] == 255));
    // Inside all three triangles: replacement red, source-over mixture, additive white.
    assert_eq!(target.pixel(100, 150)?, [255, 0, 0, 255]);
    assert_eq!(target.pixel(356, 150)?, [130, 67, 37, 255]);
    assert_eq!(target.pixel(612, 150)?, [255, 255, 255, 255]);
    png::write_rgba(&output, 768, 384, &pixels)?;
    println!(
        "EX blend comparison: replace RGB (left), alpha source-over (middle), additive (right)."
    );
    println!(
        "Depth writes are disabled for these transparent surfaces. Saved {}",
        output.display()
    );
    Ok(())
}
