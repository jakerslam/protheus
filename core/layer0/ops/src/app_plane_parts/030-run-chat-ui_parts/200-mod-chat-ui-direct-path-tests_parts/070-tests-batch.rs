    #[test]
    fn direct_run_chat_ui_meta_diagnostic_forces_no_tool_route() {
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
            Some(0)
        );
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
