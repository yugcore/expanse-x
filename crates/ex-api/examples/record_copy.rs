use ex::{BufferDescriptor, BufferUsage, Command, HostAccess, Instance};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let device = Instance::new().default_adapter().create_device();
    let source = device.create_buffer(BufferDescriptor {
        label: "upload bytes",
        size: 8,
        usages: &[BufferUsage::TransferSource],
        host_access: HostAccess::ReadWrite,
    })?;
    let destination = device.create_buffer(BufferDescriptor {
        label: "copy destination",
        size: 8,
        usages: &[BufferUsage::TransferDestination],
        host_access: HostAccess::Read,
    })?;
    source.write(1, &[7, 11, 13])?;
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 1, &destination, 3, 3)?;
    let mut bytes = [99; 8];
    destination.read(0, &mut bytes)?;
    assert_eq!(bytes, [0; 8]);
    println!("Destination after recording: {bytes:?}");
    let commands = encoder.finish();
    drop(source);
    drop(destination);
    drop(device);
    match &commands.commands()[0] {
        Command::CopyBuffer(copy) => {
            println!(
                "Recorded {} bytes: {:?} at {} -> {:?} at {}",
                copy.size(),
                copy.source().label(),
                copy.source_offset(),
                copy.destination().label(),
                copy.destination_offset(),
            );
            copy.destination().read(0, &mut bytes)?;
            assert_eq!(bytes, [0; 8]);
            println!("Destination after finishing and dropping original handles: {bytes:?}");
            assert!(copy.source().belongs_to(&commands.device()));
        }
        _ => return Err("expected the recorded copy".into()),
    }
    println!("Recording retains resources but does not execute: queue submission is next.");
    Ok(())
}
