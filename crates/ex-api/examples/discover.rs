use ex::Instance;

fn main() {
    let instance = Instance::new();
    println!("Discovered {} EX adapter(s):", instance.adapters().len());
    for adapter in instance.adapters() {
        println!("  {}: {:?}", adapter.info().name, adapter.info().backend);
    }

    let adapter = instance.default_adapter();
    let device = adapter.create_device();
    let queue = device.default_queue();
    println!("Device target: {}", device.adapter().info().name);
    println!(
        "Default queue belongs to device: {}",
        queue.belongs_to(&device)
    );

    // The queue retains the original device state, even after parent handles drop.
    drop(instance);
    drop(adapter);
    drop(device);
    let retained_device = queue.device();
    println!(
        "Retained device target: {}",
        retained_device.adapter().info().name
    );
    println!("Ownership milestone only: no resource storage, submission, or rendering yet.");
}
