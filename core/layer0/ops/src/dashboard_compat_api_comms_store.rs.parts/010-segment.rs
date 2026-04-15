// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::Path;

const COMMS_EVENTS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/comms_events.json";
const COMMS_TASKS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/comms_tasks.json";

fn comms_events_path(root: &Path) -> std::path::PathBuf {
    super::super::state_path(root, COMMS_EVENTS_REL)
}

fn comms_tasks_path(root: &Path) -> std::path::PathBuf {
    super::super::state_path(root, COMMS_TASKS_REL)
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

pub fn read_events(root: &Path) -> Vec<Value> {
    rows_from_store(
        super::super::read_json_loose(&comms_events_path(root)),
        "events",
    )
}

fn write_events(root: &Path, rows: &[Value]) {
    super::super::write_json_pretty(&comms_events_path(root), &json!({"events": rows}));
}

pub fn read_tasks(root: &Path) -> Vec<Value> {
    rows_from_store(
        super::super::read_json_loose(&comms_tasks_path(root)),
        "tasks",
    )
}

pub fn write_tasks(root: &Path, rows: &[Value]) {
    super::super::write_json_pretty(&comms_tasks_path(root), &json!({"tasks": rows}));
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn parse_task_timeout_secs(row: &Value) -> i64 {
    row.get("timeout_secs")
        .and_then(Value::as_i64)
        .unwrap_or(300)
        .clamp(15, 86_400)
}

pub fn parse_task_progress(row: &Value) -> i64 {
    row.get("completion_percent")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .clamp(0, 100)
}

pub fn parse_task_retry(row: &Value) -> i64 {
    row.get("retry_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .clamp(0, 1000)
}

fn parse_task_max_retries(row: &Value) -> i64 {
    row.get("max_retries")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .clamp(0, 20)
}

fn parse_task_status(row: &Value) -> String {
    super::super::clean_text(
        row.get("status")
            .and_then(Value::as_str)
            .unwrap_or("queued"),
        32,
    )
    .to_ascii_lowercase()
}

fn value_as_agent_ids(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let rows = value.and_then(Value::as_array).cloned().unwrap_or_default();
    for raw in rows {
        let id = super::super::clean_agent_id(raw.as_str().unwrap_or(""));
        if id.is_empty() {
            continue;
        }
        if seen.insert(id.clone()) {
            out.push(id);
        }
    }
    out
}

fn agent_ids_value(rows: &[String]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| Value::String(super::super::clean_agent_id(row)))
            .filter(|row| row.as_str().map(|v| !v.is_empty()).unwrap_or(false))
            .collect::<Vec<_>>(),
    )
}

fn parse_swarm_agents(row: &Value) -> Vec<String> {
    value_as_agent_ids(row.get("swarm_agent_ids"))
}

fn parse_completed_agents(row: &Value) -> Vec<String> {
    value_as_agent_ids(row.get("completed_agent_ids"))
}

fn parse_pending_agents(row: &Value) -> Vec<String> {
    value_as_agent_ids(row.get("pending_agent_ids"))
}

fn merge_unique_agent_ids(base: &[String], additions: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    for source in [base, additions] {
        for raw in source {
            let id = super::super::clean_agent_id(raw);
            if id.is_empty() {
                continue;
            }
            if seen.insert(id.clone()) {
                out.push(id);
            }
        }
    }
    out
}

pub fn parse_agent_ids(value: Option<&Value>) -> Vec<String> {
    value_as_agent_ids(value)
}

pub fn merge_completed_agent_ids(row: &mut Value, additions: &[String]) -> bool {
    if additions.is_empty() {
        return false;
    }
    let current = parse_completed_agents(row);
    let merged = merge_unique_agent_ids(&current, additions);
    if merged == current {
        return false;
    }
    row["completed_agent_ids"] = agent_ids_value(&merged);
    true
}

pub fn override_pending_agent_ids(row: &mut Value, pending: &[String]) -> bool {
    let next = merge_unique_agent_ids(&[], pending);
    let current = parse_pending_agents(row);
    if next == current {
        return false;
    }
    row["pending_agent_ids"] = agent_ids_value(&next);
    true
}

pub fn merge_partial_results(row: &mut Value, incoming: Option<&Value>) -> bool {
    let Some(payload) = incoming else {
        return false;
    };
    let mut changed = false;
    if !row
        .get("partial_results")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        row["partial_results"] = json!({});
        changed = true;
    }
    if let Some(existing) = row
        .get_mut("partial_results")
        .and_then(Value::as_object_mut)
    {
        if let Some(incoming_obj) = payload.as_object() {
            for (key, value) in incoming_obj {
                let safe_key = super::super::clean_text(key, 120);
                if safe_key.is_empty() {
                    continue;
                }
                let normalized = match value {
                    Value::String(text) => Value::String(super::super::clean_text(text, 8_000)),
                    _ => value.clone(),
                };
                if existing.get(&safe_key) != Some(&normalized) {
                    existing.insert(safe_key, normalized);
                    changed = true;
                }
            }
        }
    }
    changed
}

pub fn sync_swarm_progress(row: &mut Value) -> (i64, bool) {
    let swarm = parse_swarm_agents(row);
    if swarm.is_empty() {
        return (parse_task_progress(row), false);
    }
    let previous_progress = parse_task_progress(row);
    let previous_swarm = row.get("swarm_agent_ids").cloned();
    let previous_completed = row.get("completed_agent_ids").cloned();
    let previous_pending = row.get("pending_agent_ids").cloned();
    let completed_raw = parse_completed_agents(row);
    let swarm_set = swarm.iter().cloned().collect::<HashSet<_>>();
    let completed = completed_raw
        .into_iter()
        .filter(|id| swarm_set.contains(id))
        .collect::<Vec<_>>();
    let completed_set = completed.iter().cloned().collect::<HashSet<_>>();
    let mut pending = parse_pending_agents(row);
    if pending.is_empty() {
        pending = swarm
            .iter()
            .filter(|id| !completed_set.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
    } else {
        pending = pending
            .into_iter()
            .filter(|id| swarm_set.contains(id) && !completed_set.contains(id))
            .collect::<Vec<_>>();
    }
    let progress = ((completed.len() as f64 / swarm.len() as f64) * 100.0).round() as i64;
    let swarm_value = agent_ids_value(&swarm);
    let completed_value = agent_ids_value(&completed);
    let pending_value = agent_ids_value(&pending);
    row["swarm_agent_ids"] = swarm_value.clone();
    row["completed_agent_ids"] = completed_value.clone();
    row["pending_agent_ids"] = pending_value.clone();
    row["swarm_total_agents"] = Value::from(swarm.len() as i64);
    row["swarm_completed_agents"] = Value::from(completed.len() as i64);
    row["swarm_pending_agents"] = Value::from(pending.len() as i64);
    row["completion_percent"] = Value::from(progress.clamp(0, 100));
    (
        progress.clamp(0, 100),
        progress != previous_progress
            || previous_swarm != Some(swarm_value)
            || previous_completed != Some(completed_value)
            || previous_pending != Some(pending_value),
    )
}

fn task_started_at(row: &Value) -> Option<DateTime<Utc>> {
    row.get("started_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339_utc)
        .or_else(|| {
            row.get("created_at")
                .and_then(Value::as_str)
                .and_then(parse_rfc3339_utc)
        })
}

pub fn make_task_id(seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(seed);
    format!("task-{}", hash.chars().take(14).collect::<String>())
}

fn make_event_id(seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(seed);
    format!("evt-{}", hash.chars().take(14).collect::<String>())
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
    events.insert(
        0,
        json!({
            "id": make_event_id(&seed),
            "kind": super::super::clean_text(kind, 40),
            "timestamp": now,
            "source_name": super::super::clean_text(source_name, 120),
            "target_name": super::super::clean_text(target_name, 120),
            "detail": super::super::clean_text(detail, 800),
            "task_id": super::super::clean_text(task_id.unwrap_or(""), 80)
        }),
    );
    if events.len() > 400 {
        events.truncate(400);
    }
    write_events(root, &events);
}

