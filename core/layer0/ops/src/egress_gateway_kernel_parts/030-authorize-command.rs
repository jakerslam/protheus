
fn authorize_command(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let (policy, policy_path) = load_policy_model(root, payload);
    let (mut state, state_path) = load_state_model(root, payload);
    let audit_path = audit_path(root, payload);

    let scope_id = normalize_token(&clean_text(payload.get("scope"), 160), 160);
    let method = {
        let normalized =
            normalize_token(&clean_text(payload.get("method"), 20), 20).to_ascii_uppercase();
        if normalized.is_empty() {
            "GET".to_string()
        } else {
            normalized
        }
    };
    let caller = {
        let normalized = normalize_token(&clean_text(payload.get("caller"), 120), 120);
        if normalized.is_empty() {
            "unknown".to_string()
        } else {
            normalized
        }
    };
    let url = clean_text(payload.get("url"), 2000);
    let host = parse_host(&url);
    let runtime_allowlist = payload
        .get("runtime_allowlist")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| clean_text(Some(row), 160).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let now_ms = to_u64(payload.get("now_ms"))
        .map(|value| value as i64)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let apply = payload
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let ts = chrono::Utc
        .timestamp_millis_opt(now_ms)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let hour_key = iso_hour_key(now_ms);
    let day_key = iso_day_key(now_ms);

    let mut out = json!({
        "ok": true,
        "type": "egress_gateway_decision",
        "ts": ts,
        "scope": scope_id,
        "caller": caller,
        "method": method,
        "url": url,
        "host": host,
        "allow": false,
        "reason": "unknown",
        "code": "unknown",
        "policy_path": policy_path.to_string_lossy(),
        "state_path": state_path.to_string_lossy(),
        "audit_path": audit_path.to_string_lossy(),
    });

    let Some(rule) = resolve_scope_rule(&policy, &scope_id) else {
        let allow = policy.default_decision == "allow";
        out["allow"] = Value::Bool(allow);
        let reason = if allow {
            "default_allow"
        } else {
            "scope_not_allowlisted"
        };
        out["reason"] = Value::String(reason.to_string());
        out["code"] = Value::String(reason.to_string());
        return Ok(out);
    };

    if !rule.methods.iter().any(|row| row == &method) {
        out["reason"] = Value::String("method_not_allowlisted".to_string());
        out["code"] = Value::String("method_not_allowlisted".to_string());
        return Ok(out);
    }

    if host.is_empty() {
        out["reason"] = Value::String("invalid_url".to_string());
        out["code"] = Value::String("invalid_url".to_string());
        return Ok(out);
    }

    if !rule.domains.is_empty()
        && !rule
            .domains
            .iter()
            .any(|domain| domain_matches(&host, domain))
    {
        out["reason"] = Value::String("domain_not_allowlisted".to_string());
        out["code"] = Value::String("domain_not_allowlisted".to_string());
        return Ok(out);
    }

    if rule.require_runtime_allowlist {
        if runtime_allowlist.is_empty() {
            out["reason"] = Value::String("runtime_allowlist_required".to_string());
            out["code"] = Value::String("runtime_allowlist_required".to_string());
            return Ok(out);
        }
        let runtime_allowed = runtime_allowlist
            .iter()
            .any(|domain| domain_matches(&host, domain));
        if !runtime_allowed {
            out["reason"] = Value::String("runtime_allowlist_blocked".to_string());
            out["code"] = Value::String("runtime_allowlist_blocked".to_string());
            return Ok(out);
        }
    }

    let scope_hour_key = count_key(&scope_id, &hour_key);
    let scope_day_key = count_key(&scope_id, &day_key);
    let global_hour_key = count_key("__global__", &hour_key);
    let global_day_key = count_key("__global__", &day_key);

    let per_hour_view = state
        .get("per_hour")
        .and_then(Value::as_object)
        .ok_or_else(|| "egress_gateway_kernel_invalid_state_per_hour".to_string())?;
    let per_day_view = state
        .get("per_day")
        .and_then(Value::as_object)
        .ok_or_else(|| "egress_gateway_kernel_invalid_state_per_day".to_string())?;

    if !check_cap(per_hour_view, &scope_hour_key, rule.rate_caps.per_hour) {
        out["reason"] = Value::String("scope_hour_cap_exceeded".to_string());
        out["code"] = Value::String("scope_hour_cap_exceeded".to_string());
        return Ok(out);
    }
    if !check_cap(per_day_view, &scope_day_key, rule.rate_caps.per_day) {
        out["reason"] = Value::String("scope_day_cap_exceeded".to_string());
        out["code"] = Value::String("scope_day_cap_exceeded".to_string());
        return Ok(out);
    }
    if !check_cap(
        per_hour_view,
        &global_hour_key,
        policy.global_rate_caps.per_hour,
    ) {
        out["reason"] = Value::String("global_hour_cap_exceeded".to_string());
        out["code"] = Value::String("global_hour_cap_exceeded".to_string());
        return Ok(out);
    }
    if !check_cap(
        per_day_view,
        &global_day_key,
        policy.global_rate_caps.per_day,
    ) {
        out["reason"] = Value::String("global_day_cap_exceeded".to_string());
        out["code"] = Value::String("global_day_cap_exceeded".to_string());
        return Ok(out);
    }

    out["allow"] = Value::Bool(true);
    out["reason"] = Value::String("ok".to_string());
    out["code"] = Value::String("ok".to_string());
    out["scope_resolved"] = Value::String(rule.id.clone());

    if apply {
        let per_hour = state
            .get_mut("per_hour")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "egress_gateway_kernel_invalid_state_per_hour".to_string())?;
        increment_counter(per_hour, &scope_hour_key);
        increment_counter(per_hour, &global_hour_key);
        let per_day = state
            .get_mut("per_day")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "egress_gateway_kernel_invalid_state_per_day".to_string())?;
        increment_counter(per_day, &scope_day_key);
        increment_counter(per_day, &global_day_key);
        state["updated_at"] = Value::String(ts.clone());
        write_json_atomic(&state_path, &state)?;
        append_jsonl(&audit_path, &out)?;
    }

    Ok(out)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv
        .iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        usage();
        return 0;
    }

    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "authorize".to_string());
    let payload = match lane_utils::payload_json(argv, "egress_gateway_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "egress_gateway_kernel_error",
                err.as_str(),
            ));
            return 1;
        }
    };
    let payload = lane_utils::payload_obj(&payload);

    let receipt = match command.as_str() {
        "load-policy" => lane_utils::cli_receipt(
            "egress_gateway_kernel_load_policy",
            load_policy_command(root, payload),
        ),
        "load-state" => lane_utils::cli_receipt(
            "egress_gateway_kernel_load_state",
            load_state_command(root, payload),
        ),
        "authorize" => match authorize_command(root, payload) {
            Ok(value) => lane_utils::cli_receipt("egress_gateway_kernel_authorize", value),
            Err(err) => lane_utils::cli_error("egress_gateway_kernel_error", err.as_str()),
        },
        _ => {
            usage();
            lane_utils::cli_error(
                "egress_gateway_kernel_error",
                "egress_gateway_kernel_unknown_command",
            )
        }
    };

    let exit_code = if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    lane_utils::print_json_line(&receipt);
    exit_code
}
