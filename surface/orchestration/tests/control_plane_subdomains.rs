// Layer ownership: tests (regression proof for orchestration surface contracts).
use infring_orchestration_surface_v1::control_plane::{
    assert_contract_consistency, control_plane_api_contract, enforce_authority_domain,
    enforce_subdomain_kernel_input, enforce_subdomain_kernel_output,
    enforce_subdomain_message_boundary, legacy_module_bindings, subdomain_boundaries,
    subdomain_boundary_by_id, ContractViolationKind, SubdomainContract,
};
use infring_orchestration_surface_v1::control_plane::{
    decomposition_planning::DecompositionPlanningContract,
    recovery_escalation::RecoveryEscalationContract,
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

#[test]
fn executable_contract_enforcement_accepts_declared_tokens() {
    assert!(
        enforce_subdomain_kernel_input("intake_normalization", "typed_request_snapshot").is_ok()
    );
    assert!(enforce_subdomain_kernel_output(
        "result_shaping_packaging",
        "result_package_projection"
    )
    .is_ok());
    assert!(enforce_subdomain_message_boundary(
        "decomposition_planning",
        "planning_to_graph_boundary"
    )
    .is_ok());
}

#[test]
fn executable_contract_enforcement_rejects_disallowed_tokens() {
    let denied_input =
        enforce_subdomain_kernel_input("intake_normalization", "deterministic_receipt_authority")
            .expect_err("kernel input should be denied");
    assert_eq!(denied_input.kind, ContractViolationKind::KernelInputDenied);

    let unknown_domain =
        enforce_subdomain_kernel_output("unknown_domain", "result_package_projection")
            .expect_err("unknown subdomain should be rejected");
    assert_eq!(unknown_domain.kind, ContractViolationKind::UnknownSubdomain);

    let denied_boundary = enforce_subdomain_message_boundary(
        "workflow_graph_dependency_tracking",
        "packaging_to_shell_boundary",
    )
    .expect_err("cross-domain message boundary should be denied");
    assert_eq!(
        denied_boundary.kind,
        ContractViolationKind::MessageBoundaryDenied
    );
}

#[test]
fn executable_contract_enforcement_rejects_forbidden_authority_domains() {
    let forbidden = enforce_authority_domain("deterministic_receipt_authority")
        .expect_err("forbidden authority domain should be rejected");
    assert_eq!(
        forbidden.kind,
        ContractViolationKind::ForbiddenAuthorityDomain
    );

    assert!(enforce_authority_domain("result_package_projection").is_ok());
}

#[test]
fn subdomain_trait_contracts_execute_allow_deny_checks() {
    assert!(
        <DecompositionPlanningContract as SubdomainContract>::assert_kernel_input_allowed(
            "typed_request_snapshot"
        )
        .is_ok()
    );

    let denied = <RecoveryEscalationContract as SubdomainContract>::assert_kernel_output_allowed(
        "task_fabric_proposal_envelope",
    )
    .expect_err("recovery escalation cannot emit task-fabric proposals directly");
    assert_eq!(denied.kind, ContractViolationKind::KernelOutputDenied);

    assert!(
        <RecoveryEscalationContract as SubdomainContract>::assert_kernel_output_allowed(
            "recovery_recommendation_envelope"
        )
        .is_ok()
    );
}

#[test]
fn executable_contract_consistency_check_passes() {
    assert!(
        assert_contract_consistency().is_ok(),
        "subdomain declarations should align with global control-plane contract"
    );
}
