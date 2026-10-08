use ex::{
    Buffer, BufferDescriptor, BufferUsage, Command, Device, Framebuffer, FramebufferAccessError,
    FramebufferCreateError, FramebufferDescriptor, FramebufferDescriptorError, FramebufferFormat,
    HostAccess, Instance, QueueSubmitError, RenderEncodeError, RenderExecutionError,
};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BACKGROUND: [u8; 4] = [9, 17, 31, 255];

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}
fn target(device: &Device, width: u32, height: u32) -> Framebuffer {
    device
        .create_framebuffer(FramebufferDescriptor {
            label: "target",
            width,
            height,
            format: FramebufferFormat::Rgba8Unorm,
        })
        .unwrap()
}
fn encoded(points: &[[f32; 2]]) -> Vec<u8> {
    points
        .iter()
        .flat_map(|point| point.iter().flat_map(|value| value.to_le_bytes()))
        .collect()
}
fn vertices(device: &Device, points: &[[f32; 2]]) -> Buffer {
    let bytes = encoded(points);
    let buffer = device
        .create_buffer(BufferDescriptor {
            label: "vertices",
            size: bytes.len().max(8) as u64,
            usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
            host_access: HostAccess::Write,
        })
        .unwrap();
    buffer.write(0, &bytes).unwrap();
    buffer
}
fn pixels(target: &Framebuffer) -> Vec<[u8; 4]> {
    let mut pixels = vec![[0; 4]; target.width() as usize * target.height() as usize];
    target.read_pixels(&mut pixels).unwrap();
    pixels
}
fn render(device: &Device, target: &Framebuffer, buffer: &Buffer, count: u32) {
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(target, Some(BACKGROUND)).unwrap();
    pass.set_vertex_buffer(buffer, 0).unwrap();
    pass.draw(0..count, RED).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
}

#[test]
fn framebuffer_owns_metadata_zeroes_pixels_and_shares_identity() {
    let device = device();
    let framebuffer = {
        let label = String::from("owned label");
        device
            .create_framebuffer(FramebufferDescriptor {
                label: &label,
                width: 3,
                height: 2,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap()
    };
    assert_eq!(framebuffer.label(), "owned label");
    assert_eq!(framebuffer.format(), FramebufferFormat::Rgba8Unorm);
    assert!(framebuffer.device().is_same(&device));
    assert!(framebuffer.is_same(&framebuffer.clone()));
    assert!(!framebuffer.is_same(&target(&device, 3, 2)));
    assert_eq!(pixels(&framebuffer), vec![[0; 4]; 6]);
    assert!(format!("{framebuffer:?}").contains("owned label"));
}

#[test]
fn extents_are_validated_before_allocation_and_errors_retain_sources() {
    let device = device();
    for (width, height) in [(0, 1), (1, 0), (16385, 1), (1, u32::MAX)] {
        let error = device
            .create_framebuffer(FramebufferDescriptor {
                label: "invalid",
                width,
                height,
                format: FramebufferFormat::Rgba8Unorm,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            FramebufferCreateError::InvalidDescriptor(_)
        ));
        assert!(std::error::Error::source(&error).is_some());
    }
    assert!(matches!(
        FramebufferDescriptor {
            label: "zero",
            width: 0,
            height: 0,
            format: FramebufferFormat::Rgba8Unorm,
        }
        .validate(),
        Err(FramebufferDescriptorError::ZeroExtent { .. })
    ));
    // Check the supported limit without allocating a 1 GiB framebuffer.
    FramebufferDescriptor {
        label: "max",
        width: 16384,
        height: 16384,
        format: FramebufferFormat::Rgba8Unorm,
    }
    .validate()
    .unwrap();
}

#[test]
fn readback_and_pixel_errors_leave_caller_storage_unchanged() {
    let framebuffer = target(&device(), 4, 3);
    let mut wrong = [[99; 4]; 11];
    assert_eq!(
        framebuffer.read_pixels(&mut wrong),
        Err(FramebufferAccessError::WrongPixelCount {
            expected: 12,
            actual: 11
        })
    );
    assert_eq!(wrong, [[99; 4]; 11]);
    for (x, y) in [(4, 0), (0, 3), (u32::MAX, u32::MAX)] {
        assert!(matches!(
            framebuffer.pixel(x, y),
            Err(FramebufferAccessError::OutOfBounds { .. })
        ));
    }
}

#[test]
fn recording_and_unfinished_passes_never_change_pixels_or_append_work() {
    let device = device();
    let target = target(&device, 4, 4);
    let mut encoder = device.create_command_encoder();
    {
        let _discarded = encoder.begin_render_pass(&target, Some(RED)).unwrap();
    }
    assert!(encoder.commands().is_empty());
    let pass = encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap();
    pass.finish().unwrap();
    assert_eq!(pixels(&target), vec![[0; 4]; 16]);
    let commands = encoder.finish();
    assert_eq!(commands.commands().len(), 1);
    match &commands.commands()[0] {
        Command::RenderPass(render) => {
            assert!(render.target().is_same(&target));
            assert_eq!(render.clear_colour(), Some(BACKGROUND));
            assert!(render.draws().is_empty());
        }
        _ => panic!("expected a render scope"),
    }
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(pixels(&target), vec![BACKGROUND; 16]);
}

#[test]
fn no_clear_preserves_pixels_and_draws_replace_all_channels() {
    let device = device();
    let target = target(&device, 4, 4);
    let buffer = vertices(&device, &[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    let mut encoder = device.create_command_encoder();
    encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap()
        .finish()
        .unwrap();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_vertex_buffer(&buffer, 0).unwrap();
    pass.draw(0..3, [80, 90, 100, 0]).unwrap();
    pass.finish().unwrap();
    encoder
        .begin_render_pass(&target, None)
        .unwrap()
        .finish()
        .unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), [80, 90, 100, 0]);
    assert_eq!(target.pixel(3, 3).unwrap(), BACKGROUND);
}

#[test]
fn winding_does_not_change_coverage_and_degenerate_triangles_draw_nothing() {
    let device = device();
    let first = target(&device, 4, 4);
    let second = target(&device, 4, 4);
    let forward = vertices(&device, &[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    let reverse = vertices(&device, &[[-1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]);
    render(&device, &first, &forward, 3);
    render(&device, &second, &reverse, 3);
    assert_eq!(pixels(&first), pixels(&second));
    assert_eq!(pixels(&first).iter().filter(|&&p| p == RED).count(), 6);
    let degenerate = vertices(
        &device,
        &[
            [0.0, 0.0],
            [0.25, 0.25],
            [0.5, 0.5],
            [0.0, 0.0],
            [0.000001, 0.0],
            [0.0, 0.000001],
        ],
    );
    render(&device, &second, &degenerate, 6);
    assert_eq!(pixels(&second), vec![BACKGROUND; 16]);
}

#[test]
fn shared_edges_have_one_owner_without_cracks_or_draw_order_dependence() {
    let device = device();
    let buffer = vertices(
        &device,
        &[
            [-1.0, 1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
        ],
    );
    let first = target(&device, 4, 4);
    let second = target(&device, 4, 4);
    for (target, reverse) in [(&first, false), (&second, true)] {
        let mut encoder = device.create_command_encoder();
        let mut pass = encoder.begin_render_pass(target, Some(BACKGROUND)).unwrap();
        pass.set_vertex_buffer(&buffer, 0).unwrap();
        if reverse {
            pass.draw(3..6, BLUE).unwrap();
            pass.draw(0..3, RED).unwrap();
        } else {
            pass.draw(0..3, RED).unwrap();
            pass.draw(3..6, BLUE).unwrap();
        }
        pass.finish().unwrap();
        device.default_queue().submit(&encoder.finish()).unwrap();
    }
    assert_eq!(pixels(&first), pixels(&second));
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(
                first.pixel(x, y).unwrap(),
                if x + y < 3 { RED } else { BLUE }
            );
        }
    }
}

#[test]
fn clipping_handles_all_boundaries_outside_geometry_and_non_square_targets() {
    let device = device();
    let target = target(&device, 5, 3);
    let enclosing = vertices(&device, &[[-3.0, -3.0], [3.0, -3.0], [0.0, 3.0]]);
    render(&device, &target, &enclosing, 3);
    assert_eq!(pixels(&target), vec![RED; 15]);
    for points in [
        [[2.0, 0.0], [3.0, 0.0], [2.0, 1.0]],
        [[-2.0, 0.0], [-3.0, 0.0], [-2.0, 1.0]],
        [[0.0, 2.0], [1.0, 2.0], [0.0, 3.0]],
        [[0.0, -2.0], [1.0, -2.0], [0.0, -3.0]],
    ] {
        render(&device, &target, &vertices(&device, &points), 3);
        assert_eq!(pixels(&target), vec![BACKGROUND; 15]);
    }
    // Finite extreme input clips without integer overflow or invalid indices.
    let extreme = vertices(
        &device,
        &[
            [-f32::MAX, -f32::MAX],
            [f32::MAX, -f32::MAX],
            [0.0, f32::MAX],
        ],
    );
    render(&device, &target, &extreme, 3);
    assert_eq!(pixels(&target), vec![RED; 15]);
    let partial = self::target(&device, 4, 4);
    for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let points = [[-2.0 * sx, 0.0], [0.0, 2.0 * sy], [0.0, 0.0]];
        render(&device, &partial, &vertices(&device, &points), 3);
        for y in 0..4 {
            for x in 0..4 {
                let covered = (if sx > 0.0 { x < 2 } else { x >= 2 })
                    && (if sy > 0.0 { y < 2 } else { y >= 2 });
                assert_eq!(
                    partial.pixel(x, y).unwrap(),
                    if covered { RED } else { BACKGROUND }
                );
            }
        }
    }
}

#[test]
fn foreign_targets_and_buffers_and_missing_vertex_usage_are_rejected() {
    let device = device();
    let foreign = device.adapter().create_device();
    let local_target = target(&device, 2, 2);
    let foreign_target = target(&foreign, 2, 2);
    let mut encoder = device.create_command_encoder();
    assert!(matches!(
        encoder.begin_render_pass(&foreign_target, None),
        Err(RenderEncodeError::ForeignFramebuffer { .. })
    ));
    let mut pass = encoder.begin_render_pass(&local_target, None).unwrap();
    let foreign_buffer = vertices(&foreign, &[[0.0; 2]; 3]);
    assert!(matches!(
        pass.set_vertex_buffer(&foreign_buffer, 0),
        Err(RenderEncodeError::ForeignVertexBuffer { .. })
    ));
    let wrong = device
        .create_buffer(BufferDescriptor {
            label: "wrong usage",
            size: 24,
            usages: &[BufferUsage::Storage],
            host_access: HostAccess::None,
        })
        .unwrap();
    assert!(matches!(
        pass.set_vertex_buffer(&wrong, 0),
        Err(RenderEncodeError::MissingVertexUsage { .. })
    ));
    assert!(matches!(
        pass.draw(0..3, RED),
        Err(RenderEncodeError::MissingVertexBuffer)
    ));
}

#[test]
fn binding_offsets_draw_ranges_and_failed_binding_preserve_valid_work() {
    let device = device();
    let target = target(&device, 4, 4);
    let buffer = vertices(&device, &[[9.0; 2], [-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap();
    pass.set_vertex_buffer(&buffer, 8).unwrap();
    assert!(matches!(
        pass.set_vertex_buffer(&buffer, 33),
        Err(RenderEncodeError::VertexOffsetOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.set_vertex_buffer(&buffer, 1),
        Err(RenderEncodeError::UnalignedVertexOffset { .. })
    ));
    assert!(matches!(
        pass.draw(std::ops::Range { start: 3, end: 0 }, RED),
        Err(RenderEncodeError::InvalidVertexRange { .. })
    ));
    assert!(matches!(
        pass.draw(0..2, RED),
        Err(RenderEncodeError::IncompleteTriangle { .. })
    ));
    assert!(matches!(
        pass.draw(1..4, RED),
        Err(RenderEncodeError::VertexRangeOutOfBounds { .. })
    ));
    assert!(matches!(
        pass.draw(u32::MAX..u32::MAX, RED),
        Err(RenderEncodeError::VertexRangeOutOfBounds { .. })
    ));
    pass.draw(0..3, RED).unwrap();
    pass.draw(3..3, BLUE).unwrap(); // Empty at the exact allocation end.
    pass.finish().unwrap();
    let commands = encoder.finish();
    match &commands.commands()[0] {
        Command::RenderPass(pass) => {
            assert_eq!(pass.draws().len(), 2);
            assert_eq!(pass.draws()[0].vertex_offset(), 8);
            assert_eq!(pass.draws()[0].vertices(), 0..3);
            assert_eq!(pass.draws()[0].colour(), RED);
            assert!(pass.draws()[0].vertex_buffer().is_same(&buffer));
        }
        _ => panic!("expected render"),
    }
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), RED);
}

#[test]
fn first_vertex_is_relative_to_binding_and_draws_keep_independent_bindings() {
    let device = device();
    let target = target(&device, 4, 4);
    let first = vertices(&device, &[[9.0; 2], [-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    let second = vertices(&device, &[[1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder.begin_render_pass(&target, None).unwrap();
    pass.set_vertex_buffer(&first, 0).unwrap();
    pass.draw(1..4, RED).unwrap();
    pass.set_vertex_buffer(&second, 0).unwrap();
    pass.draw(0..3, BLUE).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), RED);
    assert_eq!(target.pixel(3, 3).unwrap(), BLUE);
}

#[test]
fn earlier_upload_copies_are_visible_to_host_inaccessible_vertex_fetch() {
    let device = device();
    let bytes = encoded(&[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    let source = device
        .create_buffer(BufferDescriptor {
            label: "upload",
            size: 24,
            usages: &[BufferUsage::TransferSource],
            host_access: HostAccess::Write,
        })
        .unwrap();
    source.write(0, &bytes).unwrap();
    let vertex = device
        .create_buffer(BufferDescriptor {
            label: "device vertices",
            size: 24,
            usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
            host_access: HostAccess::None,
        })
        .unwrap();
    let target = target(&device, 4, 4);
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 0, &vertex, 0, 24).unwrap();
    let mut pass = encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap();
    pass.set_vertex_buffer(&vertex, 0).unwrap();
    pass.draw(0..3, RED).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(target.pixel(0, 0).unwrap(), RED);
    assert!(vertex.read(0, &mut [0; 24]).is_err());
}

#[test]
fn nonfinite_vertices_reject_the_whole_pass_but_preserve_prior_commands() {
    let device = device();
    let target = target(&device, 4, 4);
    let valid = vertices(&device, &[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]);
    for bad_value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let bad = vertices(&device, &[[0.0, 0.0], [bad_value, 0.0], [0.0, 1.0]]);
        let mut encoder = device.create_command_encoder();
        encoder
            .begin_render_pass(&target, Some(BACKGROUND))
            .unwrap()
            .finish()
            .unwrap();
        let mut pass = encoder.begin_render_pass(&target, Some(BLUE)).unwrap();
        pass.set_vertex_buffer(&valid, 0).unwrap();
        pass.draw(0..3, RED).unwrap();
        pass.set_vertex_buffer(&bad, 0).unwrap();
        pass.draw(0..3, RED).unwrap();
        pass.finish().unwrap();
        encoder
            .begin_render_pass(&target, Some(RED))
            .unwrap()
            .finish()
            .unwrap();
        let error = device
            .default_queue()
            .submit(&encoder.finish())
            .unwrap_err();
        assert!(matches!(
            &error,
            QueueSubmitError::RenderFailed {
                command_index: 1,
                source: RenderExecutionError::NonFiniteVertex {
                    vertex_index: 1,
                    ..
                },
            }
        ));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(pixels(&target), vec![BACKGROUND; 16]);
    }
}

#[test]
fn retained_live_resources_support_resubmission_after_original_handles_drop() {
    let device = device();
    let target = target(&device, 4, 4);
    let buffer = vertices(&device, &[[9.0; 2]; 3]);
    let queue = device.default_queue();
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap();
    pass.set_vertex_buffer(&buffer, 0).unwrap();
    pass.draw(0..3, RED).unwrap();
    pass.finish().unwrap();
    let commands = encoder.finish();
    queue.submit(&commands).unwrap();
    assert_eq!(pixels(&target), vec![BACKGROUND; 16]);
    buffer
        .write(0, &encoded(&[[-1.0, 1.0], [1.0, 1.0], [-1.0, -1.0]]))
        .unwrap();
    let readback = target.clone();
    drop(buffer);
    drop(target);
    drop(device);
    queue.submit(&commands).unwrap();
    drop(commands);
    drop(queue);
    assert_eq!(readback.pixel(0, 0).unwrap(), RED);
}

#[test]
fn overlapping_draws_obey_recorded_order() {
    let device = device();
    let target = target(&device, 4, 4);
    let buffer = vertices(&device, &[[-3.0, -3.0], [3.0, -3.0], [0.0, 3.0]]);
    let mut encoder = device.create_command_encoder();
    let mut pass = encoder
        .begin_render_pass(&target, Some(BACKGROUND))
        .unwrap();
    pass.set_vertex_buffer(&buffer, 0).unwrap();
    pass.draw(0..3, RED).unwrap();
    pass.draw(0..3, BLUE).unwrap();
    pass.finish().unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(pixels(&target), vec![BLUE; 16]);
}
