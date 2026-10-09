//! Shared public workload for the example and maintained completion test.
use ex::*;
use ResourceAccess as A;
use ShaderInstruction as Op;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const WIDTH: u32 = 128;
const HEIGHT: u32 = 64;
const BACKGROUND: [u8; 4] = [16, 24, 40, 255];
const READ_PITCH: usize = WIDTH as usize * 4 + 16;
const READ_SIZE: usize = 4 + READ_PITCH * HEIGHT as usize + 4;

pub struct Proof {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
    pub command_count: usize,
    pub resource_use_count: usize,
}

fn buffer(
    d: &Device,
    label: &str,
    size: u64,
    usages: &[BufferUsage],
    host: HostAccess,
) -> Result<Buffer> {
    Ok(d.create_buffer(BufferDescriptor {
        label,
        size,
        usages,
        host_access: host,
    })?)
}
fn image(d: &Device, label: &str, width: u32, height: u32) -> Result<Texture> {
    Ok(d.create_texture(TextureDescriptor {
        label,
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        host_access: HostAccess::None,
    })?)
}
fn vectors(b: &Buffer, values: &[[f32; 4]]) -> Result<()> {
    Ok(b.write(
        0,
        &values
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )?)
}
fn bytes(b: &Buffer) -> Result<Vec<u8>> {
    let mut out = vec![0; b.size() as usize];
    b.read(0, &mut out)?;
    Ok(out)
}
fn transition(resource: Resource, before: Option<A>, after: A) -> ResourceTransition {
    ResourceTransition {
        resource,
        before,
        after,
    }
}
fn shader(
    d: &Device,
    label: &str,
    stage: ShaderStage,
    input_count: u32,
    instructions: &[Op],
    outputs: &[u32],
) -> Result<Shader> {
    Ok(d.create_shader(ShaderDescriptor {
        label,
        stage,
        input_count,
        instructions,
        outputs,
    })?)
}
fn upload(b: &Buffer, colours: [[u8; 4]; 2]) -> Result<()> {
    let mut data = [77; 288]; // Offset 4, eight 32-byte rows, 36-byte pitch.
    for y in 0..8 {
        for x in 0..8 {
            let at = 4 + y * 36 + x * 4;
            data[at..at + 4].copy_from_slice(&colours[(x + y) % 2]);
        }
    }
    Ok(b.write(0, &data)?)
}
fn transforms(b: &Buffer, alpha: f32) -> Result<()> {
    vectors(
        b,
        &[
            [1.; 4],
            [0., 0., -0.5, 0.],
            [1.; 4], // Front: depth 0.25.
            [1.; 4],
            [0., 0., 0.5, 0.],
            [0., 0., 0., 1.], // Back: black, depth 0.75.
            [0.375, 0.75, 1., 1.],
            [-0.5, 0., 0., 0.],
            [1., 1., 1., alpha],
            [0.375, -0.75, 1., 1.],
            [0.5, 0., 0., 0.],
            [1., 1., 1., alpha],
        ],
    )
}

struct Scene {
    queue: Queue,
    commands: CommandBuffer,
    missing_barrier: CommandBuffer,
    dispatch_index: usize,
    upload: Buffer,
    transforms: Buffer,
    compute_parameters: Buffer,
    marker_source: Buffer,
    marker: Buffer,
    geometry: Buffer,
    depth: DepthBuffer,
    stencil: StencilBuffer,
    offscreen_read: Buffer,
    readback: Buffer,
}

fn build(adapter: &Adapter) -> Result<Scene> {
    assert_eq!(adapter.core_profile(), CoreProfile::Preview1);
    let d = adapter.create_device();
    assert_eq!(d.core_profile(), adapter.core_profile());
    assert!(d.core_profile().limits().binding_slots >= 3);
    let upload = buffer(
        &d,
        "padded checker upload",
        288,
        &[BufferUsage::TransferSource],
        HostAccess::Write,
    )?;
    let input = image(&d, "uploaded image", 8, 8)?;
    let atlas = image(&d, "atlas", 12, 12)?;
    let view = atlas.create_view(TextureViewDescriptor {
        x: 2,
        y: 2,
        width: 8,
        height: 8,
    })?;
    let offscreen = image(&d, "depth and stencil checked image", 32, 32)?;
    let final_image = image(&d, "blended panels", WIDTH, HEIGHT)?;
    let depth = d.create_depth_buffer(DepthBufferDescriptor {
        label: "offscreen depth",
        width: 32,
        height: 32,
    })?;
    let stencil = d.create_stencil_buffer(StencilBufferDescriptor {
        label: "offscreen stencil",
        width: 32,
        height: 32,
    })?;
    let marker_source = buffer(
        &d,
        "live prefix input",
        4,
        &[BufferUsage::TransferSource],
        HostAccess::Write,
    )?;
    let marker = buffer(
        &d,
        "completed prefix",
        4,
        &[BufferUsage::TransferDestination],
        HostAccess::Read,
    )?;
    let seed_upload = buffer(
        &d,
        "quad seeds upload",
        64,
        &[BufferUsage::TransferSource],
        HostAccess::Write,
    )?;
    vectors(
        &seed_upload,
        &[
            [-0.5, 0.5, 0., 0.],
            [0.5, 0.5, 1., 0.],
            [0.5, -0.5, 1., 1.],
            [-0.5, -0.5, 0., 1.],
        ],
    )?;
    let seeds = buffer(
        &d,
        "device seeds",
        64,
        &[BufferUsage::TransferDestination, BufferUsage::Storage],
        HostAccess::None,
    )?;
    let geometry = buffer(
        &d,
        "computed indexed quad",
        64,
        &[BufferUsage::Storage, BufferUsage::Vertex],
        HostAccess::Read,
    )?;
    let index = buffer(
        &d,
        "quad indices",
        12,
        &[BufferUsage::Index],
        HostAccess::Write,
    )?;
    index.write(0, &[0, 0, 1, 0, 2, 0, 0, 0, 2, 0, 3, 0])?;
    let compute_parameters = buffer(
        &d,
        "compute offset factor",
        16,
        &[BufferUsage::Uniform],
        HostAccess::Write,
    )?;
    vectors(&compute_parameters, &[[0.; 4]])?;
    let compute_layout = d.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "seed, factor, vertex output",
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderVisibility::Compute,
                ty: BindingType::StorageBuffer { read_only: true },
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderVisibility::Compute,
                ty: BindingType::UniformBuffer { min_size: 16 },
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderVisibility::Compute,
                ty: BindingType::StorageBuffer { read_only: false },
            },
        ],
    })?;
    let kernel = shader(
        &d,
        "generate quad and conditional late overflow",
        ShaderStage::Compute,
        4,
        &[
            Op::LoadStorage { binding: 0 },
            Op::Constant([2., 2., 1., 1.]),
            Op::Multiply { left: 0, right: 1 },
            Op::Input { location: 0 },
            Op::LoadUniform {
                binding: 1,
                element: 0,
            },
            Op::Multiply { left: 3, right: 4 },
            Op::Add { left: 2, right: 5 },
        ],
        &[6],
    )?;
    let compute = d.create_compute_pipeline_with_bindings(
        ComputePipelineDescriptor {
            label: "indexed geometry",
            shader: &kernel,
            workgroup_size: [2, 1, 1],
            output_bindings: &[2],
        },
        &compute_layout,
    )?;
    let compute_group = d.create_bind_group(BindGroupDescriptor {
        label: "compute data",
        layout: &compute_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::StorageBuffer(BufferBinding {
                    buffer: seeds.clone(),
                    offset: 0,
                    size: 64,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::UniformBuffer(BufferBinding {
                    buffer: compute_parameters.clone(),
                    offset: 0,
                    size: 16,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::StorageBuffer(BufferBinding {
                    buffer: geometry.clone(),
                    offset: 0,
                    size: 64,
                }),
            },
        ],
    })?;

    let layout = d.create_bind_group_layout(BindGroupLayoutDescriptor {
        label: "texture and graphics parameters",
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderVisibility::Fragment,
                ty: BindingType::SampledTexture,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderVisibility::Graphics,
                ty: BindingType::UniformBuffer { min_size: 48 },
            },
        ],
    })?;
    let transforms = buffer(
        &d,
        "four materials",
        192,
        &[BufferUsage::Uniform],
        HostAccess::Write,
    )?;
    let sampler = d.create_sampler(SamplerDescriptor {
        label: "nearest checker",
        filter: FilterMode::Nearest,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
    })?;
    let group = |view: TextureView, offset| {
        d.create_bind_group(BindGroupDescriptor {
            label: "material",
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
                        size: 48,
                    }),
                },
            ],
        })
    };
    let front = group(view.clone(), 0)?;
    let back = group(view.clone(), 48)?;
    let left = group(offscreen.view(), 96)?;
    let right = group(offscreen.view(), 144)?;
    let vertex = shader(
        &d,
        "transform computed positions and pass UV",
        ShaderStage::Vertex,
        2,
        &[
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
        &[4, 5],
    )?;
    let fragment = shader(
        &d,
        "sample and tint including alpha",
        ShaderStage::Fragment,
        1,
        &[
            Op::Input { location: 0 },
            Op::SampleTexture {
                binding: 0,
                coordinates: 0,
            },
            Op::LoadUniform {
                binding: 1,
                element: 2,
            },
            Op::Multiply { left: 1, right: 2 },
        ],
        &[3],
    )?;
    let pipeline = |label, options, stencil| {
        d.create_graphics_pipeline_with_bindings(
            GraphicsPipelineDescriptor {
                label,
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
            options,
            stencil,
            &layout,
        )
    };
    let front_pipeline = pipeline(
        "mark visible front",
        GraphicsPipelineOptions {
            depth: Some(DepthState {
                compare: CompareFunction::Less,
                write_enabled: true,
            }),
            blend: None,
        },
        Some(StencilState::replace(3)),
    )?;
    let back_pipeline = pipeline(
        "reject farther black quad",
        GraphicsPipelineOptions {
            depth: Some(DepthState {
                compare: CompareFunction::Less,
                write_enabled: false,
            }),
            blend: None,
        },
        Some(StencilState::test_equal(3)),
    )?;
    let blocked_pipeline = pipeline(
        "reject stencil mismatch",
        GraphicsPipelineOptions::default(),
        Some(StencilState::test_equal(9)),
    )?;
    let panel_pipeline = pipeline(
        "blend sampled panels",
        GraphicsPipelineOptions {
            depth: None,
            blend: Some(BlendState::alpha_blending()),
        },
        None,
    )?;
    let offscreen_read = buffer(
        &d,
        "padded offscreen readback",
        4 + 144 * 32 + 4,
        &[BufferUsage::TransferDestination],
        HostAccess::ReadWrite,
    )?;
    let readback = buffer(
        &d,
        "padded final readback",
        READ_SIZE as u64,
        &[BufferUsage::TransferDestination],
        HostAccess::ReadWrite,
    )?;
    offscreen_read.write(0, &vec![91; offscreen_read.size() as usize])?;
    readback.write(0, &vec![91; READ_SIZE])?;

    let record = |omit_barrier: bool| -> Result<(CommandBuffer, usize)> {
        let mut e = d.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
        e.transition_resources(&[
            transition(
                Resource::Buffer(marker_source.clone()),
                None,
                A::TransferRead,
            ),
            transition(Resource::Buffer(marker.clone()), None, A::TransferWrite),
            transition(Resource::Buffer(upload.clone()), None, A::TransferRead),
            transition(Resource::Texture(input.clone()), None, A::TransferWrite),
            transition(Resource::Texture(atlas.clone()), None, A::TransferWrite),
            transition(Resource::Buffer(seed_upload.clone()), None, A::TransferRead),
            transition(Resource::Buffer(seeds.clone()), None, A::TransferWrite),
            transition(Resource::Buffer(geometry.clone()), None, A::StorageWrite),
            transition(
                Resource::Buffer(compute_parameters.clone()),
                None,
                A::UniformRead,
            ),
            transition(Resource::Buffer(transforms.clone()), None, A::UniformRead),
            transition(Resource::Buffer(index.clone()), None, A::IndexRead),
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
            transition(
                Resource::DepthBuffer(depth.clone()),
                None,
                A::DepthReadWrite,
            ),
            transition(
                Resource::StencilBuffer(stencil.clone()),
                None,
                A::StencilReadWrite,
            ),
            transition(
                Resource::Buffer(offscreen_read.clone()),
                None,
                A::TransferWrite,
            ),
            transition(Resource::Buffer(readback.clone()), None, A::TransferWrite),
        ])?;
        e.copy_buffer(&marker_source, 0, &marker, 0, 4)?;
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
        e.copy_texture(&input.view(), &view)?;
        e.copy_buffer(&seed_upload, 0, &seeds, 0, 64)?;
        e.transition_resources(&[
            transition(
                Resource::Texture(atlas.clone()),
                Some(A::TransferWrite),
                A::SampledRead,
            ),
            transition(
                Resource::Buffer(seeds.clone()),
                Some(A::TransferWrite),
                A::StorageRead,
            ),
        ])?;
        let dispatch_index = e.commands().len();
        e.dispatch_with_bind_group(&compute, [2, 1, 1], &compute_group)?;
        if !omit_barrier {
            e.transition_resources(&[transition(
                Resource::Buffer(geometry.clone()),
                Some(A::StorageWrite),
                A::VertexRead,
            )])?;
        }
        let mut pass = e.begin_render_pass_with_attachments(
            &offscreen.as_framebuffer(),
            Some([0; 4]),
            RenderAttachments {
                depth: Some(&depth),
                clear_depth: Some(1.),
                stencil: Some(&stencil),
                clear_stencil: Some(0),
            },
        )?;
        pass.set_vertex_buffer(&geometry, 0)?;
        pass.set_index_buffer(&index, 0, IndexFormat::Uint16)?;
        for (pipeline, material) in [
            (&front_pipeline, &front),
            (&back_pipeline, &back),
            (&blocked_pipeline, &back),
        ] {
            pass.set_pipeline(pipeline)?;
            pass.set_bind_group(material)?;
            pass.draw_indexed(0..6, 0)?;
        }
        pass.finish()?;
        e.transition_resources(&[transition(
            Resource::Texture(offscreen.clone()),
            Some(A::ColourReadWrite),
            A::SampledRead,
        )])?;
        let mut pass = e.begin_render_pass(&final_image.as_framebuffer(), Some(BACKGROUND))?;
        pass.set_pipeline(&panel_pipeline)?;
        pass.set_vertex_buffer(&geometry, 0)?;
        pass.set_index_buffer(&index, 0, IndexFormat::Uint16)?;
        for material in [&left, &right] {
            pass.set_bind_group(material)?;
            pass.draw_indexed(0..6, 0)?;
        }
        pass.finish()?;
        e.transition_resources(&[
            transition(
                Resource::Framebuffer(offscreen.as_framebuffer()),
                Some(A::SampledRead),
                A::TransferRead,
            ),
            transition(
                Resource::Texture(final_image.clone()),
                Some(A::ColourReadWrite),
                A::TransferRead,
            ),
        ])?;
        e.copy_texture_to_buffer(
            &offscreen.view(),
            &offscreen_read,
            TextureBufferLayout {
                offset: 4,
                bytes_per_row: 144,
            },
        )?;
        e.copy_texture_to_buffer(
            &final_image.view(),
            &readback,
            TextureBufferLayout {
                offset: 4,
                bytes_per_row: READ_PITCH as u64,
            },
        )?;
        Ok((e.finish(), dispatch_index))
    };
    let (commands, dispatch_index) = record(false)?;
    commands.validate_dependencies()?;
    let (missing_barrier, _) = record(true)?;
    // Both aliases keep the CPU permission of their original image.
    assert!(matches!(
        final_image.as_framebuffer().pixel(0, 0),
        Err(FramebufferAccessError::ReadForbidden { .. })
    ));
    Ok(Scene {
        queue: d.default_queue(),
        commands,
        missing_barrier,
        dispatch_index,
        upload,
        transforms,
        compute_parameters,
        marker_source,
        marker,
        geometry,
        depth,
        stencil,
        offscreen_read,
        readback,
    })
    // Local device, image, pipeline, group and seed handles leave scope here.
}

fn verify_rows(
    b: &Buffer,
    width: usize,
    height: usize,
    pitch: usize,
    expected: impl Fn(usize, usize) -> [u8; 4],
) -> Result<Vec<[u8; 4]>> {
    let data = bytes(b)?;
    assert_eq!(&data[..4], &[91; 4]);
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let at = 4 + y * pitch + x * 4;
            let actual = [data[at], data[at + 1], data[at + 2], data[at + 3]];
            assert_eq!(actual, expected(x, y), "pixel ({x},{y})");
            pixels.push(actual);
        }
        assert!(
            data[4 + y * pitch + width * 4..4 + (y + 1) * pitch]
                .iter()
                .all(|&v| v == 91),
            "row padding {y}"
        );
    }
    assert_eq!(&data[4 + height * pitch..], &[91; 4]);
    Ok(pixels)
}
fn attachment_snapshot(s: &Scene) -> Result<(Vec<f32>, Vec<u8>)> {
    let mut depth = Vec::with_capacity(1024);
    let mut stencil = Vec::with_capacity(1024);
    for y in 0..32 {
        for x in 0..32 {
            depth.push(s.depth.depth(x, y)?);
            stencil.push(s.stencil.stencil(x, y)?);
        }
    }
    Ok((depth, stencil))
}
fn snapshots(s: &Scene) -> Result<Vec<Vec<u8>>> {
    [&s.marker, &s.geometry, &s.offscreen_read, &s.readback]
        .into_iter()
        .map(bytes)
        .collect()
}

/// Verify two successful frames, preflight rejection, late failure and repair.
/// Assertions are intentional workload oracles, not exported production APIs.
pub fn run(adapter: &Adapter) -> Result<Proof> {
    let s = build(adapter)?;
    let original = snapshots(&s)?;
    let attachments = attachment_snapshot(&s)?;
    s.marker_source.write(0, &[9; 4])?;
    assert!(matches!(
        s.queue.submit(&s.missing_barrier),
        Err(QueueSubmitError::DependencyFailed {
            source: DependencyValidationError::MissingDependency {
                command_index: 8,
                previous_command_index: 7,
                kind: ResourceKind::Buffer,
                hazard: HazardKind::ReadAfterWrite,
                previous: A::StorageWrite,
                required: A::VertexRead,
                ..
            }
        })
    ));
    assert_eq!(snapshots(&s)?, original); // Even the first marker copy did not execute.
    assert_eq!(attachment_snapshot(&s)?, attachments);
    let palettes = [
        [[64, 192, 240, 255], [240, 128, 32, 255]],
        [[48, 160, 224, 255], [224, 96, 48, 255]],
    ];
    let mut final_pixels = Vec::new();
    for (frame, colours) in palettes.into_iter().enumerate() {
        let denominator = if frame == 0 { 2 } else { 4 };
        upload(&s.upload, colours)?;
        transforms(&s.transforms, 1. / denominator as f32)?;
        s.marker_source.write(0, &[frame as u8 + 1; 4])?;
        s.queue.clone().submit(&s.commands)?;
        assert_eq!(bytes(&s.marker)?, vec![frame as u8 + 1; 4]);
        let geometry: Vec<_> = [
            [-1., 1., 0., 0.],
            [1., 1., 1., 0.],
            [1., -1., 1., 1.],
            [-1., -1., 0., 1.],
        ]
        .into_iter()
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect();
        assert_eq!(bytes(&s.geometry)?, geometry);
        verify_rows(&s.offscreen_read, 32, 32, 144, |x, y| {
            colours[(x / 4 + y / 4) % 2]
        })?;
        assert_eq!(attachment_snapshot(&s)?, (vec![0.25; 1024], vec![3; 1024]));
        final_pixels = verify_rows(
            &s.readback,
            WIDTH as usize,
            HEIGHT as usize,
            READ_PITCH,
            |x, y| {
                let checker = if (8..56).contains(&y) && (8..56).contains(&x) {
                    Some(((x - 8) / 6 + (y - 8) / 6) % 2)
                } else if (8..56).contains(&y) && (72..120).contains(&x) {
                    Some(((x - 72) / 6 + (55 - y) / 6) % 2)
                } else {
                    None
                };
                checker.map_or(BACKGROUND, |i| {
                    std::array::from_fn(|lane| {
                        if lane == 3 {
                            255
                        } else {
                            ((u32::from(colours[i][lane])
                                + (denominator - 1) * u32::from(BACKGROUND[lane]))
                                / denominator) as u8
                        }
                    })
                })
            },
        )?;
        if frame == 0 {
            let before = snapshots(&s)?;
            let attachments = attachment_snapshot(&s)?;
            s.marker_source.write(0, &[7; 4])?;
            // x=0 and x=1 are finite; x=2 overflows. No output may publish.
            vectors(&s.compute_parameters, &[[f32::MAX, 0., 0., 0.]])?;
            let error = s.queue.submit(&s.commands);
            assert!(matches!(
                error,
                Err(QueueSubmitError::ComputeFailed {
                    command_index,
                    source: ComputeExecutionError::ShaderFailed {
                        invocation: [2, 0, 0], ..
                    },
                }) if command_index == s.dispatch_index
            ));
            let after = snapshots(&s)?;
            assert_eq!(after[0], vec![7; 4]); // Completed prefix survives.
            assert_eq!(&after[1..], &before[1..]); // Failed output and later readbacks remain.
            assert_eq!(attachment_snapshot(&s)?, attachments); // Later clears were skipped.
            vectors(&s.compute_parameters, &[[0.; 4]])?;
        }
    }
    Ok(Proof {
        width: WIDTH,
        height: HEIGHT,
        pixels: final_pixels,
        command_count: s.commands.commands().len(),
        resource_use_count: s.commands.resource_uses()?.len(),
    })
}
