    #[test]
    fn chat_ui_placeholder_rewrite_prioritizes_surface_degraded_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_surface_degraded": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_withheld_surface_degraded");
        assert!(rewritten.is_empty(), "{rewritten}");
    }

    #[test]
    fn direct_run_chat_ui_rewrites_route_classifier_ghost_copy() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "The gate system is functioning as designed - it's a decision tree that automatically classifies requests as either \"info\" (direct conversation) or \"task\" (tool workflow) routes based on semantic analysis of your input. [source:tool_decision_tree_v3]",
                        "tools": []
                    }
                ]
            }),
        );
        let payload = run_chat_ui(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-ui".to_string(),
                "--session-id=route-ghost-rewrite".to_string(),
                "--message=try again".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "run",
        );
        let assistant = payload.pointer("/turn/assistant").and_then(Value::as_str).unwrap_or("");
        let lowered = assistant.to_ascii_lowercase();
        assert!(
            lowered.contains("automatic info/task route classification is disabled")
                || lowered.contains("i could not produce a reliable response for your last message in this turn"),
            "{assistant}"
        );
        assert!(!lowered.contains("decision tree that automatically classifies"), "{assistant}");
        assert!(!lowered.contains("[source:tool_decision_tree_v3]"), "{assistant}");
    }

    #[test]
    fn chat_ui_view_logs_returns_trace_matches_for_request_id() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "done",
                        "tools": [
                            {"name":"batch_query","status":"ok","ok":true,"result":"found"}
                        ]
                    }
                ]
            }),
        );
        let run_payload = run_chat_ui(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-ui".to_string(),
                "--session-id=view-logs-demo".to_string(),
                "--message=search docs".to_string(),
                "--strict=1".to_string(),
            ]),
            true,
            "run",
        );
        let trace_id = run_payload
            .get("trace_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!trace_id.is_empty());
        let logs_payload = run_chat_ui(
            root.path(),
            &crate::parse_args(&[
                "run".to_string(),
                "--app=chat-ui".to_string(),
                "--session-id=view-logs-demo".to_string(),
                format!("--request-id={trace_id}"),
                "--strict=1".to_string(),
            ]),
            true,
            "view-logs",
        );
        assert_eq!(logs_payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            logs_payload.get("match_count").and_then(Value::as_u64),
            Some(1)
        );
        assert!(logs_payload
            .get("matches")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn chat_ui_history_includes_semantic_tool_receipt_summary() {
        let session = json!({
            "turns": [
                {
                    "user": "search rust tracing",
                    "assistant": "done",
                    "tool_summary": "Tool transaction complete for intent \"search rust tracing\": total=1 success=1 failed=0 blocked=0 not_found=0 low_signal=0 silent_failure=0."
                }
            ]
        });
        let history = chat_ui_history_messages(&session);
        assert!(history.iter().any(|row| {
            row.get("role").and_then(Value::as_str) == Some("assistant")
                && row
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("tool receipt summary")
        }));
    }

    #[test]
    fn chat_ui_tool_gate_system_prompt_is_compact_yes_no_exit_gate() {
        let prompt = chat_ui_tool_gate_system_prompt("hey");
        assert!(prompt.contains("Need tools? Yes/No"), "{prompt}");
        assert!(prompt.contains("No"), "{prompt}");
        assert!(prompt.contains("Yes"), "{prompt}");
        assert!(
            prompt.len() < 700,
            "simple first gate should stay compact, got {} chars: {prompt}",
            prompt.len()
        );
        assert!(!prompt.contains("parse_workspace"), "{prompt}");
        assert!(!prompt.contains("batch_query"), "{prompt}");
        assert!(!prompt.contains("tool_menu_by_family"), "{prompt}");
        assert!(!prompt.contains("request_example"), "{prompt}");
    }

    #[test]
    fn chat_ui_no_tool_trace_uses_simple_conversation_visibility() {
        let root = tempfile::tempdir().expect("tempdir");
        let gate = chat_ui_turn_tool_decision_tree("hey");
        let trace = chat_ui_build_response_workflow_trace(
            root.path(),
            "simple-exit-session",
            "trace-simple-exit",
            "hey",
            "Hey - I'm here.",
            &gate,
            &[],
            "not_required",
            "ok",
            false,
            "",
            "",
            false,
        );
        assert_eq!(
            trace
                .pointer("/selected_workflow/name")
                .and_then(Value::as_str),
            Some("simple_conversation_v1")
        );
        assert_eq!(
            trace
                .pointer("/gates/need_tool_access/question")
                .and_then(Value::as_str),
            Some("Need tools? Yes/No")
        );
        assert_eq!(
            trace
                .pointer("/process_position/current_stage")
                .and_then(Value::as_str),
            Some("llm_final_output")
        );
        let ui_messages = trace
            .pointer("/trace_streams/ui_status")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(ui_messages.iter().any(|row| {
            row.get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("Need tools? Yes/No")
        }));
    }
