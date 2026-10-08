use ex::*;
use std::path::PathBuf;
#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase4-textured-quad.png"));
    let d = Instance::new().default_adapter().create_device();
    let target = d.create_framebuffer(FramebufferDescriptor {
        label: "checkerboard",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let texture = d.create_texture(TextureDescriptor {
        label: "checkerboard",
        width: 8,
        height: 8,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::Write,
    })?;
    let dark = [30, 60, 100, 255];
    let light = [240, 200, 90, 255];
    let background = [13, 19, 33, 255];
    let pixels: Vec<_> = (0..64)
        .map(|i| {
            if (i % 8 + i / 8) % 2 == 0 {
                light
            } else {
                dark
            }
        })
        .collect();
    texture.write_pixels(&pixels)?;
    let view = texture.view();
    let nearest = d.create_sampler(SamplerDescriptor {
        label: "nearest repeat",
        filter: FilterMode::Nearest,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
    })?;
    let linear = d.create_sampler(SamplerDescriptor {
        label: "bilinear repeat",
        filter: FilterMode::Linear,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
    })?;
    use ShaderInstruction as Op;
    let vertex = d.create_shader(ShaderDescriptor {
        label: "UVs",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[Op::Input { location: 0 }, Op::Input { location: 1 }],
        outputs: &[0, 1],
    })?;
    let fragment = d.create_shader(ShaderDescriptor {
        label: "sample image",
        stage: ShaderStage::Fragment,
        input_count: 1,
        instructions: &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
        ],
        outputs: &[1],
    })?;
    let pipeline = d.create_graphics_pipeline(GraphicsPipelineDescriptor {
        label: "textured quad",
        vertex_shader: &vertex,
        fragment_shader: &fragment,
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
    })?;
    let mut data = Vec::new();
    // Pixel boundaries: left x=32..240, right x=272..480, both y=48..336.
    for (left, right) in [(-0.875_f32, -0.0625_f32), (0.0625_f32, 0.875_f32)] {
        for record in [
            [left, 0.75, -0.25, -0.25],
            [right, 0.75, 1.75, -0.25],
            [right, -0.75, 1.75, 1.75],
            [left, -0.75, -0.25, 1.75],
        ] {
            data.extend(record.into_iter().flat_map(f32::to_le_bytes));
        }
    }
    let vertices = d.create_buffer(BufferDescriptor {
        label: "eight vertices",
        size: data.len() as u64,
        usages: &[BufferUsage::Vertex],
        host_access: HostAccess::Write,
    })?;
    vertices.write(0, &data)?;
    let indices = d.create_buffer(BufferDescriptor {
        label: "shared six indices",
        size: 12,
        usages: &[BufferUsage::Index],
        host_access: HostAccess::Write,
    })?;
    indices.write(0, &[0, 0, 1, 0, 2, 0, 2, 0, 3, 0, 0, 0])?;
    let mut encoder = d.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)?;
    pass.set_texture(0, &view, &nearest)?;
    pass.draw_indexed(0..6, 0)?;
    pass.set_texture(0, &view, &linear)?;
    pass.draw_indexed(0..6, 4)?;
    pass.finish()?;
    let commands = encoder.finish();
    assert_eq!(target.pixel(128, 192)?, [0; 4]);
    drop(texture);
    drop(view);
    drop(nearest);
    drop(linear);
    drop(vertex);
    drop(fragment);
    drop(pipeline);
    drop(vertices);
    drop(indices);
    d.default_queue().submit(&commands)?;
    let mut pixels = vec![[0; 4]; 512 * 384];
    target.read_pixels(&mut pixels)?;
    let mut covered = 0;
    for y in 0..384 {
        for x in 0..512 {
            let inside =
                ((32..240).contains(&x) || (272..480).contains(&x)) && (48..336).contains(&y);
            let colour = pixels[y * 512 + x];
            assert_eq!(colour != background, inside);
            if inside {
                covered += 1;
                assert_eq!(colour[3], 255);
                if x < 240 {
                    assert!(colour == dark || colour == light);
                }
            }
        }
    }
    assert_eq!(covered, 119808);
    assert!(pixels
        .iter()
        .any(|&p| p != dark && p != light && p != background));
    png::write_rgba(&output, 512, 384, &pixels)?;
    println!("EX rendered checkerboards with nearest (left) and bilinear (right) repeat sampling: {covered} pixels.");
    println!("Image saved to {}", output.display());
    Ok(())
}
