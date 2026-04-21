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
