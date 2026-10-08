use ex::{BufferDescriptor, BufferUsage, HostAccess, Instance};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Instance::new().default_adapter().create_device();
    let queue = device.default_queue();
    let source = device.create_buffer(BufferDescriptor {
        label: "upload",
        size: 8,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::Write,
    })?;
    let intermediate = device.create_buffer(BufferDescriptor {
        label: "device-only intermediate",
        size: 8,
        usages: &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        host_access: HostAccess::None,
    })?;
    let destination = device.create_buffer(BufferDescriptor {
        label: "readback",
        size: 8,
        usages: &[BufferUsage::TransferDestination],
        host_access: HostAccess::Read,
    })?;
    source.write(1, &[7, 11, 13])?;
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 1, &intermediate, 2, 3)?;
    encoder.copy_buffer(&intermediate, 2, &destination, 3, 3)?;
    let commands = encoder.finish();
    let mut bytes = [99; 8];
    destination.read(0, &mut bytes)?;
    assert_eq!(bytes, [0; 8]);
    println!("Before submission: {bytes:?}");
    // Finished work retains both allocations and samples this later upload.
    source.write(1, &[17, 19, 23])?;
    drop(source);
    drop(intermediate);
    drop(device);
    queue.submit(&commands)?;
    destination.read(0, &mut bytes)?;
    assert_eq!(bytes, [0, 0, 0, 17, 19, 23, 0, 0]);
    println!("After completed submission: {bytes:?}");
    println!("EX executed both copies in order on the CPU.");
    Ok(())
}
