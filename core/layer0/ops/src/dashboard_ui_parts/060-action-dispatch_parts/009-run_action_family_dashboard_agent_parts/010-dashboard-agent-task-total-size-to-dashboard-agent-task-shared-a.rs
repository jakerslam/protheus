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

const DASHBOARD_UI_CONTROLLER_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/ui_controller_state.json";

fn normalize_workspace_path(raw: &str) -> String {
    clean_text(raw, 400)
        .replace('\\', "/")
        .trim()
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn dashboard_agent_task_workspace_match(row: &Value, workspace_path: &str) -> bool {
    let workspace = normalize_workspace_path(workspace_path);
    if workspace.is_empty() {
        return false;
    }
    for key in [
        "cwd_on_task_initialization",
        "cwdOnTaskInitialization",
        "shadow_git_config_work_tree",
        "shadowGitConfigWorkTree",
        "workspace_path",
        "workspacePath",
    ] {
        let candidate = normalize_workspace_path(row.get(key).and_then(Value::as_str).unwrap_or(""));
        if !candidate.is_empty() && candidate == workspace {
            return true;
        }
    }
    false
}

fn dashboard_agent_task_search_blob(row: &Value) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        clean_text(row.get("title").and_then(Value::as_str).unwrap_or(""), 400),
        clean_text(row.get("task").and_then(Value::as_str).unwrap_or(""), 400),
        clean_text(
            row.get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
            4000
        ),
        clean_text(
            row.get("assigned_to")
                .and_then(Value::as_str)
                .unwrap_or(""),
            140
        )
    )
    .to_ascii_lowercase()
}

fn dashboard_agent_task_cost_total(row: &Value) -> f64 {
    row.get("total_cost")
        .and_then(Value::as_f64)
        .or_else(|| row.get("totalCost").and_then(Value::as_f64))
        .unwrap_or(0.0)
}

fn dashboard_agent_task_token_total(row: &Value) -> i64 {
    i64_from_value(row.get("tokens_in"), 0)
        + i64_from_value(row.get("tokens_out"), 0)
        + i64_from_value(row.get("cache_writes"), 0)
        + i64_from_value(row.get("cache_reads"), 0)
        + i64_from_value(row.get("tokensIn"), 0)
        + i64_from_value(row.get("tokensOut"), 0)
        + i64_from_value(row.get("cacheWrites"), 0)
        + i64_from_value(row.get("cacheReads"), 0)
}

fn dashboard_agent_task_timestamp_seconds(row: &Value) -> i64 {
    if let Some(ts) = row.get("ts").and_then(Value::as_i64) {
        return ts;
    }
    let created = row
        .get("created_at")
        .and_then(Value::as_str)
        .or_else(|| row.get("createdAt").and_then(Value::as_str))
        .unwrap_or("");
    chrono::DateTime::parse_from_rfc3339(created)
        .map(|parsed| parsed.timestamp())
        .unwrap_or(0)
}

fn dashboard_agent_task_apply_favorite(root: &Path, task_id: &str, is_favorited: bool) -> Value {
    let normalized_id = clean_text(task_id, 80);
    if normalized_id.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_agent_task_error",
            "error": "task_id_required"
        });
    }
    let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
    let _ = crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks);
    let mut updated_task = Value::Null;
    for row in tasks.iter_mut() {
        let row_id = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        if row_id == normalized_id {
            row["is_favorited"] = Value::Bool(is_favorited);
            row["isFavorited"] = Value::Bool(is_favorited);
            row["updated_at"] = Value::String(crate::now_iso());
            updated_task = row.clone();
            break;
        }
    }
    if updated_task.is_null() {
        return json!({
            "ok": false,
            "type": "dashboard_agent_task_error",
            "error": "task_not_found",
            "task_id": normalized_id
        });
    }
    crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
    crate::dashboard_compat_api_comms_store::append_event(
        root,
        "task_favorite_updated",
        "DashboardUI",
        "",
        if is_favorited {
            "favorited"
        } else {
            "unfavorited"
        },
        Some(&normalized_id),
    );
    json!({
        "ok": true,
        "type": "dashboard_agent_task_favorite_updated",
        "task_id": normalized_id,
        "is_favorited": is_favorited,
        "task": updated_task
    })
}

fn dashboard_agent_task_apply_feedback(root: &Path, task_id: &str, feedback_raw: &str) -> Value {
    let feedback = clean_text(feedback_raw, 64).to_ascii_lowercase();
    if feedback.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_agent_task_error",
            "error": "feedback_required"
        });
    }
    let normalized_id = clean_text(task_id, 80);
    let feedback_at = crate::now_iso();
    if normalized_id.is_empty() {
        crate::dashboard_compat_api_comms_store::append_event(
            root,
            "task_feedback",
            "DashboardUI",
            "",
            &format!("feedback={feedback}"),
            None,
        );
        return json!({
            "ok": true,
            "type": "dashboard_agent_task_feedback_recorded",
            "task_id": Value::Null,
            "feedback": feedback,
            "applied": false
        });
    }
    let mut tasks = crate::dashboard_compat_api_comms_store::read_tasks(root);
    let _ = crate::dashboard_compat_api_comms_store::apply_task_lifecycle(root, &mut tasks);
    let mut updated_task = Value::Null;
    for row in tasks.iter_mut() {
        let row_id = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
        if row_id == normalized_id {
            row["feedback"] = Value::String(feedback.clone());
            row["feedback_at"] = Value::String(feedback_at.clone());
            row["updated_at"] = Value::String(feedback_at.clone());
            updated_task = row.clone();
            break;
        }
    }
    if updated_task.is_null() {
        return json!({
            "ok": false,
            "type": "dashboard_agent_task_error",
            "error": "task_not_found",
            "task_id": normalized_id
        });
    }
    crate::dashboard_compat_api_comms_store::write_tasks(root, &tasks);
    crate::dashboard_compat_api_comms_store::append_event(
        root,
        "task_feedback",
        "DashboardUI",
        "",
        &format!("feedback={feedback}"),
        Some(&normalized_id),
    );
    json!({
        "ok": true,
        "type": "dashboard_agent_task_feedback_recorded",
        "task_id": normalized_id,
        "feedback": feedback,
        "applied": true,
        "task": updated_task
    })
}

fn dashboard_ui_controller_default_state() -> Value {
    json!({
        "type": "dashboard_ui_controller_state",
        "initialized": false,
        "initialization_count": 0,
        "last_initialized_at": "",
        "last_boot_reason": "",
        "terminal_execution_mode": "vscodeTerminal",
        "subscriptions": {
            "add_to_input": { "count": 0, "last_event_at": "", "last_payload": "" },
            "chat_button_clicked": { "count": 0, "last_event_at": "", "last_payload": "" },
            "history_button_clicked": { "count": 0, "last_event_at": "", "last_payload": "" }
        },
        "source": "dashboard.ui.controller",
        "source_sequence": "",
        "age_seconds": 0,
        "stale": false,
        "updated_at": ""
    })
}

fn dashboard_ui_controller_state_path(root: &Path) -> std::path::PathBuf {
    root.join(DASHBOARD_UI_CONTROLLER_STATE_REL)
}

fn dashboard_ui_controller_read_state(root: &Path) -> Value {
    let path = dashboard_ui_controller_state_path(root);
    let mut state = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(dashboard_ui_controller_default_state);
    if !state.is_object() {
        state = dashboard_ui_controller_default_state();
    }
    if state.get("subscriptions").and_then(Value::as_object).is_none() {
        state["subscriptions"] = dashboard_ui_controller_default_state()["subscriptions"].clone();
    }
    state
}

fn dashboard_ui_controller_write_state(root: &Path, state: &Value) {
    let path = dashboard_ui_controller_state_path(root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(encoded) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, encoded);
    }
}

fn dashboard_ui_controller_mutate_state<F>(root: &Path, mutator: F) -> Value
where
    F: FnOnce(&mut Value),
{
    let mut state = dashboard_ui_controller_read_state(root);
    mutator(&mut state);
    state["source"] = Value::String("dashboard.ui.controller".to_string());
    state["updated_at"] = Value::String(crate::now_iso());
    state["age_seconds"] = Value::from(0);
    state["stale"] = Value::Bool(false);
    state["type"] = Value::String("dashboard_ui_controller_state".to_string());
    let mut seed = state.clone();
    seed["source_sequence"] = Value::String(String::new());
    state["source_sequence"] = Value::String(crate::deterministic_receipt_hash(&seed));
    dashboard_ui_controller_write_state(root, &state);
    state
}

fn dashboard_ui_controller_initialize(root: &Path, payload: &Value) -> Value {
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["initialized"] = Value::Bool(true);
        state["initialization_count"] = Value::from(
            i64_from_value(state.get("initialization_count"), 0).saturating_add(1),
        );
        state["last_initialized_at"] = Value::String(crate::now_iso());
        state["last_boot_reason"] = Value::String(clean_text(
            payload
                .get("reason")
                .and_then(Value::as_str)
                .or_else(|| payload.get("boot_reason").and_then(Value::as_str))
                .unwrap_or("dashboard_ui_init"),
            120,
        ));
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_initialize_webview",
        "state": state
    })
}

fn dashboard_ui_controller_set_terminal_execution_mode(root: &Path, payload: &Value) -> Value {
    let mode_from_bool = payload
        .get("value")
        .and_then(Value::as_bool)
        .or_else(|| payload.get("enabled").and_then(Value::as_bool))
        .or_else(|| payload.get("background_exec").and_then(Value::as_bool))
        .map(|enabled| {
            if enabled {
                "backgroundExec".to_string()
            } else {
                "vscodeTerminal".to_string()
            }
        });
    let mode_from_text = payload
        .get("mode")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 40).to_ascii_lowercase())
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            if raw.contains("background") {
                "backgroundExec".to_string()
            } else {
                "vscodeTerminal".to_string()
            }
        });
    let terminal_mode = mode_from_text
        .or(mode_from_bool)
        .unwrap_or_else(|| "vscodeTerminal".to_string());
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["terminal_execution_mode"] = Value::String(terminal_mode.clone());
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_terminal_execution_mode_set",
        "terminal_execution_mode": terminal_mode,
        "state": state
    })
}

fn dashboard_ui_controller_record_subscription(root: &Path, channel: &str, payload: &Value) -> Value {
    let channel_key = clean_text(channel, 80).to_ascii_lowercase();
    let payload_text = clean_text(
        payload
            .get("value")
            .and_then(Value::as_str)
            .or_else(|| payload.get("text").and_then(Value::as_str))
            .unwrap_or(""),
        500,
    );
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        if state.get("subscriptions").and_then(Value::as_object).is_none() {
            state["subscriptions"] = dashboard_ui_controller_default_state()["subscriptions"].clone();
        }
        if state["subscriptions"]
            .get(channel_key.as_str())
            .and_then(Value::as_object)
            .is_none()
        {
            state["subscriptions"][&channel_key] =
                json!({ "count": 0, "last_event_at": "", "last_payload": "" });
        }
        let count = i64_from_value(
            state["subscriptions"][&channel_key].get("count"),
            0,
        )
        .saturating_add(1);
        state["subscriptions"][&channel_key]["count"] = Value::from(count);
        state["subscriptions"][&channel_key]["last_event_at"] = Value::String(crate::now_iso());
        state["subscriptions"][&channel_key]["last_payload"] = Value::String(payload_text.clone());
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_subscription_event",
        "channel": channel_key,
        "state": state
    })
}

fn dashboard_ui_controller_get_webview_html(root: &Path) -> Value {
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["last_webview_html_requested_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_get_webview_html",
        "html": "<!-- dashboard webview html is shell-owned; runtime returns deterministic placeholder -->",
        "state": state
    })
}

fn dashboard_ui_controller_on_did_show_announcement(root: &Path, payload: &Value) -> Value {
    let announcement_id = clean_text(
        payload
            .get("announcement_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("latest_announcement_id").and_then(Value::as_str))
            .or_else(|| payload.get("id").and_then(Value::as_str))
            .unwrap_or("latest"),
        140,
    );
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["last_shown_announcement_id"] = Value::String(announcement_id.clone());
        state["announcement_ack_at"] = Value::String(crate::now_iso());
        state["announcement_should_show"] = Value::Bool(false);
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_announcement_acknowledged",
        "announcement_id": announcement_id,
        "should_show": false,
        "state": state
    })
}

fn dashboard_ui_controller_open_url(root: &Path, payload: &Value) -> Value {
    let url = clean_text(
        payload
            .get("url")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        1000,
    );
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["last_open_url"] = Value::String(url.clone());
        state["last_open_url_at"] = Value::String(crate::now_iso());
        state["open_url_count"] =
            Value::from(i64_from_value(state.get("open_url_count"), 0).saturating_add(1));
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_open_url",
        "url": url,
        "state": state
    })
}

fn dashboard_ui_controller_open_walkthrough(root: &Path) -> Value {
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["last_open_walkthrough_at"] = Value::String(crate::now_iso());
        state["open_walkthrough_count"] =
            Value::from(i64_from_value(state.get("open_walkthrough_count"), 0).saturating_add(1));
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_open_walkthrough",
        "state": state
    })
}

fn dashboard_ui_controller_scroll_to_settings(root: &Path, payload: &Value) -> Value {
    let section = clean_text(
        payload
            .get("section")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        160,
    );
    let state = dashboard_ui_controller_mutate_state(root, |state| {
        state["last_scroll_to_settings"] = Value::String(section.clone());
        state["last_scroll_to_settings_at"] = Value::String(crate::now_iso());
        state["scroll_to_settings_count"] = Value::from(
            i64_from_value(state.get("scroll_to_settings_count"), 0).saturating_add(1),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_ui_scroll_to_settings",
        "key": "scrollToSettings",
        "value": section,
        "state": state
    })
}

include!("011-dashboard-ui-worktree-and-web-helpers.rs");
include!("012-dashboard-worktree-extended-controls.rs");
include!("013-dashboard-hook-governance-helpers.rs");
include!("014-dashboard-hook-test-scenario-helpers.rs");
