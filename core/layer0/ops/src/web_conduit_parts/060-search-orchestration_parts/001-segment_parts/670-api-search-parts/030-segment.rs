    let tool_surface_ready = provider_resolution
        .get("tool_surface_ready")
        .or_else(|| {
            provider_resolution.pointer("/tool_surface_health/selected_provider_ready")
        })
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tool_surface_blocking_reason = provider_resolution
        .pointer("/tool_surface_health/blocking_reason")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_string();
    let tool_execution_gate = provider_resolution
        .get("tool_execution_gate")
        .cloned()
        .unwrap_or_else(|| {
            runtime_web_execution_gate(
                &tool_surface_status,
                tool_surface_ready,
                allow_fallback,
                &tool_surface_blocking_reason,
            )
        });
    let tool_execution_allowed = tool_execution_gate
        .get("should_execute")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let search_attempt_signature = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}|{}",
        scoped_query,
        provider_chain.join(","),
        top_k,
        summary_only,
        timeout_ms,
        allow_fallback,
        tool_surface_status
    ));
    let replay_policy =
        runtime_web_replay_policy(&policy, request, &tool_surface_status, tool_surface_ready);
    let replay_window = replay_policy
        .get("window")
        .and_then(Value::as_u64)
        .unwrap_or(24)
        .clamp(1, 200) as usize;
    let replay_threshold = replay_policy
        .get("block_threshold")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .clamp(2, 200) as usize;
    let replay_enabled = replay_policy
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let replay_bypass = runtime_web_replay_bypass(&policy, request, human_approved);
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
    let attempt_replay_guard =
        if replay_enabled && !replay_bypassed {
            recent_tool_attempt_replay_guard(
                root,
                &search_attempt_signature,
                replay_window,
                replay_threshold,
                replay_cooldown_base_seconds,
                replay_cooldown_step_seconds,
                replay_cooldown_max_seconds,
            )
        } else if replay_bypassed {
            runtime_web_replay_guard_passthrough(
                "replay_guard_bypassed",
                &search_attempt_signature,
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
                &search_attempt_signature,
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
    let replay_retry_after_seconds =
        search_retry_after_seconds_from_value(replay_retry_after_seconds);
    let replay_retry_lane = clean_text(
        attempt_replay_guard
            .get("retry_lane")
            .and_then(Value::as_str)
            .unwrap_or("change_query_or_provider"),
        80,
    );
    if !tool_execution_allowed {
        let preflight_error = if tool_surface_status == "unavailable" {
            "web_search_tool_surface_unavailable"
        } else if tool_surface_status == "degraded" {
            "web_search_tool_surface_degraded"
        } else {
            "web_search_tool_execution_blocked"
        };
        let search_url = match selected_provider.as_str() {
            "duckduckgo_lite" => lite_url.clone(),
            "bing_rss" => web_search_bing_rss_url(&scoped_query),
            _ => primary_url.clone(),
        };
        let mut receipt = build_receipt(
            &search_url,
            "deny",
            None,
            0,
            "search_preflight_gate_blocked",
            Some(preflight_error),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(search_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "gate_mode".to_string(),
                Value::String(
                    tool_execution_gate
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("blocked")
                        .to_string(),
                ),
            );
        }
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let mut out = serde_json::Map::<String, Value>::new();
        out.insert("ok".to_string(), Value::Bool(false));
        out.insert("error".to_string(), Value::String(preflight_error.to_string()));
        out.insert(
            "summary".to_string(),
            Value::String(
                "Web search execution was blocked by runtime tooling gate before provider calls were attempted."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        out.insert("query".to_string(), Value::String(query.clone()));
        out.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        out.insert("allowed_domains".to_string(), json!(allowed_domains.clone()));
        out.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        out.insert("top_k".to_string(), json!(top_k));
        out.insert("count".to_string(), json!(top_k));
        out.insert("timeout_ms".to_string(), json!(timeout_ms));
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("query_source".to_string(), json!(query_source));
        out.insert("query_source_kind".to_string(), json!(query_source_kind));
        out.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        out.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        out.insert("query_source_lineage".to_string(), query_source_lineage.clone());
        out.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        out.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        out.insert("query_shape_error".to_string(), json!(query_shape_error));
        out.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        out.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        out.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        out.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        out.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        out.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert("tool_surface_ready".to_string(), Value::Bool(tool_surface_ready));
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            search_retry_envelope_runtime(
                if preflight_error == "web_search_tool_surface_unavailable" {
                    "restore_tool_surface_or_use_supported_provider"
                } else if preflight_error == "web_search_tool_surface_degraded" {
                    "stabilize_provider_runtime_and_retry"
                } else {
                    "change_query_or_provider"
                },
                preflight_error,
                &replay_retry_lane,
                replay_retry_after_seconds,
            ),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "preflight_blocked",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &selected_provider,
                Some(preflight_error),
            ),
        );
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
