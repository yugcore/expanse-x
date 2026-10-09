//! A small explicit upload/render/readback flow using only EX's public API.
use ex::*;
use ResourceAccess as A;

pub fn render() -> Result<Vec<[u8; 4]>, Box<dyn std::error::Error>> {
    let device = Instance::new().default_adapter().create_device();
    if device.core_profile() != CoreProfile::Preview1 {
        return Err(std::io::Error::other("starter requires CoreProfile::Preview1").into());
    }
    let positions = [[-0.75_f32, 0.75], [0.75, 0.75], [0.0, -0.75]];
    let upload = device.create_buffer(BufferDescriptor {
        label: "position upload",
        size: 24,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    upload.write(
        0,
        &positions
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )?;
    let vertices = device.create_buffer(BufferDescriptor {
        label: "device vertices",
        size: 24,
        usages: &[BufferUsage::TransferDestination, BufferUsage::Vertex],
        host_access: HostAccess::None,
    })?;
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "offscreen output",
        width: 64,
        height: 64,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let vertex_shader = crate::shaders::vertex(&device)?;
    let fragment_shader = crate::shaders::fragment(&device)?;
    let pipeline = device.create_graphics_pipeline(GraphicsPipelineDescriptor {
        label: "starter triangle",
        vertex_shader: &vertex_shader,
        fragment_shader: &fragment_shader,
        vertex_layout: VertexBufferLayout {
            stride: 8,
            attributes: &[VertexAttribute {
                location: 0,
                offset: 0,
                format: VertexFormat::Float32x2,
            }],
        },
        colour_format: FramebufferFormat::Rgba8Unorm,
    })?;
    let mut encoder =
        device.create_command_encoder_with_synchronization(SynchronizationMode::Explicit);
    encoder.transition_resources(&[
        ResourceTransition {
            resource: Resource::Buffer(upload.clone()),
            before: None,
            after: A::TransferRead,
        },
        ResourceTransition {
            resource: Resource::Buffer(vertices.clone()),
            before: None,
            after: A::TransferWrite,
        },
        ResourceTransition {
            resource: Resource::Framebuffer(target.clone()),
            before: None,
            after: A::ColourReadWrite,
        },
    ])?;
    encoder.copy_buffer(&upload, 0, &vertices, 0, 24)?;
    encoder.transition_resources(&[ResourceTransition {
        resource: Resource::Buffer(vertices.clone()),
        before: Some(A::TransferWrite),
        after: A::VertexRead,
    }])?;
    let mut pass = encoder.begin_render_pass(&target, Some([16, 24, 40, 255]))?;
    pass.set_pipeline(&pipeline)?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.draw_programmable(0..3)?;
    pass.finish()?;
    device.default_queue().submit(&encoder.finish())?;
    let mut pixels = vec![[0; 4]; 64 * 64];
    target.read_pixels(&mut pixels)?;
    // Coverage checks remain valid when editing the fragment colour.
    if pixels[0] != [16, 24, 40, 255] || pixels[32 * 64 + 32] == pixels[0] {
        return Err(std::io::Error::other("starter triangle coverage check failed").into());
    }
    Ok(pixels)
}
