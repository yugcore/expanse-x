use ex::{
    BufferAccessError, BufferCreateError, BufferDescriptor, BufferDescriptorError, BufferUsage,
    Device, HostAccess, Instance,
};

fn device() -> Device {
    Instance::new().default_adapter().create_device()
}

fn request(label: &str, size: u64, host_access: HostAccess) -> BufferDescriptor<'_> {
    BufferDescriptor {
        label,
        size,
        usages: &[BufferUsage::Storage],
        host_access,
    }
}

#[test]
fn allocation_owns_metadata_and_initializes_exactly_the_requested_bytes() {
    let device = device();
    let label = String::from("owned vertex bytes");
    let usages = vec![
        BufferUsage::Vertex,
        BufferUsage::TransferDestination,
        BufferUsage::Vertex,
    ];
    let buffer = device
        .create_buffer(BufferDescriptor {
            label: &label,
            size: 8,
            usages: &usages,
            host_access: HostAccess::Read,
        })
        .unwrap();
    drop(label);
    drop(usages);

    assert_eq!(buffer.label(), "owned vertex bytes");
    assert_eq!(buffer.size(), 8);
    assert_eq!(
        buffer.usages(),
        &[
            BufferUsage::Vertex,
            BufferUsage::TransferDestination,
            BufferUsage::Vertex
        ]
    );
    assert_eq!(buffer.host_access(), HostAccess::Read);
    assert!(buffer.belongs_to(&device));
    let mut bytes = [99; 8];
    buffer.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 8]);
}

#[test]
fn creation_rejects_invalid_descriptors_and_preserves_the_original_error() {
    let device = device();
    let zero = device
        .create_buffer(request("empty", 0, HostAccess::None))
        .unwrap_err();
    assert!(matches!(
        zero,
        BufferCreateError::InvalidDescriptor(BufferDescriptorError::ZeroSize)
    ));
    assert!(std::error::Error::source(&zero).is_some());
    let missing = device
        .create_buffer(BufferDescriptor {
            usages: &[],
            ..request("missing roles", 4, HostAccess::None)
        })
        .unwrap_err();
    assert!(matches!(
        missing,
        BufferCreateError::InvalidDescriptor(BufferDescriptorError::MissingUsage)
    ));
}

#[test]
fn impossible_capacity_is_rejected_before_attempting_a_huge_allocation() {
    let error = device()
        .create_buffer(request("too large", u64::MAX, HostAccess::None))
        .unwrap_err();
    assert!(matches!(
        error,
        BufferCreateError::SizeTooLarge { size: u64::MAX }
    ));
}

#[test]
fn host_permissions_are_enforced_for_every_access_mode() {
    let device = device();
    for access in [
        HostAccess::None,
        HostAccess::Read,
        HostAccess::Write,
        HostAccess::ReadWrite,
    ] {
        let buffer = device
            .create_buffer(request("permission test", 4, access))
            .unwrap();
        let write = buffer.write(0, &[1; 4]);
        let can_write = matches!(access, HostAccess::Write | HostAccess::ReadWrite);
        if can_write {
            write.unwrap();
        } else {
            assert!(
                matches!(write, Err(BufferAccessError::WriteNotAllowed { access: actual, .. }) if actual == access)
            );
        }
        let mut bytes = [99; 4];
        let read = buffer.read(0, &mut bytes);
        if matches!(access, HostAccess::Read | HostAccess::ReadWrite) {
            read.unwrap();
            assert_eq!(bytes, if can_write { [1; 4] } else { [0; 4] });
        } else {
            let error = read.unwrap_err();
            assert!(
                matches!(error, BufferAccessError::ReadNotAllowed { access: actual, .. } if actual == access)
            );
            assert!(error.to_string().contains("permission test"));
            assert!(error.to_string().contains("request Read or ReadWrite"));
            assert_eq!(bytes, [99; 4]);
        }
    }
}

#[test]
fn transfer_roles_do_not_grant_cpu_access_even_for_empty_copies() {
    let buffer = device()
        .create_buffer(BufferDescriptor {
            usages: &[
                BufferUsage::TransferSource,
                BufferUsage::TransferDestination,
            ],
            ..request("private bytes", 4, HostAccess::None)
        })
        .unwrap();
    assert!(matches!(
        buffer.write(0, &[]),
        Err(BufferAccessError::WriteNotAllowed { .. })
    ));
    assert!(matches!(
        buffer.read(0, &mut []),
        Err(BufferAccessError::ReadNotAllowed { .. })
    ));
}

#[test]
fn partial_copies_use_byte_offsets_and_preserve_untouched_bytes() {
    let buffer = device()
        .create_buffer(request("partial bytes", 8, HostAccess::ReadWrite))
        .unwrap();
    buffer.write(2, &[10, 20, 30]).unwrap();
    buffer.write(7, &[40]).unwrap();
    let mut full = [99; 8];
    buffer.read(0, &mut full).unwrap();
    assert_eq!(full, [0, 0, 10, 20, 30, 0, 0, 40]);
    let mut subset = [99; 4];
    buffer.read(1, &mut subset).unwrap();
    assert_eq!(subset, [0, 10, 20, 30]);
}

#[test]
fn invalid_ranges_and_integer_overflow_leave_all_destinations_unchanged() {
    let buffer = device()
        .create_buffer(request("bounded bytes", 4, HostAccess::ReadWrite))
        .unwrap();
    buffer.write(0, &[1, 2, 3, 4]).unwrap();
    for (offset, length) in [(3, 2), (5, 0), (u64::MAX, 2), (u64::MAX, 0)] {
        let input = vec![9; length];
        assert!(matches!(
            buffer.write(offset, &input),
            Err(BufferAccessError::OutOfBounds { .. })
        ));
        let mut destination = vec![99; length];
        assert!(matches!(
            buffer.read(offset, &mut destination),
            Err(BufferAccessError::OutOfBounds { .. })
        ));
        assert_eq!(destination, vec![99; length]);
    }
    let mut original = [0; 4];
    buffer.read(0, &mut original).unwrap();
    assert_eq!(original, [1, 2, 3, 4]);
}

#[test]
fn empty_copies_at_the_end_are_valid_with_permission() {
    let buffer = device()
        .create_buffer(request("boundary", 4, HostAccess::ReadWrite))
        .unwrap();
    buffer.write(4, &[]).unwrap();
    buffer.read(4, &mut []).unwrap();
}

#[test]
fn clones_share_bytes_and_retain_the_creating_device_after_parent_drops() {
    let device = device();
    let buffer = device
        .create_buffer(request("retained", 4, HostAccess::ReadWrite))
        .unwrap();
    let other_buffer = device
        .create_buffer(request("retained", 4, HostAccess::ReadWrite))
        .unwrap();
    let other_device = device.adapter().create_device();
    assert!(!buffer.is_same(&other_buffer));
    assert!(!buffer.belongs_to(&other_device));
    let alias = buffer.clone();
    assert!(buffer.is_same(&alias));
    alias.write(0, &[2, 3, 5, 7]).unwrap();
    drop(alias);
    drop(device);
    let mut bytes = [0; 4];
    buffer.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [2, 3, 5, 7]);
    assert!(buffer.belongs_to(&buffer.device()));
    assert!(buffer.device().default_queue().belongs_to(&buffer.device()));
}

#[test]
fn concurrent_cpu_copies_through_cloned_handles_share_one_allocation() {
    let buffer = device()
        .create_buffer(request("shared", 8, HostAccess::ReadWrite))
        .unwrap();
    let first = buffer.clone();
    let second = buffer.clone();
    let first_writer = std::thread::spawn(move || first.write(0, &[1; 4]));
    let second_writer = std::thread::spawn(move || second.write(4, &[2; 4]));
    first_writer.join().unwrap().unwrap();
    second_writer.join().unwrap().unwrap();
    let mut bytes = [0; 8];
    buffer.read(0, &mut bytes).unwrap();
    assert_eq!(bytes, [1, 1, 1, 1, 2, 2, 2, 2]);
}
