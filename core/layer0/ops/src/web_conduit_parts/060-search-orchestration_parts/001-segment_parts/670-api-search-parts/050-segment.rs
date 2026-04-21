    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let query_mismatch_only_failure = !provider_errors.is_empty()
            && provider_errors.iter().all(|row| {
                row.get("query_mismatch")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
        if let Some(obj) = out.as_object_mut() {
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            if current_error.is_empty() || current_error == "search_providers_exhausted" {
                if query_mismatch_only_failure {
                    obj.insert(
                        "error".to_string(),
                        Value::String("query_result_mismatch".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Search providers returned off-topic results for this query. Retry with narrower terms or explicit source URLs."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "unavailable" {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_unavailable".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tool surface is currently unavailable. Retry after provider runtime is restored."
                                .to_string(),
                        ),
                    );
                } else if tool_surface_status == "degraded"
                    && (attempted.is_empty() || !tool_surface_ready)
                {
                    obj.insert(
                        "error".to_string(),
                        Value::String("web_search_tool_surface_degraded".to_string()),
                    );
                    obj.insert(
                        "summary".to_string(),
                        Value::String(
                            "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                                .to_string(),
                        ),
                    );
                }
            }
            if obj
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.".to_string(),
                    ),
                );
            }
            if obj.get("error").is_none() {
                obj.insert(
                    "error".to_string(),
                    Value::String("search_providers_exhausted".to_string()),
                );
            }
        }
    }
    let used_lite_fallback = final_selected_provider == "duckduckgo_lite";
    let used_bing_fallback = final_selected_provider == "bing_rss";
    let (
        search_lite_fallback_reason,
        search_lite_fallback_triggered_by_challenge,
        search_lite_fallback_triggered_by_low_signal,
        search_lite_fallback_trigger_provider,
    ) = search_lite_fallback_reason(
        used_lite_fallback,
        &initial_selected_provider,
        provider_errors.as_slice(),
    );
    let (
        search_bing_fallback_reason,
        search_bing_fallback_triggered_by_challenge,
        search_bing_fallback_triggered_by_low_signal,
        search_bing_fallback_trigger_provider,
    ) = search_bing_fallback_reason(
        used_bing_fallback,
        &initial_selected_provider,
        provider_errors.as_slice(),
    );
    let tool_execution_attempted = !attempted.is_empty();
    let final_error_code = out
        .get("error")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 120));
    let query_mismatch_only_failure = out
        .get("ok")
        .and_then(Value::as_bool)
        .map(|ok| !ok)
        .unwrap_or(true)
        && !provider_errors.is_empty()
        && provider_errors.iter().all(|row| {
            row.get("query_mismatch")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });
    let challenge_like_failure = search_failure_is_challenge_like(&out, provider_errors.as_slice());
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        obj.insert("query".to_string(), Value::String(query.clone()));
        obj.insert("query_source".to_string(), json!(query_source));
        obj.insert("query_source_kind".to_string(), json!(query_source_kind));
        obj.insert(
            "query_source_confidence".to_string(),
            json!(query_source_confidence),
        );
        obj.insert(
            "query_source_recovery_mode".to_string(),
            json!(query_source_recovery_mode),
        );
        obj.insert("query_source_lineage".to_string(), query_source_lineage);
        obj.insert(
            "effective_query".to_string(),
            Value::String(scoped_query.clone()),
        );
        obj.insert(
            "allowed_domains".to_string(),
            json!(allowed_domains.clone()),
        );
        obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
        obj.insert("top_k".to_string(), json!(top_k));
        obj.insert("count".to_string(), json!(top_k));
        obj.insert("timeout_ms".to_string(), json!(timeout_ms));
        obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
        obj.insert("filters".to_string(), normalized_filters.clone());
        obj.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        obj.insert("providers_attempted".to_string(), json!(attempted));
        obj.insert("providers_skipped".to_string(), json!(skipped));
        obj.insert("provider_errors".to_string(), json!(provider_errors));
        obj.insert(
            "provider_resolution".to_string(),
            provider_resolution.clone(),
        );
        obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        obj.insert("replay_policy".to_string(), replay_policy.clone());
        obj.insert("replay_bypass".to_string(), replay_bypass.clone());
        obj.insert(
            "attempt_signature".to_string(),
            Value::String(search_attempt_signature.clone()),
        );
        obj.insert(
            "attempt_replay_guard".to_string(),
            attempt_replay_guard.clone(),
        );
        obj.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &provider_chain),
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
        obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        obj.insert(
            "search_lite_fallback_reason".to_string(),
            json!(search_lite_fallback_reason),
        );
        obj.insert(
            "search_lite_fallback_triggered_by_challenge".to_string(),
            json!(search_lite_fallback_triggered_by_challenge),
        );
        obj.insert(
            "search_lite_fallback_triggered_by_low_signal".to_string(),
            json!(search_lite_fallback_triggered_by_low_signal),
        );
        obj.insert(
            "search_lite_fallback_trigger_provider".to_string(),
            json!(search_lite_fallback_trigger_provider),
        );
        obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        obj.insert(
            "search_bing_fallback_reason".to_string(),
            json!(search_bing_fallback_reason),
        );
        obj.insert(
            "search_bing_fallback_triggered_by_challenge".to_string(),
            json!(search_bing_fallback_triggered_by_challenge),
        );
        obj.insert(
            "search_bing_fallback_triggered_by_low_signal".to_string(),
            json!(search_bing_fallback_triggered_by_low_signal),
        );
        obj.insert(
            "search_bing_fallback_trigger_provider".to_string(),
            json!(search_bing_fallback_trigger_provider),
        );
        obj.insert("provider_hint".to_string(), Value::String(provider_hint));
        obj.insert(
            "query_shape_override_used".to_string(),
            json!(query_shape_override),
        );
        obj.insert(
            "query_shape_override_source".to_string(),
            json!(query_shape_override_source),
        );
        obj.insert(
            "query_shape_stats".to_string(),
            search_query_shape_stats(&query),
        );
        obj.insert(
            "query_shape_fetch_url_candidate".to_string(),
            json!(query_shape_fetch_url_candidate.clone()),
        );
        obj.insert(
            "query_shape_fetch_url_candidate_kind".to_string(),
            json!(query_shape_fetch_url_candidate_kind),
        );
        obj.insert("query_shape_error".to_string(), json!(query_shape_error));
        obj.insert(
            "query_shape_category".to_string(),
            json!(search_query_shape_category(query_shape_error)),
        );
        obj.insert(
            "query_shape_recommended_action".to_string(),
            json!(search_query_shape_recommended_action(query_shape_error)),
        );
        obj.insert(
            "query_shape_route_hint".to_string(),
            json!(search_query_shape_route_hint(query_shape_error)),
        );
        obj.insert(
            "query_shape".to_string(),
            search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source,
            ),
        );
        obj.insert(
            "suggested_next_action".to_string(),
            search_query_shape_suggested_next_action(&query, query_shape_error),
        );
        obj.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_search",
                "provider_chain_result",
                tool_execution_attempted,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(provider_chain.clone()),
                &final_selected_provider,
                final_error_code.as_deref()
            ),
        );
        obj.insert(
            "cache_store_allowed".to_string(),
            json!(!(challenge_like_failure || query_mismatch_only_failure)),
        );
        if query_mismatch_only_failure {
            obj.insert(
                "cache_skip_reason".to_string(),
                json!("query_result_mismatch"),
            );
        } else if challenge_like_failure {
            obj.insert(
                "cache_skip_reason".to_string(),
                json!("challenge_or_low_signal_response"),
            );
        }
        obj.insert("cache_status".to_string(), json!("miss"));
        let summary_raw = clean_text(
            obj.get("summary").and_then(Value::as_str).unwrap_or(""),
            1_400,
        );
        let content_raw = clean_text(
            obj.get("content").and_then(Value::as_str).unwrap_or(""),
            120_000,
        );
        let summary_wrapped = if summary_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&summary_raw, false, "Web Search")
        };
        let content_wrapped = if content_raw.is_empty() {
            String::new()
        } else {
            wrap_external_untrusted_content(&content_raw, true, "Web Search")
        };
        obj.insert(
            "summary_wrapped".to_string(),
            Value::String(summary_wrapped),
        );
        obj.insert(
            "content_wrapped".to_string(),
            Value::String(content_wrapped),
        );
        obj.insert(
            "external_content".to_string(),
            json!({
                "untrusted": true,
                "source": "web_search",
                "wrapped": true,
                "provider_chain": provider_chain.clone(),
                "tool_surface_status": tool_surface_status.clone(),
                "query_alignment_checked": true,
                "provider": if final_selected_provider.is_empty() {
                    "none"
                } else {
                    final_selected_provider.as_str()
                }
            }),
        );
    }
    let cache_status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "ok"
    } else if challenge_like_failure {
        "challenge"
    } else {
        "no_results"
    };
    let search_url = match final_selected_provider.as_str() {
        "duckduckgo_lite" => lite_url.clone(),
        "bing_rss" => web_search_bing_rss_url(&scoped_query),
        _ => primary_url.clone(),
    };
    let response_hash = out
        .get("content")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(sha256_hex);
    let error = out
        .get("error")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let receipt = build_receipt(
        &search_url,
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            "allow"
        } else {
            "deny"
        },
        response_hash.as_deref(),
        out.get("status_code").and_then(Value::as_i64).unwrap_or(0),
        "search_provider_chain",
        error,
    );
    let mut receipt = receipt;
