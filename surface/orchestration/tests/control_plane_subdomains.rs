// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::control_plane::{
    control_plane_api_contract, legacy_module_bindings, subdomain_boundaries,
    subdomain_boundary_by_id,
};

fn require_domain(id: &str) -> infring_orchestration_surface_v1::control_plane::SubdomainBoundary {
    subdomain_boundary_by_id(id).expect("missing control-plane subdomain boundary")
}

#[test]
fn intake_normalization_subdomain_contract_is_present() {
    let row = require_domain("intake_normalization");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(row
        .message_boundaries
        .contains(&"ingress_to_planning_boundary"));
}

#[test]
fn decomposition_planning_subdomain_contract_is_present() {
    let row = require_domain("decomposition_planning");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(row
        .message_boundaries
        .contains(&"planning_to_graph_boundary"));
}

#[test]
fn workflow_graph_subdomain_contract_is_present() {
    let row = require_domain("workflow_graph_dependency_tracking");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(row
        .message_boundaries
        .contains(&"graph_to_packaging_boundary"));
}

#[test]
fn recovery_escalation_subdomain_contract_is_present() {
    let row = require_domain("recovery_escalation");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(row
        .message_boundaries
        .contains(&"recovery_to_packaging_boundary"));
}

#[test]
fn result_packaging_subdomain_contract_is_present() {
    let row = require_domain("result_shaping_packaging");
    assert!(!row.legacy_module_bindings.is_empty());
    assert!(!row.allowed_kernel_inputs.is_empty());
    assert!(!row.allowed_kernel_outputs.is_empty());
    assert!(row
        .message_boundaries
        .contains(&"packaging_to_shell_boundary"));
}

#[test]
fn control_plane_api_contract_enforces_kernel_boundary_rules() {
    let contract = control_plane_api_contract();
    assert!(contract
        .allowed_kernel_inputs
        .contains(&"core_probe_envelope"));
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

#[test]
fn control_plane_subdomain_ids_are_unique() {
    let mut ids = subdomain_boundaries()
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5);
}

#[test]
fn legacy_module_bindings_are_unique_and_traceable() {
    let modules = legacy_module_bindings();
    assert!(modules
        .iter()
        .any(|row| row.module == "planner/plan_candidates"
            && row.subdomain_id == "decomposition_planning"));
    assert!(modules
        .iter()
        .any(|row| row.module == "result_packaging"
            && row.subdomain_id == "result_shaping_packaging"));

    let mut unique_pairs = modules
        .iter()
        .map(|row| format!("{}::{}", row.module, row.subdomain_id))
        .collect::<Vec<_>>();
    unique_pairs.sort();
    unique_pairs.dedup();
    assert_eq!(unique_pairs.len(), modules.len());

    let progress_domains = modules
        .iter()
        .filter(|row| row.module == "progress")
        .map(|row| row.subdomain_id)
        .collect::<Vec<_>>();
    assert_eq!(
        progress_domains,
        vec![
            "workflow_graph_dependency_tracking",
            "result_shaping_packaging"
        ]
    );
}
