use ex::{BufferDescriptor, BufferUsage, HostAccess, Instance};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Instance::new().default_adapter().create_device();
    let buffer = device.create_buffer(BufferDescriptor {
        label: "round-trip bytes",
        size: 12,
        usages: &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        host_access: HostAccess::ReadWrite,
    })?;

    buffer.write(4, &[7, 11, 13, 17])?;
    let retained = buffer.clone();
    drop(device);
    drop(buffer);
    let mut bytes = [0; 12];
    retained.read(0, &mut bytes)?;
    println!("{}: {} allocated bytes", retained.label(), retained.size());
    println!("Readback after parent handles drop: {bytes:?}");
    println!(
        "Retained device ownership: {}",
        retained.belongs_to(&retained.device())
    );

    let private = retained.device().create_buffer(BufferDescriptor {
        label: "CPU-inaccessible bytes",
        size: 4,
        usages: &[BufferUsage::Vertex, BufferUsage::TransferDestination],
        host_access: HostAccess::None,
    })?;
    match private.write(0, &[1, 2, 3, 4]) {
        Err(error) => println!("Expected access error: {error}"),
        Ok(()) => return Err("a CPU write unexpectedly bypassed HostAccess::None".into()),
    }
    println!("Synchronous CPU copies only: no queue submission or rendering yet.");
    Ok(())
}
