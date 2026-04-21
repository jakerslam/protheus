// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::control_plane::{
    control_plane_api_contract, subdomain_boundaries,
};

fn require_domain(id: &str) -> infring_orchestration_surface_v1::control_plane::SubdomainBoundary {
    subdomain_boundaries()
        .into_iter()
        .find(|row| row.id == id)
        .expect("missing control-plane subdomain boundary")
}

#[test]
fn intake_normalization_subdomain_contract_is_present() {
    let row = require_domain("intake_normalization");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(
        row.message_boundaries
            .contains(&"ingress_to_planning_boundary")
    );
}

#[test]
fn decomposition_planning_subdomain_contract_is_present() {
    let row = require_domain("decomposition_planning");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(
        row.message_boundaries
            .contains(&"planning_to_graph_boundary")
    );
}

#[test]
fn workflow_graph_subdomain_contract_is_present() {
    let row = require_domain("workflow_graph_dependency_tracking");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(
        row.message_boundaries
            .contains(&"graph_to_packaging_boundary")
    );
}

#[test]
fn recovery_escalation_subdomain_contract_is_present() {
    let row = require_domain("recovery_escalation");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(
        row.message_boundaries
            .contains(&"recovery_to_packaging_boundary")
    );
}

#[test]
fn result_packaging_subdomain_contract_is_present() {
    let row = require_domain("result_shaping_packaging");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(
        row.message_boundaries
            .contains(&"packaging_to_shell_boundary")
    );
}

#[test]
fn control_plane_api_contract_enforces_kernel_boundary_rules() {
    let contract = control_plane_api_contract();
    assert!(contract.allowed_kernel_inputs.contains(&"core_probe_envelope"));
    assert!(contract
        .allowed_kernel_outputs
        .contains(&"core_contract_call_envelope"));
    assert!(contract
        .forbidden_authority_domains
        .contains(&"deterministic_receipt_authority"));
    assert!(contract
        .message_boundary_invariants
        .contains(&"kernel_is_final_authority"));
}

