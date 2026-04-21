
pub fn create_session(root: &Path, agent_id: &str, label: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let base_label = clean_text(label, 80);
    let label_value = if base_label.is_empty() {
        "Session".to_string()
    } else {
        base_label
    };
    let mut session_id = format!(
        "s-{}",
        crate::deterministic_receipt_hash(
            &json!({"agent_id": id, "label": label_value, "ts": now_iso()})
        )
        .chars()
        .take(12)
        .collect::<String>()
    );
    {
        let sessions = as_array_mut(&mut state, "sessions");
        let mut attempt = 2usize;
        while sessions.iter().any(|row| {
            row.get("session_id")
                .and_then(Value::as_str)
                .map(|v| v == session_id)
                .unwrap_or(false)
        }) {
            session_id = format!("{}-{}", session_id, attempt);
            attempt += 1;
        }
        sessions.push(json!({
            "session_id": session_id,
            "label": label_value,
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "messages": []
        }));
    }
    state["active_session_id"] = Value::String(session_id.clone());
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_create", "agent_id": id, "active_session_id": session_id, "session": state})
}

pub fn switch_session(root: &Path, agent_id: &str, session_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let sid = clean_text(session_id, 120);
    if id.is_empty() || sid.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_session_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let exists = state
        .get("sessions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.get("session_id")
                    .and_then(Value::as_str)
                    .map(|v| v == sid)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if !exists {
        return json!({"ok": false, "error": "session_not_found", "agent_id": id, "session_id": sid});
    }
    state["active_session_id"] = Value::String(sid.clone());
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_switch", "agent_id": id, "active_session_id": sid})
}

pub fn delete_session(root: &Path, agent_id: &str, session_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let sid = clean_text(session_id, 120);
    if id.is_empty() || sid.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_session_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let removed = {
        let sessions = as_array_mut(&mut state, "sessions");
        let before = sessions.len();
        sessions.retain(|row| {
            row.get("session_id")
                .and_then(Value::as_str)
                .map(|v| v != sid)
                .unwrap_or(true)
        });
        let removed = sessions.len() != before;
        if sessions.is_empty() {
            sessions.push(json!({
                "session_id": "default",
                "label": "Session",
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "messages": []
            }));
        }
        removed
    };
    let next_active = state
        .get("sessions")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("session_id").and_then(Value::as_str))
        .unwrap_or("default")
        .to_string();
    if state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| v == sid)
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String(next_active.clone());
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_session_delete", "agent_id": id, "removed": removed, "active_session_id": next_active})
}

pub fn memory_kv_set(root: &Path, agent_id: &str, key: &str, value: &Value) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let mut state = load_session_state(root, &id);
    let memory = as_object_mut(&mut state, "memory_kv");
    memory.insert(k.clone(), value.clone());
    let tags = duality_memory_tags(root, &k, value);
    let memory_meta = as_object_mut(&mut state, "memory_kv_meta");
    memory_meta.insert(k.clone(), tags.clone());
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_set",
        "agent_id": id,
        "key": k,
        "value": value.clone(),
        "duality_tags": tags
    })
}

pub fn memory_kv_pairs(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    let mut kv_pairs = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .map(|(key, value)| {
                    let duality_tags = memory_duality_tags(&state, key);
                    json!({
                        "key": key,
                        "value": value,
                        "duality_tags": duality_tags
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    kv_pairs
        .sort_by_key(|row| clean_text(row.get("key").and_then(Value::as_str).unwrap_or(""), 160));
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_pairs",
        "agent_id": id,
        "kv_pairs": kv_pairs
    })
}

pub fn memory_kv_get(root: &Path, agent_id: &str, key: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let state = load_session_state(root, &id);
    let value = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&k))
        .cloned()
        .unwrap_or(Value::Null);
    let duality_tags = memory_duality_tags(&state, &k);
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_kv_get",
        "agent_id": id,
        "key": k,
        "value": value,
        "duality_tags": duality_tags
    })
}

pub fn memory_kv_delete(root: &Path, agent_id: &str, key: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let k = normalize_memory_key(key);
    if id.is_empty() || k.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_key_required"});
    }
    let mut state = load_session_state(root, &id);
    let memory = as_object_mut(&mut state, "memory_kv");
    let removed = memory.remove(&k).is_some();
    let memory_meta = as_object_mut(&mut state, "memory_kv_meta");
    let _ = memory_meta.remove(&k);
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_memory_kv_delete", "agent_id": id, "key": k, "removed": removed})
}
