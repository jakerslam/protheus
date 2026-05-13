use super::workflow_contracts::{
    registered_workflow_graphs, registered_workflow_validations, tool_family_contracts,
    workflow_registry_contract_ok, REQUIRED_JSON_OWNS, REQUIRED_RUST_OWNS,
};
use super::workflow_lab_replay::{
    local_coding_program_builder_lab_execution_report, local_coding_program_builder_lab_replay_report,
};
use super::workflow_runtime::select_runtime_workflow;
use serde_json::Value;
use std::path::Path;

#[test]
fn workflow_specs_compile_to_no_injection_graphs() {
    let validations = registered_workflow_validations();
    assert!(validations.iter().all(|row| row.ok), "{validations:?}");
    assert!(validations.iter().all(|row| {
        row.graph
            .as_ref()
            .map(|graph| {
                graph.visible_chat_policy == "llm_final_only_no_system_injection"
                    && graph.interaction_source == "json_workflow_spec"
                    && graph.rust_reader_role == "validate_execute_trace_only"
                    && !graph.hardcoded_interaction_behavior_allowed
                    && !graph.final_response_policy.trim().is_empty()
                    && graph
                        .final_output_contract
                        .get("schema_version")
                        .and_then(|value| value.as_str())
                        .is_some()
                    && REQUIRED_JSON_OWNS
                        .iter()
                        .all(|item| graph.json_owns.iter().any(|row| row == item))
                    && REQUIRED_RUST_OWNS
                        .iter()
                        .all(|item| graph.rust_owns.iter().any(|row| row == item))
            })
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
    assert!(select_runtime_workflow("local_coding_program_builder").is_none());
}

#[test]
fn local_coding_program_builder_declares_master_coding_loop_contract() {
    let graphs = registered_workflow_graphs();
    let graph = graphs
        .iter()
        .find(|graph| graph.workflow_id == "local_coding_program_builder")
        .expect("local coding program builder graph");
    assert_eq!(graph.workflow_tier, "lab");
    assert_eq!(graph.promotion_status, "candidate");
    assert_eq!(graph.workflow_role, "assistant_response_workflow");
    assert!(!graph.runtime_selectable);
    assert_eq!(graph.primitive_level, 3);
    for child_id in [
        "research_synthesize_verify",
        "local_coding_ingress_guard",
        "cli_intent_argument_ingress",
        "interactive_input_session_state",
        "command_prompt_generation",
        "user_prompt_context_assembly",
        "local_coding_session_bootstrap_guard",
        "agent_provider_model_binding",
        "conversation_session_bootstrap",
        "terminal_command_context_snapshot",
        "external_file_change_notice",
        "title_commit_helper_generation",
        "local_policy_permission_guard",
        "forge_config_resolution",
        "operation_permission_gate",
        "local_context_loop_guard",
        "tool_access_resolver",
        "doom_loop_interrupt",
        "pending_todo_completion_gate",
        "context_compaction_summary",
        "tool_error_reflection",
        "agent_task_delegation",
        "local_tooling_surface_guard",
        "tool_schema_registry",
        "tool_call_normalization",
        "mcp_tool_bridge",
        "custom_command_skill_loader",
        "local_runtime_execution_loop",
        "agent_request_transform_pipeline",
        "turn_retry_stream_runner",
        "tool_call_execution_dispatch",
        "lifecycle_hook_dispatch",
        "conversation_state_persistence",
        "local_runtime_observability_guard",
        "chat_response_visibility_router",
        "streaming_markdown_projection",
        "tool_output_display_format",
        "trace_event_rate_limiter",
        "plan_execute_review",
        "plan_artifact_create",
        "local_code_edit_execution",
        "safe_file_read",
        "safe_file_write",
        "safe_file_patch",
        "validation_command_runner",
        "safe_file_undo",
        "followup_clarification_gate",
        "failure_diagnosis",
        "bounded_repair_loop",
        "checkpoint_handoff",
    ] {
        assert!(graph
            .composed_of_workflow_ids
            .iter()
            .any(|row| row == child_id));
    }

    let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/control_plane/workflows/lab/frameworks/coding/local_coding_program_builder.workflow.json",
    );
    let raw = std::fs::read_to_string(source_path).expect("local coding program builder source");
    let source: Value = serde_json::from_str(&raw).expect("local coding program builder json");
    let composition = source
        .get("workflow_composition_contract")
        .expect("composition contract");
    assert_eq!(
        composition.get("cd_kind").and_then(Value::as_str),
        Some("composite")
    );
    assert_eq!(
        composition
            .get("returns_exactly_one_terminal_artifact")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        composition
            .get("child_workflow_calls")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(13)
    );

    let contract = source
        .get("program_builder_contract")
        .expect("program builder contract");
    assert_eq!(
        contract.get("version").and_then(Value::as_str),
        Some("local_coding_program_builder_v1")
    );
    for required_section in [
        "checkpoint_policy",
        "project_initialization_policy",
        "architecture_contract_policy",
        "coding_ingress_guard_contract",
        "session_bootstrap_guard_contract",
        "runtime_execution_loop_contract",
        "runtime_observability_guard_contract",
        "slice_policy",
        "loop_policy",
    ] {
        assert!(
            contract.get(required_section).is_some(),
            "missing {required_section}"
        );
    }
    assert!(contract
        .get("stop_conditions")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str().unwrap_or("").contains("validation fails")))
        .unwrap_or(false));
    assert!(contract
        .get("metrics")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("architecture_drift_events")))
        .unwrap_or(false));

    let state_contract = source
        .get("state_tracking_contract")
        .expect("state tracking contract");
    assert!(state_contract
        .get("required_state_fields")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("architecture_contract")))
        .unwrap_or(false));
    assert!(graph
        .final_output_contract
        .get("required_summary_fields")
        .and_then(Value::as_array)
        .map(|items| items
            .iter()
            .any(|item| item.as_str() == Some("recommended_next_checkpoint")))
        .unwrap_or(false));
}

#[test]
fn local_coding_program_builder_candidate_lab_replay_proof_passes() {
    let report = local_coding_program_builder_lab_replay_report();
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.workflow_id, "local_coding_program_builder");
    assert_eq!(report.promotion_status, "candidate");
    assert!(!report.runtime_selectable);
    assert_eq!(report.primitive_level, 3);
    assert_eq!(report.scenarios.len(), 3);
    for scenario_id in [
        "single_file_utility",
        "small_multi_file_app",
        "initialized_project_modification",
    ] {
        assert!(report
            .scenarios
            .iter()
            .any(|scenario| scenario.id == scenario_id && scenario.ok));
    }
}

#[test]
fn local_coding_program_builder_lab_execution_harness_emits_coding_task_plans() {
    let report = local_coding_program_builder_lab_execution_report();
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.workflow_id, "local_coding_program_builder");
    assert_eq!(
        report.harness_kind,
        "local_coding_program_builder_lab_execution_dry_run_v1"
    );
    assert_eq!(report.task_executions.len(), 3);
    for execution in &report.task_executions {
        assert!(execution.ok, "{execution:#?}");
        assert!(!execution.checkpoint.acceptance_criteria.is_empty());
        assert!(!execution.architecture_contract.boundary_rules.is_empty());
        assert!(!execution.validation_plan.is_empty());
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_code_edit_execution"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_ingress_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_coding_session_bootstrap_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_policy_permission_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_context_loop_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_tooling_surface_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_runtime_execution_loop"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "local_runtime_observability_guard"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "bounded_repair_loop"));
        assert!(execution
            .slice_invocations
            .iter()
            .any(|slice| slice.child_workflow_id == "checkpoint_handoff"));
        assert!(execution
            .final_handoff_fields
            .contains(&"recommended_next_checkpoint"));
    }
    let multi_file = report
        .task_executions
        .iter()
        .find(|execution| execution.task_id == "small_multi_file_app")
        .expect("small multi-file app execution");
    assert!(multi_file
        .slice_invocations
        .iter()
        .any(|slice| slice.name == "domain_model_slice"));
    assert!(multi_file
        .slice_invocations
        .iter()
        .any(|slice| slice.name == "primary_flow_slice"));
}
