#[path = "../examples/core_preview/workload.rs"]
mod workload;

#[test]
fn integrated_core_preview_workload() {
    let instance = ex::Instance::new();
    let mut tested = 0;
    for adapter in instance.adapters() {
        if adapter.core_profile() == ex::CoreProfile::Preview1 {
            let proof = workload::run(adapter).unwrap();
            assert_eq!((proof.width, proof.height), (128, 64));
            assert_eq!(proof.pixels.len(), 8192);
            assert_eq!(proof.command_count, 15);
            assert_eq!(proof.resource_use_count, 27);
            tested += 1;
        }
    }
    assert!(tested > 0, "no adapter exercised the integrated core proof");
}
