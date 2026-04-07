fn default_virtual_keys() -> Value {
    json!({
        "type": "infring_provider_virtual_keys",
        "updated_at": crate::now_iso(),
        "keys": {}
    })
}

fn sanitize_virtual_key_id(raw: &str) -> String {
    clean_text(raw, 100)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .collect::<String>()
}

fn normalized_virtual_key_record(key_id: &str, source: &Value, existing: Option<&Value>) -> Value {
    let existing_obj = existing.and_then(Value::as_object);
    let provider = normalize_provider_id(
        source
            .get("provider")
            .and_then(Value::as_str)
            .or_else(|| existing_obj.and_then(|row| row.get("provider").and_then(Value::as_str)))
            .unwrap_or("auto"),
    );
    let model = clean_text(
        source
            .get("model")
            .and_then(Value::as_str)
            .or_else(|| existing_obj.and_then(|row| row.get("model").and_then(Value::as_str)))
            .unwrap_or("auto"),
        240,
    );
    let team_id = clean_text(
        source
            .get("team_id")
            .and_then(Value::as_str)
            .or_else(|| existing_obj.and_then(|row| row.get("team_id").and_then(Value::as_str)))
            .unwrap_or("default"),
        120,
    );
    let active = source
        .get("active")
        .and_then(Value::as_bool)
        .or_else(|| existing_obj.and_then(|row| row.get("active").and_then(Value::as_bool)))
        .unwrap_or(true);
    let rate_limit_rpm = parse_u64_like(
        source
            .get("rate_limit_rpm")
            .or_else(|| existing_obj.and_then(|row| row.get("rate_limit_rpm"))),
        120,
        1,
        10_000,
    );
    let budget_limit_usd = parse_f64_like(
        source
            .get("budget_limit_usd")
            .or_else(|| existing_obj.and_then(|row| row.get("budget_limit_usd"))),
        100.0,
        0.0,
        1_000_000_000.0,
    );
    let spent_usd = parse_f64_like(
        source
            .get("spent_usd")
            .or_else(|| existing_obj.and_then(|row| row.get("spent_usd"))),
        0.0,
        0.0,
        1_000_000_000.0,
    );
    let window_minute = parse_u64_like(
        existing_obj.and_then(|row| row.get("window_minute")),
        0,
        0,
        u64::MAX,
    );
    let window_calls = parse_u64_like(
        existing_obj.and_then(|row| row.get("window_calls")),
        0,
        0,
        u64::MAX,
    );
    let created_at = clean_text(
        existing_obj
            .and_then(|row| row.get("created_at").and_then(Value::as_str))
            .unwrap_or(&crate::now_iso()),
        80,
    );
    json!({
        "id": key_id,
        "provider": if provider.is_empty() { "auto" } else { &provider },
        "model": if model.is_empty() { "auto" } else { &model },
        "team_id": if team_id.is_empty() { "default" } else { &team_id },
        "active": active,
        "rate_limit_rpm": rate_limit_rpm,
        "budget_limit_usd": budget_limit_usd,
        "spent_usd": spent_usd,
        "window_minute": window_minute,
        "window_calls": window_calls,
        "created_at": created_at,
        "updated_at": crate::now_iso(),
        "key_hash": crate::deterministic_receipt_hash(&json!({"id": key_id, "provider": provider, "model": model, "team_id": team_id}))
    })
}

fn load_virtual_keys(root: &Path) -> Value {
    let raw = read_json(&virtual_keys_path(root)).unwrap_or_else(default_virtual_keys);
    if !raw.is_object() {
        return default_virtual_keys();
    }
    let mut out = default_virtual_keys();
    if let Some(keys) = raw.get("keys").and_then(Value::as_object) {
        for (id, row) in keys {
            let key_id = sanitize_virtual_key_id(id);
            if key_id.is_empty() {
                continue;
            }
            out["keys"][key_id.clone()] = normalized_virtual_key_record(&key_id, row, Some(row));
        }
    }
    out
}

fn save_virtual_keys(root: &Path, mut keys: Value) {
    if !keys.is_object() {
        keys = default_virtual_keys();
    }
    keys["type"] = json!("infring_provider_virtual_keys");
    keys["updated_at"] = json!(crate::now_iso());
    write_json_pretty(&virtual_keys_path(root), &keys);
}

pub fn virtual_keys_payload(root: &Path) -> Value {
    let keys = load_virtual_keys(root);
    let rows = keys
        .get("keys")
        .and_then(Value::as_object)
        .map(|obj| obj.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "ok": true,
        "keys": rows,
        "count": rows.len()
    })
}

pub fn upsert_virtual_key(root: &Path, key_id: &str, patch: &Value) -> Value {
    let id = sanitize_virtual_key_id(key_id);
    if id.is_empty() || !patch.is_object() {
        return json!({"ok": false, "error": "virtual_key_invalid"});
    }
    let mut keys = load_virtual_keys(root);
    let existing = keys
        .get("keys")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&id));
    let row = normalized_virtual_key_record(&id, patch, existing);
    keys["keys"][id.clone()] = row.clone();
    save_virtual_keys(root, keys);
    json!({"ok": true, "key": row})
}

pub fn remove_virtual_key(root: &Path, key_id: &str) -> Value {
    let id = sanitize_virtual_key_id(key_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "virtual_key_invalid"});
    }
    let mut keys = load_virtual_keys(root);
    let removed = keys
        .get_mut("keys")
        .and_then(Value::as_object_mut)
        .and_then(|obj| obj.remove(&id))
        .is_some();
    save_virtual_keys(root, keys);
    json!({"ok": removed, "removed": removed, "id": id})
}

fn current_epoch_minute() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() / 60)
        .unwrap_or(0)
}

pub fn resolve_virtual_key_route(root: &Path, key_id: &str) -> Value {
    let id = sanitize_virtual_key_id(key_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "virtual_key_invalid"});
    }
    let keys = load_virtual_keys(root);
    let Some(row) = keys
        .get("keys")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&id))
        .cloned()
    else {
        return json!({"ok": false, "error": "virtual_key_not_found", "id": id});
    };
    json!({
        "ok": true,
        "id": id,
        "provider": clean_text(row.get("provider").and_then(Value::as_str).unwrap_or(""), 120),
        "model": clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240),
        "team_id": clean_text(row.get("team_id").and_then(Value::as_str).unwrap_or("default"), 120),
        "active": row.get("active").and_then(Value::as_bool).unwrap_or(true),
        "budget_limit_usd": parse_f64_like(row.get("budget_limit_usd"), 0.0, 0.0, 1_000_000_000.0),
        "spent_usd": parse_f64_like(row.get("spent_usd"), 0.0, 0.0, 1_000_000_000.0),
        "rate_limit_rpm": parse_u64_like(row.get("rate_limit_rpm"), 120, 1, 10_000)
    })
}

pub fn reserve_virtual_key_slot(root: &Path, key_id: &str) -> Value {
    let id = sanitize_virtual_key_id(key_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "virtual_key_invalid"});
    }
    let mut keys = load_virtual_keys(root);
    let Some(row) = keys
        .get_mut("keys")
        .and_then(Value::as_object_mut)
        .and_then(|obj| obj.get_mut(&id))
    else {
        return json!({"ok": false, "error": "virtual_key_not_found", "id": id});
    };
    if !row.get("active").and_then(Value::as_bool).unwrap_or(true) {
        return json!({"ok": false, "error": "virtual_key_inactive", "id": id});
    }
    let budget_limit = parse_f64_like(row.get("budget_limit_usd"), 0.0, 0.0, 1_000_000_000.0);
    let spent = parse_f64_like(row.get("spent_usd"), 0.0, 0.0, 1_000_000_000.0);
    if budget_limit > 0.0 && spent >= budget_limit {
        return json!({
            "ok": false,
            "error": "virtual_key_budget_exceeded",
            "id": id,
            "budget_limit_usd": budget_limit,
            "spent_usd": spent
        });
    }
    let rate_limit = parse_u64_like(row.get("rate_limit_rpm"), 120, 1, 10_000);
    let now_minute = current_epoch_minute();
    let previous_window = parse_u64_like(row.get("window_minute"), 0, 0, u64::MAX);
    let mut window_calls = parse_u64_like(row.get("window_calls"), 0, 0, u64::MAX);
    if previous_window != now_minute {
        row["window_minute"] = json!(now_minute);
        window_calls = 0;
    }
    if window_calls >= rate_limit {
        return json!({
            "ok": false,
            "error": "virtual_key_rate_limited",
            "id": id,
            "rate_limit_rpm": rate_limit,
            "retry_after_sec": 60
        });
    }
    row["window_calls"] = json!(window_calls.saturating_add(1));
    row["updated_at"] = json!(crate::now_iso());
    row["last_reserved_at"] = json!(crate::now_iso());
    let route_provider = clean_text(
        row.get("provider").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    let route_model = clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240);
    let team_id = clean_text(
        row.get("team_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    save_virtual_keys(root, keys);
    json!({
        "ok": true,
        "id": id,
        "provider": route_provider,
        "model": route_model,
        "team_id": team_id,
        "rate_limit_rpm": rate_limit,
        "budget_limit_usd": budget_limit,
        "spent_usd": spent
    })
}

pub fn record_virtual_key_usage(root: &Path, key_id: &str, cost_usd: f64) -> Value {
    let id = sanitize_virtual_key_id(key_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "virtual_key_invalid"});
    }
    let mut keys = load_virtual_keys(root);
    let Some(row) = keys
        .get_mut("keys")
        .and_then(Value::as_object_mut)
        .and_then(|obj| obj.get_mut(&id))
    else {
        return json!({"ok": false, "error": "virtual_key_not_found", "id": id});
    };
    let spent = parse_f64_like(row.get("spent_usd"), 0.0, 0.0, 1_000_000_000.0);
    let budget = parse_f64_like(row.get("budget_limit_usd"), 0.0, 0.0, 1_000_000_000.0);
    let next_spent = (spent + cost_usd.max(0.0)).max(0.0);
    row["spent_usd"] = json!(next_spent);
    row["updated_at"] = json!(crate::now_iso());
    row["last_spend_delta_usd"] = json!(cost_usd.max(0.0));
    save_virtual_keys(root, keys);
    json!({
        "ok": true,
        "id": id,
        "spent_usd": next_spent,
        "budget_limit_usd": budget,
        "remaining_usd": (budget - next_spent).max(0.0),
        "budget_exhausted": budget > 0.0 && next_spent >= budget
    })
}
