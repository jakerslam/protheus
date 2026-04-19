#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::fs;

    #[test]
    fn parse_flags_defaults() {
        let flags = parse_flags(&[]);
        assert_eq!(flags.mode, "serve");
        assert_eq!(flags.host, "127.0.0.1");
        assert_eq!(flags.port, 4173);
        assert_eq!(flags.team, "ops");
    }

    #[test]
    fn parse_flags_overrides() {
        let flags = parse_flags(&[
            "snapshot".to_string(),
            "--host=0.0.0.0".to_string(),
            "--port=8080".to_string(),
            "--team=alpha".to_string(),
            "--refresh-ms=5000".to_string(),
            "--pretty=0".to_string(),
        ]);
        assert_eq!(flags.mode, "snapshot");
        assert_eq!(flags.host, "0.0.0.0");
        assert_eq!(flags.port, 8080);
        assert_eq!(flags.team, "alpha");
        assert_eq!(flags.refresh_ms, 5000);
        assert!(!flags.pretty);
    }

    #[test]
    fn parse_json_loose_supports_multiline_logs() {
        let raw = "noise\n{\"ok\":false}\n{\"ok\":true,\"type\":\"x\"}\n";
        let parsed = parse_json_loose(raw).expect("json");
        assert_eq!(parsed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn recommended_conduit_signals_scales_with_pressure() {
        assert_eq!(recommended_conduit_signals(5, 0.10, 1, 0), 4);
        assert!(recommended_conduit_signals(80, 0.70, 4, 120) >= 12);
        assert_eq!(recommended_conduit_signals(120, 0.95, 2, 0), 16);
    }

    #[test]
    fn merge_profile_agents_adds_profile_rows_and_excludes_archived() {
        let root = tempfile::tempdir().expect("tempdir");
        let profiles_path = root.path().join(AGENT_PROFILES_REL);
        let archived_path = root.path().join(ARCHIVED_AGENTS_REL);
        if let Some(parent) = profiles_path.parent() {
            fs::create_dir_all(parent).expect("mkdir profiles");
        }
        fs::write(
            &profiles_path,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_agent_profiles",
                "agents": {
                    "runtime-a": { "role": "analyst", "updated_at": "2026-03-28T00:00:00Z" },
                    "profile-b": { "role": "orchestrator", "updated_at": "2026-03-28T01:00:00Z" },
                    "archived-c": { "role": "analyst", "updated_at": "2026-03-28T02:00:00Z" }
                }
            }))
            .expect("json profiles"),
        )
        .expect("write profiles");
        fs::write(
            &archived_path,
            serde_json::to_string_pretty(&json!({
                "type": "infring_dashboard_archived_agents",
                "agents": {
                    "archived-c": { "reason": "timeout" }
                }
            }))
            .expect("json archived"),
        )
        .expect("write archived");

        let mut collab = json!({
            "ok": true,
            "type": "collab_plane_dashboard",
            "dashboard": {
                "team": "ops",
                "agents": [
                    { "shadow": "runtime-a", "role": "analyst", "status": "running" }
                ],
                "tasks": [],
                "handoff_history": []
            }
        });
        dashboard_agent_state::merge_profiles_into_collab(root.path(), &mut collab, "ops");
        let rows = collab
            .get("dashboard")
            .and_then(|v| v.get("agents"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = rows
            .iter()
            .filter_map(|row| row.get("shadow").and_then(Value::as_str))
            .map(ToString::to_string)
            .collect::<HashSet<_>>();
        assert!(ids.contains("runtime-a"));
        assert!(ids.contains("profile-b"));
        assert!(!ids.contains("archived-c"));
    }

    #[test]
    fn runtime_apply_telemetry_remediations_action_is_rust_handled() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.runtime.applyTelemetryRemediations",
            &json!({ "team": "ops" }),
        );
        assert!(lane.ok);
        assert_eq!(lane.status, 0);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("infring_dashboard_runtime_action")
        );
        assert_eq!(
            payload.get("action").and_then(Value::as_str),
            Some("apply_telemetry_remediations")
        );
    }

    #[test]
    fn dashboard_agent_actions_round_trip_through_rust_authority() {
        let root = tempfile::tempdir().expect("tempdir");
        let model_catalog = run_action(root.path(), "dashboard.models.catalog", &json!({}));
        assert!(model_catalog.ok);
        let route_decision = run_action(
            root.path(),
            "dashboard.model.routeDecision",
            &json!({"task_type":"general","offline_required":false}),
        );
        assert!(route_decision.ok);
        let terminal_create = run_action(
            root.path(),
            "dashboard.terminal.session.create",
            &json!({"id":"term-test"}),
        );
        assert!(terminal_create.ok);
        let terminal_exec = run_action(
            root.path(),
            "dashboard.terminal.exec",
            &json!({"session_id":"term-test","command":"printf 'ok'"}),
        );
        assert!(terminal_exec.ok);
        assert_eq!(
            terminal_exec
                .payload
                .clone()
                .unwrap_or_else(|| json!({}))
                .get("stdout")
                .and_then(Value::as_str),
            Some("ok")
        );
        let terminal_close = run_action(
            root.path(),
            "dashboard.terminal.session.close",
            &json!({"session_id":"term-test"}),
        );
        assert!(terminal_close.ok);
        let upsert_profile = run_action(
            root.path(),
            "dashboard.agent.upsertProfile",
            &json!({
                "agent_id": "agent-a",
                "role": "analyst",
                "name": "Agent A"
            }),
        );
        assert!(upsert_profile.ok);

        let append_turn = run_action(
            root.path(),
            "dashboard.agent.session.appendTurn",
            &json!({
                "agent_id": "agent-a",
                "user": "Can you reduce queue depth before spikes?",
                "assistant": "Yes, running mitigation now."
            }),
        );
        assert!(append_turn.ok);

        let create_session = run_action(
            root.path(),
            "dashboard.agent.session.create",
            &json!({
                "agent_id": "agent-a",
                "label": "Deep Work"
            }),
        );
        assert!(create_session.ok);
        let active_session = create_session
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!active_session.is_empty());

        let switch_session = run_action(
            root.path(),
            "dashboard.agent.session.switch",
            &json!({
                "agent_id": "agent-a",
                "session_id": active_session
            }),
        );
        assert!(switch_session.ok);

        let set_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.set",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic",
                "value": "reliability"
            }),
        );
        assert!(set_memory.ok);

        let get_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.get",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic"
            }),
        );
        assert!(get_memory.ok);
        assert_eq!(
            get_memory
                .payload
                .clone()
                .unwrap_or_else(|| json!({}))
                .get("value")
                .and_then(Value::as_str),
            Some("reliability")
        );

        let delete_memory = run_action(
            root.path(),
            "dashboard.agent.memoryKv.delete",
            &json!({
                "agent_id": "agent-a",
                "key": "focus.topic"
            }),
        );
        assert!(delete_memory.ok);

        let suggestions = run_action(
            root.path(),
            "dashboard.agent.suggestions",
            &json!({
                "agent_id": "agent-a",
                "hint": "\"Can you reduce queue depth before spikes?\""
            }),
        );
        assert!(suggestions.ok);
        let suggestion_rows = suggestions
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(suggestion_rows.len() <= 3);
        for row in suggestion_rows {
            let text = row.as_str().unwrap_or("");
            assert!(!text.contains('"'));
            assert!(!text.contains('\''));
        }

        let upsert_contract = run_action(
            root.path(),
            "dashboard.agent.upsertContract",
            &json!({
                "agent_id": "agent-a",
                "created_at": "2000-01-01T00:00:00Z",
                "expiry_seconds": 1,
                "status": "active"
            }),
        );
        assert!(upsert_contract.ok);
        let enforce_contracts =
            run_action(root.path(), "dashboard.agent.enforceContracts", &json!({}));
        assert!(enforce_contracts.ok);
        let terminated_rows = enforce_contracts
            .payload
            .clone()
            .unwrap_or_else(|| json!({}))
            .get("terminated")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!terminated_rows.is_empty());
    }

    #[test]
    fn snapshot_history_trim_applies_age_and_line_caps() {
        let root = tempfile::tempdir().expect("tempdir");
        let history_path = root.path().join(SNAPSHOT_HISTORY_REL);
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent).expect("mkdir history");
        }
        let stale_ts = (Utc::now() - Duration::days(9)).to_rfc3339();
        let recent_ts = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        let mut rows = Vec::<String>::new();
        for idx in 0..4 {
            rows.push(
                serde_json::to_string(
                    &json!({"ts": stale_ts, "type": "snapshot", "idx": idx, "age": "stale"}),
                )
                .expect("stale row"),
            );
        }
        for idx in 4..9 {
            rows.push(
                serde_json::to_string(
                    &json!({"ts": recent_ts, "type": "snapshot", "idx": idx, "age": "recent"}),
                )
                .expect("recent row"),
            );
        }
        fs::write(&history_path, format!("{}\n", rows.join("\n"))).expect("write history");
        trim_snapshot_history_with_policy(&history_path, 10_000, 3, 7);

        let lines = fs::read_to_string(&history_path)
            .expect("read history")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        let cutoff = Utc::now() - Duration::days(7);
        for line in lines {
            let row = serde_json::from_str::<Value>(&line).expect("line json");
            let ts = row
                .get("ts")
                .and_then(Value::as_str)
                .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                .map(|v| v.with_timezone(&Utc))
                .expect("snapshot ts");
            assert!(ts >= cutoff);
        }
    }

    #[test]
    fn snapshot_history_trim_applies_byte_cap() {
        let root = tempfile::tempdir().expect("tempdir");
        let history_path = root.path().join(SNAPSHOT_HISTORY_REL);
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent).expect("mkdir history");
        }
        let ts = Utc::now().to_rfc3339();
        let mut rows = Vec::<String>::new();
        for idx in 0..16 {
            rows.push(
                serde_json::to_string(&json!({
                    "ts": ts,
                    "type": "snapshot",
                    "idx": idx,
                    "payload": "x".repeat(120)
                }))
                .expect("row"),
            );
        }
        fs::write(&history_path, format!("{}\n", rows.join("\n"))).expect("write history");
        trim_snapshot_history_with_policy(&history_path, 700, 100, 30);
        let len = fs::metadata(&history_path).expect("metadata").len();
        assert!(len <= 700, "trimmed bytes should honor cap: {len}");
    }

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

    #[test]
    fn dashboard_troubleshooting_report_message_dedupes_identical_outbox_request() {
        let root = tempfile::tempdir().expect("tempdir");
        let first = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-dedupe",
                "message_id":"msg-dedupe",
                "note":"dedupe check",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(first.ok);
        let second = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-dedupe",
                "message_id":"msg-dedupe-2",
                "note":"dedupe check",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(second.ok);
        let second_payload = second.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            second_payload
                .pointer("/outbox_item/deduped")
                .and_then(Value::as_bool),
            Some(true)
        );
        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_flush_reports_retry_timing() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-retry",
                "message_id":"msg-retry",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true}),
        );
        assert!(flush.ok);
        let payload = flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("failed_count").and_then(Value::as_i64),
            Some(1)
        );
        assert!(
            payload
                .get("next_retry_after_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
        assert!(
            payload
                .get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_flush_auth_missing_sets_auth_retry_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-auth-retry",
                "message_id":"msg-auth-retry",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true}),
        );
        assert!(flush.ok);
        let flush_payload = flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            flush_payload.get("auth_blocked_count").and_then(Value::as_i64),
            Some(1)
        );
        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/items/0/retry_lane")
                .and_then(Value::as_str),
            Some("auth_required")
        );
        assert!(
            state_payload
                .pointer("/issue_outbox/items/0/retry_after_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 3600
        );
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/error_histogram/0/error_bucket")
                .and_then(Value::as_str),
            Some("auth_missing")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_preview_is_non_destructive() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-preview",
                "message_id":"msg-preview",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let before_state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        let before_payload = before_state.payload.unwrap_or_else(|| json!({}));
        let before_depth = before_payload
            .pointer("/issue_outbox/depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let preview = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.preview",
            &json!({"max_items": 5}),
        );
        assert!(preview.ok);
        let preview_payload = preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_flush_preview")
        );
        assert_eq!(
            preview_payload.get("dry_run").and_then(Value::as_bool),
            Some(true)
        );
        let after_state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        let after_payload = after_state.payload.unwrap_or_else(|| json!({}));
        let after_depth = after_payload
            .pointer("/issue_outbox/depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert_eq!(before_depth, after_depth);
        assert!(
            preview_payload
                .pointer("/error_histogram")
                .and_then(Value::as_array)
                .is_some()
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_state_lane_reports_depth_and_histogram() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-outbox-state",
                "message_id":"msg-outbox-state",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.state",
            &json!({"limit": 10}),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
        assert_eq!(payload.get("depth").and_then(Value::as_i64), Some(1));
        assert!(
            payload
                .pointer("/error_histogram")
                .and_then(Value::as_array)
                .is_some()
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .pointer("/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/family")
                .and_then(Value::as_str),
            Some("dashboard_queue_pressure_contract_v1")
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/producer")
                .and_then(Value::as_str),
            Some("dashboard.troubleshooting")
        );
        assert_eq!(
            payload
                .get("queue_pressure_contract_family")
                .and_then(Value::as_str),
            Some("dashboard_queue_pressure_contract_v1")
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/priority")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_priority")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/action_hint")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_action_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_escalation_lane")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_runbook_id")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/escalation_owner")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_escalation_owner")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64),
            payload
                .get("queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_breach_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_blocking_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_auto_retry_allowed")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_execution_policy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_manual_gate_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_manual_gate_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_requeue_strategy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_can_execute_without_human")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/execution_window")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_execution_window")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64),
            payload
                .get("queue_pressure_manual_gate_timeout_seconds")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/next_action_after_seconds")
                .and_then(Value::as_i64),
            payload
                .get("queue_pressure_next_action_after_seconds")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/next_action_kind")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_next_action_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/retry_window_class")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_retry_window_class")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/readiness_state")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_readiness_state")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/readiness_reason")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_readiness_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/automation_safe")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_automation_safe")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload.pointer("/queue_pressure_contract/decision_vector"),
            payload.get("queue_pressure_decision_vector")
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_vector_key")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_vector_key")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_vector/decision_vector_key")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_vector_key")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_route_hint")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_route_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_urgency_tier")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_urgency_tier")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_retry_budget_class")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_retry_budget_class")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_lane_token")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_lane_token")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_dispatch_mode")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_dispatch_mode")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_manual_ack_required")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_decision_manual_ack_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_execution_guard")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_execution_guard")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_followup_required")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_decision_followup_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_vector/execution_guard")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_execution_guard")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_vector/followup_required")
                .and_then(Value::as_bool),
            payload
                .get("queue_pressure_decision_followup_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queue_pressure_contract/decision_vector_version")
                .and_then(Value::as_str),
            payload
                .get("queue_pressure_decision_vector_version")
                .and_then(Value::as_str)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_lane_reports_recommendations_and_queue_depths() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "tooling failed",
            "tools": [],
            "response_finalization": {
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "hard_guard": {"applied": true}
            },
            "error": "web_tool_not_invoked"
        });
        let _capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "find agent frameworks",
            &lane_payload,
            false,
            true,
        );
        let summary = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({"limit": 10}),
        );
        assert!(summary.ok);
        let payload = summary.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
        assert!(
            payload
                .pointer("/recent/failure_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
        assert!(
            payload
                .pointer("/recommendations")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );
        let overview = run_action(
            root.path(),
            "dashboard.troubleshooting.overview",
            &json!({"limit": 10}),
        );
        assert!(overview.ok);
        let overview_payload = overview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/queues/outbox_depth").and_then(Value::as_i64),
            overview_payload.pointer("/queues/outbox_depth").and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/family")
                .and_then(Value::as_str),
            Some("dashboard_queue_pressure_contract_v1")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/producer")
                .and_then(Value::as_str),
            Some("dashboard.troubleshooting")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract_family")
                .and_then(Value::as_str),
            Some("dashboard_queue_pressure_contract_v1")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/priority")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_priority")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/action_hint")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_action_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_escalation_lane")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_runbook_id")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/escalation_owner")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_escalation_owner")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64),
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_blocking_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_auto_retry_allowed")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_execution_policy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_manual_gate_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_manual_gate_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_requeue_strategy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_can_execute_without_human")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_window")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_execution_window")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64),
            payload
                .pointer("/queues/outbox_health/queue_pressure_manual_gate_timeout_seconds")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/next_action_after_seconds")
                .and_then(Value::as_i64),
            payload
                .pointer("/queues/outbox_health/queue_pressure_next_action_after_seconds")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/next_action_kind")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_next_action_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/retry_window_class")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_retry_window_class")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/readiness_state")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_readiness_state")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/readiness_reason")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_readiness_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/automation_safe")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_automation_safe")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload.pointer("/queues/outbox_health/queue_pressure_contract/decision_vector"),
            payload.pointer("/queues/outbox_health/queue_pressure_decision_vector")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_vector_key")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_vector_key")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_vector/decision_vector_key")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_vector_key")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_route_hint")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_route_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_urgency_tier")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_urgency_tier")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_retry_budget_class")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_retry_budget_class")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_lane_token")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_lane_token")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_dispatch_mode")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_dispatch_mode")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_manual_ack_required")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_manual_ack_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_execution_guard")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_execution_guard")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_followup_required")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_followup_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_vector/execution_guard")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_execution_guard")
                .and_then(Value::as_str)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_vector/followup_required")
                .and_then(Value::as_bool),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_followup_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract/decision_vector_version")
                .and_then(Value::as_str),
            payload
                .pointer("/queues/outbox_health/queue_pressure_decision_vector_version")
                .and_then(Value::as_str)
        );
    }

    #[test]
    fn dashboard_troubleshooting_pressure_contract_object_aliases_route_with_parity() {
        let root = tempfile::tempdir().expect("tempdir");

        let outbox_base = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.state",
            &json!({"limit": 10}),
        );
        let outbox_alias = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_object",
            &json!({"limit": 10}),
        );
        assert!(outbox_base.ok);
        assert!(outbox_alias.ok);
        let outbox_base_payload = outbox_base.payload.unwrap_or_else(|| json!({}));
        let outbox_alias_payload = outbox_alias.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/version")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/version")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/family")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/family")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/priority")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/priority")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/action_hint")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/action_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/execution_window")
                .and_then(Value::as_str),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/execution_window")
                .and_then(Value::as_str)
        );
        assert_eq!(
            outbox_base_payload
                .pointer("/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64),
            outbox_alias_payload
                .pointer("/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64)
        );

        let summary_base = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({"limit": 10}),
        );
        let summary_alias = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_object",
            &json!({"limit": 10}),
        );
        assert!(summary_base.ok);
        assert!(summary_alias.ok);
        let summary_base_payload = summary_base.payload.unwrap_or_else(|| json!({}));
        let summary_alias_payload = summary_alias.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/version")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/version")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/snapshot_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/family")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/family")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/priority")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/priority")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/action_hint")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/action_hint")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/escalation_lane")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/runbook_id")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/deadline_epoch_s")
                .and_then(Value::as_i64)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/breach_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/blocking_kind")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/auto_retry_allowed")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_policy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_required")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_reason")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/requeue_strategy")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/can_execute_without_human")
                .and_then(Value::as_bool)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_window")
                .and_then(Value::as_str),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/execution_window")
                .and_then(Value::as_str)
        );
        assert_eq!(
            summary_base_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64),
            summary_alias_payload
                .pointer("/queues/outbox_health/queue_pressure_contract/manual_gate_timeout_seconds")
                .and_then(Value::as_i64)
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_state_and_requeue_flow() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter",
                "message_id":"msg-deadletter",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);

        let first_flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        assert!(first_flush.ok);
        let first_payload = first_flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            first_payload.get("failed_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            first_payload.get("quarantined_count").and_then(Value::as_i64),
            Some(0)
        );

        let second_flush = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        assert!(second_flush.ok);
        let second_payload = second_flush.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            second_payload.get("quarantined_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            second_payload.get("deadletter_depth").and_then(Value::as_i64),
            Some(1)
        );

        let deadletter_state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        assert!(deadletter_state.ok);
        let deadletter_payload = deadletter_state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            deadletter_payload.get("depth").and_then(Value::as_i64),
            Some(1)
        );

        let requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"max_items": 1}),
        );
        assert!(requeue.ok);
        let requeue_payload = requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            requeue_payload.get("requeued_count").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            requeue_payload
                .get("deadletter_depth_after")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            requeue_payload.get("outbox_depth_after").and_then(Value::as_i64),
            Some(1)
        );

        let state = run_action(root.path(), "dashboard.troubleshooting.state", &json!({"limit": 10}));
        assert!(state.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload
                .pointer("/issue_deadletter/depth")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            state_payload
                .pointer("/issue_outbox/depth")
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_requeue_supports_item_filter_and_purge() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-filter",
                "message_id":"msg-deadletter-filter",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let deadletter_state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        assert!(deadletter_state.ok);
        let deadletter_payload = deadletter_state.payload.unwrap_or_else(|| json!({}));
        let item_id = deadletter_payload
            .pointer("/items/0/row/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!item_id.is_empty());

        let no_match_requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"item_ids": ["does-not-exist"], "max_items": 5}),
        );
        assert!(no_match_requeue.ok);
        let no_match_payload = no_match_requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            no_match_payload.get("requeued_count").and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            no_match_payload
                .get("selected_filter_applied")
                .and_then(Value::as_bool),
            Some(true)
        );

        let matched_requeue = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue",
            &json!({"item_ids": [item_id], "max_items": 5}),
        );
        assert!(matched_requeue.ok);
        let matched_payload = matched_requeue.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            matched_payload.get("requeued_count").and_then(Value::as_i64),
            Some(1)
        );

        let purge = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge",
            &json!({"all": true}),
        );
        assert!(purge.ok);
        let purge_payload = purge.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            purge_payload.get("remaining_depth").and_then(Value::as_i64),
            Some(0)
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_purge_requires_selector() {
        let root = tempfile::tempdir().expect("tempdir");
        let purge = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge",
            &json!({}),
        );
        assert!(purge.ok);
        let payload = purge.payload.unwrap_or_else(|| json!({}));
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            payload.get("error").and_then(Value::as_str),
            Some("deadletter_purge_selector_required")
        );
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_preview_lanes_are_non_destructive() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-preview",
                "message_id":"msg-deadletter-preview",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let state_before = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let state_before_payload = state_before.payload.unwrap_or_else(|| json!({}));
        let before_depth = state_before_payload
            .get("depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let requeue_preview = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.requeue.preview",
            &json!({"max_items": 5}),
        );
        assert!(requeue_preview.ok);
        let requeue_preview_payload = requeue_preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            requeue_preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_deadletter_requeue_preview")
        );
        let purge_preview = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.purge.preview",
            &json!({"all": true}),
        );
        assert!(purge_preview.ok);
        let purge_preview_payload = purge_preview.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            purge_preview_payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_deadletter_purge_preview")
        );
        let state_after = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let state_after_payload = state_after.payload.unwrap_or_else(|| json!({}));
        let after_depth = state_after_payload
            .get("depth")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert_eq!(before_depth, after_depth);
    }

    #[test]
    fn dashboard_troubleshooting_deadletter_inspect_alias_matches_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let report = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"session-deadletter-alias",
                "message_id":"msg-deadletter-alias",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(report.ok);
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let _ = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.flush",
            &json!({"max_items": 1, "force": true, "max_attempts": 1}),
        );
        let state = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.state",
            &json!({"limit": 10}),
        );
        let inspect = run_action(
            root.path(),
            "dashboard.troubleshooting.deadletter.inspect",
            &json!({"limit": 10}),
        );
        assert!(state.ok);
        assert!(inspect.ok);
        let state_payload = state.payload.unwrap_or_else(|| json!({}));
        let inspect_payload = inspect.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            state_payload.get("depth").and_then(Value::as_i64),
            inspect_payload.get("depth").and_then(Value::as_i64)
        );
    }

    #[test]
    fn request_query_param_extracts_since_hash() {
        let path = "/api/dashboard/snapshot?since=abc123&x=1";
        assert_eq!(request_path_only(path), "/api/dashboard/snapshot");
        assert_eq!(
            request_query_param(path, "since").as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn dashboard_troubleshooting_synthetic_failure_sample_bundle_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "I could not complete the requested web retrieval due to tool surface unavailability.",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "web_invariant": {
                    "classification": "tool_not_invoked"
                },
                "hard_guard": {
                    "applied": true
                }
            },
            "error": "web_tool_not_invoked"
        });

        let capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "try searching for the top agentic frameworks",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(
            capture.get("failure_detected").and_then(Value::as_bool),
            Some(true)
        );

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"sess-synth-1",
                "message_id":"msg-synth-1",
                "note":"synthetic failure for troubleshooting harness verification",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let lane_payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            lane_payload.get("queued").and_then(Value::as_bool),
            Some(true)
        );

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let latest_snapshot =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let latest_eval =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let outbox = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL))
            .unwrap_or_else(|| json!({}));

        assert_eq!(
            recent.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_recent_workflows")
        );
        assert_eq!(
            latest_snapshot.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_snapshot")
        );
        assert_eq!(
            latest_eval.get("type").and_then(Value::as_str),
            Some("dashboard_workflow_eval_report")
        );
        assert_eq!(
            latest_eval.pointer("/eval/model").and_then(Value::as_str),
            Some("gpt-5.4")
        );
        assert_eq!(
            outbox.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_issue_outbox")
        );
        assert_eq!(
            outbox.pointer("/items/0/issue_request/source").and_then(Value::as_str),
            Some("dashboard_report_popup")
        );

        let sample_bundle = json!({
            "sample_kind": "synthetic_troubleshooting_failure_bundle",
            "files": {
                "recent_workflows": recent,
                "latest_snapshot": latest_snapshot,
                "latest_eval_report": latest_eval,
                "issue_outbox": outbox
            }
        });
        println!("=== SYNTHETIC_TROUBLESHOOTING_SAMPLE_BEGIN ===");
        println!(
            "{}",
            serde_json::to_string_pretty(&sample_bundle).expect("sample json")
        );
        println!("=== SYNTHETIC_TROUBLESHOOTING_SAMPLE_END ===");
    }

    #[test]
    fn dashboard_troubleshooting_synthetic_hallucination_bundle_shape() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "Given a tree, you are supposed to list all the leaves in the order of top down and left to right.",
            "tools": [
                {
                    "name": "web_search",
                    "status": "error",
                    "error": "query_result_mismatch",
                    "query": "top agentic frameworks"
                }
            ],
            "response_finalization": {
                "outcome": "classification_guard_low_signal_fail_closed",
                "tool_transaction": {
                    "classification": "low_signal",
                    "status": "degraded"
                },
                "web_invariant": {
                    "classification": "low_signal"
                },
                "hard_guard": {
                    "applied": true
                }
            },
            "error": "query_result_mismatch"
        });

        let capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "find top agentic frameworks and summarize",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(
            capture.get("failure_detected").and_then(Value::as_bool),
            Some(true)
        );

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.report_message",
            &json!({
                "source":"dashboard_report_popup",
                "session_id":"sess-synth-2",
                "message_id":"msg-synth-2",
                "note":"synthetic hallucination style dump for troubleshooting harness verification",
                "__github_issue_mock_auth_missing": true
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(payload.get("queued").and_then(Value::as_bool), Some(true));

        let latest_eval =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        assert_eq!(
            latest_eval.pointer("/eval/model").and_then(Value::as_str),
            Some("gpt-5.4")
        );
        let recommendations = latest_eval
            .get("recommendations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let recommendation_text = recommendations
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(recommendation_text.contains("mismatched-to-intent"));
        assert!(recommendation_text.contains("alignment scoring"));

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let snapshot =
            read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
                .unwrap_or_else(|| json!({}));
        let outbox = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL))
            .unwrap_or_else(|| json!({}));
        let sample_bundle = json!({
            "sample_kind": "synthetic_troubleshooting_hallucination_bundle",
            "files": {
                "recent_workflows": recent,
                "latest_snapshot": snapshot,
                "latest_eval_report": latest_eval,
                "issue_outbox": outbox
            }
        });
        println!("=== SYNTHETIC_HALLUCINATION_SAMPLE_BEGIN ===");
        println!(
            "{}",
            serde_json::to_string_pretty(&sample_bundle).expect("hallucination sample json")
        );
        println!("=== SYNTHETIC_HALLUCINATION_SAMPLE_END ===");
    }

    #[test]
    fn dashboard_troubleshooting_capture_dedupes_repeated_exchange_signature() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "tool path failed",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "hard_guard": {"applied": true}
            },
            "error": "web_tool_not_invoked"
        });
        let first = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
        let second = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
        assert_eq!(first.get("deduped").and_then(Value::as_bool), Some(false));
        assert_eq!(second.get("deduped").and_then(Value::as_bool), Some(true));

        let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
            .unwrap_or_else(|| json!({}));
        let entries = recent
            .get("entries")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].get("repeat_count").and_then(Value::as_i64),
            Some(2)
        );
        assert_eq!(
            entries[0].get("workflow_signal").and_then(Value::as_str),
            Some("error")
        );
    }

    #[test]
    fn dashboard_troubleshooting_state_exposes_recent_and_outbox_window_contracts() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "tool path failed",
            "tools": [],
            "response_finalization": {
                "outcome": "tool_surface_error_fail_closed",
                "tool_transaction": {
                    "classification": "tool_not_invoked",
                    "status": "failed"
                },
                "hard_guard": {"applied": true}
            },
            "error": "web_tool_not_invoked"
        });
        for idx in 0..3 {
            let _ = dashboard_troubleshooting_capture_chat_exchange(
                root.path(),
                "probe-spark8",
                &format!("repeat this failure {idx}"),
                &lane_payload,
                false,
                true,
            );
        }
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.state",
            &json!({"limit": 1}),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload
                .pointer("/recent/window/total_count")
                .and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            payload
                .pointer("/recent/window/visible_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/window/show_top_indicator")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/issue_outbox/window/total_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1,
            true
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_supports_classification_filters() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_degraded"
                        }
                    },
                    {
                        "stale": true,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "classification_filter": ["tool_not_found"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/filters/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/total_entry_count")
                .and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            payload
                .pointer("/recent/classification_histogram/0/classification")
                .and_then(Value::as_str),
            Some("tool_not_found")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_filtered_alias_matches_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    },
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.filtered",
            &json!({
                "error_filter": ["web_tool_not_found"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
        assert_eq!(
            payload.pointer("/filters/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/recent/error_histogram/0/error")
                .and_then(Value::as_str),
            Some("web_tool_not_found")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_supports_wildcard_filters_and_top_cluster() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_degraded"
                        }
                    },
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_surface_degraded",
                            "error_code": "web_tool_surface_unavailable"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "classification_filter": ["tool_surface_*"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/top_classification")
                .and_then(Value::as_str),
            Some("tool_surface_degraded")
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/top_classification_count")
                .and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_reports_no_match_on_filtered_empty_result() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "stale": false,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "error_filter": ["web_tool_surface_*"]
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload.pointer("/filters/no_match").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_by_error_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.by_error",
            &json!({
                "error_filter": ["web_tool_not_found"]
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_state_reports_age_and_max_attempts() {
        let root = tempfile::tempdir().expect("tempdir");
        let outbox_path = root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL);
        if let Some(parent) = outbox_path.parent() {
            fs::create_dir_all(parent).expect("mkdir outbox");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &outbox_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_issue_outbox",
                "items": [
                    {
                        "id": "a",
                        "attempts": 2,
                        "queued_at_epoch_s": now_epoch - 120,
                        "next_retry_after_epoch_s": now_epoch - 5
                    },
                    {
                        "id": "b",
                        "attempts": 5,
                        "queued_at_epoch_s": now_epoch - 60,
                        "next_retry_after_epoch_s": now_epoch + 60
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.state",
            &json!({}),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.get("max_attempts_observed").and_then(Value::as_i64),
            Some(5)
        );
        assert!(
            payload
                .get("oldest_age_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 120
        );
        assert_eq!(
            payload.get("ready_ratio").and_then(Value::as_f64),
            Some(0.5)
        );
        assert_eq!(
            payload.get("blocked_ratio").and_then(Value::as_f64),
            Some(0.5)
        );
        assert_eq!(
            payload.get("oldest_item_id").and_then(Value::as_str),
            Some("a")
        );
        assert_eq!(
            payload.get("next_retry_item_id").and_then(Value::as_str),
            Some("b")
        );
        assert_eq!(
            payload
                .get("retry_due_within_900s_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(payload.get("stale_count").and_then(Value::as_u64), Some(0));
        assert_eq!(payload.get("stale_ratio").and_then(Value::as_f64), Some(0.0));
        assert_eq!(payload.get("fresh_count").and_then(Value::as_u64), Some(2));
        assert_eq!(payload.get("fresh_ratio").and_then(Value::as_f64), Some(1.0));
        assert_eq!(payload.get("aging_count").and_then(Value::as_u64), Some(0));
        assert_eq!(payload.get("aging_ratio").and_then(Value::as_f64), Some(0.0));
        assert_eq!(
            payload.get("queue_action_hint").and_then(Value::as_str),
            Some("increase_flush_frequency_and_monitor_auth")
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_required")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .get("queue_pressure_runbook_id")
                .and_then(Value::as_str),
            Some("runbook.troubleshooting.queue_pressure.medium")
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_owner")
                .and_then(Value::as_str),
            Some("runtime_owner")
        );
        assert_eq!(
            payload
                .get("queue_pressure_sla_minutes")
                .and_then(Value::as_i64),
            Some(15)
        );
        assert_eq!(
            payload
                .get("queue_pressure_escalation_lane")
                .and_then(Value::as_str),
            Some("dashboard.troubleshooting.eval.drain")
        );
        assert!(
            payload
                .get("queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert!(
            payload
                .get("queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .get("queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .get("queue_pressure_contract_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .get("queue_pressure_snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert_eq!(
            payload.get("health_reason").and_then(Value::as_str),
            Some("ready_ratio>=0.40_with_some_cooldown_pressure")
        );
        assert_eq!(
            payload.pointer("/items/0/source_sequence").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/items/0/stale").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload.pointer("/items/0/freshness_tier").and_then(Value::as_str),
            Some("fresh")
        );
        assert_eq!(
            payload.pointer("/items/0/source").and_then(Value::as_str),
            Some("issue_outbox")
        );
        assert!(
            payload
                .pointer("/items/0/age_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 60
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_health_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.health",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_overview_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.overview",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_queue_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.queue",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_freshness_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.freshness",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_health_metrics_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.health.metrics",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_priority_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.priority",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_lane_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_priority_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.priority",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_lane_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_escalation_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.escalation",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_escalation_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.escalation",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_runbook_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.runbook",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_runbook_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.runbook",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_sla_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.sla",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_sla_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.sla",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_escalation_lane_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.escalation_lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_escalation_lane_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.escalation_lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_deadline_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.deadline",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_deadline_remaining_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.deadline_remaining",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_breach_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.breach",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_breach_detected_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.breach_detected",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_deadline_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.deadline",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_deadline_remaining_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.deadline_remaining",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_breach_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.breach",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_breach_detected_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.breach_detected",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_reason_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_breach_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.breach_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_breach_reason_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.breach_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_blocking_kind_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.blocking_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_blocking_kind_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.blocking_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_auto_retry_allowed_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.auto_retry_allowed",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_auto_retry_allowed_alias_routes_to_summary_lane()
    {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.auto_retry_allowed",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_execution_policy_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.execution_policy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_execution_policy_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.execution_policy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_required_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_manual_gate_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_manual_gate_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_requeue_strategy_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.requeue_strategy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_requeue_strategy_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.requeue_strategy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_can_execute_without_human_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.can_execute_without_human",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_can_execute_without_human_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.can_execute_without_human",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_execution_window_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.execution_window",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_execution_window_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.execution_window",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_manual_gate_timeout_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.manual_gate_timeout",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_manual_gate_timeout_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.manual_gate_timeout",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_next_action_after_seconds_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.next_action_after_seconds",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_next_action_after_seconds_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.next_action_after_seconds",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_next_action_kind_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.next_action_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_next_action_kind_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.next_action_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_retry_window_class_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.retry_window_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_retry_window_class_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.retry_window_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_readiness_state_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.readiness_state",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_readiness_state_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.readiness_state",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_readiness_reason_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.readiness_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_readiness_reason_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.readiness_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_automation_safe_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.automation_safe",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_automation_safe_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.automation_safe",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_vector_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_vector",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_vector_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_vector",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_vector_key_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_vector_key",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_vector_key_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_vector_key",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_route_hint_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_route_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_route_hint_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_route_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_urgency_tier_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_urgency_tier",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_urgency_tier_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_urgency_tier",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_retry_budget_class_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_retry_budget_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_retry_budget_class_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_retry_budget_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_lane_token_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_lane_token",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_lane_token_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_lane_token",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_dispatch_mode_alias_routes_to_state_lane()
    {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_dispatch_mode",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_dispatch_mode_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_dispatch_mode",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_manual_ack_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_manual_ack_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_manual_ack_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_manual_ack_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_execution_guard_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_execution_guard",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_execution_guard_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_execution_guard",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_decision_followup_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.decision_followup_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_decision_followup_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.decision_followup_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_version_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_version",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_version_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_version",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_family_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_family",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_family_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_family",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_priority_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_priority",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_priority_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_priority",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_action_hint_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_action_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_action_hint_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_action_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_escalation_lane_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_escalation_lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_escalation_lane_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_escalation_lane",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_runbook_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_runbook",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_runbook_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_runbook",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_owner_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_owner",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_owner_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_owner",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_blocking_kind_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_blocking_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_blocking_kind_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_blocking_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_auto_retry_allowed_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_auto_retry_allowed",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_auto_retry_allowed_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_auto_retry_allowed",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_execution_policy_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_execution_policy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_execution_policy_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_execution_policy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_manual_gate_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_manual_gate_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_manual_gate_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_manual_gate_reason_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_manual_gate_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_manual_gate_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_requeue_strategy_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_requeue_strategy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_requeue_strategy_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_requeue_strategy",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_can_execute_without_human_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_can_execute_without_human",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_can_execute_without_human_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_can_execute_without_human",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_execution_window_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_execution_window",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_execution_window_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_execution_window",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_manual_gate_timeout_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_manual_gate_timeout",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_manual_gate_timeout_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_manual_gate_timeout",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_next_action_after_seconds_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_next_action_after_seconds",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_next_action_after_seconds_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_next_action_after_seconds",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_next_action_kind_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_next_action_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_next_action_kind_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_next_action_kind",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_retry_window_class_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_retry_window_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_retry_window_class_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_retry_window_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_readiness_state_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_readiness_state",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_readiness_state_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_readiness_state",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_readiness_reason_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_readiness_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_readiness_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_readiness_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_automation_safe_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_automation_safe",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_automation_safe_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_automation_safe",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_vector_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_vector",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_vector_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_vector",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_vector_key_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_vector_key",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_vector_key_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_vector_key",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_route_hint_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_route_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_route_hint_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_route_hint",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_urgency_tier_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_urgency_tier",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_urgency_tier_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_urgency_tier",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_retry_budget_class_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_retry_budget_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_retry_budget_class_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_retry_budget_class",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_lane_token_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_lane_token",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_lane_token_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_lane_token",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_dispatch_mode_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_dispatch_mode",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_dispatch_mode_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_dispatch_mode",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_manual_ack_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_manual_ack_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_manual_ack_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_manual_ack_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_execution_guard_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_execution_guard",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_execution_guard_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_execution_guard",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_decision_followup_required_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_decision_followup_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_decision_followup_required_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_decision_followup_required",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_deadline_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_deadline",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_deadline_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_deadline",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_contract_breach_reason_alias_routes_to_state_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.contract_breach_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_contract_breach_reason_alias_routes_to_summary_lane(
    ) {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.contract_breach_reason",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_outbox_pressure_snapshot_alias_routes_to_state_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.outbox.pressure.snapshot",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_outbox_state")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_pressure_snapshot_alias_routes_to_summary_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.pressure.snapshot",
            &json!({}),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_outbox_health_reports_pressure_deadline_and_breach() {
        let root = tempfile::tempdir().expect("tempdir");
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        let outbox_path = root.path().join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL);
        if let Some(parent) = outbox_path.parent() {
            fs::create_dir_all(parent).expect("mkdir outbox");
        }
        fs::write(
            &outbox_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_issue_outbox",
                "items": [
                    {
                        "id": "blocked-stale-item",
                        "attempts": 6,
                        "queued_at_epoch_s": now_epoch - 4_200,
                        "next_retry_after_epoch_s": now_epoch + 600,
                        "error_bucket": "github_issue_auth_missing"
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(root.path(), "dashboard.troubleshooting.summary", &json!({}));
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_tier")
                .and_then(Value::as_str),
            Some("high")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("outbox_oldest_age_seconds>1800")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_window_filter_excludes_old_entries() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "captured_at_epoch_s": now_epoch - 40,
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    },
                    {
                        "captured_at_epoch_s": now_epoch - 400,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.window",
            &json!({
                "window_seconds": 120
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/window/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/recent/failure_rate").and_then(Value::as_f64),
            Some(1.0)
        );
        assert_eq!(
            payload
                .pointer("/top_failure_cluster/severity_tier")
                .and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/health_tier")
                .and_then(Value::as_str),
            Some("empty")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/stale_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/stale_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/fresh_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/fresh_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/aging_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/aging_ratio")
                .and_then(Value::as_f64),
            Some(0.0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/health_reason")
                .and_then(Value::as_str),
            Some("outbox_empty")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_deadline_remaining_seconds")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_reason")
                .and_then(Value::as_str),
            Some("none")
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_breach_detected_at_epoch_s")
                .and_then(Value::as_i64),
            Some(0)
        );
        assert_eq!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_contract_version")
                .and_then(Value::as_str),
            Some("v1")
        );
        assert!(
            payload
                .pointer("/queues/outbox_health/queue_pressure_snapshot_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= now_epoch
        );
        assert_eq!(
            payload
                .pointer("/recent/classification_histogram/0/classification")
                .and_then(Value::as_str),
            Some("low_signal")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_recent_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.recent",
            &json!({
                "window_seconds": 3600
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_metrics_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.metrics",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_health_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.health",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_queue_health_alias_routes_to_summary() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.queue_health",
            &json!({
                "window_seconds": 300
            }),
        );
        assert!(lane.ok);
        assert_eq!(
            lane.payload
                .unwrap_or_else(|| json!({}))
                .get("type")
                .and_then(Value::as_str),
            Some("dashboard_troubleshooting_summary")
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_accepts_comma_separated_error_filter() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    },
                    {
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");

        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary",
            &json!({
                "error_filter": "web_tool_not_found,web_tool_low_signal"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/recent/entry_count").and_then(Value::as_u64),
            Some(2)
        );
    }

    #[test]
    fn dashboard_troubleshooting_summary_by_time_accepts_minutes_and_reports_filtered_out_count() {
        let root = tempfile::tempdir().expect("tempdir");
        let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent).expect("mkdir troubleshooting");
        }
        let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
        fs::write(
            &recent_path,
            serde_json::to_string_pretty(&json!({
                "type": "dashboard_troubleshooting_recent_workflows",
                "entries": [
                    {
                        "captured_at_epoch_s": now_epoch - 30,
                        "workflow": {
                            "classification": "low_signal",
                            "error_code": "web_tool_low_signal"
                        }
                    },
                    {
                        "captured_at_epoch_s": now_epoch - 900,
                        "workflow": {
                            "classification": "tool_not_found",
                            "error_code": "web_tool_not_found"
                        }
                    }
                ]
            }))
            .expect("json"),
        )
        .expect("write");
        let lane = run_action(
            root.path(),
            "dashboard.troubleshooting.summary.by_time",
            &json!({
                "window_minutes": 2
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            payload.pointer("/window/applied").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/window/filtered_out_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            payload.pointer("/window/window_seconds").and_then(Value::as_i64),
            Some(120)
        );
    }
}

#[test]
fn dashboard_troubleshooting_capture_reports_loop_detection_after_repeats() {
    let root = tempfile::tempdir().expect("tempdir");
    let lane_payload = json!({
        "response": "tool path failed",
        "tools": [],
        "response_finalization": {
            "outcome": "tool_surface_error_fail_closed",
            "tool_transaction": {
                "classification": "tool_not_invoked",
                "status": "failed"
            },
            "hard_guard": {"applied": true}
        },
        "error": "web_tool_not_invoked"
    });
    for _ in 0..6 {
        let _ = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
    }
    let recent = read_json_file(&root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL))
        .unwrap_or_else(|| json!({}));
    let entry = recent
        .get("entries")
        .and_then(Value::as_array)
        .and_then(|rows| rows.last())
        .cloned()
        .unwrap_or_else(|| json!({}));
    assert_eq!(
        entry.pointer("/loop_detection/detected").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        entry.pointer("/loop_detection/level").and_then(Value::as_str),
        Some("critical")
    );
    assert_eq!(
        entry
            .pointer("/workflow/loop_detection/detector")
            .and_then(Value::as_str),
        Some("generic_repeat")
    );
    assert_eq!(
        entry.pointer("/loop_detection/lane").and_then(Value::as_str),
        Some("tool_completion")
    );
    assert_eq!(
        entry
            .pointer("/loop_detection/recovery_hint")
            .and_then(Value::as_str),
        Some("pause_tool_calls_and_run_targeted_web_tooling_retry_audit")
    );
    assert_eq!(
        entry
            .pointer("/workflow/loop_detection/restart_workflow")
            .and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn dashboard_troubleshooting_state_exposes_recent_loop_detection() {
    let root = tempfile::tempdir().expect("tempdir");
    let lane_payload = json!({
        "response": "tool path failed",
        "tools": [],
        "response_finalization": {
            "outcome": "tool_surface_error_fail_closed",
            "tool_transaction": {
                "classification": "tool_not_invoked",
                "status": "failed"
            },
            "hard_guard": {"applied": true}
        },
        "error": "web_tool_not_invoked"
    });
    for _ in 0..4 {
        let _ = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "probe-spark8",
            "repeat this failure",
            &lane_payload,
            false,
            true,
        );
    }
    let lane = run_action(
        root.path(),
        "dashboard.troubleshooting.state",
        &json!({"limit": 5}),
    );
    assert!(lane.ok);
    let payload = lane.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        payload
            .pointer("/recent/loop_detection/detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/loop_detection/level")
            .and_then(Value::as_str),
        Some("warning")
    );
    assert_eq!(
        payload
            .pointer("/recent/loop_detection/restart_workflow")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/loop_detection/lane")
            .and_then(Value::as_str),
        Some("tool_completion")
    );
}

#[test]
fn dashboard_troubleshooting_state_supports_offset_from_latest_windowing() {
    let root = tempfile::tempdir().expect("tempdir");
    let lane_payload = json!({
        "response": "tool path failed",
        "tools": [],
        "response_finalization": {
            "outcome": "tool_surface_error_fail_closed",
            "tool_transaction": {"classification":"tool_surface_degraded"},
            "hard_guard": {"applied": false},
            "error_code": "web_tool_surface_degraded"
        },
        "response_workflow": {
            "gates": {
                "route": {"route": "task"}
            }
        }
    });
    for idx in 0..5 {
        let prompt = format!("repeat failure #{idx}");
        dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "agent-off",
            &prompt,
            &lane_payload,
            false,
            true,
        );
    }
    let lane = run_action(
        root.path(),
        "dashboard.troubleshooting.state",
        &json!({"limit": 2, "offset_from_latest": 1}),
    );
    assert!(lane.ok);
    let payload = lane.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        payload
            .pointer("/recent/window/total_count")
            .and_then(Value::as_u64),
        Some(5)
    );
    assert_eq!(
        payload
            .pointer("/recent/window/visible_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        payload
            .pointer("/recent/window/show_top_indicator")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/window/show_bottom_indicator")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/window/offset_from_latest")
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn dashboard_troubleshooting_summary_exposes_lane_health_and_recovery_hints() {
    let root = tempfile::tempdir().expect("tempdir");
    let recent_path = root.path().join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
    if let Some(parent) = recent_path.parent() {
        fs::create_dir_all(parent).expect("mkdir troubleshooting");
    }
    fs::write(
        &recent_path,
        serde_json::to_string_pretty(&json!({
            "type": "dashboard_troubleshooting_recent_workflows",
            "entries": [
                {
                    "lane_ok": false,
                    "stale": false,
                    "loop_detection": {"level": "warning"},
                    "workflow": {
                        "classification": "tool_surface_degraded",
                        "error_code": "web_tool_surface_degraded",
                        "transaction_status": "failed"
                    }
                },
                {
                    "lane_ok": false,
                    "stale": false,
                    "loop_detection": {"level": "none"},
                    "workflow": {
                        "classification": "context_mismatch",
                        "error_code": "context_alignment_mismatch",
                        "transaction_status": "failed"
                    }
                },
                {
                    "lane_ok": true,
                    "stale": false,
                    "loop_detection": {"level": "none"},
                    "workflow": {
                        "classification": "response_synthesized",
                        "error_code": "",
                        "transaction_status": "completed"
                    }
                }
            ]
        }))
        .expect("json"),
    )
    .expect("write");

    let lane = run_action(
        root.path(),
        "dashboard.troubleshooting.summary",
        &json!({ "limit": 10 }),
    );
    assert!(lane.ok);
    let payload = lane.payload.unwrap_or_else(|| json!({}));
    assert_eq!(
        payload
            .pointer("/recent/lane_health/tool_completion/failed")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/lane_health/continuity/failed")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/lane_health_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/window_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/stale_rate_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/queue_pressure_not_high")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_gate_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/provider_resolution_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_watchdog_not_triggered")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_completion_signal_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_manual_intervention_not_required")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_contract_version_supported")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_next_action_routable")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_llm_reliability_not_low")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_hallucination_pattern_not_detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_placeholder_output_not_detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_final_response_contract_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_no_result_pattern_not_detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_answer_contract_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_not_blocked")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_severity_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_severity_bucket_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_score_band_severity_bucket_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_escalation_routable")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_escalation_lane_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_escalation_reason_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_escalation_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_escalation_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_decision_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_decision_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_budget_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_reason_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_reason_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_primary_blocker_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blockers_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_severity_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_manual_review_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_priority_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_set_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_set_key_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_count_key_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_expected_blocker_count_matches")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_blocker_flags_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/checks/tooling_response_gate_contract_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/contract_version")
            .and_then(Value::as_str),
        Some("v1")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/tool_lane_rows")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/provider_missing_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/watchdog_warning_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/watchdog_triggered")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/completion_signal_missing_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/completion_signal_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/provider_quality_tier")
            .and_then(Value::as_str),
        Some("poor")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/manual_intervention_required")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/context_mismatch_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/hallucination_pattern_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/invalid_draft_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/placeholder_output_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/no_result_pattern_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/answer_emitted_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/answer_missing_after_completion_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/answer_contract_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/answer_signal_coverage")
            .and_then(Value::as_f64),
        Some(0.0)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/hallucination_pattern_detected")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/placeholder_output_detected")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/no_result_pattern_detected")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/llm_reliability_tier")
            .and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/llm_reliability_not_low")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/final_response_contract_violation_count")
            .and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/final_response_contract_ok")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/ready")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/requires_manual_review")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score")
            .and_then(Value::as_f64),
        Some(0.3)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_score")
            .and_then(Value::as_f64),
        Some(0.3)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band")
            .and_then(Value::as_str),
        Some("weak")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_score_band")
            .and_then(Value::as_str),
        Some("weak")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_vector_key")
            .and_then(Value::as_str),
        Some("score=0.3000;severity=blocked")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_score_vector_key")
            .and_then(Value::as_str),
        Some("score=0.3000;severity=blocked")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_vector_key")
            .and_then(Value::as_str),
        Some("band=weak;score=0.3000")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_score_band_vector_key")
            .and_then(Value::as_str),
        Some("band=weak;score=0.3000")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_severity_from_score_band")
            .and_then(Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_severity_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_severity_bucket_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/score_band_severity_bucket_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/severity")
            .and_then(Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_count")
            .and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/primary_blocker")
            .and_then(Value::as_str),
        Some("final_response_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/primary_blocker_expected")
            .and_then(Value::as_str),
        Some("final_response_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/primary_blocker_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blockers_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/severity_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_priority_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_set_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blockers/0")
            .and_then(Value::as_str),
        Some("final_response_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blockers/1")
            .and_then(Value::as_str),
        Some("answer_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_set_key")
            .and_then(Value::as_str),
        Some("final_response_contract|answer_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blocker_set_key")
            .and_then(Value::as_str),
        Some("final_response_contract|answer_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_set_key_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_count_key_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blocker_count")
            .and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blocker_count_matches")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_budget_max")
            .and_then(Value::as_i64),
        Some(4)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_budget_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_vector_key")
            .and_then(Value::as_str),
        Some("count=2;set=final_response_contract|answer_contract;primary=final_response_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blocker_vector_key")
            .and_then(Value::as_str),
        Some("count=2;set=final_response_contract|answer_contract;primary=final_response_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_flags_key")
            .and_then(Value::as_str),
        Some("final_response_contract=true;answer_contract=true;llm_reliability=false;watchdog=false")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_blocker_flags_key")
            .and_then(Value::as_str),
        Some("final_response_contract=true;answer_contract=true;llm_reliability=false;watchdog=false")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_flags_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/signature_key")
            .and_then(Value::as_str),
        Some("ready=false;severity=blocked;primary=final_response_contract;lane=dashboard.troubleshooting.recent.state;reason=finalization_integrity_failure;count=2;set=final_response_contract|answer_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_signature_key")
            .and_then(Value::as_str),
        Some("ready=false;severity=blocked;primary=final_response_contract;lane=dashboard.troubleshooting.recent.state;reason=finalization_integrity_failure;count=2;set=final_response_contract|answer_contract")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_lane")
            .and_then(Value::as_str),
        Some("dashboard.troubleshooting.recent.state")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_lane_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_reason_code")
            .and_then(Value::as_str),
        Some("finalization_integrity_failure")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_reason_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_vector_key")
            .and_then(Value::as_str),
        Some(
            "final_response_contract|dashboard.troubleshooting.recent.state|finalization_integrity_failure"
        )
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_escalation_vector_key")
            .and_then(Value::as_str),
        Some(
            "final_response_contract|dashboard.troubleshooting.recent.state|finalization_integrity_failure"
        )
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/decision_vector_key")
            .and_then(Value::as_str),
        Some("blocked|false|true")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_decision_vector_key")
            .and_then(Value::as_str),
        Some("blocked|false|true")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/decision_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/requires_manual_review")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_requires_manual_review")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_reason")
            .and_then(Value::as_str),
        Some("gated_response_not_ready")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_manual_review_reason")
            .and_then(Value::as_str),
        Some("gated_response_not_ready")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_reason_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_reason_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_vector_key")
            .and_then(Value::as_str),
        Some("required=true;reason=gated_response_not_ready")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/expected_manual_review_vector_key")
            .and_then(Value::as_str),
        Some("required=true;reason=gated_response_not_ready")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_vector_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/manual_review_signature_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/decision_vector_known")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/blocker_count_matches")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/primary_blocker_matches")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/escalation_contract_ok")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/response_gate/contract_consistent")
            .and_then(Value::as_bool),
        Some(true)
    );
    let response_gate_blockers = payload
        .pointer("/recent/tooling_contract/response_gate/blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    assert!(
        response_gate_blockers
            .iter()
            .any(|row| row == "final_response_contract"),
        "expected final_response_contract blocker in {:?}",
        response_gate_blockers
    );
    assert!(
        response_gate_blockers
            .iter()
            .any(|row| row == "answer_contract"),
        "expected answer_contract blocker in {:?}",
        response_gate_blockers
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/decision_confidence")
            .and_then(Value::as_f64),
        Some(0.6)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/requires_snapshot")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/next_action_lane")
            .and_then(Value::as_str),
        Some("dashboard.troubleshooting.recent.state")
    );
    assert_eq!(
        payload
            .pointer("/recent/tooling_contract/next_action_routable")
            .and_then(Value::as_bool),
        Some(true)
    );

    let hints = payload
        .pointer("/recent/recovery_hints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    assert!(
        hints
            .iter()
            .any(|row| row.contains("Tool completion lane degraded")),
        "expected tool completion recovery hint in {:?}",
        hints
    );
    assert!(
        hints.iter().any(|row| row.contains("High severity cluster")),
        "expected high severity recovery hint in {:?}",
        hints
    );
}
