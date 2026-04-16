fn normalize_ts_or_now(raw: Option<&Value>, fallback: &str) -> String {
    let value = as_str(raw);
    if value.is_empty() {
        return fallback.to_string();
    }
    chrono::DateTime::parse_from_rfc3339(value.as_str())
        .ok()
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .unwrap_or_else(|| fallback.to_string())
}

fn normalize_eye_lenses(raw: Option<&Value>, policy: &Map<String, Value>) -> Value {
    let max_terms = clamp_i64(policy.get("lens_max_terms"), 4, 64, 16) as usize;
    let max_exclude = clamp_i64(policy.get("lens_max_exclude_terms"), 0, 32, 6) as usize;
    let max_weight = clamp_i64(policy.get("lens_max_weight"), 1, 60, 20);
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (eye_raw, lens_raw) in rows {
            let eye_id = normalize_key(eye_raw, 120);
            if eye_id.is_empty() {
                continue;
            }
            let lens = as_object(Some(lens_raw));
            let include_terms =
                normalize_terms_array(lens.and_then(|v| v.get("include_terms")), max_terms);
            let include_lookup = include_terms
                .iter()
                .map(|term| term.to_ascii_lowercase())
                .collect::<BTreeSet<_>>();
            let exclude_terms =
                normalize_terms_array(lens.and_then(|v| v.get("exclude_terms")), max_exclude)
                    .into_iter()
                    .filter(|term| !include_lookup.contains(&term.to_ascii_lowercase()))
                    .take(max_exclude)
                    .collect::<Vec<_>>();
            let now_ts = now_iso();
            let mut merged = Map::new();
            merged.insert("eye_id".to_string(), Value::String(eye_id.clone()));
            merged.insert(
                "include_terms".to_string(),
                Value::Array(include_terms.iter().cloned().map(Value::String).collect()),
            );
            merged.insert(
                "exclude_terms".to_string(),
                Value::Array(exclude_terms.into_iter().map(Value::String).collect()),
            );
            merged.insert(
                "term_weights".to_string(),
                normalize_term_weights(
                    lens.and_then(|v| v.get("term_weights")),
                    &include_terms,
                    max_weight,
                ),
            );
            merged.insert(
                "baseline_topics".to_string(),
                Value::Array(
                    normalize_terms_array(lens.and_then(|v| v.get("baseline_topics")), max_terms)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            );
            merged.insert(
                "focus_hits_total".to_string(),
                Value::Number(serde_json::Number::from(clamp_i64(
                    lens.and_then(|v| v.get("focus_hits_total")),
                    0,
                    100_000_000,
                    0,
                ))),
            );
            merged.insert(
                "update_count".to_string(),
                Value::Number(serde_json::Number::from(clamp_i64(
                    lens.and_then(|v| v.get("update_count")),
                    0,
                    100_000_000,
                    0,
                ))),
            );
            merged.insert(
                "created_ts".to_string(),
                Value::String(normalize_ts_or_now(lens.and_then(|v| v.get("created_ts")), &now_ts)),
            );
            merged.insert(
                "updated_ts".to_string(),
                Value::String(normalize_ts_or_now(lens.and_then(|v| v.get("updated_ts")), &now_ts)),
            );
            out.insert(eye_id, Value::Object(merged));
        }
    }
    Value::Object(out)
}

fn normalize_recent_map(raw: Option<&Value>, policy: &Map<String, Value>) -> Value {
    let max_age_hours = clamp_i64(policy.get("dedupe_window_hours"), 1, 14 * 24, 36);
    let cutoff = Utc::now().timestamp_millis() - (max_age_hours * 60 * 60 * 1000);
    let mut out = Map::new();
    if let Some(Value::Object(rows)) = raw {
        for (key_raw, value) in rows {
            let key = normalize_key(key_raw, 120);
            if key.is_empty() {
                continue;
            }
            let ts_raw = as_str(Some(value));
            let parsed = chrono::DateTime::parse_from_rfc3339(ts_raw.as_str())
                .ok()
                .map(|dt| dt.timestamp_millis());
            match parsed {
                Some(ms) if ms >= cutoff => {
                    out.insert(
                        key,
                        Value::String(
                            chrono::DateTime::<Utc>::from_timestamp_millis(ms)
                                .unwrap_or_else(Utc::now)
                                .to_rfc3339(),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
    Value::Object(out)
}

fn normalize_policy(raw: Option<&Value>) -> Value {
    let src = as_object(raw);
    let value = json!({
        "refresh_hours": clamp_i64(src.and_then(|v| v.get("refresh_hours")), 1, 24, 4),
        "max_triggers": clamp_i64(src.and_then(|v| v.get("max_triggers")), 8, 200, 48),
        "min_focus_score": clamp_i64(src.and_then(|v| v.get("min_focus_score")), 1, 100, 58),
        "dynamic_focus_gate_enabled": as_bool(src.and_then(|v| v.get("dynamic_focus_gate_enabled")), true),
        "dynamic_focus_window_hours": clamp_i64(src.and_then(|v| v.get("dynamic_focus_window_hours")), 1, 72, 6),
        "dynamic_focus_target_per_window": clamp_i64(src.and_then(|v| v.get("dynamic_focus_target_per_window")), 0, 500, 8),
        "dynamic_focus_floor_score": clamp_i64(src.and_then(|v| v.get("dynamic_focus_floor_score")), 1, 100, 35),
        "dynamic_focus_ceiling_score": clamp_i64(src.and_then(|v| v.get("dynamic_focus_ceiling_score")), 1, 100, 85),
        "dynamic_focus_response": clamp_i64(src.and_then(|v| v.get("dynamic_focus_response")), 0, 60, 14),
        "lens_enabled": as_bool(src.and_then(|v| v.get("lens_enabled")), true),
        "lens_refresh_hours": clamp_i64(src.and_then(|v| v.get("lens_refresh_hours")), 1, 72, 6),
        "lens_window_hours": clamp_i64(src.and_then(|v| v.get("lens_window_hours")), 6, 24 * 14, 48),
        "lens_max_terms": clamp_i64(src.and_then(|v| v.get("lens_max_terms")), 4, 64, 16),
        "lens_min_weight": clamp_i64(src.and_then(|v| v.get("lens_min_weight")), 1, 40, 2),
        "lens_max_weight": clamp_i64(src.and_then(|v| v.get("lens_max_weight")), 1, 60, 20),
        "lens_decay": clamp_number(src.and_then(|v| v.get("lens_decay")), 0.5, 0.99, 0.9),
        "lens_step_up": clamp_i64(src.and_then(|v| v.get("lens_step_up")), 1, 10, 2),
        "lens_step_down": clamp_i64(src.and_then(|v| v.get("lens_step_down")), 1, 10, 1),
        "lens_exclude_threshold": clamp_i64(src.and_then(|v| v.get("lens_exclude_threshold")), 1, 50, 4),
        "lens_max_exclude_terms": clamp_i64(src.and_then(|v| v.get("lens_max_exclude_terms")), 0, 32, 6),
        "lens_min_support": clamp_i64(src.and_then(|v| v.get("lens_min_support")), 1, 20, 2),
        "lens_cross_signal_boost": clamp_i64(src.and_then(|v| v.get("lens_cross_signal_boost")), 0, 20, 3),
        "max_focus_items_per_eye": clamp_i64(src.and_then(|v| v.get("max_focus_items_per_eye")), 1, 10, 2),
        "max_focus_items_per_run": clamp_i64(src.and_then(|v| v.get("max_focus_items_per_run")), 1, 50, 6),
        "dedupe_window_hours": clamp_i64(src.and_then(|v| v.get("dedupe_window_hours")), 1, 14 * 24, 36),
        "expand_fetch_enabled": as_bool(src.and_then(|v| v.get("expand_fetch_enabled")), true),
        "focus_fetch_timeout_ms": clamp_i64(src.and_then(|v| v.get("focus_fetch_timeout_ms")), 500, 15000, 4500),
        "focus_fetch_max_bytes": clamp_i64(src.and_then(|v| v.get("focus_fetch_max_bytes")), 4096, 1048576, 131072),
        "llm_backstop_enabled": as_bool(src.and_then(|v| v.get("llm_backstop_enabled")), false),
        "llm_uncertain_min_score": clamp_i64(src.and_then(|v| v.get("llm_uncertain_min_score")), 1, 99, 48),
        "llm_uncertain_max_score": clamp_i64(src.and_then(|v| v.get("llm_uncertain_max_score")), 1, 100, 57),
    });
    value
}

fn normalize_trigger(
    raw: &Map<String, Value>,
    taken: &mut BTreeSet<String>,
    now_ts: &str,
) -> Option<Value> {
    let key = normalize_key(&as_str(raw.get("key").or_else(|| raw.get("pattern"))), 120);
    if key.is_empty() {
        return None;
    }
    let candidate = as_str(raw.get("uid"));
    let uid = if !candidate.is_empty() && is_alnum(&candidate) && !taken.contains(&candidate) {
        candidate
    } else {
        let seeded = stable_uid(&format!("focus_trigger|{key}|v1"), "ft", 24);
        if !taken.contains(&seeded) {
            seeded
        } else {
            let mut generated = random_uid("ft", 24);
            let mut attempts = 0;
            while taken.contains(&generated) && attempts < 8 {
                generated = random_uid("ft", 24);
                attempts += 1;
            }
            generated
        }
    };
    taken.insert(uid.clone());
    let source_signals = raw
        .get("source_signals")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut out = Vec::new();
            let mut seen = BTreeSet::new();
            for row in rows {
                let signal = normalize_key(&as_str(Some(row)), 120);
                if signal.is_empty() || seen.contains(&signal) {
                    continue;
                }
                seen.insert(signal.clone());
                out.push(Value::String(signal));
                if out.len() >= 8 {
                    break;
                }
            }
            out
        })
        .unwrap_or_default();
    let status_raw = as_str(raw.get("status")).to_ascii_lowercase();
    let pattern_value = {
        let value =
            clean_text(raw.get("pattern").or_else(|| raw.get("key")), 240).to_ascii_lowercase();
        if value.is_empty() {
            normalize_key(&as_str(raw.get("key")), 120)
        } else {
            value
        }
    };
    let source_value = {
        let value = clean_text(raw.get("source"), 120).to_ascii_lowercase();
        if value.is_empty() {
            "auto".to_string()
        } else {
            value
        }
    };
    let last_hit_ts = {
        let value = as_str(raw.get("last_hit_ts"));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let created_ts = {
        let value = as_str(raw.get("created_ts"));
        if value.is_empty() {
            Value::String(now_ts.to_string())
        } else {
            Value::String(value)
        }
    };
    let updated_ts = {
        let value = as_str(raw.get("updated_ts"));
        if value.is_empty() {
            Value::String(now_ts.to_string())
        } else {
            Value::String(value)
        }
    };
    Some(json!({
        "uid": uid,
        "key": key,
        "pattern": pattern_value,
        "mode": if as_str(raw.get("mode")).to_ascii_lowercase() == "exact" { "exact" } else { "contains" },
        "source": source_value,
        "source_signals": source_signals,
        "status": if status_raw == "disabled" { "disabled" } else { "active" },
        "weight": clamp_i64(raw.get("weight"), 1, 100, 1),
        "cooldown_minutes": clamp_i64(raw.get("cooldown_minutes"), 0, 24 * 60, 90),
        "hit_count": clamp_i64(raw.get("hit_count"), 0, 1_000_000, 0),
        "last_hit_ts": last_hit_ts,
        "created_ts": created_ts,
        "updated_ts": updated_ts
    }))
}

fn default_focus_state() -> Value {
    json!({
        "version": "1.0",
        "policy": normalize_policy(None),
        "triggers": [],
        "eye_lenses": {},
        "recent_focus_items": {},
        "last_refresh_ts": Value::Null,
        "last_refresh_sources": {},
        "last_lens_refresh_ts": Value::Null,
        "last_lens_refresh_sources": {},
        "stats": {
            "refresh_count": 0,
            "lens_refresh_count": 0,
            "focused_items_total": 0,
            "last_focus_ts": Value::Null
        }
    })
}

fn normalize_state(raw: Option<&Value>, fallback: Option<&Value>) -> Value {
    let base = default_focus_state();
    let src = raw
        .and_then(Value::as_object)
        .or_else(|| fallback.and_then(Value::as_object));
    let now_ts = now_iso();
    let policy_value = normalize_policy(src.and_then(|v| v.get("policy")));
    let policy = policy_value.as_object().cloned().unwrap_or_default();

    let mut taken = BTreeSet::new();
    let mut triggers = Vec::new();
    if let Some(rows) = src
        .and_then(|v| v.get("triggers"))
        .and_then(Value::as_array)
    {
        for row in rows {
            if let Some(obj) = row.as_object() {
                if let Some(normalized) = normalize_trigger(obj, &mut taken, &now_ts) {
                    triggers.push(normalized);
                }
            }
        }
    }
    triggers.sort_by(|a, b| {
        let aw = a.get("weight").and_then(Value::as_i64).unwrap_or(0);
        let bw = b.get("weight").and_then(Value::as_i64).unwrap_or(0);
        bw.cmp(&aw).then_with(|| {
            let ak = a.get("key").and_then(Value::as_str).unwrap_or("");
            let bk = b.get("key").and_then(Value::as_str).unwrap_or("");
            ak.cmp(bk)
        })
    });
    let max_triggers = clamp_i64(policy.get("max_triggers"), 8, 200, 48) as usize;
    triggers.truncate(max_triggers);

    let stats = src.and_then(|v| v.get("stats")).and_then(Value::as_object);
    let last_refresh_ts = {
        let value = as_str(src.and_then(|v| v.get("last_refresh_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let last_lens_refresh_ts = {
        let value = as_str(src.and_then(|v| v.get("last_lens_refresh_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let last_focus_ts = {
        let value = as_str(stats.and_then(|v| v.get("last_focus_ts")));
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    json!({
        "version": as_str(src.and_then(|v| v.get("version")).or(Some(&base["version"]))),
        "policy": policy_value,
        "triggers": triggers,
        "eye_lenses": normalize_eye_lenses(src.and_then(|v| v.get("eye_lenses")), &policy),
        "recent_focus_items": normalize_recent_map(src.and_then(|v| v.get("recent_focus_items")), &policy),
        "last_refresh_ts": last_refresh_ts,
        "last_refresh_sources": src.and_then(|v| v.get("last_refresh_sources")).cloned().unwrap_or_else(|| json!({})),
        "last_lens_refresh_ts": last_lens_refresh_ts,
        "last_lens_refresh_sources": src.and_then(|v| v.get("last_lens_refresh_sources")).cloned().unwrap_or_else(|| json!({})),
        "stats": {
            "refresh_count": clamp_i64(stats.and_then(|v| v.get("refresh_count")), 0, 1_000_000, 0),
            "lens_refresh_count": clamp_i64(stats.and_then(|v| v.get("lens_refresh_count")), 0, 1_000_000, 0),
            "focused_items_total": clamp_i64(stats.and_then(|v| v.get("focused_items_total")), 0, 100_000_000, 0),
            "last_focus_ts": last_focus_ts
        }
    })
}

fn load_pointer_index(root: &Path) -> Value {
    read_json_value(&adaptive_pointer_index_path(root))
        .filter(|value| value.get("pointers").and_then(Value::as_object).is_some())
        .unwrap_or_else(|| json!({"version":"1.0","pointers":{}}))
}

fn save_pointer_index(root: &Path, value: &Value) -> Result<(), String> {
    write_json_atomic(&adaptive_pointer_index_path(root), value)
}

fn append_pointer_rows(root: &Path, abs_path: &Path, state: &Value) -> Result<(), String> {
    let pointer_path = adaptive_pointers_path(root);
    let mut index = load_pointer_index(root);
    let pointers = index
        .get_mut("pointers")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "focus_trigger_store_kernel_invalid_pointer_index".to_string())?;
    let triggers = state
        .get("triggers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for trigger in triggers {
        let key = format!(
            "focus_trigger|{}|{}",
            as_str(trigger.get("uid")),
            abs_path.display()
        );
        let row = json!({
            "kind": "focus_trigger",
            "uid": trigger.get("uid").cloned().unwrap_or(Value::Null),
            "entity_id": trigger.get("key").cloned().unwrap_or(Value::Null),
            "path_ref": abs_path.to_string_lossy(),
            "status": trigger.get("status").cloned().unwrap_or(Value::Null),
            "summary": trigger.get("pattern").cloned().unwrap_or(Value::Null),
            "tags": ["focus", "adaptive"],
            "ts": now_iso()
        });
        let digest = deterministic_receipt_hash(&row);
        let existing = pointers.get(&key).and_then(Value::as_str).unwrap_or("");
        if existing == digest {
            continue;
        }
        append_jsonl(&pointer_path, &row)?;
        pointers.insert(key, Value::String(digest));
    }
    save_pointer_index(root, &index)
}

fn append_mutation_log(
    root: &Path,
    abs_path: &Path,
    meta: &Map<String, Value>,
    state: &Value,
    reason: &str,
) -> Result<(), String> {
    let reason_value = {
        let value = clean_text(meta.get("reason"), 160);
        if value.is_empty() {
            reason.to_string()
        } else {
            value
        }
    };
    let source_value = {
        let value = clean_text(meta.get("source"), 180);
        if value.is_empty() {
            "core/layer0/ops::focus_trigger_store_kernel".to_string()
        } else {
            value
        }
    };
    let actor_value = {
        let value = clean_text(meta.get("actor"), 80);
        if value.is_empty() {
            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
        } else {
            value
        }
    };
    let row = json!({
        "kind": "focus_trigger_store",
        "ts": now_iso(),
        "path": abs_path.to_string_lossy(),
        "reason": reason_value,
        "source": source_value,
        "actor": actor_value,
        "trigger_count": state.get("triggers").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "receipt_hash": deterministic_receipt_hash(state)
    });
    append_jsonl(&mutation_log_path(root), &row)
}
