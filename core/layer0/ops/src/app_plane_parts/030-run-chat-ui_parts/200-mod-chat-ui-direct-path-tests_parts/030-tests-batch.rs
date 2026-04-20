    #[test]
    fn direct_run_chat_ui_mixed_surface_signals_report_unavailable_canonically() {
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
                                "status": "error",
                                "error": "web_search_tool_surface_degraded"
                            },
                            {
                                "name": "batch_query",
                                "status": "error",
                                "error": "web_fetch_tool_surface_unavailable"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=surface-mixed-priority".to_string(),
            "--message=try searching for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_surface_error_code")
                .and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/web_invariant/tool_surface_error_code")
                .and_then(Value::as_str),
            Some("web_tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("tool_surface_unavailable")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("forced_web_tool_surface_unavailable")
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_emits_explicit_execution_receipts() {
        let diagnostics = chat_ui_tool_diagnostics(&[
            json!({"name":"batch_query","status":"ok","ok":true,"result":"found docs"}),
            json!({"name":"parse_workspace","status":"failed","error":"tool not found"}),
            json!({"name":"spawn_subagents"}),
        ]);
        assert_eq!(
            diagnostics
                .get("not_found_calls")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            1
        );
        assert_eq!(
            diagnostics
                .get("silent_failure_calls")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            1
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(receipts.len(), 3);
        assert!(receipts.iter().all(|row| {
            row.get("call_id")
                .and_then(Value::as_str)
                .map(|value| value.starts_with("toolcall_"))
                .unwrap_or(false)
        }));
        assert!(receipts.iter().any(|row| {
            row.get("tool").and_then(Value::as_str) == Some("parse_workspace")
                && row.get("status").and_then(Value::as_str) == Some("not_found")
        }));
        assert!(receipts.iter().any(|row| {
            row.get("tool").and_then(Value::as_str) == Some("spawn_subagents")
                && row.get("status").and_then(Value::as_str) == Some("unknown")
        }));
    }

    #[test]
    fn chat_ui_tool_diagnostics_preserves_surface_error_code_from_status_only_rows() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "web_tool_surface_degraded",
            "ok": false,
            "error": ""
        })]);
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_surface_degraded")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("error_code").and_then(Value::as_str)
                == Some("web_tool_surface_degraded")
        }));
    }

    #[test]
    fn chat_ui_tool_diagnostics_preserves_surface_error_code_from_result_only_rows() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "error",
            "ok": false,
            "error": "",
            "result": "provider failed: web_fetch_tool_surface_unavailable"
        })]);
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_surface_unavailable")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("error_code").and_then(Value::as_str)
                == Some("web_tool_surface_unavailable")
                && row.get("status").and_then(Value::as_str) == Some("error")
        }));
        assert_eq!(
            diagnostics
                .get("surface_unavailable_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_counts_surface_classes_separately() {
        let diagnostics = chat_ui_tool_diagnostics(&[
            json!({
                "name": "batch_query",
                "status": "web_tool_surface_unavailable",
                "ok": false
            }),
            json!({
                "name": "batch_query",
                "status": "web_tool_surface_degraded",
                "ok": false
            }),
        ]);
        assert_eq!(
            diagnostics
                .get("surface_unavailable_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .get("surface_degraded_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_treats_policy_denied_status_as_blocked() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "policy_denied",
            "ok": false
        })]);
        assert_eq!(
            diagnostics
                .get("blocked_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_policy_blocked")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("blocked")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_policy_blocked")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_policy_denied_status_as_policy_blocked() {
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
                                "status": "policy_denied",
                                "ok": false
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=policy-denied-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/blocked_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_policy_blocked")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn chat_ui_tool_diagnostics_treats_provider_low_signal_status_as_low_signal() {
        let diagnostics = chat_ui_tool_diagnostics(&[json!({
            "name": "batch_query",
            "status": "provider_low_signal",
            "ok": false
        })]);
        assert_eq!(
            diagnostics
                .get("low_signal_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            diagnostics
                .pointer("/error_codes/web_tool_low_signal")
                .and_then(Value::as_i64),
            Some(1)
        );
        let receipts = diagnostics
            .get("execution_receipts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(receipts.iter().any(|row| {
            row.get("status").and_then(Value::as_str) == Some("low_signal")
                && row.get("error_code").and_then(Value::as_str) == Some("web_tool_low_signal")
        }));
    }

    #[test]
    fn direct_run_chat_ui_classifies_provider_low_signal_status_as_low_signal() {
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
                                "status": "provider_low_signal",
                                "ok": false
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=provider-low-signal-classification".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("low_signal")
        );
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_low_signal")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/low_signal_calls")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_diagnostics/error_codes/web_tool_low_signal")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/consistent")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
