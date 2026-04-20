
fn dashboard_auto_route_from_payload(payload: &Value) -> Value {
    let input_text = payload_string(payload, "input_text", "");
    let lowered = input_text.to_ascii_lowercase();
    let token_count = payload_u64(payload, "token_count", (input_text.len() as u64 / 4).max(1))
        .clamp(1, 8_000_000);
    let has_vision = payload_bool(payload, "has_vision", false);
    let asks_speed = payload_bool(
        payload,
        "asks_speed",
        lowered.contains("fast") || lowered.contains("speed"),
    );
    let asks_cost = payload_bool(
        payload,
        "asks_cost",
        lowered.contains("cheap") || lowered.contains("cost"),
    );
    let asks_quality = payload_bool(
        payload,
        "asks_quality",
        lowered.contains("quality") || lowered.contains("best"),
    );
    let asks_long_context = payload_bool(
        payload,
        "asks_long_context",
        token_count >= 100_000 || lowered.contains("long context"),
    );

    let preferred_provider = payload_string(payload, "preferred_provider", "ollama");
    let preferred_model = payload_string(payload, "preferred_model", "llama3.2:3b");
    let fallback_provider = payload_string(payload, "fallback_provider", "cloud");
    let fallback_model = payload_string(payload, "fallback_model", "kimi2.5:cloud");

    let mut raw_candidates = payload_array(payload, "candidates");
    if raw_candidates.is_empty() {
        raw_candidates.push(json!({
            "runtime_provider": preferred_provider,
            "runtime_model": preferred_model
        }));
        raw_candidates.push(json!({
            "runtime_provider": fallback_provider,
            "runtime_model": fallback_model
        }));
    }

    let runtime_success = payload_f64(payload, "spine_success_rate", 0.90).clamp(0.0, 1.0);
    let mut scored = Vec::<Value>::new();
    for candidate in raw_candidates {
        let provider = payload_string(
            &candidate,
            "runtime_provider",
            payload_string(&candidate, "provider", "ollama").as_str(),
        );
        let model = payload_string(
            &candidate,
            "runtime_model",
            payload_string(&candidate, "model", "llama3.2:3b").as_str(),
        );
        let model_lower = model.to_ascii_lowercase();
        let (prior_latency, prior_cost, prior_success): (f64, f64, f64) =
            match provider.to_ascii_lowercase().as_str() {
                "ollama" => (120.0_f64, 0.0_f64, 0.92_f64),
                "groq" => (65.0_f64, 0.2_f64, 0.90_f64),
                "openai" => (90.0_f64, 0.55_f64, 0.95_f64),
                "frontier_provider" => (105.0_f64, 0.7_f64, 0.95_f64),
                "google" => (95.0_f64, 0.6_f64, 0.94_f64),
                "cloud" => (80.0_f64, 0.3_f64, 0.93_f64),
                _ => (110.0_f64, 0.45_f64, 0.90_f64),
            };
        let model_is_small = model_lower.contains("3b")
            || model_lower.contains("mini")
            || model_lower.contains("small");
        let latency_ms =
            (prior_latency * if model_is_small { 0.85_f64 } else { 1.0_f64 }).max(1.0_f64);
        let cost_per_1k =
            (prior_cost * if model_is_small { 0.7_f64 } else { 1.0_f64 }).max(0.0_f64);
        let context_window = candidate
            .get("context_window")
            .and_then(Value::as_u64)
            .unwrap_or(8192)
            .clamp(1024, 8_000_000);
        let context_score = if token_count <= context_window {
            1.0
        } else {
            (context_window as f64 / token_count as f64).clamp(0.1, 1.0)
        };
        let latency_score = 1.0 / (1.0 + (latency_ms / 120.0));
        let cost_score = 1.0 / (1.0 + cost_per_1k);
        let success_rate = ((prior_success * 0.65) + (runtime_success * 0.35)).clamp(0.2, 0.99);
        let supports_vision = candidate
            .get("supports_vision")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let vision_penalty = if has_vision && !supports_vision {
            0.55
        } else {
            0.0
        };
        let speed_weight = if asks_speed { 1.55 } else { 1.05 };
        let cost_weight = if asks_cost { 1.35 } else { 0.75 };
        let quality_weight = if asks_quality { 1.8 } else { 1.3 };
        let context_weight = if asks_long_context { 1.45 } else { 1.1 };
        let score = (latency_score * speed_weight)
            + (cost_score * cost_weight)
            + (success_rate * quality_weight)
            + (context_score * context_weight)
            - vision_penalty;
        scored.push(json!({
            "provider": provider,
            "model": model,
            "score": (score * 1_000_000.0).round() / 1_000_000.0,
            "latency_ms": latency_ms.round() as u64,
            "cost_per_1k": ((cost_per_1k * 10_000.0).round()) / 10_000.0,
            "success_rate": (success_rate * 10_000.0).round() / 10_000.0,
            "context_window": context_window,
            "supports_vision": supports_vision
        }));
    }

    scored.sort_by(|a, b| {
        let lhs = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let rhs = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        lhs.partial_cmp(&rhs).unwrap_or(std::cmp::Ordering::Equal)
    });

    let selected = scored.first().cloned().unwrap_or_else(|| {
        json!({
            "provider": preferred_provider,
            "model": preferred_model,
            "score": 0.0,
            "latency_ms": 120,
            "cost_per_1k": 0.0,
            "success_rate": 0.9,
            "context_window": 8192,
            "supports_vision": false
        })
    });
    let selected_provider = payload_string(&selected, "provider", "ollama");
    let selected_model = payload_string(&selected, "model", "llama3.2:3b");
    let selected_context_window = selected
        .get("context_window")
        .and_then(Value::as_u64)
        .unwrap_or(8192);
    let reason = format!(
        "rust auto-route selected {} / {} by weighted latency-cost-success-context scoring",
        selected_provider, selected_model
    );
    let fallback_chain = scored
        .iter()
        .skip(1)
        .take(3)
        .map(|row| {
            json!({
                "provider": payload_string(row, "provider", ""),
                "model": payload_string(row, "model", ""),
                "score": row.get("score").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .collect::<Vec<_>>();

    let mut decision = json!({
        "authority": "rust_runtime_systems",
        "policy": "V6-DASHBOARD-008.1",
        "route_lane": "runtime-systems.run",
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "selected_model_id": format!(
            "{}/{}",
            payload_string(&selected, "provider", "ollama"),
            payload_string(&selected, "model", "llama3.2:3b")
        ),
        "selected_context_window": selected_context_window,
        "reason": reason,
        "context": {
            "token_count": token_count,
            "has_vision": has_vision,
            "asks_speed": asks_speed,
            "asks_cost": asks_cost,
            "asks_quality": asks_quality,
            "asks_long_context": asks_long_context
        },
        "fallback_chain": fallback_chain,
        "candidates": scored,
        "runtime_sync": {
            "spine_success_rate": runtime_success,
            "receipt_latency_p99_ms": payload_f64(payload, "receipt_latency_p99_ms", 0.0).max(0.0)
        }
    });
    let hash = receipt_hash(&decision);
    decision["receipt_hash"] = Value::String(hash);
    decision
}

fn payload_string_list(payload: &Value, key: &str) -> Vec<String> {
    payload_array(payload, key)
        .into_iter()
        .map(|row| match row {
            Value::String(text) => text,
            other => other
                .as_str()
                .map(|text| text.to_string())
                .unwrap_or_default(),
        })
        .filter(|row| !row.trim().is_empty())
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>()
}

fn push_if_disabled(violations: &mut Vec<String>, enabled: bool, code: &str) {
    if !enabled {
        violations.push(code.to_string());
    }
}
