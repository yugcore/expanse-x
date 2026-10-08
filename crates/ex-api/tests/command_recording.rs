use ex::{
    Buffer, BufferCopy, BufferDescriptor, BufferUsage, Command, CommandEncodeError, CopyBufferRole,
    DebugMarkerBuffer, Device, HostAccess, Instance,
};

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}

fn buffer(device: &Device, label: &str, usages: &[BufferUsage], access: HostAccess) -> Buffer {
    device
        .create_buffer(BufferDescriptor {
            label,
            size: 8,
            usages,
            host_access: access,
        })
        .unwrap()
}

fn transfer(device: &Device, label: &str) -> Buffer {
    buffer(
        device,
        label,
        &[
            BufferUsage::TransferSource,
            BufferUsage::TransferDestination,
        ],
        HostAccess::ReadWrite,
    )
}

fn copy(command: &Command) -> &BufferCopy {
    match command {
        Command::CopyBuffer(copy) => copy,
        _ => panic!("expected a buffer-copy description"),
    }
}

#[test]
fn recording_and_finishing_preserve_bytes_and_command_order() {
    let device = device();
    let source = transfer(&device, "upload");
    let destination = transfer(&device, "readback");
    source.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    destination.write(0, &[99; 8]).unwrap();
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 1, &destination, 3, 3).unwrap();
    encoder.copy_buffer(&destination, 0, &source, 6, 2).unwrap();
    let mut bytes = [0; 8];
    destination.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [99; 8]);
    let commands = encoder.finish();
    assert_eq!(commands.commands().len(), 2);
    let first = copy(&commands.commands()[0]);
    assert!(first.source().is_same(&source));
    assert!(first.destination().is_same(&destination));
    assert_eq!(
        (
            first.source_offset(),
            first.destination_offset(),
            first.size()
        ),
        (1, 3, 3)
    );
    let second = copy(&commands.commands()[1]);
    assert!(second.source().is_same(&destination));
    assert!(second.destination().is_same(&source));
    assert_eq!(
        (
            second.source_offset(),
            second.destination_offset(),
            second.size()
        ),
        (0, 6, 2)
    );
    destination.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [99; 8]);
    source.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn empty_encoders_and_finished_work_retain_the_exact_device() {
    let device = device();
    let foreign = device.adapter().create_device();
    let encoder = device.clone().create_command_encoder();
    assert!(encoder.belongs_to(&device));
    assert!(encoder.device().is_same(&device));
    assert!(!encoder.belongs_to(&foreign));
    let queue = device.default_queue();
    drop(device);
    let commands = encoder.finish();
    assert!(commands.commands().is_empty());
    assert!(commands.belongs_to(&queue.device()));
    assert!(commands.device().is_same(&queue.device()));
    assert!(!commands.belongs_to(&foreign));
}

#[test]
fn foreign_devices_are_rejected_before_usage_and_range_checks() {
    let device = device();
    let foreign = device.adapter().create_device();
    let local = transfer(&device, "local");
    let foreign_source = buffer(
        &foreign,
        "foreign source",
        &[BufferUsage::Storage],
        HostAccess::None,
    );
    let foreign_destination = buffer(
        &foreign,
        "foreign destination",
        &[BufferUsage::Storage],
        HostAccess::None,
    );
    let mut encoder = device.create_command_encoder();
    let error = encoder
        .copy_buffer(&foreign_source, u64::MAX, &foreign_destination, u64::MAX, 1)
        .unwrap_err();
    assert!(
        matches!(&error, CommandEncodeError::ForeignBuffer { role: CopyBufferRole::Source, label } if label == "foreign source")
    );
    drop(foreign_source);
    assert!(error.to_string().contains("foreign source"));
    assert!(std::error::Error::source(&error).is_none());
    assert!(matches!(
        encoder.copy_buffer(&local, 0, &foreign_destination, 0, 0),
        Err(CommandEncodeError::ForeignBuffer {
            role: CopyBufferRole::Destination,
            ..
        })
    ));
    assert!(encoder.commands().is_empty());
}

#[test]
fn both_transfer_roles_are_required_even_for_empty_copies() {
    let device = device();
    let source = buffer(
        &device,
        "source",
        &[BufferUsage::TransferSource],
        HostAccess::None,
    );
    let destination = buffer(
        &device,
        "destination",
        &[BufferUsage::TransferDestination],
        HostAccess::None,
    );
    let wrong = buffer(
        &device,
        "no transfer roles",
        &[BufferUsage::Storage],
        HostAccess::ReadWrite,
    );
    let mut encoder = device.create_command_encoder();
    assert!(matches!(
        encoder.copy_buffer(&wrong, u64::MAX, &wrong, u64::MAX, 0),
        Err(CommandEncodeError::MissingUsage {
            role: CopyBufferRole::Source,
            required: BufferUsage::TransferSource,
            ..
        })
    ));
    assert!(matches!(
        encoder.copy_buffer(&source, 0, &wrong, 0, 0),
        Err(CommandEncodeError::MissingUsage {
            role: CopyBufferRole::Destination,
            required: BufferUsage::TransferDestination,
            ..
        })
    ));
    // No CPU access is needed, and byte offsets/sizes need not be four-byte aligned.
    encoder.copy_buffer(&source, 1, &destination, 3, 3).unwrap();
    assert_eq!(encoder.finish().commands().len(), 1);
}

#[test]
fn source_and_destination_ranges_reject_bounds_and_overflow() {
    let device = device();
    let source = transfer(&device, "source");
    let destination = transfer(&device, "destination");
    let mut encoder = device.create_command_encoder();
    for (offset, size) in [(8, 1), (9, 0), (7, 2), (u64::MAX, 1), (1, u64::MAX)] {
        assert!(matches!(
            encoder.copy_buffer(&source, offset, &destination, 0, size),
            Err(CommandEncodeError::OutOfBounds { role: CopyBufferRole::Source, offset: bad_offset, size: bad_size, buffer_size: 8, .. })
                if bad_offset == offset && bad_size == size
        ));
    }
    for (offset, size) in [(8, 1), (9, 0), (7, 2), (u64::MAX, 1)] {
        assert!(matches!(
            encoder.copy_buffer(&source, 0, &destination, offset, size),
            Err(CommandEncodeError::OutOfBounds {
                role: CopyBufferRole::Destination,
                ..
            })
        ));
    }
    assert!(encoder.commands().is_empty());
    encoder.copy_buffer(&source, 7, &destination, 7, 1).unwrap();
}

#[test]
fn zero_length_copies_allow_end_offsets_and_retain_the_description() {
    let device = device();
    let buffer = transfer(&device, "empty ranges");
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&buffer, 8, &buffer, 8, 0).unwrap();
    encoder.copy_buffer(&buffer, 0, &buffer, 8, 0).unwrap();
    let commands = encoder.finish();
    let first = copy(&commands.commands()[0]);
    assert_eq!(
        (
            first.source_offset(),
            first.destination_offset(),
            first.size()
        ),
        (8, 8, 0)
    );
    assert!(first.source().is_same(first.destination()));
    assert_eq!(commands.commands().len(), 2);
}

#[test]
fn alias_overlap_checks_use_allocation_identity_in_both_directions() {
    let device = device();
    let buffer = transfer(&device, "shared bytes");
    let alias = buffer.clone();
    let independent = transfer(&device, "shared bytes");
    let mut encoder = device.create_command_encoder();
    for (source_offset, destination_offset, size) in [(0, 1, 4), (1, 0, 4), (2, 2, 2)] {
        assert!(matches!(
            encoder.copy_buffer(&buffer, source_offset, &alias, destination_offset, size),
            Err(CommandEncodeError::OverlappingCopy { source_offset: actual_source, destination_offset: actual_destination, size: actual_size, .. })
                if actual_source == source_offset && actual_destination == destination_offset && actual_size == size
        ));
    }
    encoder.copy_buffer(&buffer, 0, &alias, 4, 4).unwrap();
    encoder.copy_buffer(&alias, 4, &buffer, 0, 4).unwrap();
    encoder.copy_buffer(&buffer, 0, &independent, 0, 8).unwrap();
    assert_eq!(encoder.finish().commands().len(), 3);
}

#[test]
fn rejected_requests_leave_prior_work_intact_and_recording_can_continue() {
    let device = device();
    let source = transfer(&device, "source");
    let destination = transfer(&device, "destination");
    let wrong = buffer(
        &device,
        "wrong",
        &[BufferUsage::Storage],
        HostAccess::ReadWrite,
    );
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 0, &destination, 0, 4).unwrap();
    let error = encoder.copy_buffer(&source, 0, &wrong, 0, 4).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("destination"));
    assert!(message.contains("wrong"));
    assert!(message.contains("TransferDestination"));
    assert_eq!(encoder.commands().len(), 1);
    encoder.copy_buffer(&source, 4, &destination, 4, 4).unwrap();
    let commands = encoder.finish();
    assert_eq!(copy(&commands.commands()[0]).source_offset(), 0);
    assert_eq!(copy(&commands.commands()[1]).source_offset(), 4);
    let mut bytes = [99; 8];
    destination.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 8]);
}

#[test]
fn commands_retain_live_resources_after_original_handles_drop() {
    let device = device();
    let source = transfer(&device, "source");
    let destination = transfer(&device, "destination");
    source.write(0, &[1, 2, 3, 4]).unwrap();
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 0, &destination, 0, 4).unwrap();
    let commands = encoder.finish();
    source.write(0, &[7, 11, 13, 17]).unwrap();
    drop(source);
    drop(destination);
    drop(device);
    let recorded = copy(&commands.commands()[0]);
    let mut bytes = [0; 4];
    recorded.source().read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [7, 11, 13, 17]); // Handles, not a recording-time snapshot.
    recorded.destination().read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 4]);
    assert!(recorded.source().belongs_to(&commands.device()));
    assert!(recorded.destination().belongs_to(&commands.device()));
}

#[test]
fn standalone_markers_have_a_distinct_finished_type() {
    let mut recorder = ex::CommandRecorder::new();
    recorder.insert_debug_marker("copy (label only)");
    let markers: DebugMarkerBuffer = recorder.finish();
    assert_eq!(markers.debug_markers(), &["copy (label only)"]);
}
