    #[test]
    fn chat_ui_tool_diagnostics_treats_unknown_tool_not_found_result_as_not_found() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "unknown",
            "ok": false,
            "result": "tool not found: batch_query is unavailable"
        })]);
        assert_eq!(
            diagnostics
                .get("not_found_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_not_found")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("not_found")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_not_found")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_unknown_tool_not_found_result_as_tool_not_found() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "working on it",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "unknown",
                                "ok": false,
                                "result": "tool not found: batch_query is unavailable"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=unknown-not-found-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/not_found_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_not_found")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn direct_run_chat_ui_not_invoked_without_findings_fail_closes_via_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "analysis pending",
                        "tools": [
                            {
                                "name": "parse_workspace",
                                "status": "ok",
                                "ok": true,
                                "result": ""
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-not-invoked".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .get("error")
                .and_then(Value::as_str),
            Some("web_tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_not_invoked_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("classification_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/not_invoked_fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("fail_close")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_not_invoked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("rerun_with_tool_call")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("immediate")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/auto")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert!(
            payload
                .pointer("/turn/assistant")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("before any search tool call was recorded")
        );
    }

    #[test]
    fn direct_run_chat_ui_low_signal_without_findings_fail_closes_via_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "analysis pending",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "provider_low_signal",
                                "ok": false,
                                "result": ""
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-low-signal".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_low_signal_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/source")
                .and_then(Value::as_str),
            Some("classification_guard")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/status")
                .and_then(Value::as_str),
            Some("provider_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("fail_close")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("narrow_query")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("immediate")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/auto")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(1)
        );
    }
