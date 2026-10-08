use ex::{BackendKind, Instance};

#[test]
fn discovery_exposes_only_the_builtin_software_target() {
    for instance in [Instance::new(), Instance::default()] {
        assert_eq!(instance.adapters().len(), 1);
        let discovered = &instance.adapters()[0];
        let default = instance.default_adapter();
        assert_eq!(discovered.info(), default.info());
        assert_eq!(default.info().backend, BackendKind::Reference);
        assert!(default.info().name.contains("software"));
    }
}

#[test]
fn clones_and_retrieved_queues_retain_the_original_device_identity() {
    let device = Instance::new().default_adapter().create_device();
    let cloned_device = device.clone();
    let queue = device.default_queue();
    let cloned_queue = queue.clone();
    let retrieved_again = cloned_device.default_queue();

    assert!(device.is_same(&cloned_device));
    for handle in [queue, cloned_queue, retrieved_again] {
        assert!(handle.belongs_to(&device));
        assert!(handle.belongs_to(&cloned_device));
        assert!(handle.device().is_same(&device));
    }
}

#[test]
fn devices_on_the_same_adapter_have_separate_ownership_domains() {
    let adapter = Instance::new().default_adapter();
    let first = adapter.create_device();
    let second = adapter.create_device();
    assert_eq!(first.adapter().info(), second.adapter().info());
    assert!(!first.is_same(&second));
    assert!(!second.is_same(&first));
    assert!(!first.default_queue().belongs_to(&second));
    assert!(!second.default_queue().belongs_to(&first));
}

#[test]
fn devices_from_different_instances_do_not_share_identity() {
    let first = Instance::new().default_adapter().create_device();
    let second = Instance::new().default_adapter().create_device();
    assert!(!first.is_same(&second));
    assert!(!first.default_queue().belongs_to(&second));
}

#[test]
fn owned_children_remain_usable_after_parent_handles_drop() {
    let instance = Instance::new();
    let adapter = instance.adapters()[0].clone();
    drop(instance);

    let device = adapter.create_device();
    drop(adapter);
    assert_eq!(device.adapter().info().backend, BackendKind::Reference);

    let queue = device.default_queue();
    drop(device);
    let retained = queue.device();
    assert_eq!(retained.adapter().info().backend, BackendKind::Reference);
    assert!(queue.belongs_to(&retained));
    assert!(retained.default_queue().device().is_same(&retained));
}
