fn parse_u64_like(value: Option<&Value>, fallback: u64, min: u64, max: u64) -> u64 {
    let parsed = match value {
        Some(Value::Number(n)) => n.as_u64().or_else(|| n.as_i64().map(|v| v.max(0) as u64)),
        Some(Value::String(raw)) => raw.trim().parse::<u64>().ok(),
        Some(Value::Bool(true)) => Some(1),
        Some(Value::Bool(false)) => Some(0),
        _ => None,
    }
    .unwrap_or(fallback);
    parsed.clamp(min, max)
}

fn parse_f64_like(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    let parsed = match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(raw)) => raw.trim().parse::<f64>().ok(),
        Some(Value::Bool(true)) => Some(1.0),
        Some(Value::Bool(false)) => Some(0.0),
        _ => None,
    }
    .filter(|v| v.is_finite())
    .unwrap_or(fallback);
    parsed.clamp(min, max)
}

fn parse_provider_model_ref(raw: &str) -> Option<(String, String)> {
    let cleaned = clean_text(raw, 320);
    let (provider_raw, model_raw) = cleaned.split_once('/')?;
    let provider = normalize_provider_id(provider_raw);
    let model = clean_text(model_raw, 240);
    if provider.is_empty() || model.is_empty() {
        None
    } else {
        Some((provider, model))
    }
}

fn canonical_fallback_rows(value: Option<&Value>) -> Vec<Value> {
    let mut rows = Vec::<Value>::new();
    if let Some(entries) = value.and_then(Value::as_array) {
        for entry in entries {
            let (provider, model) = if let Some(text) = entry.as_str() {
                parse_provider_model_ref(text).unwrap_or_default()
            } else {
                let provider = normalize_provider_id(
                    entry.get("provider").and_then(Value::as_str).unwrap_or(""),
                );
                let model = clean_text(
                    entry.get("model").and_then(Value::as_str).unwrap_or(""),
                    240,
                );
                (provider, model)
            };
            if provider.is_empty() || model.is_empty() {
                continue;
            }
            if rows.iter().any(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    120,
                ) == provider
                    && clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240)
                        == model
            }) {
                continue;
            }
            rows.push(json!({"provider": provider, "model": model}));
        }
    }
    rows
}

fn default_routing_policy() -> Value {
    let mut policy = json!({
        "type": "infring_provider_routing_policy",
        "version": 1,
        "mode": "production",
        "retry": {
            "max_attempts_per_route": 2,
            "max_total_attempts": 5,
            "base_backoff_ms": 220,
            "max_backoff_ms": 1800,
            "factor": 2.0
        },
        "load_balancing": {
            "strategy": "score_weighted",
            "seed": "stable"
        },
        "fallback_chain": [
            {"provider": "moonshot", "model": "kimi-k2.5"},
            {"provider": "openrouter", "model": "deepseek/deepseek-chat-v3-0324:free"},
            {"provider": "ollama", "model": "llama3.2:latest"}
        ],
        "signature": "builtin:infring-routing-v1",
        "updated_at": crate::now_iso()
    });
    let hash = crate::deterministic_receipt_hash(&policy);
    policy["policy_hash"] = json!(hash);
    policy
}

fn sanitize_routing_policy(value: &Value) -> Value {
    let defaults = default_routing_policy();
    let mode = clean_text(
        value
            .get("mode")
            .or_else(|| defaults.get("mode"))
            .and_then(Value::as_str)
            .unwrap_or("production"),
        40,
    )
    .to_ascii_lowercase();
    let normalized_mode = if mode == "simulation" {
        "simulation"
    } else {
        "production"
    };
    let retry_source = value
        .get("retry")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let retry_defaults = defaults
        .get("retry")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let max_attempts_per_route = parse_u64_like(
        retry_source
            .get("max_attempts_per_route")
            .or_else(|| retry_defaults.get("max_attempts_per_route")),
        2,
        1,
        6,
    );
    let max_total_attempts = parse_u64_like(
        retry_source
            .get("max_total_attempts")
            .or_else(|| retry_defaults.get("max_total_attempts")),
        5,
        1,
        12,
    );
    let base_backoff_ms = parse_u64_like(
        retry_source
            .get("base_backoff_ms")
            .or_else(|| retry_defaults.get("base_backoff_ms")),
        220,
        20,
        10_000,
    );
    let max_backoff_ms = parse_u64_like(
        retry_source
            .get("max_backoff_ms")
            .or_else(|| retry_defaults.get("max_backoff_ms")),
        1800,
        50,
        30_000,
    );
    let factor = parse_f64_like(
        retry_source
            .get("factor")
            .or_else(|| retry_defaults.get("factor")),
        2.0,
        1.0,
        4.0,
    );
    let load_balancing_source = value
        .get("load_balancing")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let strategy = clean_text(
        load_balancing_source
            .get("strategy")
            .or_else(|| defaults.pointer("/load_balancing/strategy"))
            .and_then(Value::as_str)
            .unwrap_or("score_weighted"),
        40,
    )
    .to_ascii_lowercase();
    let strategy = if strategy == "round_robin" {
        "round_robin"
    } else {
        "score_weighted"
    };
    let seed = clean_text(
        load_balancing_source
            .get("seed")
            .or_else(|| defaults.pointer("/load_balancing/seed"))
            .and_then(Value::as_str)
            .unwrap_or("stable"),
        120,
    );
    let mut fallback_chain = canonical_fallback_rows(value.get("fallback_chain"));
    if fallback_chain.is_empty() {
        fallback_chain = canonical_fallback_rows(defaults.get("fallback_chain"));
    }
    let signature = clean_text(
        value
            .get("signature")
            .or_else(|| defaults.get("signature"))
            .and_then(Value::as_str)
            .unwrap_or("builtin:infring-routing-v1"),
        260,
    );
    let version = parse_u64_like(
        value.get("version").or_else(|| defaults.get("version")),
        1,
        1,
        1_000_000,
    );
    let mut out = json!({
        "type": "infring_provider_routing_policy",
        "version": version,
        "mode": normalized_mode,
        "retry": {
            "max_attempts_per_route": max_attempts_per_route,
            "max_total_attempts": max_total_attempts,
            "base_backoff_ms": base_backoff_ms,
            "max_backoff_ms": max_backoff_ms,
            "factor": factor
        },
        "load_balancing": {
            "strategy": strategy,
            "seed": seed
        },
        "fallback_chain": fallback_chain,
        "signature": signature,
        "updated_at": clean_text(
            value
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or(&crate::now_iso()),
            80
        )
    });
    let hash = crate::deterministic_receipt_hash(&out);
    out["policy_hash"] = json!(hash);
    out
}

pub fn routing_policy(root: &Path) -> Value {
    let path = routing_policy_path(root);
    let raw = read_json(&path).unwrap_or_else(default_routing_policy);
    let sanitized = sanitize_routing_policy(&raw);
    if sanitized != raw {
        write_json_pretty(&path, &sanitized);
    }
    sanitized
}

fn recent_routing_events(root: &Path, limit: usize) -> Vec<Value> {
    let path = routing_events_path(root);
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .take(limit.max(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

pub fn routing_policy_payload(root: &Path) -> Value {
    let policy = routing_policy(root);
    let recent_events = recent_routing_events(root, 25);
    json!({
        "ok": true,
        "policy": policy,
        "recent_events": recent_events,
        "recent_event_count": recent_events.len()
    })
}

pub fn update_routing_policy(root: &Path, patch: &Value) -> Value {
    if !patch.is_object() {
        return json!({"ok": false, "error": "routing_policy_patch_invalid"});
    }
    let signature = clean_text(
        patch.get("signature").and_then(Value::as_str).unwrap_or(""),
        260,
    );
    if signature.is_empty() || !(signature.starts_with("sig:") || signature.starts_with("builtin:"))
    {
        return json!({"ok": false, "error": "routing_policy_signature_required"});
    }
    let current = routing_policy(root);
    let mut merged = current.clone();
    if let Some(mode) = patch.get("mode").and_then(Value::as_str) {
        merged["mode"] = json!(clean_text(mode, 40));
    }
    if let Some(retry) = patch.get("retry") {
        merged["retry"] = retry.clone();
    }
    if let Some(load_balancing) = patch.get("load_balancing") {
        merged["load_balancing"] = load_balancing.clone();
    }
    if let Some(fallback_chain) = patch.get("fallback_chain") {
        merged["fallback_chain"] = fallback_chain.clone();
    }
    merged["signature"] = json!(signature);
    merged["version"] =
        json!(parse_u64_like(current.get("version"), 1, 1, 1_000_000).saturating_add(1));
    merged["updated_at"] = json!(crate::now_iso());
    let sanitized = sanitize_routing_policy(&merged);
    write_json_pretty(&routing_policy_path(root), &sanitized);
    json!({"ok": true, "policy": sanitized})
}

fn retry_policy_limits(root: &Path) -> (usize, usize, u64, u64, f64) {
    let policy = routing_policy(root);
    let retry = policy.get("retry").unwrap_or(&Value::Null);
    let per_route = parse_u64_like(retry.get("max_attempts_per_route"), 2, 1, 6) as usize;
    let total = parse_u64_like(retry.get("max_total_attempts"), 5, 1, 12) as usize;
    let base_ms = parse_u64_like(retry.get("base_backoff_ms"), 220, 20, 10_000);
    let max_ms = parse_u64_like(retry.get("max_backoff_ms"), 1800, 50, 30_000);
    let factor = parse_f64_like(retry.get("factor"), 2.0, 1.0, 4.0);
    (per_route, total, base_ms, max_ms, factor)
}

fn fallback_routes(
    root: &Path,
    primary_provider: &str,
    primary_model: &str,
) -> Vec<(String, String)> {
    let mut routes = vec![(
        normalize_provider_id(primary_provider),
        clean_text(primary_model, 240),
    )];
    let policy = routing_policy(root);
    let fallback_rows = canonical_fallback_rows(policy.get("fallback_chain"));
    for row in fallback_rows {
        let provider =
            normalize_provider_id(row.get("provider").and_then(Value::as_str).unwrap_or(""));
        let model = clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240);
        if provider.is_empty() || model.is_empty() {
            continue;
        }
        // Fail closed on stale local fallback entries: do not route to local models
        // that are not present in the authoritative model profile registry.
        if provider_is_local(&provider) && model_profile_for(root, &provider, &model).is_none() {
            continue;
        }
        if routes.iter().any(|(existing_provider, existing_model)| {
            existing_provider == &provider && existing_model == &model
        }) {
            continue;
        }
        routes.push((provider, model));
    }
    routes
}

pub fn routing_fallback_chain(
    root: &Path,
    primary_provider: &str,
    primary_model: &str,
) -> Vec<Value> {
    fallback_routes(root, primary_provider, primary_model)
        .into_iter()
        .map(|(provider, model)| json!({"provider": provider, "model": model}))
        .collect()
}

fn append_routing_event(root: &Path, event: &Value) {
    let path = routing_events_path(root);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(line) = serde_json::to_string(event) else {
        return;
    };
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
}

fn is_retryable_model_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    if lower.contains("provider key missing") || lower.contains("message_required") {
        return false;
    }
    // Missing model variants are deterministic configuration issues, not transient.
    if lower.contains("not found") || lower.contains("no such model") {
        return false;
    }
    lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("temporarily")
        || lower.contains("unavailable")
        || lower.contains("connection reset")
}

fn backoff_for_attempt(base_ms: u64, max_ms: u64, factor: f64, attempt_index: usize) -> u64 {
    let exponent = attempt_index.saturating_sub(1) as f64;
    let expanded = (base_ms as f64) * factor.powf(exponent);
    expanded.round().clamp(base_ms as f64, max_ms as f64) as u64
}

fn model_profile_for(root: &Path, provider_id: &str, model_name: &str) -> Option<Value> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    provider_row(root, &provider)
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&model))
        .cloned()
}

fn model_cost_rates_usd_per_1k(
    root: &Path,
    provider_id: &str,
    model_name: &str,
) -> (f64, f64, bool) {
    let provider = normalize_provider_id(provider_id);
    if provider_is_local(&provider) {
        return (0.0, 0.0, true);
    }
    let profile = model_profile_for(root, &provider, model_name);
    let input_rate = parse_f64_like(
        profile.as_ref().and_then(|row| row.get("usd_per_1k_input")),
        -1.0,
        -1.0,
        1000.0,
    );
    let output_rate = parse_f64_like(
        profile
            .as_ref()
            .and_then(|row| row.get("usd_per_1k_output")),
        -1.0,
        -1.0,
        1000.0,
    );
    if input_rate >= 0.0 && output_rate >= 0.0 {
        return (input_rate, output_rate, false);
    }
    let cost_scale = parse_u64_like(
        profile.as_ref().and_then(|row| row.get("cost_rating")),
        3,
        1,
        5,
    );
    match cost_scale {
        1 => (0.00015, 0.00030, false),
        2 => (0.00050, 0.00100, false),
        3 => (0.00150, 0.00300, false),
        4 => (0.00400, 0.00800, false),
        _ => (0.01000, 0.02000, false),
    }
}

fn estimated_chat_cost_usd(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    input_tokens: i64,
    output_tokens: i64,
) -> f64 {
    let (input_rate, output_rate, local) =
        model_cost_rates_usd_per_1k(root, provider_id, model_name);
    if local {
        return 0.0;
    }
    let input = (input_tokens.max(0) as f64 / 1000.0) * input_rate.max(0.0);
    let output = (output_tokens.max(0) as f64 / 1000.0) * output_rate.max(0.0);
    (input + output).max(0.0)
}

fn round_usd(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
