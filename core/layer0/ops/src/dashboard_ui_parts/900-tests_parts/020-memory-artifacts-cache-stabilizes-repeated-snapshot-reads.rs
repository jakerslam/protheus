    #[test]
    fn memory_artifacts_cache_stabilizes_repeated_snapshot_reads() {
        let root = tempfile::tempdir().expect("tempdir");
        let state_path = root
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/latest.json");
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).expect("mkdir state");
        }
        fs::write(
            &state_path,
            serde_json::to_string_pretty(&json!({"ok": true, "type": "state"})).expect("json"),
        )
        .expect("write");
        let first = collect_memory_artifacts(root.path());
        let second = collect_memory_artifacts(root.path());
        assert_eq!(first, second, "cache should return stable rows inside cache window");
    }

    #[test]
    fn snapshot_includes_web_tooling_summary_and_checksum() {
        let root = tempfile::tempdir().expect("tempdir");
        let channel_registry = root.path().join(DASHBOARD_CHANNEL_REGISTRY_REL);
        if let Some(parent) = channel_registry.parent() {
            fs::create_dir_all(parent).expect("mkdir channel registry");
        }
        fs::write(
            &channel_registry,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_channel_registry",
                "channels": {
                    "webchat": {
                        "name": "webchat",
                        "configured": true,
                        "requires_token": false,
                        "runtime_supported": true,
                        "connected": true,
                        "web_tooling_ready": true,
                        "transport_kind": "internal",
                        "auth_mode": "none"
                    }
                }
            }))
            .expect("json channel registry"),
        )
        .expect("write channel registry");
        fs::write(
            root.path().join(DASHBOARD_PROVIDER_REGISTRY_REL),
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "openai": {
                        "id": "openai",
                        "auth_status": "configured",
                        "reachable": true,
                        "is_local": false,
                        "needs_key": true
                    }
                }
            }))
            .expect("json provider registry"),
        )
        .expect("write provider registry");

        let flags = parse_flags(&[]);
        let snapshot = build_snapshot(root.path(), &flags);
        assert_eq!(
            snapshot.pointer("/web_tooling/status").and_then(Value::as_str),
            Some("ok")
        );
        let checksum = snapshot
            .pointer("/sync/component_checksums/web_tooling")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!checksum.is_empty());
    }

    #[test]
    fn snapshot_web_tooling_runtime_diagnostics_roll_up_action_history() {
        let root = tempfile::tempdir().expect("tempdir");
        let channel_registry = root.path().join(DASHBOARD_CHANNEL_REGISTRY_REL);
        if let Some(parent) = channel_registry.parent() {
            fs::create_dir_all(parent).expect("mkdir channel registry");
        }
        fs::write(
            &channel_registry,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_channel_registry",
                "channels": {
                    "webchat": {
                        "name": "webchat",
                        "configured": true,
                        "requires_token": false,
                        "runtime_supported": true,
                        "connected": true,
                        "web_tooling_ready": true,
                        "transport_kind": "internal",
                        "auth_mode": "none"
                    }
                }
            }))
            .expect("json channel registry"),
        )
        .expect("write channel registry");
        fs::write(
            root.path().join(DASHBOARD_PROVIDER_REGISTRY_REL),
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_provider_registry",
                "providers": {
                    "openai": {
                        "id": "openai",
                        "auth_status": "configured",
                        "reachable": true,
                        "is_local": false,
                        "needs_key": true
                    }
                }
            }))
            .expect("json provider registry"),
        )
        .expect("write provider registry");

        let action_history = root.path().join(ACTION_HISTORY_REL);
        if let Some(parent) = action_history.parent() {
            fs::create_dir_all(parent).expect("mkdir action history");
        }
        let row = json!({
            "ts": Utc::now().to_rfc3339(),
            "payload": {
                "response_finalization": {
                    "tool_diagnostics": {
                        "total_calls": 3,
                        "search_calls": 2,
                        "fetch_calls": 1,
                        "successful_calls": 0,
                        "failed_calls": 3,
                        "no_result_calls": 1,
                        "error_codes": {
                            "web_tool_invalid_response": 2,
                            "web_tool_policy_blocked": 1
                        }
                    }
                }
            }
        });
        fs::write(
            &action_history,
            format!("{}\n", serde_json::to_string(&row).expect("row json")),
        )
        .expect("write action history");

        let flags = parse_flags(&[]);
        let snapshot = build_snapshot(root.path(), &flags);
        assert_eq!(
            snapshot
                .pointer("/web_tooling/runtime/total_calls")
                .and_then(Value::as_i64),
            Some(3)
        );
        assert_eq!(
            snapshot
                .pointer("/web_tooling/runtime/failed_calls")
                .and_then(Value::as_i64),
            Some(3)
        );
        assert_eq!(
            snapshot
                .pointer("/web_tooling/runtime/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            snapshot
                .pointer("/runtime_autoheal/web_tooling_degraded")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn dashboard_github_issue_create_contract_surface() {
        let root = tempfile::tempdir().expect("tempdir");
        for (payload, expected_error) in [
            (json!({"body":"x"}), "github_issue_title_required"),
            (json!({"title":"x"}), "github_issue_body_required"),
            (json!({"title":"x","body":"y","owner":"bad owner","repo":"InfRing","__github_issue_mock_auth_missing":true}), "github_issue_repo_invalid"),
            (json!({"title":"x","body":"y","__github_issue_mock_auth_missing":true}), "github_issue_auth_missing"),
        ] {
            let lane = run_action(root.path(), "dashboard.github.issue.create", &payload);
            let out = lane.payload.unwrap_or_else(|| json!({}));
            let err = out.get("error").and_then(Value::as_str).unwrap_or("");
            assert!(!lane.ok);
            assert_eq!(out.get("type").and_then(Value::as_str), Some("github_issue_create"));
            assert_eq!(err, expected_error);
            assert!(!err.starts_with("unsupported_action:"));
            if expected_error == "github_issue_auth_missing" {
                assert_eq!(
                    out.get("message").and_then(Value::as_str),
                    Some("no github auth token, please input your token first")
                );
            }
        }
        let lane = run_action(root.path(), "dashboard.github.issue.create", &json!({"title":"Queue pressure report","body":"Please triage queue pressure spike.","source":"dashboard_report_popup","owner":"protheuslabs","repo":"InfRing","__github_issue_mock_token":"test-token","__github_issue_mock_status":201,"__github_issue_mock_body":{"number":777,"html_url":"https://github.com/protheuslabs/InfRing/issues/777","url":"https://api.github.com/repos/protheuslabs/InfRing/issues/777"}}));
        let out = lane.payload.unwrap_or_else(|| json!({}));
        assert!(lane.ok);
        assert_eq!(out.get("type").and_then(Value::as_str), Some("github_issue_create"));
        assert_eq!(out.get("owner").and_then(Value::as_str), Some("protheuslabs"));
        assert_eq!(out.get("repo").and_then(Value::as_str), Some("InfRing"));
        assert_eq!(out.get("number").and_then(Value::as_i64), Some(777));
        assert_eq!(out.get("html_url").and_then(Value::as_str), Some("https://github.com/protheuslabs/InfRing/issues/777"));
        assert_eq!(out.get("issue_url").and_then(Value::as_str), Some("https://api.github.com/repos/protheuslabs/InfRing/issues/777"));
    }

    #[test]
    fn dashboard_troubleshooting_report_message_queues_outbox_and_emits_eval_report() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-a",
                "message_id":"msg-a",
                "note":"web search failed",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_report")
        );
        assert_eq!(payload.get("submitted").and_then(Value::as_bool), Some(false));
        assert_eq!(payload.get("queued").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload
                .pointer("/eval_drain/processed_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/eval_drain/reports/0/eval/model")
                .and_then(Value::as_str),
            Some("gpt-5.4")
        );
        assert_eq!(
            payload
                .pointer("/eval_drain/reports/0/eval/model_strength")
                .and_then(Value::as_str),
            Some("strong")
        );

        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit":10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            state_payload
                .pointer("/eval_queue/depth")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            state_payload
                .pointer("/latest_eval_report/eval/model")
                .and_then(Value::as_str),
            Some("gpt-5.4")
        );
    }

    #[test]
    fn dashboard_troubleshooting_report_message_success_clears_active_context_and_outbox() {
        let root = tempfile::tempdir().expect("tempdir");
        let first = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-b",
                "message_id":"msg-b-1",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(first.ok);
        let initial_state =
            run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit":10}));
        let initial_payload = initial_state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            initial_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );

        let second = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-b",
                "message_id":"msg-b-2",
                "__github_issue_mock_token":"test-token",
                "__github_issue_mock_status":201,
                "__github_issue_mock_body":{
                    "number":41,
                    "html_url":"https://github.com/protheuslabs/InfRing/issues/41",
                    "url":"https://api.github.com/repos/protheuslabs/InfRing/issues/41"
                }
            }),
        );
        assert!(second.ok);
        let second_payload = second.payload.unwrap_or_else(|| json!({}));
        assert_eq!(second_payload.get("submitted").and_then(Value::as_bool), Some(true));
        assert_eq!(second_payload.get("queued").and_then(Value::as_bool), Some(false));
        assert_eq!(
            second_payload
                .pointer("/issue/number")
                .and_then(Value::as_i64),
            Some(41)
        );

        let final_state =
            run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit":10}));
        assert!(final_state.ok);
        let final_payload = final_state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            final_payload.pointer("/recent/count").and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            final_payload
                .pointer("/eval_queue/depth")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            final_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(0)
        );
    }

    #[test]
    fn dashboard_troubleshooting_eval_model_override_flows_to_eval_report() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-c",
                "message_id":"msg-c",
                "eval_model":"gpt-5.4-mini",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload
                .pointer("/eval_drain/reports/0/eval/model")
                .and_then(Value::as_str),
            Some("gpt-5.4-mini")
        );
        assert_eq!(
            payload
                .pointer("/eval_drain/reports/0/eval/model_source")
                .and_then(Value::as_str),
            Some("payload")
        );
    }

    #[test]
    fn dashboard_troubleshooting_report_message_auth_missing_uses_explicit_hint() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-auth-hint",
                "message_id":"msg-auth-hint",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("issue_error").and_then(Value::as_str),
            Some("github_issue_auth_missing")
        );
        assert_eq!(
            payload.get("issue_error_hint").and_then(Value::as_str),
            Some("no github auth token, please input your token first")
        );
    }

