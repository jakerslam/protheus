
fn memory_hygiene_payload(root: &Path, continuity: &Value) -> Value {
    let stale_48h_count = continuity
        .pointer("/sessions/stale_48h_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let stale_7d_count = continuity
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
