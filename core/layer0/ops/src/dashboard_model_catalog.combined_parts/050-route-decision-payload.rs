
pub fn route_decision_payload(root: &Path, snapshot: &Value, request: &Value) -> Value {
    let tuning = load_session_analytics_tuning(root);
    let catalog = catalog_payload(root, snapshot);
    let mut rows = catalog
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let offline_required = parse_bool(request.get("offline_required"), false);
    let prefer_local = parse_bool(request.get("prefer_local"), false) || offline_required;
    let complexity = clean_text(
        request
            .get("complexity")
            .and_then(Value::as_str)
            .unwrap_or("general"),
        40,
    )
    .to_ascii_lowercase();
    let task_type = clean_text(
        request
            .get("task_type")
            .or_else(|| request.get("role"))
            .and_then(Value::as_str)
            .unwrap_or("general"),
        80,
    )
    .to_ascii_lowercase();
    let mut budget_mode = clean_text(
        request
            .get("budget_mode")
            .and_then(Value::as_str)
            .unwrap_or("balanced"),
        40,
    )
    .to_ascii_lowercase();
    let tuned_budget_mode = clean_text(
        tuning
            .pointer("/routing/default_budget_mode")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    let default_budget_override_applied =
        if (budget_mode.is_empty() || budget_mode == "balanced") && !tuned_budget_mode.is_empty() {
            budget_mode = tuned_budget_mode.clone();
            true
        } else {
            false
        };
    let model_biases = tuning
        .pointer("/routing/model_bias")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !model_biases.is_empty() {
        for row in &mut rows {
            let provider = clean_text(
                row.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            );
            let model = clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240);
            let key = format!("{provider}/{model}");
            let bias = parse_f64_value(
                model_biases
                    .get(row.get("id").and_then(Value::as_str).unwrap_or(""))
                    .or_else(|| model_biases.get(&key))
                    .or_else(|| model_biases.get(&model)),
            );
            if bias.abs() > f64::EPSILON {
                row["route_bias"] = json!(bias);
            }
        }
    }
    if offline_required {
        rows.retain(|row| parse_bool(row.get("is_local"), false));
        let has_ollama = rows.iter().any(|row| {
            clean_text(
                row.get("provider").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .eq_ignore_ascii_case("ollama")
                && parse_bool(row.get("available"), false)
        });
        if has_ollama {
            rows.retain(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case("ollama")
            });
        }
    }

    rows.sort_by(|a, b| {
        let score_a = route_score(a, prefer_local, &complexity, &task_type, &budget_mode);
        let score_b = route_score(b, prefer_local, &complexity, &task_type, &budget_mode);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 200).cmp(&clean_text(
                    b.get("id").and_then(Value::as_str).unwrap_or(""),
                    200,
                ))
            })
    });

    let routing_policy = crate::dashboard_provider_runtime::routing_policy(root);
    let strategy = clean_text(
        routing_policy
            .pointer("/load_balancing/strategy")
            .and_then(Value::as_str)
            .unwrap_or("score_weighted"),
        40,
    )
    .to_ascii_lowercase();
    let strategy_is_round_robin = strategy == "round_robin";
    let pool_limit = if strategy_is_round_robin {
        rows.len().min(3)
    } else {
        1
    }
    .max(1);
    let selected_index = if strategy_is_round_robin {
        let selector_seed = crate::deterministic_receipt_hash(&json!({
            "agent_id": request.get("agent_id").cloned().unwrap_or(Value::Null),
            "task_type": task_type,
            "complexity": complexity,
            "budget_mode": budget_mode,
            "token_count": request.get("token_count").cloned().unwrap_or(Value::Null),
            "seed": routing_policy.pointer("/load_balancing/seed").cloned().unwrap_or_else(|| json!("stable"))
        }));
        let hex = selector_seed.chars().take(8).collect::<String>();
        let seed = u64::from_str_radix(&hex, 16).unwrap_or(0);
        (seed as usize) % pool_limit
    } else {
        0
    };
    let selected = rows
        .get(selected_index)
        .cloned()
        .or_else(|| rows.first().cloned())
        .unwrap_or_else(|| json!({}));
    let top = rows.into_iter().take(5).collect::<Vec<_>>();
    let selected_provider = clean_text(
        selected
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let selected_model = clean_text(
        selected.get("model").and_then(Value::as_str).unwrap_or(""),
        240,
    );
    let fallback_chain = crate::dashboard_provider_runtime::routing_fallback_chain(
        root,
        &selected_provider,
        &selected_model,
    );
    let retry_policy = routing_policy
        .get("retry")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let route = json!({
        "provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "selected_provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "selected_model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "selected_model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "context_window": selected
            .get("context_window")
            .cloned()
            .unwrap_or_else(|| json!(0)),
        "context_window_tokens": selected
            .get("context_window_tokens")
            .cloned()
            .unwrap_or_else(|| json!(0)),
        "fallback_chain": fallback_chain,
        "retry_policy": retry_policy,
        "load_balancing": routing_policy
            .get("load_balancing")
            .cloned()
            .unwrap_or_else(|| json!({})),
        "selection_strategy": strategy,
        "selection_index": selected_index
    });
    json!({
        "ok": true,
        "type": "dashboard_model_route_decision",
        "selected": selected,
        "route": route,
        "selected_provider": selected.get("provider").cloned().unwrap_or_else(|| json!("")),
        "selected_model": selected.get("model").cloned().unwrap_or_else(|| json!("")),
        "selected_model_id": selected.get("id").cloned().unwrap_or_else(|| json!("")),
        "candidates": top,
        "routing_policy": routing_policy,
        "analytics_tuning": {
            "enabled": bool_env("INFRING_SESSION_ANALYTICS_ROUTING_ENABLED", true),
            "default_budget_override_applied": default_budget_override_applied,
            "default_budget_mode": tuned_budget_mode,
            "model_bias_entries": model_biases.len()
        },
        "input": {
            "prefer_local": prefer_local,
            "offline_required": offline_required,
            "complexity": complexity,
            "task_type": task_type,
            "budget_mode": budget_mode
        }
    })
}
