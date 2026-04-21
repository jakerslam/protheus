mod dashboard_compat_api_comms_store {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    const COMMS_EVENTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/comms_events.json";
    const COMMS_TASKS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/comms_tasks.json";

    fn comms_events_path(root: &Path) -> PathBuf {
        root.join(COMMS_EVENTS_REL)
    }

    fn comms_tasks_path(root: &Path) -> PathBuf {
        root.join(COMMS_TASKS_REL)
    }

    fn read_json_loose(path: &Path) -> Option<Value> {
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str::<Value>(&raw).ok()
    }

    fn write_json_pretty(path: &Path, payload: &Value) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(encoded) = serde_json::to_string_pretty(payload) {
            let _ = fs::write(path, encoded);
        }
    }

    fn rows_from_store(value: Option<Value>, key: &str) -> Vec<Value> {
        value
            .and_then(|v| {
                if v.is_array() {
                    v.as_array().cloned()
                } else {
                    v.get(key).and_then(Value::as_array).cloned()
                }
            })
            .unwrap_or_default()
    }

    fn read_events(root: &Path) -> Vec<Value> {
        rows_from_store(read_json_loose(&comms_events_path(root)), "events")
    }

    fn write_events(root: &Path, rows: &[Value]) {
        write_json_pretty(&comms_events_path(root), &json!({ "events": rows }));
    }

    pub fn read_tasks(root: &Path) -> Vec<Value> {
        rows_from_store(read_json_loose(&comms_tasks_path(root)), "tasks")
    }

    pub fn write_tasks(root: &Path, rows: &[Value]) {
        write_json_pretty(&comms_tasks_path(root), &json!({ "tasks": rows }));
    }

    pub fn make_task_id(seed: &Value) -> String {
        let hash = crate::deterministic_receipt_hash(seed);
        format!("task-{}", hash.chars().take(14).collect::<String>())
    }

    fn parse_agent_ids(value: Option<&Value>) -> Vec<String> {
        value
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(|s| super::clean_agent_id(s)))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>()
    }

    pub fn sync_swarm_progress(row: &mut Value) -> (i64, bool) {
        let swarm = parse_agent_ids(row.get("swarm_agent_ids"));
        if swarm.is_empty() {
            let current = row
                .get("completion_percent")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .clamp(0, 100);
            return (current, false);
        }
        let completed = parse_agent_ids(row.get("completed_agent_ids"));
        let completed_set = completed.iter().cloned().collect::<std::collections::BTreeSet<_>>();
        let pending = swarm
            .iter()
            .filter(|id| !completed_set.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        let progress = ((completed.len() as f64 / swarm.len() as f64) * 100.0).round() as i64;
        let prev = row
            .get("completion_percent")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(0, 100);
        row["swarm_agent_ids"] = Value::Array(
            swarm
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect::<Vec<_>>(),
        );
        row["completed_agent_ids"] = Value::Array(
            completed
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect::<Vec<_>>(),
        );
        row["pending_agent_ids"] = Value::Array(
            pending
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect::<Vec<_>>(),
        );
        row["swarm_total_agents"] = Value::from(swarm.len() as i64);
        row["swarm_completed_agents"] = Value::from(completed.len() as i64);
        row["swarm_pending_agents"] = Value::from(pending.len() as i64);
        row["completion_percent"] = Value::from(progress.clamp(0, 100));
        (progress.clamp(0, 100), progress != prev)
    }

    pub fn apply_task_lifecycle(_root: &Path, tasks: &mut Vec<Value>) -> bool {
        let mut changed = false;
        for row in tasks.iter_mut() {
            let status = super::clean_text(
                row.get("status").and_then(Value::as_str).unwrap_or("queued"),
                40,
            )
            .to_ascii_lowercase();
            if matches!(
                status.as_str(),
                "completed" | "failed" | "timed_out" | "paused" | "cancelled" | "canceled" | "aborted"
            ) {
                continue;
            }
            let (_progress, sync_changed) = sync_swarm_progress(row);
            if sync_changed {
                row["updated_at"] = Value::String(crate::now_iso());
                changed = true;
            }
        }
        changed
    }

    pub fn append_event(
        root: &Path,
        kind: &str,
        source_name: &str,
        target_name: &str,
        detail: &str,
        task_id: Option<&str>,
    ) {
        let now = crate::now_iso();
        let seed = json!({
            "kind": kind,
            "source_name": source_name,
            "target_name": target_name,
            "detail": detail,
            "task_id": task_id.unwrap_or(""),
            "ts": now
        });
        let mut events = read_events(root);
        let event_id = format!(
            "evt-{}",
            crate::deterministic_receipt_hash(&seed)
                .chars()
                .take(14)
                .collect::<String>()
        );
        events.insert(
            0,
            json!({
                "id": event_id,
                "kind": super::clean_text(kind, 40),
                "timestamp": now,
                "source_name": super::clean_text(source_name, 120),
                "target_name": super::clean_text(target_name, 120),
                "detail": super::clean_text(detail, 800),
                "task_id": super::clean_text(task_id.unwrap_or(""), 80)
            }),
        );
        if events.len() > 400 {
            events.truncate(400);
        }
        write_events(root, &events);
    }
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            out.push(ch.to_ascii_lowercase());
        }
    }
    clean_text(&out, 140)
}

fn dashboard_agent_task_total_size(tasks: &[Value]) -> i64 {
    tasks
        .iter()
        .map(|row| serde_json::to_vec(row).map(|bytes| bytes.len() as i64).unwrap_or(0))
        .sum::<i64>()
}

fn dashboard_agent_task_status_counts(tasks: &[Value]) -> Value {
    let mut counts = serde_json::Map::<String, Value>::new();
    for row in tasks {
        let status = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("queued"),
            40,
        )
        .to_ascii_lowercase();
        let entry = counts.entry(status).or_insert_with(|| Value::from(0));
        let next = entry.as_i64().unwrap_or(0) + 1;
        *entry = Value::from(next);
    }
    Value::Object(counts)
}

fn dashboard_agent_task_shared_and_changed(before: &Value, after: &Value) -> (Value, Value) {
    let mut shared = serde_json::Map::<String, Value>::new();
    let mut changed = Vec::<Value>::new();
    let before_obj = before.as_object().cloned().unwrap_or_default();
    let after_obj = after.as_object().cloned().unwrap_or_default();

    let mut keys = std::collections::BTreeSet::<String>::new();
    for key in before_obj.keys() {
        keys.insert(clean_text(key, 120));
    }
    for key in after_obj.keys() {
        keys.insert(clean_text(key, 120));
    }

    for key in keys {
        if key.is_empty() {
            continue;
        }
        let before_value = before_obj.get(&key).cloned().unwrap_or(Value::Null);
        let after_value = after_obj.get(&key).cloned().unwrap_or(Value::Null);
        if before_value == after_value {
            shared.insert(key, after_value);
        } else {
            changed.push(json!({
                "field": key,
                "before": before_value,
                "after": after_value
            }));
        }
    }

    (Value::Object(shared), Value::Array(changed))
}
