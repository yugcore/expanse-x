use ex::{
    Buffer, BufferDescriptor, BufferUsage, CommandBuffer, Device, HostAccess, Instance,
    QueueSubmitError,
};
use std::sync::{Arc, Barrier};

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}

fn buffer(device: &Device, label: &str, access: HostAccess) -> Buffer {
    device
        .create_buffer(BufferDescriptor {
            label,
            size: 8,
            usages: &[
                BufferUsage::TransferSource,
                BufferUsage::TransferDestination,
            ],
            host_access: access,
        })
        .unwrap()
}

fn copy(device: &Device, source: &Buffer, destination: &Buffer) -> CommandBuffer {
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(source, 0, destination, 0, 8).unwrap();
    encoder.finish()
}

fn bytes(buffer: &Buffer) -> [u8; 8] {
    let mut bytes = [0; 8];
    buffer.read(0, &mut bytes).unwrap();
    bytes
}

#[test]
fn submission_copies_partial_byte_ranges_and_preserves_other_bytes() {
    let device = device();
    let source = buffer(&device, "upload", HostAccess::Write);
    let destination = buffer(&device, "readback", HostAccess::ReadWrite);
    source.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    destination.write(0, &[99; 8]).unwrap();
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 1, &destination, 3, 3).unwrap();
    let commands = encoder.finish();
    assert_eq!(bytes(&destination), [99; 8]);
    device.default_queue().submit(&commands).unwrap();
    assert_eq!(bytes(&destination), [99, 99, 99, 2, 3, 4, 99, 99]);
}

#[test]
fn copy_chains_execute_in_order_through_host_inaccessible_storage() {
    let device = device();
    let source = buffer(&device, "upload", HostAccess::Write);
    let intermediate = buffer(&device, "device only", HostAccess::None);
    let destination = buffer(&device, "readback", HostAccess::Read);
    source.write(0, &[7; 8]).unwrap();
    let mut encoder = device.create_command_encoder();
    encoder
        .copy_buffer(&source, 0, &intermediate, 0, 8)
        .unwrap();
    encoder
        .copy_buffer(&intermediate, 0, &destination, 0, 8)
        .unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(bytes(&destination), [7; 8]);
    assert!(intermediate.read(0, &mut [0; 8]).is_err());
}

#[test]
fn reusable_commands_sample_late_writes_and_survive_original_handles() {
    let device = device();
    let source = buffer(&device, "source", HostAccess::ReadWrite);
    let destination = buffer(&device, "destination", HostAccess::ReadWrite);
    source.write(0, &[1; 8]).unwrap();
    let commands = copy(&device, &source, &destination);
    let queue = device.default_queue();
    source.write(0, &[2; 8]).unwrap();
    queue.submit(&commands).unwrap();
    assert_eq!(bytes(&destination), [2; 8]);
    source.write(0, &[3; 8]).unwrap();
    let readback = destination.clone();
    drop(source);
    drop(destination);
    drop(device);
    queue.clone().submit(&commands).unwrap();
    drop(commands);
    drop(queue);
    assert_eq!(bytes(&readback), [3; 8]);
}

#[test]
fn foreign_work_is_rejected_even_when_empty_and_can_use_its_own_queue() {
    let device = device();
    let foreign = device.adapter().create_device();
    let source = buffer(&foreign, "source", HostAccess::ReadWrite);
    let destination = buffer(&foreign, "destination", HostAccess::ReadWrite);
    source.write(0, &[5; 8]).unwrap();
    let commands = copy(&foreign, &source, &destination);
    let error = device.default_queue().submit(&commands).unwrap_err();
    assert_eq!(error, QueueSubmitError::ForeignCommandBuffer);
    assert!(error.to_string().contains("another device"));
    assert!(std::error::Error::source(&error).is_none());
    assert_eq!(bytes(&destination), [0; 8]);
    let empty = foreign.create_command_encoder().finish();
    assert_eq!(
        device.default_queue().submit(&empty),
        Err(QueueSubmitError::ForeignCommandBuffer)
    );
    foreign.default_queue().submit(&commands).unwrap();
    assert_eq!(bytes(&destination), [5; 8]);
}

#[test]
fn empty_submissions_and_zero_byte_aliases_are_no_ops() {
    let device = device();
    let buffer = buffer(&device, "same allocation", HostAccess::ReadWrite);
    buffer.write(0, &[9; 8]).unwrap();
    let queue = device.default_queue();
    queue
        .submit(&device.create_command_encoder().finish())
        .unwrap();
    let mut encoder = device.create_command_encoder();
    encoder
        .copy_buffer(&buffer, 8, &buffer.clone(), 8, 0)
        .unwrap();
    queue.submit(&encoder.finish()).unwrap();
    assert_eq!(bytes(&buffer), [9; 8]);
}

#[test]
fn same_allocation_copies_work_in_both_directions_and_observe_prior_results() {
    let device = device();
    let buffer = buffer(&device, "aliases", HostAccess::ReadWrite);
    buffer.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    let mut encoder = device.create_command_encoder();
    encoder
        .copy_buffer(&buffer, 4, &buffer.clone(), 0, 4)
        .unwrap();
    encoder
        .copy_buffer(&buffer.clone(), 0, &buffer, 4, 3)
        .unwrap();
    device.default_queue().submit(&encoder.finish()).unwrap();
    assert_eq!(bytes(&buffer), [5, 6, 7, 8, 5, 6, 7, 8]);
}

#[test]
fn concurrent_submissions_keep_whole_chains_together() {
    let device = device();
    let first = buffer(&device, "ones", HostAccess::ReadWrite);
    let second = buffer(&device, "twos", HostAccess::ReadWrite);
    let intermediate = buffer(&device, "shared intermediate", HostAccess::None);
    let first_output = buffer(&device, "first output", HostAccess::Read);
    let second_output = buffer(&device, "second output", HostAccess::Read);
    first.write(0, &[1; 8]).unwrap();
    second.write(0, &[2; 8]).unwrap();
    let mut one = device.create_command_encoder();
    let mut two = device.create_command_encoder();
    for _ in 0..128 {
        one.copy_buffer(&first, 0, &intermediate, 0, 8).unwrap();
        one.copy_buffer(&intermediate, 0, &first_output, 0, 8)
            .unwrap();
        two.copy_buffer(&second, 0, &intermediate, 0, 8).unwrap();
        two.copy_buffer(&intermediate, 0, &second_output, 0, 8)
            .unwrap();
    }
    let one = one.finish();
    let two = two.finish();
    let start = Barrier::new(2);
    let queue = device.default_queue();
    let alternate_queue = device.default_queue();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            start.wait();
            queue.submit(&one).unwrap();
        });
        scope.spawn(|| {
            start.wait();
            alternate_queue.submit(&two).unwrap();
        });
    });
    assert_eq!(bytes(&first_output), [1; 8]);
    assert_eq!(bytes(&second_output), [2; 8]);
}

#[test]
fn concurrent_uploads_cannot_change_a_source_mid_submission() {
    let device = device();
    let source = buffer(&device, "changing source", HostAccess::Write);
    let first = buffer(&device, "first sample", HostAccess::Read);
    let second = buffer(&device, "second sample", HostAccess::Read);
    let mut encoder = device.create_command_encoder();
    encoder.copy_buffer(&source, 0, &first, 0, 8).unwrap();
    encoder.copy_buffer(&source, 0, &second, 0, 8).unwrap();
    let commands = encoder.finish();
    let start = Arc::new(Barrier::new(2));
    let queue = device.default_queue();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            start.wait();
            for value in 0..128 {
                source.write(0, &[value; 8]).unwrap();
            }
        });
        scope.spawn(|| {
            start.wait();
            for _ in 0..128 {
                queue.submit(&commands).unwrap();
                assert_eq!(bytes(&first), bytes(&second));
            }
        });
    });
}

#[test]
fn concurrent_opposite_direction_copies_finish_without_a_lock_cycle() {
    let device = device();
    let first = buffer(&device, "first", HostAccess::ReadWrite);
    let second = buffer(&device, "second", HostAccess::ReadWrite);
    first.write(0, &[1; 8]).unwrap();
    second.write(0, &[2; 8]).unwrap();
    let forward = copy(&device, &first, &second);
    let backward = copy(&device, &second, &first);
    let queue = device.default_queue();
    let other_queue = queue.clone();
    let start = Barrier::new(2);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            start.wait();
            for _ in 0..128 {
                queue.submit(&forward).unwrap();
            }
        });
        scope.spawn(|| {
            start.wait();
            for _ in 0..128 {
                other_queue.submit(&backward).unwrap();
            }
        });
    });
    let result = bytes(&first);
    assert!(result == [1; 8] || result == [2; 8]);
    assert_eq!(result, bytes(&second));
}
