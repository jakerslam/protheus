use super::workflow_contracts::{registered_workflow_validations, tool_family_contracts};

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
