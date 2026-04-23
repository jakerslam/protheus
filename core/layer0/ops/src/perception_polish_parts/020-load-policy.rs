
pub fn load_policy(root: &Path, policy_path: &Path) -> Policy {
    let base = default_policy(root);
    let raw = read_json(policy_path);

    let mut out = base.clone();
    if let Some(v) = raw.get("version").and_then(Value::as_str) {
        let c = clean(v, 24);
        if !c.is_empty() {
            out.version = c;
        }
    }
    out.enabled = raw
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(base.enabled);
    out.strict_default = raw
        .get("strict_default")
        .and_then(Value::as_bool)
        .unwrap_or(base.strict_default);
    out.items = raw
        .get("items")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = normalize_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
                    if id.is_empty() {
                        return None;
                    }
                    let title = clean(row.get("title").and_then(Value::as_str).unwrap_or(&id), 240);
                    Some(Item {
                        id: id.clone(),
                        title: if title.is_empty() { id } else { title },
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| base.items.clone());

    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    out.paths = Paths {
        state_path: resolve_path(
            root,
            paths.get("state_path"),
            "local/state/ops/perception_polish_program/state.json",
        ),
        latest_path: resolve_path(
            root,
            paths.get("latest_path"),
            "local/state/ops/perception_polish_program/latest.json",
        ),
        receipts_path: resolve_path(
            root,
            paths.get("receipts_path"),
            "local/state/ops/perception_polish_program/receipts.jsonl",
        ),
        history_path: resolve_path(
            root,
            paths.get("history_path"),
            "local/state/ops/perception_polish_program/history.jsonl",
        ),
        flags_path: resolve_path(
            root,
            paths.get("flags_path"),
            "client/runtime/config/feature_flags/perception_flags.json",
        ),
        observability_panel_path: resolve_path(
            root,
            paths.get("observability_panel_path"),
            "local/state/ops/infring_top/observability_panel.json",
        ),
        reasoning_footer_path: resolve_path(
            root,
            paths.get("reasoning_footer_path"),
            "local/state/ops/infring_top/reasoning_mirror_footer.txt",
        ),
        tone_policy_path: resolve_path(
            root,
            paths.get("tone_policy_path"),
            "client/runtime/config/perception_tone_policy.json",
        ),
        post_reveal_easter_egg_path: resolve_path(
            root,
            paths.get("post_reveal_easter_egg_path"),
            "docs/client/blog/the_fort_was_empty_easter_egg.md",
        ),
    };
    out.policy_path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };

    out
}

fn default_state() -> Value {
    json!({
        "schema_id": "perception_polish_program_state",
        "schema_version": "1.0",
        "updated_at": now_iso(),
        "flags": {
            "illusion_mode": false,
            "alien_aesthetic": false,
            "lens_mode": "hidden",
            "post_reveal_enabled": false
        },
        "tone_policy": Value::Null,
        "observability_panel": Value::Null
    })
}

fn load_state(policy: &Policy) -> Value {
    let raw = read_json(&policy.paths.state_path);
    if !raw.is_object() {
        return default_state();
    }
    let mut merged = default_state().as_object().cloned().unwrap_or_default();
    for (k, v) in raw.as_object().cloned().unwrap_or_default() {
        merged.insert(k, v);
    }
    if !merged.get("flags").map(Value::is_object).unwrap_or(false) {
        merged.insert("flags".to_string(), default_state()["flags"].clone());
    }
    Value::Object(merged)
}

fn save_state(policy: &Policy, state: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }
    let mut payload = state.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("updated_at".to_string(), Value::String(now_iso()));
    }
    write_json_atomic(&policy.paths.state_path, &payload)
}

fn write_receipt(policy: &Policy, payload: &Value, apply: bool) -> Result<(), String> {
    if !apply {
        return Ok(());
    }
    write_json_atomic(&policy.paths.latest_path, payload)?;
    append_jsonl(&policy.paths.receipts_path, payload)?;
    append_jsonl(&policy.paths.history_path, payload)
}
