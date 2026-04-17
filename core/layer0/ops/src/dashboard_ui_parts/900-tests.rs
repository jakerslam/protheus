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
    fn request_query_param_extracts_since_hash() {
        let path = "/api/dashboard/snapshot?since=abc123&x=1";
        assert_eq!(request_path_only(path), "/api/dashboard/snapshot");
        assert_eq!(
            request_query_param(path, "since").as_deref(),
            Some("abc123")
        );
    }
}
