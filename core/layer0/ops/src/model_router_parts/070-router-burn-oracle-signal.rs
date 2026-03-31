fn router_budget_policy_value(policy: &RouterBudgetPolicy) -> Value {
    let mut out = Map::<String, Value>::new();
    out.insert("enabled".to_string(), Value::Bool(policy.enabled));
    out.insert(
        "state_dir".to_string(),
        Value::String(policy.state_dir.clone()),
    );
    out.insert(
        "allow_strategy_override".to_string(),
        Value::Bool(policy.allow_strategy_override),
    );
    out.insert("soft_ratio".to_string(), number_value(policy.soft_ratio));
    out.insert("hard_ratio".to_string(), number_value(policy.hard_ratio));
    out.insert(
        "enforce_hard_cap".to_string(),
        Value::Bool(policy.enforce_hard_cap),
    );
    out.insert(
        "escalate_on_no_local_fallback".to_string(),
        Value::Bool(policy.escalate_on_no_local_fallback),
    );
    out.insert(
        "cloud_penalty_soft".to_string(),
        number_value(policy.cloud_penalty_soft),
    );
    out.insert(
        "cloud_penalty_hard".to_string(),
        number_value(policy.cloud_penalty_hard),
    );
    out.insert(
        "cheap_local_bonus_soft".to_string(),
        number_value(policy.cheap_local_bonus_soft),
    );
    out.insert(
        "cheap_local_bonus_hard".to_string(),
        number_value(policy.cheap_local_bonus_hard),
    );
    out.insert(
        "model_token_multipliers".to_string(),
        Value::Object(policy.model_token_multipliers.clone()),
    );
    out.insert(
        "class_token_multipliers".to_string(),
        Value::Object(policy.class_token_multipliers.clone()),
    );
    Value::Object(out)
}

pub fn router_burn_oracle_signal(raw_signal: Option<&Value>, default_source_path: &str) -> Value {
    let src = raw_signal.and_then(Value::as_object);
    let pressure_input = string_like(src.and_then(|v| v.get("pressure")));
    let reason_codes = src
        .and_then(|v| v.get("reason_codes"))
        .and_then(Value::as_array)
        .map(|rows| rows.iter().take(10).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let source_path = normalized_optional_string(src.and_then(|v| v.get("latest_path_rel")))
        .unwrap_or_else(|| default_source_path.to_string());

    json!({
        "available": matches!(src.and_then(|v| v.get("available")), Some(Value::Bool(true))),
        "pressure": normalize_router_pressure(&pressure_input),
        "pressure_rank": pressure_order(&pressure_input),
        "projected_runway_days": finite_number(src.and_then(|v| v.get("projected_runway_days"))),
        "projected_days_to_reset": finite_number(src.and_then(|v| v.get("projected_days_to_reset"))),
        "reason_codes": reason_codes,
        "source_path": source_path
    })
}

pub fn router_budget_state(input: RouterBudgetStateInput<'_>) -> Value {
    let policy = router_budget_policy(input.cfg, input.repo_root, input.default_state_dir);
    let policy_value = router_budget_policy_value(&policy);
    let date = budget_date_str(input.today_override, input.now_iso);
    let oracle = router_burn_oracle_signal(input.oracle_signal, input.default_oracle_source_path);

    let mut out = Map::<String, Value>::new();
    out.insert("enabled".to_string(), Value::Bool(policy.enabled));
    out.insert("available".to_string(), Value::Bool(false));
    out.insert("pressure".to_string(), Value::String("none".to_string()));
    out.insert("ratio".to_string(), Value::Null);
    out.insert("token_cap".to_string(), Value::Null);
    out.insert("used_est".to_string(), Value::Null);
    out.insert("path".to_string(), Value::Null);
    out.insert("oracle".to_string(), oracle.clone());
    out.insert("policy".to_string(), policy_value);

    if !policy.enabled {
        return Value::Object(out);
    }

    let fallback_path = Path::new(&policy.state_dir)
        .join(format!("{date}.json"))
        .to_string_lossy()
        .to_string();
    let budget_obj = input.budget_state.and_then(Value::as_object);
    let path =
        normalized_optional_string(budget_obj.and_then(|v| v.get("path"))).unwrap_or(fallback_path);
    out.insert("path".to_string(), Value::String(path));

    if !matches!(
        budget_obj
            .and_then(|v| v.get("available"))
            .and_then(Value::as_bool),
        Some(true)
    ) {
        return Value::Object(out);
    }

    let cap = finite_number(budget_obj.and_then(|v| v.get("token_cap"))).unwrap_or(0.0);
    let used = finite_number(budget_obj.and_then(|v| v.get("used_est"))).unwrap_or(0.0);
    if !(cap.is_finite() && cap > 0.0 && used.is_finite()) {
        return Value::Object(out);
    }

    let ratio = (used / cap).max(0.0);
    let mut pressure = if ratio >= policy.hard_ratio {
        "hard".to_string()
    } else if ratio >= policy.soft_ratio {
        "soft".to_string()
    } else {
        "none".to_string()
    };

    let oracle_available = matches!(
        oracle
            .as_object()
            .and_then(|v| v.get("available"))
            .and_then(Value::as_bool),
        Some(true)
    );
    let oracle_pressure = oracle
        .as_object()
        .and_then(|v| v.get("pressure"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    if oracle_available && pressure_order(oracle_pressure) > pressure_order(&pressure) {
        pressure = oracle_pressure.to_string();
    }

    out.insert("available".to_string(), Value::Bool(true));
    out.insert("pressure".to_string(), Value::String(pressure));
    out.insert("ratio".to_string(), number_value(rounded_4(ratio)));
    out.insert("token_cap".to_string(), number_value(cap));
    out.insert("used_est".to_string(), number_value(used));
    out.insert(
        "strategy_id".to_string(),
        budget_obj
            .and_then(|v| v.get("strategy_id"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(out)
}

fn autopause_state_from_value(value: Option<&Value>) -> RouterBudgetAutopauseState {
    let src = value.and_then(Value::as_object);
    RouterBudgetAutopauseState {
        active: matches!(src.and_then(|v| v.get("active")), Some(Value::Bool(true))),
        source: normalized_optional_string(src.and_then(|v| v.get("source"))),
        reason: normalized_optional_string(src.and_then(|v| v.get("reason"))),
        until: normalized_optional_string(src.and_then(|v| v.get("until"))),
    }
}

fn guard_pressure_key(guard: Option<&Value>) -> String {
    let src = guard.and_then(Value::as_object);
    let raw = src
        .and_then(|v| v.get("projected_pressure"))
        .filter(|value| js_truthy(Some(*value)))
        .map(|value| string_like(Some(value)))
        .or_else(|| {
            src.and_then(|v| v.get("pressure"))
                .filter(|value| js_truthy(Some(*value)))
                .map(|value| string_like(Some(value)))
        })
        .unwrap_or_else(|| "none".to_string());
    normalize_key(&raw)
}

pub fn evaluate_router_global_budget_gate(
    input: RouterGlobalBudgetGateInput<'_>,
) -> RouterGlobalBudgetGateResult {
    let dry_run_mode = bool_or_one_like(input.dry_run);
    let execution_mode = bool_or_one_like(input.execution_intent);
    let request_tokens = input
        .request_tokens_est
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let mut autopause = autopause_state_from_value(input.autopause);

    let non_execute_bypass = input.enforce_execution_only
        && !execution_mode
        && request_tokens <= input.nonexec_max_tokens as f64;
    if non_execute_bypass {
        return RouterGlobalBudgetGateResult {
            enabled: true,
            blocked: false,
            deferred: false,
            bypassed: true,
            reason: Some("budget_guard_nonexecute_bypass".to_string()),
            autopause_active: autopause.active,
            autopause,
            guard: None,
            oracle: None,
        };
    }

    let oracle =
        router_burn_oracle_signal(input.oracle, ROUTER_BURN_ORACLE_LATEST_PATH_REL_DEFAULT);
    let oracle_available = matches!(
        oracle
            .as_object()
            .and_then(|v| v.get("available"))
            .and_then(Value::as_bool),
        Some(true)
    );
    let oracle_pressure = oracle
        .as_object()
        .and_then(|v| v.get("pressure"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    if oracle_available && oracle_pressure == "hard" && execution_mode {
        return RouterGlobalBudgetGateResult {
            enabled: true,
            blocked: true,
            deferred: false,
            bypassed: false,
            reason: Some("budget_oracle_runway_critical".to_string()),
            autopause_active: autopause.active,
            autopause,
            guard: None,
            oracle: Some(oracle),
        };
    }

    let guard = input.guard.cloned();
    let autopause_source_model_router = autopause
        .source
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        == "model_router";

    if autopause.active && autopause_source_model_router {
        let hard_stop = matches!(
            guard
                .as_ref()
                .and_then(Value::as_object)
                .and_then(|v| v.get("hard_stop"))
                .and_then(Value::as_bool),
            Some(true)
        );
        let pressure = guard_pressure_key(guard.as_ref());
        if !hard_stop && pressure != "hard" {
            autopause.active = false;
            autopause.until = None;
            autopause.source = Some("model_router".to_string());
        }
    }

    if autopause.active {
        if dry_run_mode {
            return RouterGlobalBudgetGateResult {
                enabled: true,
                blocked: false,
                deferred: true,
                bypassed: false,
                reason: Some("budget_autopause_active_dry_run".to_string()),
                autopause_active: true,
                autopause,
                guard,
                oracle: None,
            };
        }
        return RouterGlobalBudgetGateResult {
            enabled: true,
            blocked: true,
            deferred: false,
            bypassed: false,
            reason: Some("budget_autopause_active".to_string()),
            autopause_active: true,
            autopause,
            guard,
            oracle: None,
        };
    }

    let hard_stop = matches!(
        guard
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|v| v.get("hard_stop"))
            .and_then(Value::as_bool),
        Some(true)
    );
    if hard_stop {
        let hard_reason = guard
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|v| v.get("hard_stop_reasons"))
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(Value::as_str)
            .unwrap_or("budget_guard_hard_stop")
            .to_string();
        if dry_run_mode {
            return RouterGlobalBudgetGateResult {
                enabled: true,
                blocked: false,
                deferred: true,
                bypassed: false,
                reason: Some(format!("{hard_reason}_dry_run")),
                autopause_active: autopause.active,
                autopause,
                guard,
                oracle: None,
            };
        }
        autopause.active = true;
        autopause.source = Some("model_router".to_string());
        autopause.reason = Some(hard_reason.clone());
        return RouterGlobalBudgetGateResult {
            enabled: true,
            blocked: true,
            deferred: false,
            bypassed: false,
            reason: Some(hard_reason),
            autopause_active: true,
            autopause,
            guard,
            oracle: None,
        };
    }

    RouterGlobalBudgetGateResult {
        enabled: true,
        blocked: false,
        deferred: false,
        bypassed: false,
        reason: None,
        autopause_active: false,
        autopause,
        guard,
        oracle: None,
    }
}

pub fn project_budget_state(budget_state: Option<&Value>, request_tokens: Option<f64>) -> Value {
    let safe_req = request_tokens
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round() as i64)
        .unwrap_or(0)
        .max(0);

    let mut out = object_or_empty(budget_state);
    let available = out
        .get("available")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    out.insert(
        "request_tokens_est".to_string(),
        Value::Number(serde_json::Number::from(safe_req)),
    );

    if !available {
        let projected_pressure = out
            .get("pressure")
            .filter(|value| js_truthy(Some(*value)))
            .cloned()
            .unwrap_or_else(|| Value::String("none".to_string()));
        out.insert("projected_used_est".to_string(), Value::Null);
        out.insert("projected_ratio".to_string(), Value::Null);
        out.insert("projected_pressure".to_string(), projected_pressure);
        return Value::Object(out);
    }

    let policy = out.get("policy").and_then(Value::as_object);
    let soft_ratio =
        to_bounded_number_like_f64(policy.and_then(|v| v.get("soft_ratio")), 0.75, 0.2, 0.99);
    let hard_ratio =
        to_bounded_number_like_f64(policy.and_then(|v| v.get("hard_ratio")), 0.92, 0.3, 1.0);
    let cap = finite_number(out.get("token_cap"));
    let used = finite_number(out.get("used_est"));

    let Some(cap) = cap else {
        out.insert("projected_used_est".to_string(), Value::Null);
        out.insert("projected_ratio".to_string(), Value::Null);
        out.insert(
            "projected_pressure".to_string(),
            Value::String("none".to_string()),
        );
        return Value::Object(out);
    };
    let Some(used) = used else {
        out.insert("projected_used_est".to_string(), Value::Null);
        out.insert("projected_ratio".to_string(), Value::Null);
        out.insert(
            "projected_pressure".to_string(),
            Value::String("none".to_string()),
        );
        return Value::Object(out);
    };
    if cap <= 0.0 || used < 0.0 {
        out.insert("projected_used_est".to_string(), Value::Null);
        out.insert("projected_ratio".to_string(), Value::Null);
        out.insert(
            "projected_pressure".to_string(),
            Value::String("none".to_string()),
        );
        return Value::Object(out);
    }

    let projected_used = used + safe_req as f64;
    let projected_ratio = projected_used / cap;
    let projected_pressure = if projected_ratio >= hard_ratio {
        "hard"
    } else if projected_ratio >= soft_ratio {
        "soft"
    } else {
        "none"
    };

    out.insert(
        "projected_used_est".to_string(),
        number_value(projected_used),
    );
    out.insert(
        "projected_ratio".to_string(),
        number_value(rounded_4(projected_ratio)),
    );
    out.insert(
        "projected_pressure".to_string(),
        Value::String(projected_pressure.to_string()),
    );
    Value::Object(out)
}

