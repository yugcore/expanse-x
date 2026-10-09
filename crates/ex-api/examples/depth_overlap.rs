use ex::*;
use std::path::PathBuf;
#[path = "triangle/png.rs"]
mod png;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase4-depth-overlap.png"));
    let d = Instance::new().default_adapter().create_device();
    let target = d.create_framebuffer(FramebufferDescriptor {
        label: "overlap",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let depth = d.create_depth_buffer(DepthBufferDescriptor {
        label: "visibility",
        width: 512,
        height: 384,
    })?;
    let positions = [
        [-0.75_f32, 0.75, -0.5, 1.],
        [0., 0.75, -0.5, 1.],
        [-0.375, -0.75, -0.5, 1.],
        [-0.5, 0.5, 0., 1.],
        [0.75, 0.5, 0., 1.],
        [0.125, -0.75, 0., 1.],
        [-0.875, 0.875, 0.5, 1.],
        [0.875, 0.875, 0.5, 1.],
        [0., -0.875, 0.5, 1.],
    ];
    let bytes: Vec<_> = positions
        .iter()
        .flat_map(|p| p.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let vertices = d.create_buffer(BufferDescriptor {
        label: "triangles",
        size: bytes.len() as u64,
        usages: &[BufferUsage::Vertex],
        host_access: HostAccess::Write,
    })?;
    vertices.write(0, &bytes)?;
    let vs = d.create_shader(ShaderDescriptor {
        label: "positions",
        stage: ShaderStage::Vertex,
        input_count: 1,
        instructions: &[ShaderInstruction::Input { location: 0 }],
        outputs: &[0],
    })?;
    let mut pipelines = Vec::new();
    for rgba in [
        [1., 0.25, 0.2, 1.],
        [0.2, 0.85, 0.35, 1.],
        [0.2, 0.45, 1., 1.],
    ] {
        let fs = d.create_shader(ShaderDescriptor {
            label: "colour",
            stage: ShaderStage::Fragment,
            input_count: 0,
            instructions: &[ShaderInstruction::Constant(rgba)],
            outputs: &[0],
        })?;
        pipelines.push(d.create_graphics_pipeline_with_depth(
            GraphicsPipelineDescriptor {
                label: "visible surface",
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
            DepthState {
                compare: CompareFunction::Less,
                write_enabled: true,
            },
        )?);
    }
    let mut reference = None;
    for order in [[0, 1, 2], [2, 1, 0]] {
        let mut e = d.create_command_encoder();
        let mut pass =
            e.begin_render_pass_with_depth(&target, Some([13, 19, 33, 255]), &depth, Some(1.))?;
        pass.set_vertex_buffer(&vertices, 0)?;
        for i in order {
            pass.set_pipeline(&pipelines[i])?;
            pass.draw_programmable(i as u32 * 3..i as u32 * 3 + 3)?;
        }
        pass.finish()?;
        let commands = e.finish();
        d.default_queue().submit(&commands)?;
        let mut pixels = vec![[0; 4]; 512 * 384];
        let mut depths = vec![0.; 512 * 384];
        target.read_pixels(&mut pixels)?;
        depth.read_depths(&mut depths)?;
        if let Some((old_pixels, old_depths)) = &reference {
            assert_eq!(&pixels, old_pixels);
            assert_eq!(&depths, old_depths);
        } else {
            reference = Some((pixels.clone(), depths.clone()));
        }
        for (p, z) in pixels.iter().zip(&depths) {
            assert_eq!(
                *z,
                match *p {
                    [255, 64, 51, 255] => 0.25,
                    [51, 217, 89, 255] => 0.5,
                    [51, 115, 255, 255] => 0.75,
                    [13, 19, 33, 255] => 1.,
                    _ => panic!("unexpected pixel"),
                }
            );
        }
        png::write_rgba(&output, 512, 384, &pixels)?;
    }
    println!(
        "EX rendered three overlapping triangles with identical colour/depth in both draw orders."
    );
    println!("Image saved to {}", output.display());
    Ok(())
}
