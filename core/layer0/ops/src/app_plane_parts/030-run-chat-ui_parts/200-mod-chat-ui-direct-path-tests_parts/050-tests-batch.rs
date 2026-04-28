    #[test]
    fn direct_run_chat_ui_healthy_with_findings_does_not_apply_classification_guard() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "Here are current top agentic AI frameworks.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "ok": true,
                                "result": "LangGraph docs captured; OpenAI Agents SDK docs captured"
                            }
                        ]
                    }
                ]
            }),
        );
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--app=chat-ui".to_string(),
            "--session-id=classification-guard-healthy".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/classification")
                .and_then(Value::as_str),
            Some("healthy")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/fail_closed")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/mode")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload.pointer("/response_finalization/classification_guard/active_error_code"),
            Some(&Value::Null)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_plan/attempts")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/tool_transaction/complete")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(payload.get("error"), None);
    }

    #[test]
    fn direct_run_chat_ui_placeholder_with_policy_blocked_uses_policy_guard_fallback() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I'll get you an update on the current best AI agent frameworks.",
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
            "--session-id=placeholder-policy-blocked".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/status")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/hard_guard/error_code")
                .and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_policy_blocked_fail_closed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
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
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_policy_blocked")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("operator_policy_action")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("blocked")
        );
    }

    #[test]
    fn direct_run_chat_ui_not_found_without_findings_fail_closes_via_classification_guard() {
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
            "--session-id=classification-guard-tool-not-found".to_string(),
            "--message=search for current top agent frameworks".to_string(),
            "--strict=1".to_string(),
        ]);
        let payload = run_chat_ui(root.path(), &parsed, true, "run");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/outcome")
                .and_then(Value::as_str),
            Some("classification_guard_tool_not_found_fail_closed")
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
            Some("failed")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/applied")
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
                .pointer("/response_finalization/classification_guard/fail_closed_class")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/active_error_code")
                .and_then(Value::as_str),
            Some("web_tool_not_found")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_strategy")
                .and_then(Value::as_str),
            Some("adjust_tool_selection")
        );
        assert_eq!(
            payload
                .pointer("/response_finalization/classification_guard/retry_lane")
                .and_then(Value::as_str),
            Some("blocked")
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_missing_required_web_calls_as_failed() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 0,
                "successful_calls": 0,
                "failed_calls": 0,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction failed"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_surface_degraded_as_degraded() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 1,
                "successful_calls": 0,
                "failed_calls": 1,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0,
                "error_codes": {
                    "web_tool_surface_degraded": 1
                }
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction degraded"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_receipt_summary_marks_surface_unavailable_as_failed() {
        let summary = chat_ui_semantic_receipt_summary(
            &json!({
                "total_calls": 1,
                "successful_calls": 0,
                "failed_calls": 1,
                "blocked_calls": 0,
                "not_found_calls": 0,
                "low_signal_calls": 0,
                "silent_failure_calls": 0,
                "error_codes": {
                    "web_tool_surface_unavailable": 1
                }
            }),
            true,
            "latest agent frameworks",
        );
        assert!(
            summary.to_ascii_lowercase().contains("tool transaction failed"),
            "{summary}"
        );
        assert!(
            summary
                .to_ascii_lowercase()
                .contains("surface_unavailable=1"),
            "{summary}"
        );
    }

    #[test]
    fn chat_ui_placeholder_rewrite_returns_canonical_error_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_auth_missing": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_detected_auth");
        assert_eq!(rewritten, "Web search completed.");
    }

    #[test]
    fn chat_ui_placeholder_rewrite_prioritizes_surface_unavailable_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            "Web search completed.",
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_surface_unavailable": 1,
                    "web_tool_error": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_detected_surface_unavailable");
        assert_eq!(rewritten, "Web search completed.");
    }

    #[test]
    fn chat_ui_placeholder_detection_emits_telemetry_without_replacing_visible_chat() {
        let original = "Web search completed.";
        let (rewritten, outcome) = rewrite_chat_ui_placeholder_with_tool_diagnostics(
            original,
            &json!({
                "total_calls": 1,
                "error_codes": {
                    "web_tool_policy_blocked": 1
                }
            }),
        );
        assert_eq!(outcome, "placeholder_detected_policy");
        assert_eq!(rewritten, original);
    }

    #[test]
    fn chat_ui_route_classifier_ghost_copy_rewrite_replaces_legacy_text() {
        let (rewritten, outcome) = rewrite_chat_ui_legacy_route_classifier_copy(
            "The first gate (\"workflow_route\") is a binary classification that determines whether the system routes the request through a workflow (task route) or handles it as a direct conversational response (info route). It's not a true/false decision I control - it's an automated classification based on semantic analysis of the user's input. [source:tool_decision_tree_v3]",
        );
        assert_eq!(outcome, "legacy_route_classifier_copy_detected");
        assert!(rewritten.contains("[source:tool_decision_tree_v3]"), "{rewritten}");
    }

    #[test]
    fn chat_ui_route_classifier_ghost_copy_rewrite_strips_extended_source_tags_and_bypass_copy() {
        let (rewritten, outcome) = rewrite_chat_ui_legacy_route_classifier_copy(
            "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route. [source:workflow_route_classification] [source:gate_enforcement_mode] [source:tool_decision_policy] I do have web search capabilities, but the system is currently in conversation bypass mode which restricts tool usage.",
        );
        assert_eq!(outcome, "legacy_route_classifier_copy_detected");
        assert!(rewritten.contains("[source:workflow_route_classification]"), "{rewritten}");
    }

    #[test]
    fn chat_ui_legacy_route_detection_emits_telemetry_without_replacing_visible_chat() {
        let original = "The first gate (\"workflow_route\") is still classifying this as an \"info\" route rather than a \"task\" route. [source:workflow_route_classification]";
        let (rewritten, outcome) = rewrite_chat_ui_legacy_route_classifier_copy(original);
        assert_eq!(outcome, "legacy_route_classifier_copy_detected");
        assert_eq!(rewritten, original);
    }

    #[test]
    fn chat_ui_route_classifier_ghost_copy_rewrite_keeps_normal_text() {
        let (rewritten, outcome) = rewrite_chat_ui_legacy_route_classifier_copy(
            "Need tools? Yes/No",
        );
        assert_eq!(outcome, "unchanged");
        assert_eq!(rewritten, "Need tools? Yes/No");
    }
