use ex::{
    BufferDescriptor, BufferUsage, FramebufferDescriptor, FramebufferFormat,
    GraphicsPipelineDescriptor, HostAccess, Instance, ShaderDescriptor, ShaderInstruction as Op,
    ShaderStage, VertexAttribute, VertexBufferLayout, VertexFormat,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase3-programmable-triangle.png"));
    let device = Instance::new().default_adapter().create_device();
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "programmable triangle",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let vertex = device.create_shader(ShaderDescriptor {
        label: "scale and translate positions",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[
            Op::Input { location: 0 },
            Op::Input { location: 1 },
            Op::Constant([0.9, 0.9, 1.0, 1.0]),
            Op::Multiply { left: 0, right: 2 },
            Op::Constant([0.05, 0.0, 0.0, 0.0]),
            Op::Add { left: 3, right: 4 },
        ],
        outputs: &[5, 1],
    })?;
    let fragment = device.create_shader(ShaderDescriptor {
        label: "tint interpolated colour",
        stage: ShaderStage::Fragment,
        input_count: 1,
        instructions: &[
            Op::Input { location: 0 },
            Op::Constant([1.0, 0.9, 1.0, 1.0]),
            Op::Multiply { left: 0, right: 1 },
        ],
        outputs: &[2],
    })?;
    let pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
        label: "position and colour pipeline",
        vertex_shader: &vertex,
        fragment_shader: &fragment,
        vertex_layout: VertexBufferLayout {
            stride: 20,
            attributes: &[
                VertexAttribute {
                    location: 0,
                    offset: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    location: 1,
                    offset: 8,
                    format: VertexFormat::Float32x3,
                },
            ],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    })?;
    // x,y then r,g,b. Layout expansion supplies z=0, w=1 and alpha=1.
    let data = [
        [0.0_f32, 0.8, 1.0, 0.2, 0.1],
        [-0.8, -0.7, 0.1, 1.0, 0.2],
        [0.8, -0.7, 0.2, 0.1, 1.0],
    ];
    let bytes: Vec<u8> = data
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let upload = device.create_buffer(BufferDescriptor {
        label: "upload",
        size: bytes.len() as u64,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    upload.write(0, &bytes)?;
    let vertices = device.create_buffer(BufferDescriptor {
        label: "vertices",
        size: bytes.len() as u64,
        usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    let background = [13, 19, 33, 255];
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &vertices, 0, bytes.len() as u64)?;
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.draw_programmable(0..3)?;
    pass.finish()?;
    let commands = encoder.finish();
    assert_eq!(target.pixel(256, 192)?, [0; 4]);
    drop(vertex);
    drop(fragment);
    drop(pipeline);
    drop(vertices);
    drop(upload);
    device.default_queue().submit(&commands)?;
    assert_eq!(target.pixel(0, 0)?, background);
    assert_ne!(target.pixel(256, 192)?, background);
    let top = target.pixel(269, 70)?;
    let left = target.pixel(107, 300)?;
    let right = target.pixel(430, 300)?;
    assert!(top[0] > top[1] && top[0] > top[2]);
    assert!(left[1] > left[0] && left[1] > left[2]);
    assert!(right[2] > right[0] && right[2] > right[1]);
    let mut pixels = vec![[0; 4]; target.width() as usize * target.height() as usize];
    target.read_pixels(&mut pixels)?;
    let covered = pixels.iter().filter(|&&p| p != background).count();
    let colours: BTreeSet<_> = pixels
        .iter()
        .copied()
        .filter(|&p| p != background)
        .collect();
    assert!(colours.len() > 10000);
    png::write_rgba(&output, target.width(), target.height(), &pixels)?;
    println!(
        "EX shaders rendered {covered} pixels with {} distinct colours.",
        colours.len()
    );
    println!("Image saved to {}", output.display());
    Ok(())
}
