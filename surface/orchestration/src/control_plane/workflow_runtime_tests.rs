use super::workflow_runtime::{
    adapt_tool_request, run_registered_replay_fixtures, run_workflow_replay,
    workflow_runtime_contract_ok,
};
use super::workflow_runtime_fixtures::{WorkflowInput, WorkflowReplayFixture};

#[test]
fn runtime_replays_cover_core_tool_and_abort_paths_without_chat_injection() {
    let reports = run_registered_replay_fixtures();
    assert!(workflow_runtime_contract_ok(&reports), "{reports:#?}");
    assert_eq!(reports.len(), 5);
    assert!(reports.iter().all(|report| {
        report
            .events
            .iter()
            .all(|event| event.stream == "final_answer" || !event.visible_chat_eligible)
    }));
}

#[test]
fn runtime_exports_separated_inspector_streams_and_budget_state() {
    let reports = run_registered_replay_fixtures();
    for report in reports {
        assert_eq!(
            report.inspector.selected_graph_source,
            "orchestration_typed_graph_v1"
        );
        assert!(!report.graph_hash.is_empty());
        assert!(report
            .source_json_path
            .starts_with("surface/orchestration/src/control_plane/workflows/"));
        assert!(report.source_json_path.ends_with(".workflow.json"));
        assert_eq!(
            report.contract_schema_version,
            "typed_execution_contract_v1"
        );
        assert_eq!(report.inspector.workflow_id, report.workflow_id);
        assert_eq!(report.inspector.graph_hash, report.graph_hash);
        assert_eq!(report.inspector.source_json_path, report.source_json_path);
        assert_eq!(
            report.inspector.contract_schema_version,
            report.contract_schema_version
        );
        assert!(report.budget.loop_guard_active);
        assert!(!report.budget.budget_exceeded);
        assert!(!report.budget.loop_signature_repeated);
        for stream in [
            "workflow_state",
            "agent_internal_notes",
            "tool_trace",
            "eval_trace",
            "final_answer",
        ] {
            assert!(
                report.inspector.trace_streams.contains_key(stream),
                "missing stream {stream} for {}",
                report.fixture_id
            );
        }
        assert!(!report.inspector.system_chat_injection_allowed);
        assert!(report
            .inspector
            .tool_family_diagnostics
            .iter()
            .any(|row| row.family == "workspace"));
        assert!(report.inspector.tool_family_diagnostics.iter().all(|row| {
            row.status == "menu_available_probe_required_before_execution"
                && row.reason == "workflow_reader_exposes_family_without_autoselection"
        }));
    }
}

#[test]
fn llm_menu_adapter_builds_typed_tool_request_without_recommending_tools() {
    let request = adapt_tool_request("workspace", "search_workspace", "workflow policy")
        .expect("workspace request");
    assert_eq!(request.family, "workspace");
    assert_eq!(request.request_schema, "workspace_tool_request_v1");
    assert!(request.receipt_binding_required);
    assert!(adapt_tool_request("unknown", "search", "payload").is_err());
}

#[test]
fn unregistered_workflow_ids_fail_closed_instead_of_using_private_stage_graphs() {
    let report = run_workflow_replay(&WorkflowReplayFixture {
        id: "unregistered_private_stage_graph",
        workflow_id: "private_hardcoded_chat_stage_graph",
        user_input: "run a private hardcoded workflow",
        inputs: vec![WorkflowInput::FinalAnswer("should not run")],
    });

    assert!(!report.ok, "{report:#?}");
    assert_eq!(report.terminal_state, "failed");
    assert!(report
        .failures
        .iter()
        .any(|reason| reason == "unregistered_workflow_id:private_hardcoded_chat_stage_graph"));
    assert!(report.events.iter().any(|event| {
        event.stream == "eval_trace" && event.event_kind == "workflow_selection_rejected"
    }));
}
