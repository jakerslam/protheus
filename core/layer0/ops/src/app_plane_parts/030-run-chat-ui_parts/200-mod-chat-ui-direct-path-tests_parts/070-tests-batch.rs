    #[test]
    fn direct_run_chat_ui_meta_diagnostic_is_telemetry_only() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I can answer directly.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "ok": true,
                                "result": "this should be suppressed on meta diagnostics"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=meta-diagnostic-no-tool".to_string(),
            "--message=whats going on? is your web search kicking in randomly again?".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_gate/meta_diagnostic_request")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_gate/requires_live_web")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_gate/should_call_tools")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("tools")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn direct_run_chat_ui_emits_response_workflow_trace_streams_and_exports() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "LangGraph and OpenAI Agents SDK are both widely used.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "ok": true,
                                "query": "top agent frameworks",
                                "result": "LangGraph, OpenAI Agents SDK, AutoGen"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=workflow-trace-streams".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_workflow/contract")
                .and_then(Value::as_str),
            Some("response_workflow_control_plane_trace_v1")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/required")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/submission_status")
                .and_then(Value::as_str),
            Some("awaiting_llm_submission")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/gate_submission/gate_id")
                .and_then(Value::as_str),
            Some("gate_1_need_tool_access_menu")
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/gate_submission/input_shape/type")
                .and_then(Value::as_str),
            Some("multiple_choice")
        );
        assert!(payload
            .pointer("/response_workflow/gates/need_tool_access/gate_submission/llm_submission")
            .is_some_and(Value::is_null));
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/gate_submission/accepted")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_workflow/gates/need_tool_access/gate_submission/resume_token")
                .and_then(Value::as_str),
            Some("gate_1_need_tool_access_menu.awaiting_llm_submission")
        );
        assert!(payload
            .pointer("/response_workflow/gates/need_tool_access/value")
            .is_some_and(Value::is_null));
        assert!(payload
            .pointer("/response_workflow/trace_streams/workflow_state")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));
        assert!(payload
            .pointer("/response_workflow/trace_streams/ui_status")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| {
                row.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("workflow gate presented")
            })));
        assert!(payload
            .pointer("/response_workflow/trace_streams/decision_summary")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));
        assert!(payload
            .pointer("/response_workflow/trace_streams/tool_execution")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty()));

        let jsonl_path = payload
            .pointer("/response_workflow/export/jsonl_path")
            .and_then(Value::as_str)
            .unwrap_or("");
        let json_path = payload
            .pointer("/response_workflow/export/json_path")
            .and_then(Value::as_str)
            .unwrap_or("");
        let timeline_path = payload
            .pointer("/response_workflow/export/timeline_path")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!jsonl_path.is_empty());
        assert!(!json_path.is_empty());
        assert!(!timeline_path.is_empty());
        assert!(Path::new(jsonl_path).exists(), "{jsonl_path}");
        assert!(Path::new(json_path).exists(), "{json_path}");
        assert!(Path::new(timeline_path).exists(), "{timeline_path}");
    }

    #[test]
    fn direct_run_chat_ui_claim_evidence_guard_fail_closes_unverified_routing_claims() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "The system is automatically triggering web searches through backend automation that bypasses my conscious tool selection.",
                        "tools": []
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=claim-evidence-guard".to_string(),
            "--message=why did that happen?".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_unverified_routing_claim")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("claim_evidence_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/routing_claim_guard_applied")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn direct_run_chat_ui_inline_tool_call_guard_repairs_schema_and_suppresses() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "<function=file_list>{\"path\":\".\"",
                        "tools": []
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=inline-tool-schema-repaired".to_string(),
            "--message=try looking at the local directory".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("inline_tool_call_schema_repaired_suppressed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("inline_tool_call_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/schema_repaired")
                .and_then(Value::as_bool),
            Some(true)
        );
        let assistant = payload
            .pointer("/turn/assistant")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!assistant.contains("<function="), "{assistant}");
    }

    #[test]
    fn direct_run_chat_ui_inline_tool_call_guard_fail_closes_invalid_schema() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "<function=file_list>{\"path\":",
                        "tools": []
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=inline-tool-schema-invalid".to_string(),
            "--message=try looking at the local directory".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("inline_tool_call_schema_invalid")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("inline_tool_call_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/schema_valid")
                .and_then(Value::as_bool),
            Some(false)
        );
    }
