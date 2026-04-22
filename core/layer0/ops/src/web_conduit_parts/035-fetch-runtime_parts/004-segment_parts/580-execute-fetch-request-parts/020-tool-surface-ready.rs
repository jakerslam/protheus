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
    let allow_rfc2544_benchmark_range = request
        .pointer("/ssrf_policy/allow_rfc2544_benchmark_range")
        .or_else(|| request.pointer("/ssrfPolicy/allowRfc2544BenchmarkRange"))
        .and_then(Value::as_bool)
        .or_else(|| {
            policy
                .pointer("/web_conduit/ssrf_policy/allow_rfc2544_benchmark_range")
                .or_else(|| {
                    policy.pointer("/web_conduit/ssrfPolicy/allowRfc2544BenchmarkRange")
                })
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    let ssrf_guard =
        evaluate_fetch_ssrf_guard(&resolved_url, allow_rfc2544_benchmark_range, None);
    if !ssrf_guard
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let error = clean_text(
            ssrf_guard
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("blocked_private_network_target"),
            220,
        );
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            &error,
            Some(error.as_str()),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": error,
            "type": "web_conduit_fetch",
            "requested_url": raw_requested_url,
            "requested_url_input": requested_url_input,
            "requested_url_source": requested_url_source,
            "requested_url_source_kind": requested_url_source_kind,
            "requested_url_source_confidence": requested_url_source_confidence,
            "requested_url_source_recovery_mode": requested_url_source_recovery_mode,
            "requested_url_source_lineage": requested_url_source_lineage,
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider.clone(),
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain.clone(),
            "provider_resolution": provider_resolution,
            "provider_health": provider_health_snapshot(root, &fetch_provider_chain),
            "fetch_provider_catalog": fetch_provider_catalog_snapshot(root, &policy),
            "tool_surface_status": tool_surface_status.clone(),
            "tool_surface_ready": tool_surface_ready,
            "tool_surface_blocking_reason": tool_surface_blocking_reason,
            "fetch_url_shape_stats": fetch_url_shape_stats(&raw_requested_url),
            "fetch_url_shape_error": fetch_url_shape_error,
            "fetch_url_shape_category": fetch_url_shape_category(fetch_url_shape_error),
            "fetch_url_shape_recommended_action": fetch_url_shape_recommended_action(fetch_url_shape_error),
            "fetch_url_shape_route_hint": fetch_url_shape_route_hint(fetch_url_shape_error),
            "fetch_url_shape_override_used": fetch_url_override_used,
            "fetch_url_shape_override_source": fetch_url_override_source,
            "fetch_url_shape": fetch_url_shape_contract(
                &requested_url_input,
                &raw_requested_url,
                fetch_url_shape_error,
                fetch_url_override_used,
                fetch_url_override_source
            ),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "reason": "ssrf_blocked",
                "source": "preflight"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "ssrf_blocked",
            "retry": fetch_retry_envelope_runtime(
                "use_public_http_or_https_target",
                "ssrf_blocked",
                "web_fetch",
                0
            ),
            "ssrf_guard": ssrf_guard,
            "receipt": receipt
        });
    }
    let policy_eval = crate::infring_layer1_security_bridge::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": resolved_url,
            "domain": extract_domain(&resolved_url),
            "human_approved": effective_human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let decision = clean_text(
        policy_eval
            .get("decision")
            .and_then(Value::as_str)
            .unwrap_or("deny"),
        20,
    );
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let approval = if reason == "human_approval_required_for_sensitive_domain" {
            ensure_sensitive_web_approval(root, &resolved_url, &policy_eval)
        } else {
            None
        };
        let cache_skip_reason = if approval.is_some() {
            "approval_required"
        } else {
            "policy_denied"
        };
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            &reason,
            Some(cache_skip_reason),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "type": "web_conduit_fetch",
            "requested_url": raw_requested_url,
            "requested_url_input": requested_url_input,
            "requested_url_source": requested_url_source,
            "requested_url_source_kind": requested_url_source_kind,
            "requested_url_source_confidence": requested_url_source_confidence,
            "requested_url_source_recovery_mode": requested_url_source_recovery_mode,
            "requested_url_source_lineage": requested_url_source_lineage,
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider.clone(),
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain.clone(),
            "provider_resolution": provider_resolution,
            "provider_health": provider_health_snapshot(root, &fetch_provider_chain),
            "fetch_provider_catalog": fetch_provider_catalog_snapshot(root, &policy),
            "tool_surface_status": tool_surface_status.clone(),
            "tool_surface_ready": tool_surface_ready,
            "tool_surface_blocking_reason": tool_surface_blocking_reason,
            "fetch_url_shape_stats": fetch_url_shape_stats(&raw_requested_url),
            "fetch_url_shape_error": fetch_url_shape_error,
            "fetch_url_shape_category": fetch_url_shape_category(fetch_url_shape_error),
            "fetch_url_shape_recommended_action": fetch_url_shape_recommended_action(fetch_url_shape_error),
            "fetch_url_shape_route_hint": fetch_url_shape_route_hint(fetch_url_shape_error),
            "fetch_url_shape_override_used": fetch_url_override_used,
            "fetch_url_shape_override_source": fetch_url_override_source,
            "fetch_url_shape": fetch_url_shape_contract(
                &requested_url_input,
                &raw_requested_url,
                fetch_url_shape_error,
                fetch_url_override_used,
                fetch_url_override_source
            ),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "reason": "policy_denied",
                "source": "preflight"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": cache_skip_reason,
            "retry": fetch_retry_envelope_runtime(
                if reason == "human_approval_required_for_sensitive_domain" {
                    "approve_and_retry"
                } else {
                    "adjust_policy_or_target"
                },
                &reason,
                "web_fetch",
                0
            ),
            "policy_decision": policy_eval,
            "receipt": receipt,
            "approval_required": approval.is_some(),
            "approval": approval,
            "approval_state": approval_state,
            "retry_with": if reason == "human_approval_required_for_sensitive_domain" {
                json!({
                    "url": raw_requested_url,
                    "approval_id": approval
                        .as_ref()
                        .and_then(|row| row.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or(approval_id.as_str()),
                    "summary_only": summary_only
                })
            } else {
                Value::Null
            }
        });
    }
    let timeout_ms = parse_fetch_u64(
        request.get("timeout_ms").or_else(|| request.get("timeoutMs")),
        policy_eval
            .pointer("/policy/timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(9000),
        1000,
        120_000,
    );
    let max_response_bytes = parse_fetch_u64(
        request
            .get("max_response_bytes")
            .or_else(|| request.get("maxResponseBytes")),
        policy_eval
            .pointer("/policy/max_response_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(350_000),
        4096,
        4_000_000,
    ) as usize;
    let max_chars = parse_fetch_u64(
        request.get("max_chars").or_else(|| request.get("maxChars")),
        max_response_bytes.min(120_000) as u64,
        100,
        200_000,
    ) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let cache_ttl_minutes = parse_fetch_u64(
        request
            .get("cache_ttl_minutes")
            .or_else(|| request.get("cacheTtlMinutes")),
        15,
        0,
        240,
    );
    let fetch_cache_key = fetch_cache_key(
        &raw_requested_url,
        &resolved_url,
        &extract_mode,
        max_chars,
        summary_only,
        &fetch_provider_chain,
    );
    let tool_execution_gate = provider_resolution
        .get("tool_execution_gate")
        .cloned()
        .unwrap_or_else(|| {
            let allow_fallback = provider_resolution
                .get("allow_fallback")
                .and_then(Value::as_bool)
                .unwrap_or(true);
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
    let fetch_attempt_signature = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        raw_requested_url,
        resolved_url,
        extract_mode,
        max_chars,
        summary_only,
        timeout_ms,
        fetch_provider_chain.join(","),
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
