//! Device upload -> image copy -> compute geometry -> offscreen rendering ->
//! later sampled rendering -> padded buffer readback, with explicit dependencies.
use ex::*;
use std::path::PathBuf;
use ResourceAccess as A;
use ShaderInstruction as Op;
#[path = "triangle/png.rs"]
mod png;

fn transition(resource: Resource, before: Option<A>, after: A) -> ResourceTransition {
    ResourceTransition {
        resource,
        before,
        after,
    }
}
fn image(d: &Device, label: &str, width: u32, height: u32) -> Result<Texture, TextureCreateError> {
    d.create_texture(TextureDescriptor {
        label,
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::None,
    })
}
fn upload_pattern(buffer: &Buffer, colours: [[u8; 4]; 2]) -> Result<(), BufferAccessError> {
    let mut bytes = [77; 288]; // offset 4, 8 rows, 36-byte pitch, no final padding.
    for y in 0..8 {
        for x in 0..8 {
            let start = 4 + y * 36 + x * 4;
            bytes[start..start + 4].copy_from_slice(&colours[(x + y) % 2]);
        }
    }
    buffer.write(0, &bytes)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/core-texture-flow.png"));
    let d = Instance::new().default_adapter().create_device();
    let upload = d.create_buffer(BufferDescriptor {
        label: "padded checkerboard upload",
        size: 288,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    let input = image(&d, "uploaded checkerboard", 8, 8)?;
    let atlas = image(&d, "atlas", 12, 12)?;
    let atlas_view = atlas.create_view(TextureViewDescriptor {
        x: 2,
        y: 2,
        width: 8,
        height: 8,
    })?;
    let offscreen = image(&d, "rendered checkerboard", 128, 128)?;
    let final_image = image(&d, "two passes", 640, 320)?;
    let sampler = d.create_sampler(SamplerDescriptor {
        label: "nearest",
        filter: FilterMode::Nearest,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
    })?;

    // Compute scales positions by two while leaving UV coordinates unchanged.
    let seeds = [
        [-0.5_f32, 0.5, 0., 0.],
        [0.5, 0.5, 1., 0.],
        [0.5, -0.5, 1., 1.],
        [-0.5, 0.5, 0., 0.],
        [0.5, -0.5, 1., 1.],
        [-0.5, -0.5, 0., 1.],
    ];
    let geometry_upload = d.create_buffer(BufferDescriptor {
        label: "geometry seed upload",
        size: 96,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    geometry_upload.write(
        0,
        &seeds
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )?;
    let geometry = d.create_buffer(BufferDescriptor {
        label: "computed quad",
        size: 96,
        usages: &[
            BufferUsage::Storage,
            BufferUsage::Vertex,
            BufferUsage::TransferDestination,
        ],
        host_access: HostAccess::None,
    })?;
    let kernel = d.create_shader(ShaderDescriptor {
        label: "expand quad",
        stage: ShaderStage::Compute,
        input_count: 4,
        instructions: &[
            Op::LoadStorage { binding: 0 },
            Op::Constant([2., 2., 1., 1.]),
            Op::Multiply { left: 0, right: 1 },
        ],
        outputs: &[2],
    })?;
    let compute = d.create_compute_pipeline(ComputePipelineDescriptor {
        label: "quad generation",
        shader: &kernel,
        workgroup_size: [6, 1, 1],
        output_bindings: &[0],
    })?;

    let layout = d.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "image and transform",
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderVisibility::Fragment,
                ty: BindingType::SampledTexture,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderVisibility::Vertex,
                ty: BindingType::UniformBuffer { min_size: 32 },
            },
        ],
    })?;
    let transforms = d.create_buffer(BufferDescriptor {
        label: "three transforms",
        size: 96,
        usages: &[BufferUsage::Uniform],
        host_access: HostAccess::Write,
    })?;
    let values = [
        [1.0_f32; 4],
        [0.; 4],
        [0.4, 0.8, 1., 1.],
        [-0.5, 0., 0., 0.],
        [0.4, -0.8, 1., 1.],
        [0.5, 0., 0., 0.],
    ];
    transforms.write(
        0,
        &values
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )?;
    let make_group = |label: &str, view: TextureView, offset: u64| {
        d.create_bind_group(BindGroupDescriptor {
            label,
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::SampledTexture {
                        view,
                        sampler: sampler.clone(),
                    },
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::UniformBuffer(BufferBinding {
                        buffer: transforms.clone(),
                        offset,
                        size: 32,
                    }),
                },
            ],
        })
    };
    let first = make_group("atlas to offscreen", atlas_view.clone(), 0)?;
    let left = make_group("offscreen to left", offscreen.view(), 32)?;
    let right = make_group("offscreen to flipped right", offscreen.view(), 64)?;
    let vertex = d.create_shader(ShaderDescriptor {
        label: "transform position, preserve UV",
        stage: ShaderStage::Vertex,
        input_count: 2,
        instructions: &[
            Op::Input { location: 0 },
            Op::LoadUniform {
                binding: 1,
                element: 0,
            },
            Op::Multiply { left: 0, right: 1 },
            Op::LoadUniform {
                binding: 1,
                element: 1,
            },
            Op::Add { left: 2, right: 3 },
            Op::Input { location: 1 },
        ],
        outputs: &[4, 5],
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
    let pipeline = d.create_graphics_pipeline_with_bindings(
        GraphicsPipelineDescriptor {
            label: "image quad",
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
        },
        GraphicsPipelineOptions::default(),
        None,
        &layout,
    )?;
    let readback = d.create_buffer(BufferDescriptor {
        label: "padded readback",
        size: 824328,
        usages: &[BufferUsage::TransferDestination],
        host_access: HostAccess::ReadWrite,
    })?;
    let sentinel = vec![91; 824328];
    readback.write(0, &sentinel)?;
    let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    e.transition_resources(&[
        transition(Resource::Buffer(upload.clone()), None, A::TransferRead),
        transition(Resource::Texture(input.clone()), None, A::TransferWrite),
        transition(Resource::Texture(atlas.clone()), None, A::TransferWrite),
        transition(
            Resource::Buffer(geometry_upload.clone()),
            None,
            A::TransferRead,
        ),
        transition(Resource::Buffer(geometry.clone()), None, A::TransferWrite),
        transition(Resource::Buffer(transforms.clone()), None, A::UniformRead),
        transition(
            Resource::Framebuffer(offscreen.as_framebuffer()),
            None,
            A::ColourReadWrite,
        ),
        transition(
            Resource::Texture(final_image.clone()),
            None,
            A::ColourReadWrite,
        ),
        transition(Resource::Buffer(readback.clone()), None, A::TransferWrite),
    ])?;
    e.copy_buffer_to_texture(
        &upload,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: 36,
        },
        &input.view(),
    )?;
    e.transition_resources(&[transition(
        Resource::Texture(input.clone()),
        Some(A::TransferWrite),
        A::TransferRead,
    )])?;
    e.copy_texture(&input.view(), &atlas_view)?;
    e.copy_buffer(&geometry_upload, 0, &geometry, 0, 96)?;
    e.transition_resources(&[
        transition(
            Resource::Texture(atlas.clone()),
            Some(A::TransferWrite),
            A::SampledRead,
        ),
        transition(
            Resource::Buffer(geometry.clone()),
            Some(A::TransferWrite),
            A::StorageReadWrite,
        ),
    ])?;
    e.dispatch(
        &compute,
        [1, 1, 1],
        &[StorageBufferBinding {
            binding: 0,
            buffer: &geometry,
            offset: 0,
            size: 96,
        }],
    )?;
    e.transition_resources(&[transition(
        Resource::Buffer(geometry.clone()),
        Some(A::StorageReadWrite),
        A::VertexRead,
    )])?;
    let mut pass = e.begin_render_pass(&offscreen.as_framebuffer(), Some([0; 4]))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&geometry, 0)?;
    pass.set_bind_group(&first)?;
    pass.draw_programmable(0..6)?;
    pass.finish()?;
    // Declare through the texture alias after using its framebuffer alias.
    e.transition_resources(&[transition(
        Resource::Texture(offscreen.clone()),
        Some(A::ColourReadWrite),
        A::SampledRead,
    )])?;
    let background = [16, 24, 40, 255];
    let mut pass = e.begin_render_pass(&final_image.as_framebuffer(), Some(background))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&geometry, 0)?;
    pass.set_bind_group(&left)?;
    pass.draw_programmable(0..6)?;
    pass.set_bind_group(&right)?;
    pass.draw_programmable(0..6)?;
    pass.finish()?;
    e.transition_resources(&[transition(
        Resource::Framebuffer(final_image.as_framebuffer()),
        Some(A::ColourReadWrite),
        A::TransferRead,
    )])?;
    e.copy_texture_to_buffer(
        &final_image.view(),
        &readback,
        TextureBufferLayout {
            offset: 4,
            bytes_per_row: 2576,
        },
    )?;
    let commands = e.finish();
    commands.validate_dependencies()?;
    assert_eq!(commands.commands().len(), 13);
    assert_eq!(commands.resource_uses()?.len(), 17);
    // Every device image is HostAccess::None; the upload/readback buffers bridge CPU access.
    assert!(matches!(
        final_image.as_framebuffer().pixel(0, 0),
        Err(FramebufferAccessError::ReadForbidden { .. })
    ));
    for (iteration, colours) in [
        [[240, 200, 90, 255], [30, 60, 100, 255]],
        [[64, 191, 255, 255], [255, 128, 32, 255]],
    ]
    .into_iter()
    .enumerate()
    {
        upload_pattern(&upload, colours)?;
        d.default_queue().submit(&commands)?;
        let mut bytes = vec![0; sentinel.len()];
        readback.read(0, &mut bytes)?;
        assert_eq!(&bytes[..4], &sentinel[..4]);
        let mut result = Vec::with_capacity(640 * 320);
        for y in 0..320 {
            for x in 0..640 {
                let start = 4 + y * 2576 + x * 4;
                let actual = [
                    bytes[start],
                    bytes[start + 1],
                    bytes[start + 2],
                    bytes[start + 3],
                ];
                let expected = if (32..288).contains(&y) && (32..288).contains(&x) {
                    colours[((x - 32) / 32 + (y - 32) / 32) % 2]
                } else if (32..288).contains(&y) && (352..608).contains(&x) {
                    colours[((x - 352) / 32 + (287 - y) / 32) % 2]
                } else {
                    background
                };
                assert_eq!(
                    actual, expected,
                    "pixel ({x},{y}) on submission {iteration}"
                );
                result.push(actual);
            }
            let end = 4 + y * 2576 + 2560;
            assert_eq!(&bytes[end..end + 16], &sentinel[end..end + 16]);
        }
        assert_eq!(&bytes[824324..], &sentinel[824324..]);
        if iteration == 1 {
            png::write_rgba(&output, 640, 320, &result)?;
        }
    }
    println!(
        "EX verified all 204800 pixels across two reusable submissions, including padded readback."
    );
    println!(
        "Compute geometry, upload, image copy, two render passes, and explicit dependencies: {}",
        output.display()
    );
    Ok(())
}
