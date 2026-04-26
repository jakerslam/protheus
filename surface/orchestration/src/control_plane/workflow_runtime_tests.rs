use super::workflow_runtime::{
    adapt_tool_request, run_registered_replay_fixtures, workflow_runtime_contract_ok,
};

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
