use ex::{
    BufferDescriptor, BufferUsage, FramebufferDescriptor, FramebufferFormat, HostAccess, Instance,
};
use std::path::PathBuf;

#[path = "triangle/png.rs"]
mod png;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/phase2-triangle.png"));
    let device = Instance::new().default_adapter().create_device();
    let target = device.create_framebuffer(FramebufferDescriptor {
        label: "first EX software triangle",
        width: 512,
        height: 384,
        format: FramebufferFormat::Rgba8Unorm,
    })?;
    let positions = [[0.0_f32, 0.72], [-0.72, -0.65], [0.72, -0.65]];
    let bytes: Vec<u8> = positions
        .iter()
        .flat_map(|point| point.iter().flat_map(|value| value.to_le_bytes()))
        .collect();
    let upload = device.create_buffer(BufferDescriptor {
        label: "vertex upload",
        size: bytes.len() as u64,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    upload.write(0, &bytes)?;
    let vertices = device.create_buffer(BufferDescriptor {
        label: "triangle vertices",
        size: bytes.len() as u64,
        usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    let background = [13, 19, 33, 255];
    let foreground = [248, 174, 66, 255];
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&upload, 0, &vertices, 0, bytes.len() as u64)?;
    let mut pass = encoder.begin_render_pass(&target, Some(background))?;
    pass.set_vertex_buffer(&vertices, 0)?;
    pass.draw(0..3, foreground)?;
    pass.finish()?;
    let commands = encoder.finish();
    assert_eq!(target.pixel(256, 192)?, [0; 4]); // Recording changes no pixels.
    drop(upload);
    drop(vertices);
    device.default_queue().submit(&commands)?;
    assert_eq!(target.pixel(256, 192)?, foreground);
    assert_eq!(target.pixel(0, 0)?, background);
    assert_eq!(target.pixel(511, 383)?, background);
    let mut pixels = vec![[0; 4]; target.width() as usize * target.height() as usize];
    target.read_pixels(&mut pixels)?;
    let covered = pixels.iter().filter(|&&pixel| pixel == foreground).count();
    assert!(covered > 0 && covered < pixels.len());
    png::write_rgba(&output, target.width(), target.height(), &pixels)?;
    println!("EX rendered {covered} triangle pixels on its CPU reference target.");
    println!("Image saved to {}", output.display());
    Ok(())
}
