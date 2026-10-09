use ex::{
    BufferDescriptor, BufferUsage, ComputePipelineDescriptor, FramebufferDescriptor,
    FramebufferFormat, GraphicsPipelineDescriptor, HostAccess, Instance, ShaderDescriptor,
    ShaderInstruction as Op, ShaderStage, StorageBufferBinding, VertexAttribute,
    VertexBufferLayout, VertexFormat,
};
use std::path::PathBuf;

#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase4-compute-tiles.png"));
    let device = Instance::new().default_adapter().create_device();
    // Each workgroup supplies six corners (two triangles). The compute shader
    // scales the local square and places it according to its workgroup ID.
    let corners = [
        [-1.0_f32, -1.0, 0.0, 1.0],
        [1.0, -1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [-1.0, 1.0, 0.0, 1.0],
    ];
    let data: Vec<u8> = (0..48)
        .flat_map(|_| corners.iter().flatten().flat_map(|lane| lane.to_le_bytes()))
        .collect();
    let upload = device.create_buffer(BufferDescriptor {
        label: "tile corner upload",
        size: data.len() as u64,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    upload.write(0, &data)?;
    let vertices = device.create_buffer(BufferDescriptor {
        label: "compute generated vertices",
        size: data.len() as u64,
        usages: &[
            BufferUsage::Storage,
            BufferUsage::Vertex,
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        host_access: HostAccess::None,
    })?;
    let readback = device.create_buffer(BufferDescriptor {
        label: "computed vertex readback",
        size: data.len() as u64,
        usages: &[BufferUsage::TransferDestination],
        host_access: HostAccess::Read,
    })?;
    let kernel = device.create_shader(ShaderDescriptor {
        label: "place a tile per workgroup",
        stage: ShaderStage::Compute,
        input_count: 4,
        instructions: &[
            Op::LoadStorage { binding: 0 },
            Op::Constant([0.09, 0.12, 1.0, 1.0]),
            Op::Multiply { left: 0, right: 1 },
            Op::Input { location: 2 },
            Op::Constant([0.24, 0.32, 0.0, 0.0]),
            Op::Multiply { left: 3, right: 4 },
            Op::Add { left: 2, right: 5 },
            Op::Constant([-0.84, -0.8, 0.0, 0.0]),
            Op::Add { left: 6, right: 7 },
        ],
        outputs: &[8],
    })?;
    let compute = device.create_compute_pipeline(ComputePipelineDescriptor {
        label: "8 by 6 tile grid",
        shader: &kernel,
        workgroup_size: [6, 1, 1],
        output_bindings: &[0],
    })?;
    let vertex = device.create_shader(ShaderDescriptor {
        label: "computed position",
        stage: ShaderStage::Vertex,
        input_count: 1,
        instructions: &[Op::Input { location: 0 }],
        outputs: &[0],
    })?;
    let fragment = device.create_shader(ShaderDescriptor {
        label: "tile colour",
        stage: ShaderStage::Fragment,
        input_count: 0,
        instructions: &[Op::Constant([0.25, 0.75, 1.0, 1.0])],
        outputs: &[0],
    })?;
    let graphics = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
        label: "draw computed geometry",
        vertex_shader: &vertex,
        fragment_shader: &fragment,
        vertex_layout: VertexBufferLayout {
            stride: 16,
            attributes: &[VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x4,
            }],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    })?;
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "compute tiles",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let background = [16, 24, 40, 255];
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &vertices, 0, vertices.size())?;
    encoder.dispatch(
        &compute,
        [8, 6, 1],
        &[StorageBufferBinding {
            binding: 0,
            buffer: &vertices,
            offset: 0,
            size: vertices.size(),
        }],
    )?;
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_pipeline(&graphics)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.draw_programmable(0..288)?;
    pass.finish()?;
    encoder.copy_buffer(&vertices, 0, &readback, 0, vertices.size())?;
    let commands = encoder.finish();
    let mut bytes = vec![99; data.len()];
    readback.read(0, &mut bytes)?;
    assert!(bytes.iter().all(|&byte| byte == 0));
    drop(upload);
    drop(vertices);
    drop(kernel);
    drop(compute);
    drop(vertex);
    drop(fragment);
    drop(graphics);
    device.default_queue().submit(&commands)?;
    readback.read(0, &mut bytes)?;
    for (linear, vertex) in bytes.as_chunks::<16>().0.iter().enumerate() {
        let x = linear % 48;
        let y = linear / 48;
        let corner = corners[x % 6];
        let actual: [f32; 4] = std::array::from_fn(|lane| {
            let offset = lane * 4;
            f32::from_le_bytes([
                vertex[offset],
                vertex[offset + 1],
                vertex[offset + 2],
                vertex[offset + 3],
            ])
        });
        let expected = [
            (corner[0] * 0.09 + (x / 6) as f32 * 0.24) - 0.84,
            (corner[1] * 0.12 + y as f32 * 0.32) - 0.8,
            0.0,
            1.0,
        ];
        assert_eq!(actual, expected);
    }
    let mut pixels = vec![[0; 4]; 512 * 384];
    target.read_pixels(&mut pixels)?;
    assert_eq!(target.pixel(0, 0)?, background);
    for y in 0..6 {
        for x in 0..8 {
            let centre_x = ((-0.84 + x as f32 * 0.24) * 0.5 + 0.5) * 512.0;
            let centre_y = (0.5 - (-0.8 + y as f32 * 0.32) * 0.5) * 384.0;
            assert_eq!(
                target.pixel(centre_x as u32, centre_y as u32)?,
                [64, 191, 255, 255]
            );
        }
    }
    let covered = pixels.iter().filter(|&&pixel| pixel != background).count();
    assert!(covered > 90_000);
    png::write_rgba(&output, 512, 384, &pixels)?;
    println!("EX compute ran 288 invocations and generated 48 tiles covering {covered} pixels.");
    println!("All 288 computed vertices and all 48 tile centres verified.");
    println!("Image saved to {}", output.display());
    Ok(())
}
