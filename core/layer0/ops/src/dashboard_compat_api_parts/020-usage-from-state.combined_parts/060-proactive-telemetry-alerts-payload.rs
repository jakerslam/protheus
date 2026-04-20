
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
