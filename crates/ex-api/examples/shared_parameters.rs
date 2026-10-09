use ex::*;
use std::path::PathBuf;
use ResourceAccess as Access;
use ShaderInstruction as Op;

#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/core-bindings-blue.png"));
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
    // One reusable interface supplies all three shader stages.
    let layout = device.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "tile resources",
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderVisibility::All,
                ty: BindingType::UniformBuffer { min_size: 80 },
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderVisibility::Compute,
                ty: BindingType::StorageBuffer { read_only: false },
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderVisibility::Fragment,
                ty: BindingType::SampledTexture,
            },
        ],
    })?;
    let values = [
        [0.09_f32, 0.12, 1.0, 1.0],
        [0.24, 0.32, 0.0, 0.0],
        [-0.84, -0.8, 0.0, 0.0],
        [0.25, 0.75, 1.0, 1.0],
        [1.0; 4],
    ];
    let parameter_upload = device.create_buffer(BufferDescriptor {
        label: "parameter upload",
        size: 80,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    parameter_upload.write(
        0,
        &values
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )?;
    let parameters = device.create_buffer(BufferDescriptor {
        label: "shared tile parameters",
        size: 80,
        usages: &[BufferUsage::Uniform, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    let texture = device.create_texture(TextureDescriptor {
        label: "white material texture",
        width: 1,
        height: 1,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::Write,
    })?;
    texture.write_pixels(&[[255; 4]])?;
    let sampler = device.create_sampler(SamplerDescriptor {
        label: "material sampler",
        filter: FilterMode::Nearest,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
    })?;
    let group = device.create_bind_group(BindGroupDescriptor {
        label: "shared tile group",
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::UniformBuffer(BufferBinding {
                    buffer: parameters.clone(),
                    offset: 0,
                    size: 80,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::StorageBuffer(BufferBinding {
                    buffer: vertices.clone(),
                    offset: 0,
                    size: vertices.size(),
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::SampledTexture {
                    view: texture.view(),
                    sampler: sampler.clone(),
                },
            },
        ],
    })?;
    let kernel = device.create_shader(ShaderDescriptor {
        label: "place a tile per workgroup",
        stage: ShaderStage::Compute,
        input_count: 4,
        instructions: &[
            Op::LoadStorage { binding: 1 },
            Op::LoadUniform {
                binding: 0,
                element: 0,
            },
            Op::Multiply { left: 0, right: 1 },
            Op::Input { location: 2 },
            Op::LoadUniform {
                binding: 0,
                element: 1,
            },
            Op::Multiply { left: 3, right: 4 },
            Op::Add { left: 2, right: 5 },
            Op::LoadUniform {
                binding: 0,
                element: 2,
            },
            Op::Add { left: 6, right: 7 },
        ],
        outputs: &[8],
    })?;
    let compute = device.create_compute_pipeline_with_bindings(
        ComputePipelineDescriptor {
            label: "8 by 6 tile grid",
            shader: &kernel,
            workgroup_size: [6, 1, 1],
            output_bindings: &[1],
        },
        &layout,
    )?;
    let vertex = device.create_shader(ShaderDescriptor {
        label: "computed position",
        stage: ShaderStage::Vertex,
        input_count: 1,
        instructions: &[
            Op::Input { location: 0 },
            Op::LoadUniform {
                binding: 0,
                element: 4,
            },
            Op::Multiply { left: 0, right: 1 },
        ],
        outputs: &[2],
    })?;
    let fragment = device.create_shader(ShaderDescriptor {
        label: "tile colour",
        stage: ShaderStage::Fragment,
        input_count: 0,
        instructions: &[
            Op::Constant([0.; 4]),
            Op::SampleTexture {
                binding: 2,
                coordinates: 0,
            },
            Op::LoadUniform {
                binding: 0,
                element: 3,
            },
            Op::Multiply { left: 1, right: 2 },
        ],
        outputs: &[3],
    })?;
    let graphics = device.create_graphics_pipeline_with_bindings(
        GraphicsPipelineDescriptor {
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
        },
        GraphicsPipelineOptions::default(),
        None,
        &layout,
    )?;
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "compute tiles",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let background = [16, 24, 40, 255];
    let mut encoder =
        device.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    encoder.transition_resources(&[
        ResourceTransition {
            resource: Resource::Buffer(parameter_upload.clone()),
            before: None,
            after: Access::TransferRead,
        },
        ResourceTransition {
            resource: Resource::Buffer(parameters.clone()),
            before: None,
            after: Access::TransferWrite,
        },
        ResourceTransition {
            resource: Resource::Texture(texture.clone()),
            before: None,
            after: Access::SampledRead,
        },
        ResourceTransition {
            resource: Resource::Buffer(upload.clone()),
            before: None,
            after: Access::TransferRead,
        },
        ResourceTransition {
            resource: Resource::Buffer(vertices.clone()),
            before: None,
            after: Access::TransferWrite,
        },
        ResourceTransition {
            resource: Resource::Buffer(readback.clone()),
            before: None,
            after: Access::TransferWrite,
        },
        ResourceTransition {
            resource: Resource::Framebuffer(target.clone()),
            before: None,
            after: Access::ColourReadWrite,
        },
    ])?;
    encoder.copy_buffer(&parameter_upload, 0, &parameters, 0, 80)?;
    encoder.transition_resources(&[ResourceTransition {
        resource: Resource::Buffer(parameters.clone()),
        before: Some(Access::TransferWrite),
        after: Access::UniformRead,
    }])?;
    encoder.copy_buffer(&upload, 0, &vertices, 0, vertices.size())?;
    encoder.transition_resources(&[ResourceTransition {
        resource: Resource::Buffer(vertices.clone()),
        before: Some(Access::TransferWrite),
        after: Access::StorageReadWrite,
    }])?;
    encoder.dispatch_with_bind_group(&compute, [8, 6, 1], &group)?;
    encoder.transition_resources(&[ResourceTransition {
        resource: Resource::Buffer(vertices.clone()),
        before: Some(Access::StorageReadWrite),
        after: Access::VertexRead,
    }])?;
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_pipeline(&graphics)?;
    pass.set_bind_group(&group)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.draw_programmable(0..288)?;
    pass.finish()?;
    encoder.transition_resources(&[ResourceTransition {
        resource: Resource::Buffer(vertices.clone()),
        before: Some(Access::VertexRead),
        after: Access::TransferRead,
    }])?;
    encoder.copy_buffer(&vertices, 0, &readback, 0, vertices.size())?;
    let commands = encoder.finish();
    commands.validate_dependencies()?;
    let uses = commands.resource_uses()?;
    assert_eq!(commands.commands().len(), 10);
    assert_eq!(uses.len(), 12);
    for usage in &uses {
        println!(
            "Command {}: {:?} {:?} uses {:?}",
            usage.command_index,
            usage.resource.kind(),
            usage.resource.label(),
            usage.access
        );
    }
    let mut bytes = vec![99; data.len()];
    readback.read(0, &mut bytes)?;
    assert!(bytes.iter().all(|&byte| byte == 0));
    drop(group);
    drop(layout);
    drop(parameters);
    drop(texture);
    drop(sampler);
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
    println!("EX shared bind group and parameters validated; compute ran 288 invocations and generated 48 tiles covering {covered} pixels.");
    println!("All 288 computed vertices and all 48 tile centres verified.");
    println!("Image saved to {}", output.display());
    // Reuse exactly the same pipelines, group and finished commands with live parameters.
    let orange = [1.0_f32, 0.5, 0.125, 1.0];
    parameter_upload.write(
        48,
        &orange
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )?;
    device.default_queue().submit(&commands)?;
    let mut second = vec![[0; 4]; pixels.len()];
    target.read_pixels(&mut second)?;
    for (first, later) in pixels.iter().zip(&second) {
        assert_eq!(
            *later,
            if *first == background {
                background
            } else {
                [255, 128, 32, 255]
            }
        );
    }
    let orange_output = output.with_file_name("core-bindings-orange.png");
    png::write_rgba(&orange_output, 512, 384, &second)?;
    println!("Same finished work reused with an orange material: every pixel verified.");
    println!("Updated image saved to {}", orange_output.display());
    Ok(())
}
