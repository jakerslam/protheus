
pub fn sessions_payload(root: &Path) -> Value {
    let state = load_state(root);
    let mut rows = state
        .get("sessions")
        .and_then(Value::as_object)
        .map(|obj| obj.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    json!({"ok": true, "sessions": rows})
}

pub fn create_session(root: &Path, request: &Value) -> Value {
    let requested_id = clean_text(request.get("id").and_then(Value::as_str).unwrap_or(""), 120);
    let mut session_id = if requested_id.is_empty() {
        format!(
            "term-{}",
            crate::deterministic_receipt_hash(&json!({"ts": now_iso()}))
                .chars()
                .take(12)
                .collect::<String>()
        )
    } else {
        normalize_session_id(&requested_id)
    };
    if session_id.is_empty() {
        session_id = "term-default".to_string();
    }
    let cwd = resolve_cwd(
        root,
        request.get("cwd").and_then(Value::as_str).unwrap_or(""),
    );
    if !cwd_allowed(root, &cwd) {
        return json!({"ok": false, "error": "cwd_outside_workspace"});
    }
    let mut state = load_state(root);
    let sessions = as_object_mut(&mut state, "sessions");
    sessions.insert(
        session_id.clone(),
        json!({
            "id": session_id,
            "cwd": cwd.to_string_lossy().to_string(),
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "last_exit_code": Value::Null,
            "last_output": ""
        }),
    );
    let out = sessions
        .get(&session_id)
        .cloned()
        .unwrap_or_else(|| json!({}));
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_create", "session": out})
}

pub fn close_session(root: &Path, session_id: &str) -> Value {
    let sid = normalize_session_id(session_id);
    if sid.is_empty() {
        return json!({"ok": false, "error": "session_id_required"});
    }
    let mut state = load_state(root);
    let removed = as_object_mut(&mut state, "sessions").remove(&sid).is_some();
    save_state(root, state);
    json!({"ok": true, "type": "dashboard_terminal_session_close", "session_id": sid, "removed": removed})
}
