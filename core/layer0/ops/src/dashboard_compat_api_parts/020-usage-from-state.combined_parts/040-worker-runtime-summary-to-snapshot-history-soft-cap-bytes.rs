
fn worker_runtime_summary(root: &Path) -> Value {
    let path = root.join("local/state/runtime/task_runtime/worker_state.json");
    let state = read_json(&path).unwrap_or_else(|| json!({}));
    let active_workers = state
        .get("active_workers")
        .and_then(Value::as_object)
        .map(|rows| rows.len())
        .unwrap_or(0) as i64;
    json!({
        "active_workers": active_workers,
        "total_hibernations": state.get("total_hibernations").and_then(Value::as_i64).unwrap_or(0).max(0),
        "last_hibernated": state.get("last_hibernated").cloned().unwrap_or(Value::Null),
        "last_event": state.get("last_event").cloned().unwrap_or(Value::Null),
        "updated_at_ms": state.get("updated_at_ms").cloned().unwrap_or(Value::Null)
    })
}

fn session_pending_rows(root: &Path, snapshot: &Value, max_rows: usize) -> Vec<Value> {
    let now = Utc::now();
    let mut rows = Vec::<Value>::new();
    for row in session_summary_rows(root, snapshot).into_iter() {
        let message_count = row
            .get("message_count")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0);
        if message_count <= 0 {
            continue;
        }
        let agent_id = clean_text(
            row.get("agent_id").and_then(Value::as_str).unwrap_or(""),
            140,
        );
        if agent_id.is_empty() {
            continue;
        }
        let updated_at = clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let age_hours = parse_rfc3339_utc(&updated_at)
            .map(|ts| {
                let delta = now.signed_duration_since(ts).num_minutes().max(0);
                delta as f64 / 60.0
            })
            .unwrap_or(0.0);
        rows.push(json!({
            "agent_id": agent_id,
            "active_session_id": clean_text(row.get("active_session_id").and_then(Value::as_str).unwrap_or(""), 120),
            "message_count": message_count,
            "updated_at": updated_at,
            "age_hours": (age_hours * 10.0).round() / 10.0,
            "stale_48h": age_hours >= 48.0
        }));
    }
    rows.sort_by(|a, b| {
        let left = a.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        let right = b.get("age_hours").and_then(Value::as_f64).unwrap_or(0.0);
        right
            .partial_cmp(&left)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(max_rows.clamp(1, 100));
    rows
}

fn agent_continuity_markers_from_session_rows(session_rows: &[Value], max_rows: usize) -> Vec<Value> {
    session_rows
        .iter()
        .take(max_rows.clamp(1, 24))
        .map(|row| {
            let agent_id = clean_agent_id(row.get("agent_id").and_then(Value::as_str).unwrap_or(""));
            let message_count = row
                .get("message_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            json!({
                "agent_id": agent_id,
                "name": agent_id,
                "state": "active",
                "objective": format!("{message_count} messages in active session."),
                "completion_percent": 100,
                "updated_at": clean_text(row.get("updated_at").and_then(Value::as_str).unwrap_or(""), 80)
            })
        })
        .collect::<Vec<_>>()
}

fn continuity_pending_payload(root: &Path, snapshot: &Value) -> Value {
    let tasks = task_runtime_summary(root);
    let workers = worker_runtime_summary(root);
    let sessions = session_pending_rows(root, snapshot, 24);
    let continuity_agents = agent_continuity_markers_from_session_rows(&sessions, 12);
    let stale_sessions = sessions
        .iter()
        .filter(|row| {
            row.get("stale_48h")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    let channel_rows = dashboard_compat_api_channels::channels_payload(root)
        .get("channels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let channel_attention = channel_rows
        .into_iter()
        .filter(|row| {
            let configured = row.get("configured").and_then(Value::as_bool).unwrap_or(false);
            let connected = row.get("connected").and_then(Value::as_bool).unwrap_or(false);
            configured && !connected
        })
        .map(|row| {
            json!({
                "name": clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 80),
                "provider": clean_text(row.get("provider").and_then(Value::as_str).unwrap_or(""), 80),
                "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            })
        })
        .collect::<Vec<_>>();

    let pending_total = tasks
        .get("pending")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
        + stale_sessions.len() as i64
        + channel_attention.len() as i64;
    json!({
        "ok": true,
        "type": "cross_channel_project_continuity",
        "pending_total": pending_total,
        "tasks": tasks,
        "workers": workers,
        "sessions": {
            "rows": sessions,
            "stale_48h_count": stale_sessions.len(),
            "stale_48h": stale_sessions
        },
        "active_agents": {
            "count": continuity_agents.len(),
            "rows": continuity_agents
        },
        "channels": {
            "attention_needed_count": channel_attention.len(),
            "attention_needed": channel_attention
        }
    })
}

const SNAPSHOT_HISTORY_SOFT_CAP_BYTES: i64 = 100 * 1024 * 1024;
