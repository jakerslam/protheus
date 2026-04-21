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
        assert_eq!(outcome, "placeholder_replaced_surface_degraded");
        assert!(
            rewritten
                .to_ascii_lowercase()
                .contains("web tool surface is degraded"),
            "{rewritten}"
        );
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
