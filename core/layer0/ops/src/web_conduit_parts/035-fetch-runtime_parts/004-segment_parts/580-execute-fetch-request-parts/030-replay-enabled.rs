    let replay_enabled = replay_policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let replay_bypass = runtime_web_replay_bypass(&policy, request, effective_human_approved);
    let replay_bypassed = replay_bypass
        .get("bypassed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_cooldown_base_seconds = replay_policy
        .get("cooldown_base_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30);
    let replay_cooldown_step_seconds = replay_policy
        .get("cooldown_step_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(15);
    let replay_cooldown_max_seconds = replay_policy
        .get("cooldown_max_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(180);
    let attempt_replay_guard = if replay_enabled && !replay_bypassed {
        recent_tool_attempt_replay_guard(
            root,
            &fetch_attempt_signature,
            replay_window,
            replay_threshold,
            replay_cooldown_base_seconds,
            replay_cooldown_step_seconds,
            replay_cooldown_max_seconds,
        )
    } else if replay_bypassed {
        runtime_web_replay_guard_passthrough(
            "replay_guard_bypassed",
            &fetch_attempt_signature,
            replay_window,
            replay_threshold,
            replay_cooldown_base_seconds,
            replay_cooldown_step_seconds,
            replay_cooldown_max_seconds,
            &replay_bypass,
        )
    } else {
        runtime_web_replay_guard_passthrough(
            "replay_policy_disabled",
            &fetch_attempt_signature,
            replay_window,
            replay_threshold,
            replay_cooldown_base_seconds,
            replay_cooldown_step_seconds,
            replay_cooldown_max_seconds,
            &replay_bypass,
        )
    };
    let attempt_replay_blocked = attempt_replay_guard
        .get("blocked")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let replay_retry_after_seconds = attempt_replay_guard.get("retry_after_seconds");
    let replay_retry_after_seconds = fetch_retry_after_seconds_from_value(replay_retry_after_seconds);
    let replay_retry_lane = clean_text(
        attempt_replay_guard
            .get("retry_lane")
            .and_then(Value::as_str)
            .unwrap_or("change_query_or_provider"),
        80,
    );
    if let Some(mut cached) = load_fetch_cache(root, &fetch_cache_key) {
        if let Some(obj) = cached.as_object_mut() {
            obj.insert(
                "requested_url".to_string(),
                Value::String(raw_requested_url.clone()),
            );
            obj.insert("resolved_url".to_string(), Value::String(resolved_url.clone()));
            obj.insert("extract_mode".to_string(), Value::String(extract_mode.clone()));
            obj.insert(
                "citation_redirect_resolved".to_string(),
                Value::Bool(redirect_resolved),
            );
            obj.insert("cache_status".to_string(), json!("hit"));
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
            obj.insert(
                "provider_resolution".to_string(),
                provider_resolution.clone(),
            );
            obj.insert(
                "fetch_url_shape_stats".to_string(),
                fetch_url_shape_stats(&raw_requested_url),
            );
            obj.insert(
                "fetch_url_shape_error".to_string(),
                json!(fetch_url_shape_error),
            );
            obj.insert(
                "fetch_url_shape_category".to_string(),
                json!(fetch_url_shape_category(fetch_url_shape_error)),
            );
            obj.insert(
                "fetch_url_shape_recommended_action".to_string(),
                json!(fetch_url_shape_recommended_action(fetch_url_shape_error)),
            );
            obj.insert(
                "fetch_url_shape_route_hint".to_string(),
                json!(fetch_url_shape_route_hint(fetch_url_shape_error)),
            );
            obj.insert(
                "fetch_url_shape_override_used".to_string(),
                json!(fetch_url_override_used),
            );
            obj.insert(
                "fetch_url_shape_override_source".to_string(),
                json!(fetch_url_override_source),
            );
            obj.insert(
                "requested_url_input".to_string(),
                Value::String(requested_url_input.clone()),
            );
            obj.insert(
                "requested_url_source".to_string(),
                Value::String(requested_url_source.to_string()),
            );
            obj.insert(
                "requested_url_source_kind".to_string(),
                Value::String(requested_url_source_kind.to_string()),
            );
            obj.insert(
                "requested_url_source_confidence".to_string(),
                Value::String(requested_url_source_confidence.to_string()),
            );
            obj.insert(
                "requested_url_source_recovery_mode".to_string(),
                Value::String(requested_url_source_recovery_mode.to_string()),
            );
            obj.insert(
                "requested_url_source_lineage".to_string(),
                requested_url_source_lineage.clone(),
            );
            obj.insert(
                "fetch_url_shape".to_string(),
                fetch_url_shape_contract(
                    &requested_url_input,
                    &raw_requested_url,
                    fetch_url_shape_error,
                    fetch_url_override_used,
                    fetch_url_override_source,
                ),
            );
            obj.insert(
                "provider_health".to_string(),
                provider_health_snapshot(root, &fetch_provider_chain),
            );
            obj.insert(
                "tool_surface_status".to_string(),
                Value::String(tool_surface_status.clone()),
            );
            obj.insert("tool_surface_ready".to_string(), json!(tool_surface_ready));
            obj.insert(
                "tool_surface_blocking_reason".to_string(),
                Value::String(tool_surface_blocking_reason.clone()),
            );
            obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
            obj.insert("replay_policy".to_string(), replay_policy.clone());
            obj.insert("replay_bypass".to_string(), replay_bypass.clone());
            obj.insert(
                "attempt_signature".to_string(),
                Value::String(fetch_attempt_signature.clone()),
            );
            obj.insert(
                "attempt_replay_guard".to_string(),
                attempt_replay_guard.clone(),
            );
        }
        return cached;
    }
