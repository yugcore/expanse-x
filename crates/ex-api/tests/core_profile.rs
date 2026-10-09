//! The same contract cases run against every adapter advertising this profile.
#[path = "core_profile/cases.rs"]
mod cases;
use ex::{Adapter, CoreProfile, Instance};

fn each_preview_adapter(case: fn(&Adapter)) {
    let instance = Instance::new();
    let mut tested = 0;
    for adapter in instance.adapters() {
        if adapter.core_profile() == CoreProfile::Preview1 {
            case(adapter);
            tested += 1;
        }
    }
    assert!(tested > 0, "no adapter exercised the Preview1 contract");
}

#[test]
fn profile_bounds_and_rejections() {
    each_preview_adapter(cases::limits);
}
#[test]
fn finite_shader_arithmetic_and_stage_rules() {
    each_preview_adapter(cases::shaders);
}
#[test]
fn ordered_live_transfer_and_host_permissions() {
    each_preview_adapter(cases::copies);
}
#[test]
fn explicit_dependency_preflight_and_visibility() {
    each_preview_adapter(cases::dependencies);
}
#[test]
fn padded_image_flow_indexed_sampling_and_parameters() {
    each_preview_adapter(cases::images);
}
#[test]
fn depth_blend_stencil_and_render_rollback() {
    each_preview_adapter(cases::attachments);
}
#[test]
fn compute_parameters_outputs_and_rollback() {
    each_preview_adapter(cases::compute);
}
#[test]
fn compute_ids_eight_outputs_and_late_failure() {
    each_preview_adapter(cases::compute_ids);
}
#[test]
fn retained_work_and_reusable_completion() {
    each_preview_adapter(cases::lifetime);
}
