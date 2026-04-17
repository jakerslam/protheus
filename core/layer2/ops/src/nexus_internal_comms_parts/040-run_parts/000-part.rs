fn round_metric(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn decode_numeric_hint(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = trimmed.parse::<f64>() {
        return Some(parsed);
    }
    match trimmed.to_ascii_uppercase().as_str() {
        "H" => Some(90.0),
        "M" => Some(50.0),
        "L" => Some(10.0),
        _ => None,
    }
}

fn numeric_hint_from_message(message: &NexusMessage, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(raw) = message.kv.get(*key) {
            if let Some(parsed) = decode_numeric_hint(raw) {
                return Some(parsed);
            }
        }
    }
    None
}

fn queue_depth_from_message(message: &NexusMessage) -> Option<f64> {
    numeric_hint_from_message(message, &["Q", "QUEUE_DEPTH", "QD"])
}

fn latency_ms_from_message(message: &NexusMessage) -> Option<f64> {
    numeric_hint_from_message(message, &["LAT", "LATENCY_MS"])
}

fn read_latency_samples(latest: &Value) -> Vec<u64> {
    latest
        .get("latency_samples_ms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_u64())
        .collect::<Vec<_>>()
}

fn p95_latency_ms(samples: &[u64]) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted[idx]
}

fn build_perf_snapshot(
    total_raw_tokens: u64,
    total_nexus_tokens: u64,
    message_count: u64,
    first_message_ts_epoch_ms: u64,
    last_message_ts_epoch_ms: u64,
    queue_depth: Option<f64>,
    latency_samples_ms: &[u64],
) -> Value {
    let token_burn_pct = if total_raw_tokens > 0 {
        round_metric((total_nexus_tokens as f64 / total_raw_tokens as f64) * 100.0)
    } else {
        0.0
    };
    let compression_ratio = if total_nexus_tokens > 0 {
        round_metric(total_raw_tokens as f64 / total_nexus_tokens as f64)
    } else {
        0.0
    };
    let elapsed_ms = last_message_ts_epoch_ms
        .saturating_sub(first_message_ts_epoch_ms)
        .max(1);
    let ops_per_sec = if message_count > 0 {
        round_metric(message_count as f64 / (elapsed_ms as f64 / 1000.0))
    } else {
        0.0
    };
    json!({
        "compression_ratio": compression_ratio,
        "token_burn_pct": token_burn_pct,
        "queue_depth": queue_depth,
        "p95_latency_ms": p95_latency_ms(latency_samples_ms),
        "ops_per_sec": ops_per_sec,
        "message_count": message_count
    })
}

fn perf_snapshot_from_latest(latest: &Value) -> Value {
    let total_raw_tokens = latest
        .get("total_raw_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_nexus_tokens = latest
        .get("total_nexus_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let message_count = latest
        .get("message_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let first_message_ts_epoch_ms = latest
        .get("first_message_ts_epoch_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let last_message_ts_epoch_ms = latest
        .get("last_message_ts_epoch_ms")
        .and_then(Value::as_u64)
        .unwrap_or(first_message_ts_epoch_ms);
    let queue_depth = latest.get("last_queue_depth").and_then(Value::as_f64);
    let latency_samples_ms = read_latency_samples(latest);
    build_perf_snapshot(
        total_raw_tokens,
        total_nexus_tokens,
        message_count,
        first_message_ts_epoch_ms,
        last_message_ts_epoch_ms,
        queue_depth,
        latency_samples_ms.as_slice(),
    )
}

fn summarize_burn(root: &Path) -> Value {
    let latest = read_json(&latest_path(root)).unwrap_or_else(|| json!({}));
    let total_raw_tokens = latest
        .get("total_raw_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_nexus_tokens = latest
        .get("total_nexus_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let fallback_count = latest
        .get("fallback_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let perf = latest
        .get("perf")
        .cloned()
        .unwrap_or_else(|| perf_snapshot_from_latest(&latest));
    let burn_rate = if total_raw_tokens > 0 {
        round_metric((total_nexus_tokens as f64 / total_raw_tokens as f64) * 100.0)
    } else {
        0.0
    };
    json!({
        "total_raw_tokens": total_raw_tokens,
        "total_nexus_tokens": total_nexus_tokens,
        "internal_token_burn_rate_pct": burn_rate,
        "fallback_count": fallback_count,
        "compression_ratio": perf.get("compression_ratio").cloned().unwrap_or(json!(0.0)),
        "queue_depth": perf.get("queue_depth").cloned().unwrap_or(Value::Null),
        "p95_latency_ms": perf.get("p95_latency_ms").cloned().unwrap_or(json!(0)),
        "ops_per_sec": perf.get("ops_per_sec").cloned().unwrap_or(json!(0.0)),
        "perf": perf
    })
}

fn context_flags(argv: &[String]) -> (Option<String>, Option<String>, Option<String>) {
    let task = parse_flag(argv, "task");
    let role = parse_flag(argv, "role");
    let text = parse_flag(argv, "text").or_else(|| parse_flag(argv, "context"));
    (task, role, text)
}

fn persist_message_event(
    root: &Path,
    command: &str,
    message: &NexusMessage,
    decompressed: &Value,
    raw_text: Option<&str>,
    fallback_used: bool,
) -> Result<Value, String> {
    let nexus_line = format_nexus_message(message);
    let now_ms = now_epoch_ms();
    let nexus_tokens = with_arena_bytes(nexus_line.len(), |scratch| {
        scratch.copy_from_slice(nexus_line.as_bytes());
        std::str::from_utf8(scratch)
            .map(estimate_tokens)
            .unwrap_or_else(|_| estimate_tokens(&nexus_line))
    });
    let raw_tokens = raw_text
        .map(|raw| {
            with_slab_buffer(raw.len().saturating_add(8), |buffer| {
                buffer.clear();
                buffer.extend_from_slice(raw.as_bytes());
                std::str::from_utf8(buffer.as_slice())
                    .map(estimate_tokens)
                    .unwrap_or_else(|_| estimate_tokens(raw))
            })
        })
        .unwrap_or(nexus_tokens);
    mark_hot_path_batch(
        nexus_line.len(),
        raw_text.map(|raw| raw.len()).unwrap_or_default(),
    );
    let current = read_json(&latest_path(root)).unwrap_or_else(|| json!({}));
    let before_perf = perf_snapshot_from_latest(&current);
    let current_total_raw = current
        .get("total_raw_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_total_nexus = current
        .get("total_nexus_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_fallback_count = current
        .get("fallback_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_message_count = current
        .get("message_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let first_message_ts_epoch_ms = current
        .get("first_message_ts_epoch_ms")
        .and_then(Value::as_u64)
        .unwrap_or(now_ms);
    let mut latency_samples = read_latency_samples(&current);
    if let Some(latency_ms) = latency_ms_from_message(message) {
        latency_samples.push(latency_ms.round() as u64);
        if latency_samples.len() > 128 {
            let trim = latency_samples.len().saturating_sub(128);
            latency_samples.drain(0..trim);
        }
    }
    let queue_depth = queue_depth_from_message(message)
        .or_else(|| current.get("last_queue_depth").and_then(Value::as_f64));
    let total_raw_tokens = current_total_raw.saturating_add(raw_tokens as u64);
    let total_nexus_tokens = current_total_nexus.saturating_add(nexus_tokens as u64);
    let fallback_count = current_fallback_count.saturating_add(if fallback_used { 1 } else { 0 });
    let message_count = current_message_count.saturating_add(1);
    let after_perf = build_perf_snapshot(
        total_raw_tokens,
        total_nexus_tokens,
        message_count,
        first_message_ts_epoch_ms,
        now_ms,
        queue_depth,
        latency_samples.as_slice(),
    );
    let savings_pct = estimate_savings(raw_tokens, nexus_tokens);
    let row = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_message",
        "ts_epoch_ms": now_ms,
        "command": command,
        "message": nexus_line,
        "decompressed": decompressed,
        "raw_text": raw_text,
        "raw_tokens": raw_tokens,
        "nexus_tokens": nexus_tokens,
        "savings_pct": savings_pct,
        "fallback_used": fallback_used,
        "perf_proof": {
            "before": before_perf,
            "after": after_perf
        }
    }));
    append_jsonl(&messages_path(root), &row)?;
    let updated = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_latest",
        "updated_ts_epoch_ms": now_ms,
        "last_message": row,
        "total_raw_tokens": total_raw_tokens,
        "total_nexus_tokens": total_nexus_tokens,
        "fallback_count": fallback_count,
        "message_count": message_count,
        "first_message_ts_epoch_ms": first_message_ts_epoch_ms,
        "last_message_ts_epoch_ms": now_ms,
        "last_queue_depth": queue_depth,
        "latency_samples_ms": latency_samples,
        "perf": after_perf
    }));
    write_json(&latest_path(root), &updated)?;
    Ok(row)
}

fn validate_command(root: &Path, argv: &[String]) -> (Value, i32) {
    let message_raw = parse_flag(argv, "message").unwrap_or_default();
    if message_raw.trim().is_empty() {
        return (
            error_payload(
                "nexus_internal_comms_error",
                "validate",
                "missing_message_flag",
            ),
            2,
        );
    }
    let message = match parse_nexus_message(&message_raw) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "validate", &e),
                2,
            )
        }
    };
    let (task, role, text) = context_flags(argv);
    let seeded_modules = message.module.clone().into_iter().collect::<Vec<String>>();
    let modules = match resolve_modules_for_context(
        argv,
        &seeded_modules,
        task.as_deref(),
        role.as_deref(),
        text.as_deref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "validate", &e),
                2,
            )
        }
    };
    let lexicon = match active_lexicon(&modules) {
        Ok(v) => v,
        Err(e) => {
            return (
                error_payload("nexus_internal_comms_error", "validate", &e),
                3,
            )
        }
    };
    if let Err(e) = validate_module_rules(&message, &modules) {
        return (
            error_payload("nexus_internal_comms_error", "validate", &e),
            3,
        );
    }
    let decompressed = decompress_message(&message, &lexicon);
    let mut out = with_hash(json!({
        "ok": true,
        "type": "nexus_internal_comms_validate",
        "format": "[FROM>TO|MOD] CMD k=v k=v ...",
        "message": format_nexus_message(&message),
        "decompressed": decompressed,
        "modules_loaded": modules,
        "claim_evidence": [
            {
                "id": "V6-INTERNAL-COMMS-001.1",
                "claim": "nexus_messages_use_strict_one_line_format_and_deterministic_parser",
                "evidence": {
                    "validated": true
                }
            }
        ]
    }));
    if let Ok(row) = persist_message_event(root, "validate", &message, &decompressed, None, false) {
        out["perf_proof"] = row.get("perf_proof").cloned().unwrap_or(Value::Null);
    }
    out["burn"] = summarize_burn(root);
    (out, 0)
}
