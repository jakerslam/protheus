use super::workflow_contracts::{
    registered_workflow_graphs, registered_workflow_validations, tool_family_contracts,
    workflow_registry_contract_ok,
};
use super::workflow_runtime::select_runtime_workflow;

#[test]
fn workflow_specs_compile_to_no_injection_graphs() {
    let validations = registered_workflow_validations();
    assert!(validations.iter().all(|row| row.ok), "{validations:?}");
    assert!(validations.iter().all(|row| {
        row.graph
            .as_ref()
            .map(|graph| graph.visible_chat_policy == "llm_final_only_no_system_injection")
            .unwrap_or(false)
    }));
}

#[test]
fn tool_family_contracts_are_receipt_bound_and_non_leaking() {
    let contracts = tool_family_contracts();
    assert_eq!(contracts.len(), 6);
    assert!(contracts.iter().all(|row| {
        row.receipt_binding_required
            && row.visible_chat_leakage_forbidden
            && !row.request_schema.is_empty()
            && !row.observation_schema.is_empty()
    }));
}

#[test]
fn workflow_registry_separates_official_and_lab_profiles() {
    assert!(workflow_registry_contract_ok());
    let graphs = registered_workflow_graphs();
    assert!(graphs.iter().any(|graph| graph.workflow_tier == "official"));
    assert!(graphs.iter().any(|graph| graph.workflow_tier == "lab"));
    assert!(graphs.iter().all(|graph| {
        if graph.workflow_tier == "official" {
            graph.runtime_selectable
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/official/")
        } else {
            !graph.runtime_selectable
                && graph
                    .source_json_path
                    .starts_with("orchestration/src/control_plane/workflows/lab/")
        }
    }));
}

#[test]
fn lab_framework_workflows_are_not_runtime_selectable() {
    assert!(select_runtime_workflow("clarify_then_coordinate").is_some());
    assert!(select_runtime_workflow("openhands_control_plane_assimilation").is_none());
    assert!(select_runtime_workflow("codex_tooling_synthesis").is_none());
}
