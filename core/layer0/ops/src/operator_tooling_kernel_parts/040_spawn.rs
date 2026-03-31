fn run_sync_allowed_models(openclaw_root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let policy_path = routing_policy_path(openclaw_root, parsed);
    let policy = read_json_file(&policy_path).unwrap_or_else(|| json!({}));
    let models = all_models_from_policy(&policy);
    let models_path = agent_root(openclaw_root).join("models.json");
    let mut payload = read_json_file(&models_path).unwrap_or_else(|| json!({}));
    payload["type"] = json!("operator_tooling_models_allowlist");
    payload["updated_at"] = json!(crate::now_iso());
    payload["allowed_models"] =
        Value::Array(models.iter().cloned().map(Value::String).collect::<Vec<_>>());
    write_json_file(&models_path, &payload)?;
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_sync_allowed_models",
        "policy_path": policy_path.to_string_lossy().to_string(),
        "models_path": models_path.to_string_lossy().to_string(),
        "allowed_model_count": models.len()
    })))
}

fn run_smoke_routing(openclaw_root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let policy_path = routing_policy_path(openclaw_root, parsed);
    let policy = read_json_file(&policy_path).unwrap_or_else(|| json!({}));
    let scenarios = vec![
        vec!["general".to_string(), "analysis".to_string()],
        vec!["security".to_string(), "prod".to_string()],
        vec!["creative".to_string()],
    ];
    let rows = scenarios
        .into_iter()
        .map(|tags| {
            let route = route_model_with_policy(&policy, &tags, DEFAULT_MODEL);
            json!({
                "tags": tags,
                "model": route.get("model").cloned().unwrap_or(Value::Null),
                "tier": route.get("tier").cloned().unwrap_or(Value::Null)
            })
        })
        .collect::<Vec<_>>();
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_smoke_routing",
        "policy_path": policy_path.to_string_lossy().to_string(),
        "scenarios": rows
    }))
}

fn pseudo_uuid(seed: &str) -> String {
    let hash = crate::deterministic_receipt_hash(&json!({ "seed": seed }));
    format!(
        "{}-{}-{}-{}-{}",
        &hash[0..8],
        &hash[8..12],
        &hash[12..16],
        &hash[16..20],
        &hash[20..32]
    )
}

fn spawn_routing_config(openclaw_root: &Path, parsed: &crate::ParsedArgs) -> (usize, usize, bool, HashSet<String>) {
    let state_file = state_path(openclaw_root, parsed);
    let state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let routing = state
        .get("routing")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let min_tags = routing
        .get("required_tags_min")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(3)
        .clamp(1, 12);
    let max_tags = routing
        .get("required_tags_max")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(6)
        .max(min_tags)
        .clamp(min_tags, 24);
    let high_risk_requires_plan = routing
        .get("high_risk_requires_plan")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let configured_tags = routing
        .get("high_risk_tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean_text(v, 64).to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect::<HashSet<_>>();
    let high_risk_tag_set = if configured_tags.is_empty() {
        high_risk_tags()
            .iter()
            .map(|v| (*v).to_string())
            .collect::<HashSet<_>>()
    } else {
        configured_tags
    };
    (min_tags, max_tags, high_risk_requires_plan, high_risk_tag_set)
}

fn build_spawn_packet(
    openclaw_root: &Path,
    parsed: &crate::ParsedArgs,
    payload: &Value,
    require_plan: bool,
    strict_plan: bool,
) -> Result<Value, String> {
    let task = clean_text(payload.get("task").and_then(Value::as_str).unwrap_or(""), 500);
    if task.is_empty() {
        return Err("task_required".to_string());
    }
    let tags = norm_tags(payload.get("tags"));
    let (min_tags, max_tags, high_risk_requires_plan, high_risk_tag_set) =
        spawn_routing_config(openclaw_root, parsed);
    if tags.len() < min_tags || tags.len() > max_tags {
        return Err(format!("tags_len_out_of_range:{min_tags}-{max_tags}"));
    }
    let high_risk = tags.iter().any(|tag| high_risk_tag_set.contains(tag));
    let plan = payload.get("plan").cloned().unwrap_or_else(|| json!({}));
    let has_plan = plan
        .as_object()
        .map(|obj| !obj.is_empty())
        .unwrap_or(false);
    if (require_plan || (high_risk_requires_plan && high_risk)) && !has_plan {
        return Err("plan_required_for_high_risk_tags".to_string());
    }
    if strict_plan && has_plan {
        run_plan_validate(&plan)?;
    }
    let postflight_checks = payload
        .get("postflightChecks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean_text(v, 320))
        .filter(|v| !v.is_empty())
        .map(Value::String)
        .collect::<Vec<_>>();
    let policy_path = routing_policy_path(openclaw_root, parsed);
    let policy = read_json_file(&policy_path).unwrap_or_else(|| json!({}));
    let escalation = run_escalate_model(&policy, payload, &policy_path);
    let model_chain = escalation
        .get("modelChain")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!(DEFAULT_MODEL)]);
    let selected_model = model_chain
        .first()
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 240))
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let created_utc = crate::now_iso();
    let seed = format!("{task}|{}|{created_utc}", tags.join("|"));
    let trace_id = pseudo_uuid(&seed);
    let handoff_id = crate::deterministic_receipt_hash(&json!({
        "task": task,
        "tags": tags,
        "plan": plan,
        "created_minute": created_utc.chars().take(16).collect::<String>()
    }))
    .chars()
    .take(32)
    .collect::<String>();
    let idempotency_key = crate::deterministic_receipt_hash(&json!({
        "task": task,
        "tags": tags,
        "plan": plan
    }))
    .chars()
    .take(32)
    .collect::<String>();
    let cache_key = tags.join("|");
    let max_attempts = payload
        .pointer("/onFailure/maxAttempts")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or_else(|| model_chain.len().max(1))
        .clamp(1, 12);
    let retry_policy = if high_risk {
        "single_attempt_high_risk".to_string()
    } else if max_attempts <= 1 {
        "single_attempt".to_string()
    } else {
        "escalate_chain".to_string()
    };

    let escalation_order = model_chain.clone();
    let packet = json!({
        "validated": true,
        "handoff": {
            "version": "1.0",
            "trace_id": trace_id,
            "handoff_id": handoff_id,
            "idempotency_key": idempotency_key,
            "created_utc": created_utc,
            "task": task,
            "tags": tags,
            "plan": plan,
            "timeout_sec": parse_usize_flag(&parsed.flags, "timeout-sec", 30, 5, 3600),
            "model_chain": model_chain,
            "selected_model": selected_model,
            "cache_key": cache_key,
            "high_risk": high_risk,
            "postflight_checks": postflight_checks,
            "retry_policy": {
                "max_attempts": max_attempts,
                "on_failure": retry_policy,
                "escalation_order": escalation_order
            }
        }
    });
    let state_file = state_path(openclaw_root, parsed);
    let mut state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let mut handoffs = state
        .get("handoffs_recent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    handoffs.insert(
        idempotency_key.clone(),
        json!({
            "handoff_id": handoff_id,
            "trace_id": trace_id,
            "ts": created_utc,
            "ttl_sec": 3600
        }),
    );
    state["handoffs_recent"] = Value::Object(handoffs);
    write_json_file(&state_file, &state)?;
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_spawn_safe",
        "packet": packet,
        "policy_path": policy_path.to_string_lossy().to_string(),
        "state_path": state_file.to_string_lossy().to_string()
    })))
}

fn run_smart_spawn(openclaw_root: &Path, parsed: &crate::ParsedArgs, payload: &Value) -> Result<Value, String> {
    let state_file = state_path(openclaw_root, parsed);
    let state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let always_sync_allowlist = state
        .pointer("/preferences/always_sync_allowlist")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if always_sync_allowlist {
        let _ = run_sync_allowed_models(openclaw_root, parsed);
    }
    let strict_plan = bool_flag(&parsed.flags, "strict-plan", false);
    let safe = build_spawn_packet(openclaw_root, parsed, payload, false, strict_plan)?;
    let mut packet = safe
        .get("packet")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let cache_key = packet
        .pointer("/handoff/cache_key")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 240))
        .unwrap_or_default();
    let mut cache_hit = false;
    if let Some(state) = read_json_file(&state_file) {
        if let Some(cached_model) = state
            .pointer(&format!("/spawn_cache/{cache_key}/model"))
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 240))
        {
            let chain = packet
                .pointer("/handoff/model_chain")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if chain
                .iter()
                .filter_map(Value::as_str)
                .any(|model| model == cached_model)
            {
                packet["handoff"]["selected_model"] = json!(cached_model);
                cache_hit = true;
            }
        }
    }
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_smart_spawn",
        "cache_hit": cache_hit,
        "packet": packet
    })))
}

fn run_execute_handoff(openclaw_root: &Path, parsed: &crate::ParsedArgs, payload: &Value) -> Result<Value, String> {
    let handoff = payload.get("handoff").cloned().unwrap_or_else(|| payload.clone());
    if !handoff.is_object() {
        return Err("handoff_object_required".to_string());
    }
    for required in ["task", "tags", "selected_model", "model_chain", "timeout_sec"] {
        if handoff.get(required).is_none() {
            return Err(format!("handoff_missing_{required}"));
        }
    }
    let idempotency_key = clean_text(
        handoff
            .get("idempotency_key")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    if idempotency_key.is_empty() {
        return Err("handoff_idempotency_key_required".to_string());
    }
    let state_file = state_path(openclaw_root, parsed);
    let mut state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let mut executions = state
        .get("executions_recent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    executions.insert(
        idempotency_key.clone(),
        json!({
            "trace_id": handoff.get("trace_id").cloned().unwrap_or(Value::Null),
            "handoff_id": handoff.get("handoff_id").cloned().unwrap_or(Value::Null),
            "selected_model": handoff.get("selected_model").cloned().unwrap_or(Value::Null),
            "task": handoff.get("task").cloned().unwrap_or(Value::Null),
            "success": true,
            "ts": crate::now_iso()
        }),
    );
    state["executions_recent"] = Value::Object(executions);
    write_json_file(&state_file, &state)?;
    let postflight = json!({
        "files_touched": [],
        "commands_run": handoff
            .get("postflight_checks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        "routing_tags": handoff.get("tags").cloned().unwrap_or_else(|| json!([])),
        "model_used": handoff.get("selected_model").cloned().unwrap_or(Value::Null),
        "result_summary": "execute_handoff_recorded"
    });
    let postflight_valid = run_postflight_validate(&postflight).is_ok();
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_execute_handoff",
        "idempotency_key": idempotency_key,
        "postflight": postflight,
        "postflight_valid": postflight_valid,
        "state_path": state_file.to_string_lossy().to_string()
    })))
}

fn run_auto_spawn(openclaw_root: &Path, parsed: &crate::ParsedArgs, payload: &Value) -> Result<Value, String> {
    let smart = run_smart_spawn(openclaw_root, parsed, payload)?;
    let packet = smart.get("packet").cloned().unwrap_or_else(|| json!({}));
    let handoff = packet.get("handoff").cloned().unwrap_or_else(|| json!({}));
    let chain = handoff
        .get("model_chain")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let max_attempts = parse_usize_flag(&parsed.flags, "max-attempts", 3, 1, 12);
    let success_on = payload
        .get("success_on")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(1)
        .clamp(1, max_attempts);
    let mut attempts = Vec::<Value>::new();
    let mut selected_model = handoff
        .get("selected_model")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 240))
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    for idx in 0..max_attempts {
        let model = chain
            .get(idx)
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 240))
            .unwrap_or_else(|| selected_model.clone());
        let success = idx + 1 == success_on;
        if success {
            selected_model = model.clone();
        }
        attempts.push(json!({
            "attempt": idx + 1,
            "model": model,
            "success": success
        }));
        if success {
            break;
        }
    }

    let state_file = state_path(openclaw_root, parsed);
    let mut state = read_json_file(&state_file).unwrap_or_else(|| json!({}));
    let cache_key = clean_text(
        handoff.get("cache_key").and_then(Value::as_str).unwrap_or(""),
        240,
    );
    if !cache_key.is_empty() {
        let mut cache = state
            .get("spawn_cache")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        cache.insert(
            cache_key,
            json!({
                "model": selected_model,
                "last_success_utc": crate::now_iso(),
                "fail_streak": 0
            }),
        );
        state["spawn_cache"] = Value::Object(cache);
    }
    write_json_file(&state_file, &state)?;

    let postflight = json!({
        "files_touched": [],
        "commands_run": ["spawn-safe", "smart-spawn", "auto-spawn"],
        "routing_tags": handoff.get("tags").cloned().unwrap_or_else(|| json!([])),
        "model_used": selected_model,
        "result_summary": "auto_spawn_prepared"
    });
    let postflight_ok = run_postflight_validate(&postflight).is_ok();

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_auto_spawn",
        "packet": packet,
        "attempts": attempts,
        "postflight": postflight,
        "postflight_valid": postflight_ok,
        "state_path": state_file.to_string_lossy().to_string()
    })))
}
