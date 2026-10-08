use ex::{BufferDescriptor, BufferUsage, CommandRecorder, HostAccess};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Three vertices, each with three 4-byte f32 coordinates: 3 * 3 * 4.
    let descriptor = BufferDescriptor {
        label: "triangle positions",
        size: 36,
        usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    };
    descriptor.validate()?;
    println!("Valid buffer request: {descriptor:?}");

    let mut recorder = CommandRecorder::new();
    recorder.insert_debug_marker("prepare triangle (planned)");
    recorder.insert_debug_marker("draw triangle (planned)");
    let commands = recorder.finish();
    println!("Finished labels: {:?}", commands.debug_markers());
    println!("API prototype only: no GPU memory allocated or work submitted.");
    Ok(())
}
