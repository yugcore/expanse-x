use ex::{
    BufferDescriptor, BufferUsage, FramebufferDescriptor, FramebufferFormat,
    GraphicsPipelineDescriptor, HostAccess, IndexFormat, Instance, ShaderDescriptor,
    ShaderInstruction as Op, ShaderStage, VertexAttribute, VertexBufferLayout, VertexFormat,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase3-indexed-quad.png"));
    let device = Instance::new().default_adapter().create_device();
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "indexed quad",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let vertex = device.create_shader(ShaderDescriptor {
        label: "position and colour",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[Op::Input { location: 0 }, Op::Input { location: 1 }],
        outputs: &[0, 1],
    })?;
    let fragment = device.create_shader(ShaderDescriptor {
        label: "tint colour",
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
        label: "indexed colour pipeline",
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
    let data = [
        [-0.75_f32, 0.75, 1.0, 0.1, 0.1],
        [0.75, 0.75, 0.1, 1.0, 0.1],
        [0.75, -0.75, 0.1, 0.1, 1.0],
        [-0.75, -0.75, 1.0, 1.0, 0.1],
    ];
    let mut bytes: Vec<u8> = data
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect();
    let vertex_size = bytes.len() as u64;
    let indices = [0_u16, 1, 2, 2, 3, 0];
    bytes.extend(indices.iter().flat_map(|n| n.to_le_bytes()));
    let upload = device.create_buffer(BufferDescriptor {
        label: "combined upload",
        size: bytes.len() as u64,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    upload.write(0, &bytes)?;
    let vertices = device.create_buffer(BufferDescriptor {
        label: "four shared vertices",
        size: vertex_size,
        usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    let index_buffer = device.create_buffer(BufferDescriptor {
        label: "six index elements",
        size: 12,
        usages: &[BufferUsage::Index, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    let background = [13, 19, 33, 255];
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &vertices, 0, vertex_size)?;
    encoder.copy_buffer(&upload, vertex_size, &index_buffer, 0, 12)?;
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.set_index_buffer(&index_buffer, 0, IndexFormat::Uint16)?;
    pass.draw_indexed(0..6, 0)?;
    pass.finish()?;
    let commands = encoder.finish();
    assert_eq!(target.pixel(256, 192)?, [0; 4]);
    drop(vertex);
    drop(fragment);
    drop(pipeline);
    drop(vertices);
    drop(index_buffer);
    drop(upload);
    device.default_queue().submit(&commands)?;
    let mut pixels = vec![[0; 4]; 512 * 384];
    target.read_pixels(&mut pixels)?;
    for y in 0..384 {
        for x in 0..512 {
            let inside = (64..448).contains(&x) && (48..336).contains(&y);
            assert_eq!(pixels[y * 512 + x] != background, inside);
        }
    }
    let covered = pixels.iter().filter(|&&p| p != background).count();
    assert_eq!(covered, 110592);
    let colours: BTreeSet<_> = pixels
        .iter()
        .copied()
        .filter(|&p| p != background)
        .collect();
    assert!(colours.len() > 10000);
    png::write_rgba(&output, 512, 384, &pixels)?;
    println!(
        "EX rendered two indexed triangles using four vertices: {covered} pixels, {} colours.",
        colours.len()
    );
    println!(
        "Geometry storage: 80 vertex bytes + 12 index bytes; image saved to {}",
        output.display()
    );
    Ok(())
}
