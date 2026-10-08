use ex::{
    Buffer, BufferDescriptor, BufferUsage, Device, Framebuffer, FramebufferDescriptor,
    FramebufferFormat, GraphicsPipeline, GraphicsPipelineDescriptor, HostAccess, IndexFormat,
    Instance, QueueSubmitError, RenderEncodeError, RenderExecutionError, ShaderDescriptor,
    ShaderInstruction as Op, ShaderStage, VertexAttribute, VertexBufferLayout, VertexFormat,
};

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn image(device: &Device) -> Framebuffer {
    device
        .create_framebuffer(FramebufferDescriptor {
            label: "indexed",
            width: 4,
            height: 4,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap()
}
fn pipeline(device: &Device) -> GraphicsPipeline {
    let vertex = device
        .create_shader(ShaderDescriptor {
            label: "vertex",
            stage: ShaderStage::Vertex,
            input_count: 2,
            instructions: &[Op::Input { location: 0 }, Op::Input { location: 1 }],
            outputs: &[0, 1],
        })
        .unwrap();
    let fragment = device
        .create_shader(ShaderDescriptor {
            label: "fragment",
            stage: ShaderStage::Fragment,
            input_count: 1,
            instructions: &[Op::Input { location: 0 }],
            outputs: &[0],
        })
        .unwrap();
    device
        .create_graphics_pipeline(GraphicsPipelineDescriptor {
            label: "indexed pipeline",
            vertex_shader: &vertex,
            fragment_shader: &fragment,
            vertex_layout: VertexBufferLayout {
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
            },
            colour_format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap()
}
fn quad() -> [[f32; 8]; 4] {
    [
        [-1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        [1.0, -1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
        [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0],
    ]
}
fn vertex_bytes(vertices: &[[f32; 8]]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| v.iter().flat_map(|n| n.to_le_bytes()))
        .collect()
}
fn index_bytes(indices: &[u32], format: IndexFormat) -> Vec<u8> {
    indices
        .iter()
        .flat_map(|&n| match format {
            IndexFormat::Uint16 => (n as u16).to_le_bytes().to_vec(),
            IndexFormat::Uint32 => n.to_le_bytes().to_vec(),
        })
        .collect()
}
fn buffer(device: &Device, bytes: &[u8], usages: &[BufferUsage]) -> Buffer {
    let buffer = device
        .create_buffer(BufferDescriptor {
            label: "test source",
            size: bytes.len() as u64,
            usages,
            host_access: HostAccess::Write,
        })
        .unwrap();
    buffer.write(0, bytes).unwrap();
    buffer
}
fn submit(
    device: &Device,
    target: &Framebuffer,
    pipeline: &GraphicsPipeline,
    vertices: &Buffer,
    indices: &Buffer,
    format: IndexFormat,
    base: i32,
) -> Result<(), QueueSubmitError> {
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(target, Some([0; 4])).unwrap();
    pass.set_pipeline(pipeline).unwrap();
    pass.set_vertex_buffer(vertices, 0).unwrap();
    pass.set_index_buffer(indices, 0, format).unwrap();
    pass.draw_indexed(0..6, base).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish())
}
fn pixels(image: &Framebuffer) -> [[u8; 4]; 16] {
    let mut pixels = [[0; 4]; 16];
    image.read_pixels(&mut pixels).unwrap();
    pixels
}

#[test]
fn both_index_formats_match_expanded_direct_vertices_and_share_edges() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    let selection = [0, 1, 2, 2, 3, 0];
    let expanded: Vec<_> = selection.iter().map(|&n| quad()[n as usize]).collect();
    let direct = buffer(&device, &vertex_bytes(&expanded), &[BufferUsage::Vertex]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&direct, 0).unwrap();
    pass.draw_programmable(0..6).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    let expected = pixels(&target);
    assert!(expected.iter().all(|p| p[3] == 255));
    assert_eq!(expected[0], [223, 0, 32, 255]);
    for format in [IndexFormat::Uint16, IndexFormat::Uint32] {
        assert_eq!(
            format.size(),
            if format == IndexFormat::Uint16 { 2 } else { 4 }
        );
        let indices = buffer(
            &device,
            &index_bytes(&selection, format),
            &[BufferUsage::Index],
        );
        submit(&device, &target, &p, &vertices, &indices, format, 0).unwrap();
        assert_eq!(pixels(&target), expected);
    }
}

#[test]
fn binding_offsets_first_index_and_positive_base_vertex_are_relative() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let mut data = vec![0; 4 + 64];
    data.extend(vertex_bytes(&quad()));
    let vertices = buffer(&device, &data, &[BufferUsage::Vertex]);
    let mut data = vec![0; 4];
    data.extend(index_bytes(
        &[99, 99, 99, 0, 1, 2, 2, 3, 0],
        IndexFormat::Uint16,
    ));
    let indices = buffer(&device, &data, &[BufferUsage::Index]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 4).unwrap();
    pass.set_index_buffer(&indices, 4, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(3..9, 2).unwrap();
    pass.finish().unwrap();
    let ex::Command::RenderPass(pass) = &encoder.commands()[0] else {
        panic!("render expected")
    };
    let draw = &pass.draws()[0];
    let indexed = draw.indexed().unwrap();
    assert_eq!(draw.vertices(), 0..0);
    assert_eq!(draw.colour(), [0; 4]);
    assert!(draw.pipeline().unwrap().is_same(&p));
    assert!(indexed.buffer().is_same(&indices));
    assert_eq!(indexed.offset(), 4);
    assert_eq!(indexed.format(), IndexFormat::Uint16);
    assert_eq!(indexed.indices(), 3..9);
    assert_eq!(indexed.base_vertex(), 2);
    assert_eq!(pixels(&target), [[0; 4]; 16]);
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), [223, 0, 32, 255]);
}

#[test]
fn negative_base_and_maximum_u16_values_are_ordinary_indices() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    let indices = buffer(
        &device,
        &index_bytes(
            &[65532, 65533, 65534, 65534, 65535, 65532],
            IndexFormat::Uint16,
        ),
        &[BufferUsage::Index],
    );
    submit(
        &device,
        &target,
        &p,
        &vertices,
        &indices,
        IndexFormat::Uint16,
        -65532,
    )
    .unwrap();
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
}

#[test]
fn recording_rejects_missing_state_bad_bindings_and_ranges_without_losing_prior_binding() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    let indices = buffer(
        &device,
        &index_bytes(&[0, 1, 2, 2, 3, 0], IndexFormat::Uint16),
        &[BufferUsage::Index],
    );
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    assert!(matches!(
        pass.draw_indexed(0..6, 0),
        Err(RenderEncodeError::MissingPipeline)
    ));
    pass.set_pipeline(&p).unwrap();
    assert!(matches!(
        pass.draw_indexed(0..6, 0),
        Err(RenderEncodeError::MissingVertexBuffer)
    ));
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    assert!(matches!(
        pass.draw_indexed(0..6, 0),
        Err(RenderEncodeError::MissingIndexBuffer)
    ));
    pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)
        .unwrap();
    let foreign = buffer(
        &device.adapter().create_device(),
        &[0; 12],
        &[BufferUsage::Index],
    );
    assert!(matches!(
        pass.set_index_buffer(&foreign, 0, IndexFormat::Uint16),
        Err(RenderEncodeError::ForeignIndexBuffer { .. })
    ));
    assert!(matches!(
        pass.set_index_buffer(&vertices, 0, IndexFormat::Uint16),
        Err(RenderEncodeError::MissingIndexUsage { .. })
    ));
    assert!(matches!(
        pass.set_index_buffer(&indices, 13, IndexFormat::Uint16),
        Err(RenderEncodeError::IndexOffsetOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.set_index_buffer(&indices, 1, IndexFormat::Uint16),
        Err(RenderEncodeError::UnalignedIndexOffset { alignment: 2, .. })
    ));
    assert!(matches!(
        pass.set_index_buffer(&indices, 2, IndexFormat::Uint32),
        Err(RenderEncodeError::UnalignedIndexOffset { alignment: 4, .. })
    ));
    assert!(matches!(
        pass.draw_indexed(std::ops::Range { start: 3, end: 0 }, 0),
        Err(RenderEncodeError::InvalidIndexRange { .. })
    ));
    assert!(matches!(
        pass.draw_indexed(0..4, 0),
        Err(RenderEncodeError::IncompleteTriangle { vertex_count: 4 })
    ));
    assert!(matches!(
        pass.draw_indexed(3..9, 0),
        Err(RenderEncodeError::IndexRangeOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.draw_indexed(7..7, 0),
        Err(RenderEncodeError::IndexRangeOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.draw_indexed(u32::MAX..u32::MAX, 0),
        Err(RenderEncodeError::IndexRangeOutOfBounds { .. })
    ));
    pass.draw_indexed(6..6, i32::MIN).unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    assert!(matches!(
        pass.draw_indexed(0..0, 0),
        Err(RenderEncodeError::MissingIndexBuffer)
    ));
    pass.set_index_buffer(&indices, 12, IndexFormat::Uint32)
        .unwrap();
    pass.draw_indexed(0..0, i32::MAX).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
}

#[test]
fn invalid_live_reference_preserves_scope_and_completed_submission_prefix() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let mut bytes = vertex_bytes(&quad()[..3]);
    bytes.extend_from_slice(&[0; 4]);
    let vertices = buffer(&device, &bytes, &[BufferUsage::Vertex]);
    let indices = buffer(
        &device,
        &index_bytes(&[0, 0, 0, 0, 3, 2], IndexFormat::Uint32),
        &[BufferUsage::Index],
    );
    let mut encoder = device.create_command_encoder();
    encoder
        .begin_render_pass(&target, Some([7; 4]))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = encoder.begin_render_pass(&target, Some([99; 4])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.set_index_buffer(&indices, 0, IndexFormat::Uint32)
        .unwrap();
    pass.draw_indexed(3..6, 0).unwrap();
    pass.finish().unwrap();
    encoder
        .begin_render_pass(&target, Some([88; 4]))
        .unwrap()
        .finish()
        .unwrap();
    assert!(matches!(
        device.default_queue().submit(&encoder.finish()),
        Err(QueueSubmitError::RenderFailed {
            command_index: 1,
            source: RenderExecutionError::InvalidVertexIndex {
                index_position: 4,
                index_value: 3,
                base_vertex: 0,
                available_vertices: 3,
                ..
            }
        })
    ));
    assert_eq!(pixels(&target), [[7; 4]; 16]);
}

#[test]
fn signed_index_arithmetic_rejects_underflow_overflow_and_capacity_errors() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    for (value, base) in [
        (0, -1),
        (u32::MAX, i32::MAX),
        (u32::MAX, i32::MIN),
        (4, 0),
        (3, 1),
    ] {
        let indices = buffer(
            &device,
            &index_bytes(&[value; 6], IndexFormat::Uint32),
            &[BufferUsage::Index],
        );
        assert!(matches!(
            submit(
                &device,
                &target,
                &p,
                &vertices,
                &indices,
                IndexFormat::Uint32,
                base
            ),
            Err(QueueSubmitError::RenderFailed {
                source: RenderExecutionError::InvalidVertexIndex {
                    index_position: 0,
                    ..
                },
                ..
            })
        ));
        assert_eq!(pixels(&target), [[0; 4]; 16]);
    }
}

#[test]
fn one_allocation_can_supply_vertices_and_indices() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let mut bytes = vertex_bytes(&quad());
    bytes.extend(index_bytes(&[0, 1, 2, 2, 3, 0], IndexFormat::Uint16));
    let shared = buffer(&device, &bytes, &[BufferUsage::Vertex, BufferUsage::Index]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&shared, 0).unwrap();
    pass.set_index_buffer(&shared, 128, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
}

#[test]
fn only_referenced_vertices_are_evaluated_and_shader_errors_use_effective_index() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let mut data = vec![[f32::NAN; 8]];
    data.extend(quad());
    let vertices = buffer(&device, &vertex_bytes(&data), &[BufferUsage::Vertex]);
    let indices = buffer(
        &device,
        &index_bytes(&[1, 2, 3, 3, 4, 1], IndexFormat::Uint32),
        &[BufferUsage::Index],
    );
    submit(
        &device,
        &target,
        &p,
        &vertices,
        &indices,
        IndexFormat::Uint32,
        0,
    )
    .unwrap();
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
    vertices.write(32 * 3, &f32::NAN.to_le_bytes()).unwrap();
    assert!(matches!(
        submit(
            &device,
            &target,
            &p,
            &vertices,
            &indices,
            IndexFormat::Uint32,
            0
        ),
        Err(QueueSubmitError::RenderFailed {
            source: RenderExecutionError::NonFiniteShaderInput {
                stage: ShaderStage::Vertex,
                invocation: 3,
                location: 0,
                ..
            },
            ..
        })
    ));
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
}

#[test]
fn copies_late_writes_reuse_and_dropped_original_handles_keep_live_index_sources() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    let upload = buffer(
        &device,
        &index_bytes(&[0, 1, 9, 2, 3, 0], IndexFormat::Uint16),
        &[BufferUsage::TransferSource],
    );
    let indices = device
        .create_buffer(BufferDescriptor {
            label: "device indices",
            size: 12,
            usages: &[BufferUsage::Index, BufferUsage::TransferDestination],
            host_access: HostAccess::None,
        })
        .unwrap();
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &indices, 0, 12).unwrap();
    let mut pass = encoder.begin_render_pass(&target, Some([0; 4])).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_index_buffer(&indices, 0, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(0..6, 0).unwrap();
    pass.finish().unwrap();
    let commands = encoder.finish();
    drop(p);
    drop(vertices);
    drop(indices);
    assert!(matches!(
        device.default_queue().submit(&commands),
        Err(QueueSubmitError::RenderFailed {
            command_index: 1,
            source: RenderExecutionError::InvalidVertexIndex {
                index_position: 2,
                index_value: 9,
                ..
            }
        })
    ));
    upload
        .write(0, &index_bytes(&[0, 1, 2, 2, 3, 0], IndexFormat::Uint16))
        .unwrap();
    device.default_queue().submit(&commands).unwrap();
    assert!(pixels(&target).iter().all(|p| p[3] == 255));
    upload
        .write(0, &index_bytes(&[0, 1, 2, 0, 1, 2], IndexFormat::Uint16))
        .unwrap();
    drop(upload);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(target.pixel(0, 3).unwrap(), [0; 4]);
    assert_ne!(target.pixel(3, 0).unwrap(), [0; 4]);
}

#[test]
fn index_bindings_are_retained_per_draw_and_mixed_draws_execute_in_order() {
    let device = device();
    let target = image(&device);
    let p = pipeline(&device);
    let vertices = buffer(&device, &vertex_bytes(&quad()), &[BufferUsage::Vertex]);
    let first = buffer(
        &device,
        &index_bytes(&[0, 1, 2], IndexFormat::Uint16),
        &[BufferUsage::Index],
    );
    let second = buffer(
        &device,
        &index_bytes(&[2, 3, 0], IndexFormat::Uint32),
        &[BufferUsage::Index],
    );
    let fixed_bytes: Vec<_> = [-1.0_f32, 1.0, 1.0, 1.0, -1.0, -1.0]
        .iter()
        .flat_map(|n| n.to_le_bytes())
        .collect();
    let fixed = buffer(&device, &fixed_bytes, &[BufferUsage::Vertex]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_pipeline(&p).unwrap();
    pass.set_vertex_buffer(&vertices, 0).unwrap();
    pass.set_index_buffer(&first, 0, IndexFormat::Uint16)
        .unwrap();
    pass.draw_indexed(0..3, 0).unwrap();
    pass.set_index_buffer(&second, 0, IndexFormat::Uint32)
        .unwrap();
    pass.draw_indexed(0..3, 0).unwrap();
    pass.draw_programmable(0..3).unwrap();
    pass.set_vertex_buffer(&fixed, 0).unwrap();
    pass.draw(0..3, [44; 4]).unwrap();
    pass.finish().unwrap();
    let ex::Command::RenderPass(pass) = &encoder.commands()[0] else {
        panic!("render expected")
    };
    assert!(pass.draws()[0].indexed().unwrap().buffer().is_same(&first));
    assert_eq!(
        pass.draws()[1].indexed().unwrap().format(),
        IndexFormat::Uint32
    );
    assert!(pass.draws()[2].indexed().is_none());
    assert!(pass.draws()[3].indexed().is_none());
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), [44; 4]);
    assert_ne!(target.pixel(0, 3).unwrap(), [0; 4]);
}
