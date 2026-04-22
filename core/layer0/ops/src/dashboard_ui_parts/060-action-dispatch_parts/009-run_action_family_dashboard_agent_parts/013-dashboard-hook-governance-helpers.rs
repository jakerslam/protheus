const DASHBOARD_HOOK_STATE_REL: &str = "client/runtime/local/state/ui/infring_dashboard/hook_controller_state.json";

fn dashboard_hook_state_path(root: &Path) -> std::path::PathBuf {
    root.join(DASHBOARD_HOOK_STATE_REL)
}

fn dashboard_hook_default_state() -> Value {
    json!({
        "type": "dashboard_hook_controller_state",
        "source": "dashboard.hooks.controller",
        "source_sequence": "",
        "age_seconds": 0,
        "stale": false,
        "updated_at": "",
        "registry": {},
        "discovery_cache": [],
        "runs": [],
        "register_count": 0,
        "discovery_refresh_count": 0,
        "start_count": 0,
        "complete_count": 0
    })
}

fn dashboard_hook_read_state(root: &Path) -> Value {
    let path = dashboard_hook_state_path(root);
    let mut state = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(dashboard_hook_default_state);
    if !state.is_object() {
        state = dashboard_hook_default_state();
    }
    if !state.get("registry").map(Value::is_object).unwrap_or(false) {
        state["registry"] = json!({});
    }
    if !state.get("discovery_cache").map(Value::is_array).unwrap_or(false) {
        state["discovery_cache"] = Value::Array(Vec::new());
    }
    if !state.get("runs").map(Value::is_array).unwrap_or(false) {
        state["runs"] = Value::Array(Vec::new());
    }
    state
}

fn dashboard_hook_write_state(root: &Path, state: &Value) {
    let path = dashboard_hook_state_path(root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(encoded) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, encoded);
    }
}

fn dashboard_hook_mutate_state<F>(root: &Path, mutator: F) -> Value
where
    F: FnOnce(&mut Value),
{
    let mut state = dashboard_hook_read_state(root);
    mutator(&mut state);
    state["type"] = Value::String("dashboard_hook_controller_state".to_string());
    state["source"] = Value::String("dashboard.hooks.controller".to_string());
    state["updated_at"] = Value::String(crate::now_iso());
    state["age_seconds"] = Value::from(0);
    state["stale"] = Value::Bool(false);
    let mut seed = state.clone();
    seed["source_sequence"] = Value::String(String::new());
    state["source_sequence"] = Value::String(crate::deterministic_receipt_hash(&seed));
    dashboard_hook_write_state(root, &state);
    state
}

fn dashboard_hook_resolve_id(payload: &Value) -> String {
    clean_text(
        payload
            .get("hook_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("hookId").and_then(Value::as_str))
            .or_else(|| payload.get("id").and_then(Value::as_str))
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .unwrap_or(""),
        140,
    )
}

fn dashboard_hook_registry_rows(state: &Value) -> Vec<Value> {
    let mut rows = state
        .get("registry")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(hook_id, row)| {
                    let mut entry = row.clone();
                    if !entry.get("hook_id").map(Value::is_string).unwrap_or(false) {
                        entry["hook_id"] = Value::String(clean_text(hook_id, 140));
                    }
                    entry
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("hook_id").and_then(Value::as_str).unwrap_or(""), 140).cmp(&clean_text(
            b.get("hook_id").and_then(Value::as_str).unwrap_or(""),
            140,
        ))
    });
    rows
}

fn dashboard_hook_upsert_discovery_cache(state: &mut Value, hook_row: &Value) {
    let hook_id = clean_text(
        hook_row.get("hook_id").and_then(Value::as_str).unwrap_or(""),
        140,
    );
    if hook_id.is_empty() {
        return;
    }
    let mut rows = state
        .get("discovery_cache")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    rows.retain(|row| clean_text(row.get("hook_id").and_then(Value::as_str).unwrap_or(""), 140) != hook_id);
    rows.push(json!({
        "hook_id": hook_id,
        "phase": clean_text(hook_row.get("phase").and_then(Value::as_str).unwrap_or("pre_tool_use"), 60),
        "enabled": hook_row.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "description": clean_text(hook_row.get("description").and_then(Value::as_str).unwrap_or(""), 240),
        "updated_at": crate::now_iso()
    }));
    rows.sort_by(|a, b| {
        clean_text(a.get("hook_id").and_then(Value::as_str).unwrap_or(""), 140).cmp(&clean_text(
            b.get("hook_id").and_then(Value::as_str).unwrap_or(""),
            140,
        ))
    });
    state["discovery_cache"] = Value::Array(rows);
}

fn dashboard_hook_registry_register(root: &Path, payload: &Value) -> Value {
    let hook_id = dashboard_hook_resolve_id(payload);
    if hook_id.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_hooks_registry_register",
            "error": "hook_id_required"
        });
    }
    let phase = clean_text(
        payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or("pre_tool_use"),
        60,
    );
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .or_else(|| payload.get("exec").and_then(Value::as_str))
            .or_else(|| payload.get("shell").and_then(Value::as_str))
            .unwrap_or(""),
        500,
    );
    let description = clean_text(
        payload
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let enabled = payload
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let hook_row = json!({
        "hook_id": hook_id,
        "phase": phase,
        "command": command,
        "description": description,
        "enabled": enabled,
        "updated_at": crate::now_iso()
    });
    let state = dashboard_hook_mutate_state(root, |state| {
        if !state.get("registry").map(Value::is_object).unwrap_or(false) {
            state["registry"] = json!({});
        }
        state["registry"][hook_id.as_str()] = hook_row.clone();
        dashboard_hook_upsert_discovery_cache(state, &hook_row);
        state["register_count"] =
            Value::from(i64_from_value(state.get("register_count"), 0).saturating_add(1));
        state["last_registered_hook_id"] = Value::String(hook_id.clone());
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_registry_register",
        "hook": hook_row,
        "registry_count": dashboard_hook_registry_rows(&state).len() as i64,
        "state": state
    })
}

fn dashboard_hook_registry_list(root: &Path) -> Value {
    let state = dashboard_hook_read_state(root);
    let rows = dashboard_hook_registry_rows(&state);
    json!({
        "ok": true,
        "type": "dashboard_hooks_registry_list",
        "hooks": rows.clone(),
        "count": rows.len() as i64,
        "state": state
    })
}

fn dashboard_hook_discovery_cache_get(root: &Path) -> Value {
    let state = dashboard_hook_read_state(root);
    let rows = state
        .get("discovery_cache")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    json!({
        "ok": true,
        "type": "dashboard_hooks_discovery_cache_get",
        "entries": rows.clone(),
        "count": rows.len() as i64,
        "state": state
    })
}

fn dashboard_hook_discovery_cache_refresh(root: &Path) -> Value {
    let state = dashboard_hook_mutate_state(root, |state| {
        let mut refreshed = dashboard_hook_registry_rows(state)
            .into_iter()
            .map(|mut row| {
                row["cache_refreshed_at"] = Value::String(crate::now_iso());
                row
            })
            .collect::<Vec<_>>();
        refreshed.sort_by(|a, b| {
            clean_text(a.get("hook_id").and_then(Value::as_str).unwrap_or(""), 140).cmp(
                &clean_text(b.get("hook_id").and_then(Value::as_str).unwrap_or(""), 140),
            )
        });
        state["discovery_cache"] = Value::Array(refreshed);
        state["discovery_refresh_count"] = Value::from(
            i64_from_value(state.get("discovery_refresh_count"), 0).saturating_add(1),
        );
    });
    let count = state
        .get("discovery_cache")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as i64)
        .unwrap_or(0);
    json!({
        "ok": true,
        "type": "dashboard_hooks_discovery_cache_refresh",
        "count": count,
        "state": state
    })
}

fn dashboard_hook_error_class(payload: &Value) -> (String, bool) {
    let code = clean_text(
        payload
            .get("error_code")
            .and_then(Value::as_str)
            .or_else(|| payload.get("errorCode").and_then(Value::as_str))
            .or_else(|| payload.get("reason").and_then(Value::as_str))
            .or_else(|| payload.get("error").and_then(Value::as_str))
            .unwrap_or(""),
        200,
    )
    .to_ascii_lowercase();
    if (code.contains("pre_tool") || code.contains("pretool")) && code.contains("cancel") {
        return ("pre_tool_use_cancellation".to_string(), true);
    }
    if code.contains("hook_cancelled") {
        return ("pre_tool_use_cancellation".to_string(), true);
    }
    if code.contains("timeout") {
        return ("hook_timeout".to_string(), false);
    }
    if code.contains("spawn") || code.contains("process") || code.contains("exit") {
        return ("hook_process_failure".to_string(), false);
    }
    if code.is_empty() {
        return ("none".to_string(), false);
    }
    ("hook_error".to_string(), false)
}

fn dashboard_hook_process_start(root: &Path, payload: &Value) -> Value {
    let hook_id = dashboard_hook_resolve_id(payload);
    if hook_id.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_hooks_process_start",
            "error": "hook_id_required"
        });
    }
    let phase = clean_text(
        payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or("pre_tool_use"),
        60,
    );
    let context = clean_text(
        payload
            .get("context")
            .and_then(Value::as_str)
            .or_else(|| payload.get("input").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    let run_seed = json!({
        "hook_id": hook_id,
        "phase": phase,
        "context": context,
        "ts": crate::now_iso()
    });
    let run_id = format!(
        "hookrun-{}",
        crate::deterministic_receipt_hash(&run_seed)
            .chars()
            .take(14)
            .collect::<String>()
    );
    let run_row = json!({
        "run_id": run_id,
        "hook_id": hook_id,
        "phase": phase,
        "status": "running",
        "started_at": crate::now_iso(),
        "context": context
    });
    let state = dashboard_hook_mutate_state(root, |state| {
        let mut runs = state
            .get("runs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        runs.insert(0, run_row.clone());
        if runs.len() > 80 {
            runs.truncate(80);
        }
        state["runs"] = Value::Array(runs);
        state["start_count"] =
            Value::from(i64_from_value(state.get("start_count"), 0).saturating_add(1));
        state["last_run_id"] = Value::String(run_id.clone());
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_process_start",
        "run": run_row,
        "state": state
    })
}

fn dashboard_hook_process_complete(root: &Path, payload: &Value) -> Value {
    let run_id = clean_text(
        payload
            .get("run_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("runId").and_then(Value::as_str))
            .unwrap_or(""),
        160,
    );
    if run_id.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_hooks_process_complete",
            "error": "run_id_required"
        });
    }
    let message = clean_text(
        payload
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| payload.get("error").and_then(Value::as_str))
            .or_else(|| payload.get("reason").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    let (error_class, pre_tool_use_cancelled) = dashboard_hook_error_class(payload);
    let status = if pre_tool_use_cancelled {
        "cancelled".to_string()
    } else {
        clean_text(
            payload
                .get("status")
                .and_then(Value::as_str)
                .or_else(|| payload.get("outcome").and_then(Value::as_str))
                .unwrap_or(if error_class == "none" { "completed" } else { "failed" }),
            60,
        )
    };
    let mut run_found = false;
    let state = dashboard_hook_mutate_state(root, |state| {
        let mut runs = state
            .get("runs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for row in &mut runs {
            let current = clean_text(row.get("run_id").and_then(Value::as_str).unwrap_or(""), 160);
            if current == run_id {
                row["status"] = Value::String(status.to_string());
                row["error_class"] = Value::String(error_class.clone());
                row["pre_tool_use_cancelled"] = Value::Bool(pre_tool_use_cancelled);
                row["completed_at"] = Value::String(crate::now_iso());
                if !message.is_empty() {
                    row["message"] = Value::String(message.clone());
                }
                run_found = true;
                break;
            }
        }
        if !run_found {
            runs.insert(
                0,
                json!({
                    "run_id": run_id.clone(),
                    "hook_id": "",
                    "phase": "pre_tool_use",
                    "status": status.clone(),
                    "error_class": error_class.clone(),
                    "pre_tool_use_cancelled": pre_tool_use_cancelled,
                    "started_at": "",
                    "completed_at": crate::now_iso(),
                    "message": message.clone()
                }),
            );
        }
        if runs.len() > 80 {
            runs.truncate(80);
        }
        state["runs"] = Value::Array(runs);
        state["complete_count"] =
            Value::from(i64_from_value(state.get("complete_count"), 0).saturating_add(1));
        state["last_completed_run_id"] = Value::String(run_id.clone());
        if pre_tool_use_cancelled {
            state["last_pre_tool_use_cancellation_at"] = Value::String(crate::now_iso());
        }
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_process_complete",
        "run_found": run_found,
        "run_id": run_id,
        "status": status.clone(),
        "error_class": error_class.clone(),
        "pre_tool_use_cancelled": pre_tool_use_cancelled,
        "state": state
    })
}

fn dashboard_hook_process_registry(root: &Path) -> Value {
    let state = dashboard_hook_read_state(root);
    let runs = state
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let running_count = runs
        .iter()
        .filter(|row| {
            let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 60);
            status == "running" || status == "queued"
        })
        .count() as i64;
    json!({
        "ok": true,
        "type": "dashboard_hooks_process_registry",
        "runs": runs.clone(),
        "run_count": runs.len() as i64,
        "running_count": running_count,
        "state": state
    })
}

fn dashboard_hook_route(root: &Path, normalized: &str, payload: &Value) -> Value {
    match normalized {
        "dashboard.hooks.registry.register" => dashboard_hook_registry_register(root, payload),
        "dashboard.hooks.registry.list" => dashboard_hook_registry_list(root),
        "dashboard.hooks.discoveryCache.get" => dashboard_hook_discovery_cache_get(root),
        "dashboard.hooks.discoveryCache.refresh" => dashboard_hook_discovery_cache_refresh(root),
        "dashboard.hooks.process.start" => dashboard_hook_process_start(root, payload),
        "dashboard.hooks.process.complete" => dashboard_hook_process_complete(root, payload),
        "dashboard.hooks.process.registry" => dashboard_hook_process_registry(root),
        _ => json!({
            "ok": false,
            "type": "dashboard_hooks_route_error",
            "error": format!("unsupported_hook_action:{normalized}")
        }),
    }
}
