fn event_priority(row: &Value) -> i64 {
    row.get("priority")
        .and_then(Value::as_i64)
        .unwrap_or(20)
        .clamp(1, 1000)
}

fn event_attention_lane_rank(row: &Value) -> i64 {
    let lane = row
        .get("queue_lane")
        .and_then(Value::as_str)
        .unwrap_or("standard");
    attention_lane_rank(lane)
}

fn event_deadline_ts_ms(row: &Value) -> i64 {
    let direct = row
        .get("deadline_at")
        .and_then(Value::as_str)
        .and_then(parse_ts_ms);
    let raw_event = row
        .pointer("/raw_event/deadline_at")
        .and_then(Value::as_str)
        .and_then(parse_ts_ms);
    direct.or(raw_event).unwrap_or(i64::MAX)
}

fn event_ts_ms(row: &Value) -> i64 {
    row.get("ts")
        .and_then(Value::as_str)
        .and_then(parse_ts_ms)
        .unwrap_or(i64::MAX)
}

fn event_attention_key(row: &Value) -> String {
    row.get("attention_key")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn sort_active_rows(rows: &mut [Value]) {
    rows.sort_by(|a, b| {
        event_attention_lane_rank(b)
            .cmp(&event_attention_lane_rank(a))
            .then_with(|| event_band_rank(b).cmp(&event_band_rank(a)))
            .then_with(|| event_priority(b).cmp(&event_priority(a)))
            .then_with(|| {
                event_score(b)
                    .partial_cmp(&event_score(a))
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| event_deadline_ts_ms(a).cmp(&event_deadline_ts_ms(b)))
            .then_with(|| event_ts_ms(a).cmp(&event_ts_ms(b)))
            .then_with(|| event_attention_key(a).cmp(&event_attention_key(b)))
    });
}

fn sort_active_rows_with_authority(rows: &mut Vec<Value>) {
    if let Some(prioritized) = prioritize_rows_via_layer2(rows.as_slice()) {
        *rows = prioritized;
        return;
    }
    sort_active_rows(rows.as_mut_slice());
}

fn default_priority_map() -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    out.insert("critical".to_string(), 100);
    out.insert("warn".to_string(), 60);
    out.insert("info".to_string(), 20);
    out
}

fn load_contract(root: &Path) -> AttentionContract {
    let default_policy = root.join("config").join("mech_suit_mode_policy.json");
    let policy_path = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or(default_policy);
    let policy = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let enabled = bool_from_env("MECH_SUIT_MODE_FORCE").unwrap_or_else(|| {
        policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    });
    let eyes = policy.get("eyes").and_then(Value::as_object);
    let contract_obj = eyes
        .and_then(|v| v.get("attention_contract"))
        .and_then(Value::as_object);

    let mut priority_map = default_priority_map();
    if let Some(obj) = contract_obj
        .and_then(|v| v.get("priority_map"))
        .and_then(Value::as_object)
    {
        for (k, v) in obj {
            if let Some(n) = v.as_i64() {
                priority_map.insert(k.trim().to_ascii_lowercase(), n.clamp(1, 1000));
            }
        }
    }

    let escalate_levels = contract_obj
        .and_then(|v| v.get("escalate_levels"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec!["critical".to_string()]);
    let allow_layer0_fallback = bool_from_env("INFRING_ATTENTION_ALLOW_LAYER0_FALLBACK")
        .or_else(|| {
            contract_obj
                .and_then(|v| v.get("allow_layer0_importance_fallback"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);

    let max_queue_depth = contract_obj
        .and_then(|v| v.get("max_queue_depth"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(2048)
        .clamp(1, 200_000);
    let soft_watermark_pct = contract_obj
        .and_then(|v| v.get("backpressure_soft_watermark_pct"))
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(0.75)
        .clamp(0.10, 1.0);
    let hard_watermark_pct = contract_obj
        .and_then(|v| v.get("backpressure_hard_watermark_pct"))
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(1.0)
        .clamp(soft_watermark_pct, 1.0);
    let backpressure_soft_watermark =
        ((max_queue_depth as f64 * soft_watermark_pct).ceil() as usize).clamp(1, max_queue_depth);
    let backpressure_hard_watermark = ((max_queue_depth as f64 * hard_watermark_pct).ceil()
        as usize)
        .clamp(backpressure_soft_watermark, max_queue_depth);

    AttentionContract {
        enabled,
        push_attention_queue: eyes
            .and_then(|v| v.get("push_attention_queue"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        queue_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("attention_queue_path")),
            "local/state/attention/queue.jsonl",
        ),
        receipts_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("receipts_path")),
            "local/state/attention/receipts.jsonl",
        ),
        latest_path: normalize_path(
            root,
            eyes.and_then(|v| v.get("latest_path")),
            "local/state/attention/latest.json",
        ),
        cursor_state_path: normalize_path(
            root,
            contract_obj.and_then(|v| v.get("cursor_state_path")),
            "local/state/attention/cursor_state.json",
        ),
        max_queue_depth,
        backpressure_soft_watermark,
        backpressure_hard_watermark,
        max_batch_size: contract_obj
            .and_then(|v| v.get("max_batch_size"))
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(64)
            .clamp(1, 512),
        ttl_hours: contract_obj
            .and_then(|v| v.get("ttl_hours"))
            .and_then(Value::as_i64)
            .unwrap_or(48)
            .clamp(1, 24 * 90),
        dedupe_window_hours: contract_obj
            .and_then(|v| v.get("dedupe_window_hours"))
            .and_then(Value::as_i64)
            .unwrap_or(24)
            .clamp(1, 24 * 90),
        backpressure_drop_below: contract_obj
            .and_then(|v| v.get("backpressure_drop_below"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "critical".to_string()),
        escalate_levels,
        priority_map,
        require_layer2_authority: !allow_layer0_fallback,
    }
}

fn parse_event(flags: &BTreeMap<String, String>) -> Result<Value, String> {
    if let Some(raw) = flags.get("event-json-base64") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.as_bytes())
            .map_err(|err| format!("event_json_base64_invalid:{err}"))?;
        let text =
            String::from_utf8(bytes).map_err(|err| format!("event_json_utf8_invalid:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("event_json_invalid:{err}"));
    }
    if let Some(raw) = flags.get("event-json") {
        return serde_json::from_str::<Value>(raw)
            .map_err(|err| format!("event_json_invalid:{err}"));
    }
    Err("missing_event_json".to_string())
}

fn normalize_event(event: &Value, contract: &AttentionContract) -> Result<Value, String> {
    let ts = clean_text(event.get("ts").and_then(Value::as_str), 64);
    let ts = if ts.is_empty() { now_iso() } else { ts };
    let source = clean_text(event.get("source").and_then(Value::as_str), 80);
    let source = if source.is_empty() {
        "unknown_source".to_string()
    } else {
        source
    };
    let source_type = clean_text(
        event
            .get("source_type")
            .and_then(Value::as_str)
            .or_else(|| event.get("type").and_then(Value::as_str)),
        80,
    );
    let source_type = if source_type.is_empty() {
        "unknown_type".to_string()
    } else {
        source_type
    };
    let severity = normalize_severity(event.get("severity").and_then(Value::as_str));
    let summary = clean_text(event.get("summary").and_then(Value::as_str), 180);
    let summary = if summary.is_empty() {
        format!("{source_type}:{source}")
    } else {
        summary
    };
    let attention_key = clean_text(event.get("attention_key").and_then(Value::as_str), 240);
    let attention_key = if attention_key.is_empty() {
        format!("{source}:{source_type}:{severity}:{summary}")
    } else {
        attention_key
    };
    let importance_fallback = infer_from_event(event, &severity, &contract.priority_map);
    let layer2_decision = evaluate_importance_via_layer2(event, &importance_fallback);
    if layer2_decision.is_none() && contract.require_layer2_authority {
        return Err("layer2_priority_authority_unavailable".to_string());
    }
    let score = layer2_decision
        .as_ref()
        .map(|row| row.score)
        .unwrap_or(importance_fallback.score);
    let band = layer2_decision
        .as_ref()
        .map(|row| row.band.clone())
        .unwrap_or_else(|| importance_fallback.band.clone());
    let priority = layer2_decision
        .as_ref()
        .map(|row| row.priority)
        .unwrap_or(importance_fallback.priority);
    let queue_lane = classify_attention_lane(&source, &source_type, &severity, &summary, &band);
    let ttl_ms = contract.ttl_hours.saturating_mul(60 * 60 * 1000);
    let event_ts_ms = parse_ts_ms(&ts).unwrap_or_else(|| Utc::now().timestamp_millis());
    let expires_at = ts_ms_to_iso(event_ts_ms.saturating_add(ttl_ms));
    let escalate_required_by_policy = contract.escalate_levels.iter().any(|row| row == &severity);
    let escalate_required_by_importance = score >= 0.85;
    let escalate_required = escalate_required_by_policy || escalate_required_by_importance;
    let initiative_action = layer2_decision
        .as_ref()
        .map(|row| row.initiative_action.clone())
        .unwrap_or_else(|| importance_fallback.initiative_action.clone());
    let initiative_policy_version = layer2_decision
        .as_ref()
        .map(|row| row.initiative_policy_version.clone())
        .unwrap_or_else(|| importance_fallback.initiative_policy_version.clone());
    let initiative_repeat_after_sec = layer2_decision
        .as_ref()
        .map(|row| row.initiative_repeat_after_sec)
        .unwrap_or(importance_fallback.initiative_repeat_after_sec);
    let initiative_max_messages = layer2_decision
        .as_ref()
        .map(|row| row.initiative_max_messages)
        .unwrap_or(importance_fallback.initiative_max_messages);
    let queue_front = layer2_decision
        .as_ref()
        .map(|row| row.front_jump)
        .unwrap_or(importance_fallback.queue_front);
    let mut importance_json = importance_to_json(&importance_fallback);
    importance_json["authority"] = Value::String(if layer2_decision.is_some() {
        "core.layer2.execution.initiative".to_string()
    } else {
        "core.layer0.ops.importance_fallback".to_string()
    });
    importance_json["score"] = json!(score);
    importance_json["band"] = json!(band.clone());
    importance_json["priority"] = json!(priority);
    importance_json["initiative_policy_version"] = json!(initiative_policy_version.clone());
    if let Some(decision) = &layer2_decision {
        importance_json["layer2"] = json!({
            "front_jump": decision.front_jump,
            "initiative_action": decision.initiative_action,
            "initiative_policy_version": decision.initiative_policy_version,
            "initiative_repeat_after_sec": decision.initiative_repeat_after_sec,
            "initiative_max_messages": decision.initiative_max_messages
        });
    }
    let mut out = json!({
        "ts": ts,
        "type": "attention_event",
        "source": source,
        "source_type": source_type,
        "severity": severity,
        "priority": priority,
        "score": score,
        "band": band,
        "queue_lane": queue_lane,
        "summary": summary,
        "attention_key": attention_key,
        "ttl_hours": contract.ttl_hours,
        "dedupe_window_hours": contract.dedupe_window_hours,
        "expires_at": expires_at,
        "escalate_required": escalate_required,
        "escalation_authority": "runtime_policy",
        "initiative_action": initiative_action,
        "initiative_policy_version": initiative_policy_version,
        "initiative_repeat_after_sec": initiative_repeat_after_sec,
        "initiative_max_messages": initiative_max_messages,
        "queue_front": queue_front,
        "importance": importance_json,
        "raw_event": event
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    Ok(out)
}

fn dedupe_hit(active_rows: &[Value], candidate: &Value, dedupe_window_hours: i64) -> bool {
    let key = candidate
        .get("attention_key")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if key.trim().is_empty() {
        return false;
    }
    let candidate_ts = candidate
        .get("ts")
        .and_then(Value::as_str)
        .and_then(parse_ts_ms)
        .unwrap_or_else(|| Utc::now().timestamp_millis());
    let window_ms = dedupe_window_hours.saturating_mul(60 * 60 * 1000);
    active_rows.iter().any(|row| {
        let row_key = row
            .get("attention_key")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if row_key != key {
            return false;
        }
        let row_ts = row
            .get("ts")
            .and_then(Value::as_str)
            .and_then(parse_ts_ms)
            .unwrap_or(0);
        candidate_ts.saturating_sub(row_ts).abs() <= window_ms
    })
}

fn prune_expired(rows: Vec<Value>) -> (Vec<Value>, usize) {
    let now_ms = Utc::now().timestamp_millis();
    let mut kept = Vec::with_capacity(rows.len());
    let mut dropped = 0usize;
    for row in rows {
        let expired = row
            .get("expires_at")
            .and_then(Value::as_str)
            .and_then(parse_ts_ms)
            .map(|ts| ts <= now_ms)
            .unwrap_or(false);
        if expired {
            dropped += 1;
        } else {
            kept.push(row);
        }
    }
    (kept, dropped)
}

fn contract_snapshot(contract: &AttentionContract) -> Value {
    let queue_capacity = contract.max_queue_depth.max(1) as f64;
    json!({
        "enabled": contract.enabled,
        "push_attention_queue": contract.push_attention_queue,
        "queue_path": contract.queue_path.to_string_lossy().to_string(),
        "receipts_path": contract.receipts_path.to_string_lossy().to_string(),
        "latest_path": contract.latest_path.to_string_lossy().to_string(),
        "cursor_state_path": contract.cursor_state_path.to_string_lossy().to_string(),
        "max_queue_depth": contract.max_queue_depth,
        "backpressure_soft_watermark": contract.backpressure_soft_watermark,
        "backpressure_hard_watermark": contract.backpressure_hard_watermark,
        "backpressure_soft_watermark_ratio": (contract.backpressure_soft_watermark as f64 / queue_capacity),
        "backpressure_hard_watermark_ratio": (contract.backpressure_hard_watermark as f64 / queue_capacity),
        "max_batch_size": contract.max_batch_size,
        "ttl_hours": contract.ttl_hours,
        "dedupe_window_hours": contract.dedupe_window_hours,
        "backpressure_drop_below": contract.backpressure_drop_below,
        "escalate_levels": contract.escalate_levels,
        "priority_map": contract.priority_map,
        "require_layer2_authority": contract.require_layer2_authority
    })
}

fn emit(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| {
            "{\"ok\":false,\"type\":\"attention_queue_encode_failed\"}".to_string()
        })
    );
}

fn update_latest(
    contract: &AttentionContract,
    action: &str,
    queue_depth: usize,
    event: Option<&Value>,
    expired_pruned: usize,
) -> Value {
    let mut latest = read_json(&contract.latest_path).unwrap_or_else(|| json!({}));
    if !latest.is_object() {
        latest = json!({});
    }
    let ts = now_iso();
    latest["ts"] = Value::String(ts.clone());
    latest["active"] = Value::Bool(true);
    latest["queue_depth"] = Value::Number((queue_depth as u64).into());
    latest["last_action"] = Value::String(action.to_string());
    latest["expired_pruned"] = Value::Number((expired_pruned as u64).into());
    let queued_total = latest
        .get("queued_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let deduped_total = latest
        .get("deduped_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let dropped_total = latest
        .get("dropped_total")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    match action {
        "admitted" => {
            latest["queued_total"] = Value::Number((queued_total + 1).into());
        }
        "deduped" => {
            latest["deduped_total"] = Value::Number((deduped_total + 1).into());
        }
        "dropped_backpressure" => {
            latest["dropped_total"] = Value::Number((dropped_total + 1).into());
        }
        _ => {}
    }
    if let Some(evt) = event {
        latest["last_event"] = json!({
            "ts": evt.get("ts").and_then(Value::as_str).unwrap_or(&ts),
            "source": evt.get("source").and_then(Value::as_str).unwrap_or("unknown_source"),
            "source_type": evt.get("source_type").and_then(Value::as_str).unwrap_or("unknown_type"),
            "severity": evt.get("severity").and_then(Value::as_str).unwrap_or("info"),
            "summary": evt.get("summary").and_then(Value::as_str).unwrap_or("attention_event"),
            "priority": evt.get("priority").cloned().unwrap_or(Value::Number(20.into())),
            "score": evt.get("score").cloned().unwrap_or(Value::Number(serde_json::Number::from_f64(0.0).unwrap_or(0.into()))),
            "band": evt.get("band").cloned().unwrap_or(Value::String("p4".to_string())),
            "queue_lane": evt.get("queue_lane").cloned().unwrap_or(Value::String("standard".to_string())),
            "initiative_action": evt.get("initiative_action").cloned().unwrap_or(Value::String("silent".to_string())),
            "initiative_policy_version": evt.get("initiative_policy_version").cloned().unwrap_or(Value::Null)
        });
    }
    write_json(&contract.latest_path, &latest);
    latest
}
