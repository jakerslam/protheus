        .pointer("/sessions/stale_48h")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0) >= 168.0)
                .count() as i64
        })
        .unwrap_or(0);
    let snapshot_path = state_path(
        root,
        "client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl",
    );
    let snapshot_bytes_u64 = fs::metadata(&snapshot_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    let snapshot_bytes = if snapshot_bytes_u64 > i64::MAX as u64 {
        i64::MAX
    } else {
        snapshot_bytes_u64 as i64
    };
    let snapshot_over_soft_cap = snapshot_bytes >= SNAPSHOT_HISTORY_SOFT_CAP_BYTES;

    let mut recommendations = Vec::<Value>::new();
    if stale_7d_count > 0 {
        recommendations.push(json!({
            "command": "/continuity",
            "reason": format!("{stale_7d_count} stale memory-backed session(s) exceed 7 days")
        }));
    }
    if snapshot_over_soft_cap {
        recommendations.push(json!({
            "command": "infring cleanup purge --aggressive",
            "reason": format!("snapshot_history.jsonl exceeds soft cap ({} bytes)", snapshot_bytes)
        }));
    }
    if recommendations.is_empty() {
        recommendations.push(json!({
            "command": "/status",
            "reason": "memory hygiene is healthy"
        }));
    }

    json!({
        "ok": true,
        "type": "memory_hygiene",
        "stale_contexts_48h": stale_48h_count,
        "stale_contexts_7d": stale_7d_count,
        "snapshot_history_path": snapshot_path.to_string_lossy().to_string(),
        "snapshot_history_bytes": snapshot_bytes,
        "snapshot_history_soft_cap_bytes": SNAPSHOT_HISTORY_SOFT_CAP_BYTES,
        "snapshot_history_over_soft_cap": snapshot_over_soft_cap,
        "recommendations": recommendations
    })
}

fn predicted_next_actions(
    task_pending: i64,
    queue_depth: i64,
    stale_sessions: i64,
    channel_attention: i64,
    dashboard_alerts: i64,
    memory_hygiene: &Value,
) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    let mut push = |command: &str, reason: String, priority: &str| {
        let key = clean_text(command, 60).to_ascii_lowercase();
        if key.is_empty() || seen.contains(&key) {
            return;
        }
        seen.insert(key);
        out.push(json!({
            "command": command,
            "reason": reason,
            "priority": priority
        }));
    };

    if dashboard_alerts > 0 {
        push(
            "/alerts",
            format!("Health lane has {} active alert(s)", dashboard_alerts),
            "high",
        );
    }
    if task_pending > 0 || queue_depth > 0 {
        push(
            "/queue",
            format!(
                "Queue pressure pending={} depth={}",
                task_pending, queue_depth
            ),
            "high",
        );
    }
    if stale_sessions > 0 || channel_attention > 0 {
        push(
            "/continuity",
            format!(
                "Pending continuity work (stale_sessions={}, channel_attention={})",
                stale_sessions, channel_attention
            ),
            "medium",
        );
    }
    if memory_hygiene
        .get("snapshot_history_over_soft_cap")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        push(
            "infring cleanup purge --aggressive",
            "Memory hygiene indicates snapshot history bloat".to_string(),
            "medium",
        );
    }
    if task_pending == 0
        && queue_depth == 0
        && stale_sessions == 0
        && channel_attention == 0
        && dashboard_alerts == 0
    {
        push(
            "/status",
            "System is healthy; run status for a quick confidence check".to_string(),
            "low",
        );
    }
    out
}

fn proactive_telemetry_alerts_payload(root: &Path, snapshot: &Value) -> Value {
    let continuity = continuity_pending_payload(root, snapshot);
    let runtime = runtime_sync_summary(snapshot);
    let task_pending = continuity
        .pointer("/tasks/pending")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let active_workers = continuity
        .pointer("/workers/active_workers")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let stale_sessions = continuity
        .pointer("/sessions/stale_48h_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let channel_attention = continuity
        .pointer("/channels/attention_needed_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let dashboard_alerts = parse_non_negative_i64(snapshot.pointer("/health/alerts/count"), 0);
    let queue_depth = runtime
        .get("queue_depth")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let memory_hygiene = memory_hygiene_payload(root, &continuity);
    let stale_memory_7d = memory_hygiene
        .get("stale_contexts_7d")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let snapshot_over_soft_cap = memory_hygiene
        .get("snapshot_history_over_soft_cap")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut alerts = Vec::<Value>::new();
    if dashboard_alerts > 0 {
        alerts.push(json!({
            "id": "health_alerts_present",
            "severity": "high",
            "message": format!("Health checks report {} alert(s).", dashboard_alerts),
            "recommended_command": "/status",
            "source": "health"
        }));
    }
    if task_pending >= 22 || queue_depth >= 22 {
        alerts.push(json!({
            "id": "queue_pressure_high",
            "severity": "high",
            "message": format!("Queue pressure is elevated (pending={}, depth={}).", task_pending, queue_depth),
            "recommended_command": "/queue",
            "source": "task_runtime"
        }));
    }
    if stale_sessions > 0 {
        alerts.push(json!({
            "id": "stale_sessions_detected",
            "severity": "medium",
            "message": format!("{} session(s) have pending context older than 48h.", stale_sessions),
            "recommended_command": "/continuity",
            "source": "sessions"
        }));
    }
    if channel_attention > 0 {
        alerts.push(json!({
            "id": "channel_attention_needed",
            "severity": "medium",
            "message": format!("{} configured channel(s) are disconnected.", channel_attention),
            "recommended_command": "/continuity",
            "source": "channels"
        }));
    }
    if active_workers > 0 && task_pending == 0 {
        alerts.push(json!({
            "id": "worker_hibernation_candidate",
            "severity": "low",
            "message": "Workers are active with zero pending tasks; hibernation path can reclaim compute.",
            "recommended_command": "infring task worker --service=1 --idle-hibernate-ms=15000",
            "source": "task_runtime"
        }));
    }
    if stale_memory_7d > 0 {
        alerts.push(json!({
            "id": "memory_hygiene_stale_contexts",
            "severity": "medium",
            "message": format!("{} memory-backed session context(s) are older than 7 days and should be compacted.", stale_memory_7d),
            "recommended_command": "/memory",
            "source": "memory_hygiene"
        }));
    }
    if snapshot_over_soft_cap {
        let bytes = memory_hygiene
            .get("snapshot_history_bytes")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        alerts.push(json!({
            "id": "snapshot_history_bloat",
            "severity": "high",
            "message": format!("snapshot_history.jsonl is large ({} bytes); cleanup should run aggressively.", bytes),
            "recommended_command": "infring cleanup purge --aggressive",
            "source": "memory_hygiene"
        }));
    }

    let next_actions = predicted_next_actions(
        task_pending,
        queue_depth,
        stale_sessions,
        channel_attention,
        dashboard_alerts,
        &memory_hygiene,
    );

    json!({
        "ok": true,
        "type": "proactive_telemetry_alerts",
        "generated_at": crate::now_iso(),
        "count": alerts.len(),
        "alerts": alerts,
        "continuity": continuity,
        "memory_hygiene": memory_hygiene,
        "next_actions": next_actions
    })
}

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
        assert!(out
            .pointer("/active_agents/rows/0/objective")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("investigate pending deployment"));
        assert_eq!(
            out.pointer("/active_agents/rows/0/completion_percent")
                .and_then(Value::as_i64),
            Some(20)
        );
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
