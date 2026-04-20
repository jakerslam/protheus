
#[cfg(test)]
mod continuity_tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let raw = serde_json::to_string_pretty(value).expect("json");
        fs::write(path, raw).expect("write");
    }

    fn run_git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git spawn");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn task_runtime_summary_counts_pending_and_done() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp
                .path()
                .join("local/state/runtime/task_runtime/registry.json"),
            &json!({
                "version": "v1",
                "tasks": [
                    {"id":"a","status":"queued"},
                    {"id":"b","status":"running"},
                    {"id":"c","status":"done"},
                    {"id":"d","status":"cancelled"}
                ]
            }),
        );
        let out = task_runtime_summary(temp.path());
        assert_eq!(out.get("pending").and_then(Value::as_i64), Some(2));
        assert_eq!(out.get("done").and_then(Value::as_i64), Some(1));
        assert_eq!(out.get("cancelled").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn continuity_payload_surfaces_stale_sessions_and_channel_attention() {
        let temp = tempdir().expect("tempdir");
        let stale_iso = (Utc::now() - chrono::Duration::hours(72)).to_rfc3339();
        write_json(
            &temp.path().join(
                "client/runtime/local/state/ui/infring_dashboard/agent_sessions/agent-alpha.json",
            ),
            &json!({
                "agent_id": "agent-alpha",
                "active_session_id": "default",
                "sessions": [
                    {
                        "session_id": "default",
                        "updated_at": stale_iso,
                        "messages": [
                            {"role": "user", "text": "investigate pending deployment"}
                        ]
                    }
                ]
            }),
        );
        write_json(
            &temp
                .path()
                .join("client/runtime/local/state/ui/infring_dashboard/channel_registry.json"),
            &json!({
                "type": "infring_dashboard_channel_registry",
                "channels": {
                    "slack": {
                        "name": "slack",
                        "provider": "slack",
                        "configured": true,
                        "has_token": false,
                        "status": "disconnected"
                    }
                }
            }),
        );

        let out = continuity_pending_payload(temp.path(), &json!({}));
        assert_eq!(
            out.pointer("/sessions/stale_48h_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            out.pointer("/channels/attention_needed_count")
                .and_then(Value::as_i64),
            Some(1)
        );
        assert!(out
            .pointer("/active_agents/rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn proactive_alerts_raise_queue_pressure_signal() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp
                .path()
                .join("local/state/runtime/task_runtime/registry.json"),
            &json!({
                "version": "v1",
                "tasks": (0..24).map(|idx| json!({"id": format!("t-{idx}"), "status": "queued"})).collect::<Vec<_>>()
            }),
        );
        let out = proactive_telemetry_alerts_payload(
            temp.path(),
            &json!({
                "ok": true,
                "health": {
                    "dashboard_metrics": {
                        "queue_depth": { "value": 24 }
                    },
                    "alerts": { "count": 0 }
                }
            }),
        );
        let alerts = out
            .get("alerts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = alerts
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"queue_pressure_high"));
        let next_actions = out
            .get("next_actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_queue_next = next_actions.iter().any(|row| {
            row.get("command")
                .and_then(Value::as_str)
                .map(|cmd| cmd == "/queue")
                .unwrap_or(false)
        });
        assert!(has_queue_next);
    }

    #[test]
    fn memory_hygiene_flags_snapshot_history_bloat() {
        let temp = tempdir().expect("tempdir");
        let snapshot_path = temp
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl");
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent).expect("mkdirs");
        }
        fs::write(&snapshot_path, vec![b'x'; 101 * 1024 * 1024]).expect("write large snapshot");

        let out = proactive_telemetry_alerts_payload(
            temp.path(),
            &json!({
                "ok": true,
                "health": {
                    "dashboard_metrics": {
                        "queue_depth": { "value": 0 }
                    },
                    "alerts": { "count": 0 }
                }
            }),
        );
        assert_eq!(
            out.pointer("/memory_hygiene/snapshot_history_over_soft_cap")
                .and_then(Value::as_bool),
            Some(true)
        );
        let alert_rows = out
            .get("alerts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = alert_rows
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"snapshot_history_bloat"));
    }

    #[test]
    fn dashboard_runtime_version_info_prefers_latest_git_tag_over_stale_contract_files() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp.path().join("package.json"),
            &json!({
                "version": "0.2.1-alpha.1"
            }),
        );
        write_json(
            &temp
                .path()
                .join("client/runtime/config/runtime_version.json"),
            &json!({
                "version": "0.2.1-alpha.1",
                "tag": "v0.2.1-alpha.1",
                "source": "runtime_version_contract"
            }),
        );
        fs::write(temp.path().join("README.md"), "demo\n").expect("write readme");
        run_git(temp.path(), &["init"]);
        run_git(temp.path(), &["config", "user.email", "tests@example.com"]);
        run_git(temp.path(), &["config", "user.name", "Dashboard Tests"]);
        run_git(temp.path(), &["add", "."]);
        run_git(temp.path(), &["commit", "-m", "test repo"]);
        run_git(temp.path(), &["tag", "v0.3.10-alpha"]);

        let payload = dashboard_runtime_version_info(temp.path());
        assert_eq!(
            payload.get("version").and_then(Value::as_str),
            Some("0.3.10-alpha")
        );
        assert_eq!(
            payload.get("tag").and_then(Value::as_str),
            Some("v0.3.10-alpha")
        );
        assert_eq!(
            payload.get("source").and_then(Value::as_str),
            Some("git_latest_tag")
        );
    }
}
