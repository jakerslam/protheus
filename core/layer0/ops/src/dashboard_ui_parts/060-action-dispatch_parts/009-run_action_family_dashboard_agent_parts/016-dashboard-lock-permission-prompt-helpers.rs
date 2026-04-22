const DASHBOARD_LPP_STATE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/lock_permission_prompt_state.json";

fn dashboard_lpp_state_path(root: &Path) -> std::path::PathBuf {
    root.join(DASHBOARD_LPP_STATE_REL)
}

fn dashboard_lpp_default_state() -> Value {
    json!({
        "type": "dashboard_lock_permission_prompt_state",
        "source": "dashboard.lpp.controller",
        "source_sequence": "",
        "age_seconds": 0,
        "stale": false,
        "updated_at": "",
        "locks": {},
        "permissions": {
            "default_decision": "deny",
            "allow_commands": [],
            "deny_commands": []
        },
        "prompt_context": {},
        "mcp_docs": {},
        "mentions": []
    })
}

fn dashboard_lpp_read_state(root: &Path) -> Value {
    let path = dashboard_lpp_state_path(root);
    let mut state = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(dashboard_lpp_default_state);
    if !state.is_object() {
        state = dashboard_lpp_default_state();
    }
    if !state.get("locks").map(Value::is_object).unwrap_or(false) {
        state["locks"] = json!({});
    }
    if !state.get("permissions").map(Value::is_object).unwrap_or(false) {
        state["permissions"] = json!({
            "default_decision": "deny",
            "allow_commands": [],
            "deny_commands": []
        });
    }
    if !state.get("prompt_context").map(Value::is_object).unwrap_or(false) {
        state["prompt_context"] = json!({});
    }
    if !state.get("mcp_docs").map(Value::is_object).unwrap_or(false) {
        state["mcp_docs"] = json!({});
    }
    if !state.get("mentions").map(Value::is_array).unwrap_or(false) {
        state["mentions"] = Value::Array(Vec::new());
    }
    state
}

fn dashboard_lpp_write_state(root: &Path, state: &Value) {
    let path = dashboard_lpp_state_path(root);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(encoded) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, encoded);
    }
}

fn dashboard_lpp_mutate_state<F>(root: &Path, mutator: F) -> Value
where
    F: FnOnce(&mut Value),
{
    let mut state = dashboard_lpp_read_state(root);
    mutator(&mut state);
    state["type"] = Value::String("dashboard_lock_permission_prompt_state".to_string());
    state["source"] = Value::String("dashboard.lpp.controller".to_string());
    state["updated_at"] = Value::String(crate::now_iso());
    state["age_seconds"] = Value::from(0);
    state["stale"] = Value::Bool(false);
    let mut seed = state.clone();
    seed["source_sequence"] = Value::String(String::new());
    state["source_sequence"] = Value::String(crate::deterministic_receipt_hash(&seed));
    dashboard_lpp_write_state(root, &state);
    state
}

fn dashboard_lock_key(payload: &Value) -> String {
    clean_text(
        payload
            .get("lock_key")
            .and_then(Value::as_str)
            .or_else(|| payload.get("lockKey").and_then(Value::as_str))
            .or_else(|| payload.get("path").and_then(Value::as_str))
            .or_else(|| payload.get("resource").and_then(Value::as_str))
            .unwrap_or(""),
        300,
    )
}

fn dashboard_lock_holder(payload: &Value) -> String {
    let holder = clean_text(
        payload
            .get("holder")
            .and_then(Value::as_str)
            .or_else(|| payload.get("agent_id").and_then(Value::as_str))
            .or_else(|| payload.get("agentId").and_then(Value::as_str))
            .unwrap_or("dashboard-agent"),
        140,
    );
    if holder.is_empty() {
        "dashboard-agent".to_string()
    } else {
        holder
    }
}

fn dashboard_locks_list_rows(state: &Value) -> Vec<Value> {
    let mut rows = state
        .get("locks")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, row)| {
                    let mut out = row.clone();
                    if !out.get("lock_key").map(Value::is_string).unwrap_or(false) {
                        out["lock_key"] = Value::String(clean_text(key, 300));
                    }
                    out
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("lock_key").and_then(Value::as_str).unwrap_or(""), 300).cmp(&clean_text(
            b.get("lock_key").and_then(Value::as_str).unwrap_or(""),
            300,
        ))
    });
    rows
}

fn dashboard_lock_acquire(root: &Path, payload: &Value) -> Value {
    let lock_key = dashboard_lock_key(payload);
    if lock_key.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_lock_acquire",
            "error": "lock_key_required"
        });
    }
    let holder = dashboard_lock_holder(payload);
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("exclusive"),
        40,
    );
    let lease_seconds = payload
        .get("lease_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(300)
        .clamp(1, 86_400);
    let force = payload.get("force").and_then(Value::as_bool).unwrap_or(false);

    let existing = dashboard_lpp_read_state(root)
        .get("locks")
        .and_then(Value::as_object)
        .and_then(|map| map.get(&lock_key).cloned());
    if let Some(row) = existing {
        let existing_holder = clean_text(row.get("holder").and_then(Value::as_str).unwrap_or(""), 140);
        if !existing_holder.is_empty() && existing_holder != holder && !force {
            return json!({
                "ok": false,
                "type": "dashboard_lock_acquire",
                "error": "lock_already_held",
                "lock_key": lock_key,
                "holder": existing_holder
            });
        }
    }

    let lock_row = json!({
        "lock_key": lock_key,
        "holder": holder,
        "mode": mode,
        "lease_seconds": lease_seconds,
        "acquired_at": crate::now_iso(),
        "renewed_at": crate::now_iso()
    });
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["locks"][lock_key.as_str()] = lock_row.clone();
        state["lock_acquire_count"] = Value::from(
            i64_from_value(state.get("lock_acquire_count"), 0).saturating_add(1),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_lock_acquire",
        "lock": lock_row,
        "state": state
    })
}

fn dashboard_lock_release(root: &Path, payload: &Value) -> Value {
    let lock_key = dashboard_lock_key(payload);
    if lock_key.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_lock_release",
            "error": "lock_key_required"
        });
    }
    let holder = dashboard_lock_holder(payload);
    let force = payload.get("force").and_then(Value::as_bool).unwrap_or(false);
    let mut released = false;
    let state = dashboard_lpp_mutate_state(root, |state| {
        let existing = state
            .get("locks")
            .and_then(Value::as_object)
            .and_then(|map| map.get(&lock_key).cloned());
        if let Some(row) = existing {
            let existing_holder =
                clean_text(row.get("holder").and_then(Value::as_str).unwrap_or(""), 140);
            if force || existing_holder.is_empty() || existing_holder == holder {
                if let Some(map) = state.get_mut("locks").and_then(Value::as_object_mut) {
                    map.remove(&lock_key);
                    released = true;
                }
            }
        }
        if released {
            state["lock_release_count"] = Value::from(
                i64_from_value(state.get("lock_release_count"), 0).saturating_add(1),
            );
        }
    });
    json!({
        "ok": released,
        "type": "dashboard_lock_release",
        "lock_key": lock_key,
        "released": released,
        "state": state
    })
}

fn dashboard_lock_status(root: &Path, payload: &Value) -> Value {
    let lock_key = dashboard_lock_key(payload);
    if lock_key.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_lock_status",
            "error": "lock_key_required"
        });
    }
    let state = dashboard_lpp_read_state(root);
    let lock = state
        .get("locks")
        .and_then(Value::as_object)
        .and_then(|map| map.get(&lock_key).cloned());
    json!({
        "ok": true,
        "type": "dashboard_lock_status",
        "lock_key": lock_key,
        "locked": lock.is_some(),
        "lock": lock,
        "state": state
    })
}

fn dashboard_locks_list(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    let rows = dashboard_locks_list_rows(&state);
    json!({
        "ok": true,
        "type": "dashboard_locks_list",
        "locks": rows.clone(),
        "count": rows.len() as i64,
        "state": state
    })
}

fn dashboard_mentions_extract(root: &Path, payload: &Value) -> Value {
    let text = clean_text(
        payload
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| payload.get("input").and_then(Value::as_str))
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        4000,
    );
    let mut set = std::collections::BTreeSet::<String>::new();
    for token in text.split_whitespace() {
        if let Some(stripped) = token.strip_prefix('@') {
            let normalized = stripped
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-' || *ch == '.')
                .collect::<String>();
            let cleaned = clean_text(&normalized, 120);
            if !cleaned.is_empty() {
                set.insert(cleaned);
            }
        }
    }
    let mentions = set
        .into_iter()
        .map(Value::String)
        .collect::<Vec<_>>();
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["mentions"] = Value::Array(mentions.clone());
        state["last_mentions_extract_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_mentions_extract",
        "mentions": mentions,
        "state": state
    })
}

fn dashboard_permissions_normalize_list(value: Option<&Value>) -> Vec<String> {
    let mut set = std::collections::BTreeSet::<String>::new();
    let rows = value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in rows {
        if let Some(raw) = row.as_str() {
            let cleaned = clean_text(raw, 180).to_ascii_lowercase();
            if !cleaned.is_empty() {
                set.insert(cleaned);
            }
        }
    }
    set.into_iter().collect::<Vec<_>>()
}

fn dashboard_permissions_set_policy(root: &Path, payload: &Value) -> Value {
    let allow = dashboard_permissions_normalize_list(
        payload.get("allow_commands").or_else(|| payload.get("allow")),
    );
    let deny =
        dashboard_permissions_normalize_list(payload.get("deny_commands").or_else(|| payload.get("deny")));
    let mut default_decision = clean_text(
        payload
            .get("default_decision")
            .and_then(Value::as_str)
            .or_else(|| payload.get("default").and_then(Value::as_str))
            .unwrap_or("deny"),
        20,
    )
    .to_ascii_lowercase();
    if default_decision != "allow" {
        default_decision = "deny".to_string();
    }
    let state = dashboard_lpp_mutate_state(root, |state| {
        state["permissions"] = json!({
            "default_decision": default_decision,
            "allow_commands": allow,
            "deny_commands": deny
        });
        state["permission_policy_updated_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_permissions_set_policy",
        "policy": state.get("permissions").cloned().unwrap_or_else(|| json!({})),
        "state": state
    })
}

fn dashboard_permissions_get_policy(root: &Path) -> Value {
    let state = dashboard_lpp_read_state(root);
    json!({
        "ok": true,
        "type": "dashboard_permissions_get_policy",
        "policy": state.get("permissions").cloned().unwrap_or_else(|| json!({})),
        "state": state
    })
}

fn dashboard_permissions_command_matches(command: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        command.starts_with(prefix)
    } else {
        command == pattern
    }
}

fn dashboard_permissions_evaluate_command(root: &Path, payload: &Value) -> Value {
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        400,
    )
    .to_ascii_lowercase();
    if command.is_empty() {
        return json!({
            "ok": false,
            "type": "dashboard_permissions_evaluate_command",
            "error": "command_required"
        });
    }
    let state = dashboard_lpp_read_state(root);
    let policy = state.get("permissions").cloned().unwrap_or_else(|| json!({}));
    let allow = dashboard_permissions_normalize_list(policy.get("allow_commands"));
    let deny = dashboard_permissions_normalize_list(policy.get("deny_commands"));
    let default_decision = clean_text(
        policy
            .get("default_decision")
            .and_then(Value::as_str)
            .unwrap_or("deny"),
        20,
    )
    .to_ascii_lowercase();
    let denied = deny
        .iter()
        .any(|pattern| dashboard_permissions_command_matches(&command, pattern));
    let allowed = allow
        .iter()
        .any(|pattern| dashboard_permissions_command_matches(&command, pattern));
    let decision = if denied {
        "deny"
    } else if allowed {
        "allow"
    } else if default_decision == "allow" {
        "allow"
    } else {
        "deny"
    };
    json!({
        "ok": true,
        "type": "dashboard_permissions_evaluate_command",
        "command": command,
        "decision": decision,
        "allowed": decision == "allow",
        "policy": policy
    })
}

include!("017-dashboard-lock-permission-prompt-route-tail.rs");
