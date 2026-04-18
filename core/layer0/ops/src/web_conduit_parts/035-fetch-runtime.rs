fn fetch_override_flag_enabled(value: &Value) -> bool {
    runtime_web_truthy_flag(value)
}

fn fetch_meta_query_override(request: &Value) -> bool {
    let pointers = [
        "/allow_meta_query_search",
        "/allowMetaQuerySearch",
        "/force_web_search",
        "/forceWebSearch",
        "/force_web_lookup",
        "/forceWebLookup",
        "/allow_meta_query_fetch",
        "/allowMetaQueryFetch",
        "/force_web_fetch",
        "/forceWebFetch",
        "/search_policy/allow_meta_query_search",
        "/searchPolicy/allowMetaQuerySearch",
        "/search_policy/force_web_search",
        "/searchPolicy/forceWebSearch",
        "/search_policy/force_web_lookup",
        "/searchPolicy/forceWebLookup",
        "/fetch_policy/allow_meta_query_fetch",
        "/fetchPolicy/allowMetaQueryFetch",
        "/fetch_policy/force_web_fetch",
        "/fetchPolicy/forceWebFetch",
    ];
    runtime_web_request_flag(request, &pointers)
}

fn fetch_requested_url_looks_meta_diagnostic(raw_requested_url: &str) -> bool {
    let lowered = clean_text(raw_requested_url, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("www.")
        || lowered.contains("://")
    {
        return false;
    }
    if !lowered.contains(' ') && !lowered.contains('?') {
        return false;
    }
    if [
        "that was just a test",
        "that was a test",
        "did you do the web request",
        "did you try it",
        "where did that come from",
        "why did my last prompt",
        "you returned no result",
    ]
    .iter()
    .any(|marker| lowered.contains(*marker))
    {
        return true;
    }
    let meta_hits = [
        "what happened",
        "workflow",
        "tool call",
        "web tooling",
        "hallucination",
        "hallucinated",
        "training data",
        "context issue",
        "last response",
        "previous response",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    let urlish_hits = [
        ".com", ".org", ".net", ".io", "site:", "docs", "api.", "www.", "http", "https",
    ]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    meta_hits >= 2 && urlish_hits == 0
}

fn execute_fetch_request(root: &Path, request: &Value) -> Value {
    let raw_requested_url = clean_text(
        request
            .get("requested_url")
            .or_else(|| request.get("url"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2200,
    );
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = clean_text(
        request
            .get("approval_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        160,
    );
    let extract_mode = fetch_extract_mode(request);
    let requested_timeout_ms = parse_fetch_u64(
        request.get("timeout_ms").or_else(|| request.get("timeoutMs")),
        9000,
        1000,
        120_000,
    );
    let resolve_redirect = request
        .get("resolve_citation_redirect")
        .or_else(|| request.get("resolveCitationRedirect"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let (resolved_url, redirect_resolved) = if resolve_redirect {
        resolve_citation_redirect_url(&raw_requested_url, requested_timeout_ms)
    } else {
        (raw_requested_url.clone(), false)
    };
    let approval_state = approval_state_for_request(root, &approval_id, &resolved_url);
    let token_approved = approval_state.as_deref() == Some("approved");
    let effective_human_approved = human_approved || token_approved;
    let (policy, _policy_path_value) = load_policy(root);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("fetch_provider"))
            .or_else(|| request.get("fetchProvider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    if !fetch_meta_query_override(request)
        && fetch_requested_url_looks_meta_diagnostic(&raw_requested_url)
    {
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "non_fetch_meta_query",
            Some("meta_diagnostic_url_input"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "non_fetch_meta_query",
            "type": "web_conduit_fetch",
            "requested_url": raw_requested_url,
            "resolved_url": "",
            "citation_redirect_resolved": false,
            "provider": "none",
            "provider_hint": provider_hint,
            "provider_chain": [],
            "provider_resolution": {
                "status": "not_evaluated",
                "reason": "meta_query_blocked",
                "source": "early_validation",
                "tool_surface_health": {
                    "status": "not_evaluated",
                    "selected_provider_ready": false,
                    "blocking_reason": "early_validation"
                }
            },
            "tool_surface_status": "not_evaluated",
            "tool_surface_ready": false,
            "tool_surface_blocking_reason": "early_validation",
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "reason": "meta_query_blocked",
                "source": "early_validation"
            },
            "meta_query_blocked": true,
            "cache_status": "blocked_meta_query",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "meta_query_blocked",
            "process_summary": runtime_web_process_summary(
                "web_fetch",
                "early_validation",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "meta_query_blocked",
                    "source": "early_validation"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("non_fetch_meta_query")
            ),
            "summary": "Requested fetch URL appears to be conversational/tooling diagnostics rather than a valid web URL. Answer directly without running web fetch. To force fetch evaluation, set force_web_fetch=true or force_web_search=true.",
            "content": "",
            "override_hint": "force_web_fetch=true|force_web_search=true",
            "receipt": receipt
        });
    }
    if let Some(unknown_provider) = validate_explicit_fetch_provider_hint(&provider_hint) {
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "unknown_fetch_provider",
            Some(&unknown_provider),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "unknown_fetch_provider",
            "type": "web_conduit_fetch",
            "requested_url": raw_requested_url,
            "resolved_url": "",
            "citation_redirect_resolved": false,
            "provider": "none",
            "provider_hint": provider_hint,
            "provider_chain": [],
            "provider_resolution": {
                "status": "not_evaluated",
                "reason": "unknown_fetch_provider",
                "source": "early_validation",
                "tool_surface_health": {
                    "status": "not_evaluated",
                    "selected_provider_ready": false,
                    "blocking_reason": "early_validation"
                }
            },
            "tool_surface_status": "not_evaluated",
            "tool_surface_ready": false,
            "tool_surface_blocking_reason": "early_validation",
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "reason": "unknown_fetch_provider",
                "source": "early_validation"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "unknown_fetch_provider",
            "process_summary": runtime_web_process_summary(
                "web_fetch",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "unknown_fetch_provider",
                    "source": "early_validation"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("unknown_fetch_provider")
            ),
            "requested_provider": unknown_provider,
            "fetch_provider_catalog": fetch_provider_catalog_snapshot(root, &policy),
            "receipt": receipt
        });
    }
    let (provider_resolution, fetch_provider_chain, selected_provider) =
        resolved_fetch_provider_selection(root, &policy, request, &provider_hint);
    let tool_surface_status = provider_resolution
        .get("tool_surface_status")
        .or_else(|| provider_resolution.pointer("/tool_surface_health/status"))
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .to_string();
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
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider.clone(),
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain.clone(),
            "provider_resolution": provider_resolution,
            "provider_health": provider_health_snapshot(root, &fetch_provider_chain),
            "tool_surface_status": tool_surface_status.clone(),
            "tool_surface_ready": tool_surface_ready,
            "tool_surface_blocking_reason": tool_surface_blocking_reason,
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
            "ssrf_guard": ssrf_guard,
            "receipt": receipt
        });
    }
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
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
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider.clone(),
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain.clone(),
            "provider_resolution": provider_resolution,
            "provider_health": provider_health_snapshot(root, &fetch_provider_chain),
            "tool_surface_status": tool_surface_status.clone(),
            "tool_surface_ready": tool_surface_ready,
            "tool_surface_blocking_reason": tool_surface_blocking_reason,
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
    let replay_retry_after_seconds = attempt_replay_guard
        .get("retry_after_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(0);
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
    if !tool_execution_allowed {
        let preflight_error = if tool_surface_status == "unavailable" {
            "web_fetch_tool_surface_unavailable"
        } else if tool_surface_status == "degraded" {
            "web_fetch_tool_surface_degraded"
        } else {
            "web_fetch_tool_execution_blocked"
        };
        let cache_skip_reason = if preflight_error == "web_fetch_tool_surface_unavailable" {
            "tool_surface_unavailable"
        } else if preflight_error == "web_fetch_tool_surface_degraded" {
            "tool_surface_degraded"
        } else {
            "tool_execution_blocked"
        };
        let mut receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "fetch_preflight_gate_blocked",
            Some(preflight_error),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(fetch_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "provider".to_string(),
                Value::String(selected_provider.clone()),
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
            "type".to_string(),
            Value::String("web_conduit_fetch".to_string()),
        );
        out.insert(
            "requested_url".to_string(),
            Value::String(raw_requested_url.clone()),
        );
        out.insert("resolved_url".to_string(), Value::String(resolved_url.clone()));
        out.insert(
            "citation_redirect_resolved".to_string(),
            Value::Bool(redirect_resolved),
        );
        out.insert(
            "provider".to_string(),
            Value::String(selected_provider.clone()),
        );
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &fetch_provider_chain),
        );
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert(
            "tool_surface_ready".to_string(),
            Value::Bool(tool_surface_ready),
        );
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_attempted".to_string(), Value::Bool(false));
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("meta_query_blocked".to_string(), Value::Bool(false));
        out.insert(
            "cache_status".to_string(),
            Value::String("skipped_validation".to_string()),
        );
        out.insert("cache_store_allowed".to_string(), Value::Bool(false));
        out.insert("cache_write_attempted".to_string(), Value::Bool(false));
        out.insert(
            "cache_skip_reason".to_string(),
            Value::String(cache_skip_reason.to_string()),
        );
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_fetch",
                "preflight_blocked",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(fetch_provider_chain.clone()),
                &selected_provider,
                Some(preflight_error),
            ),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Web fetch execution was blocked by runtime tooling gate before provider calls were attempted."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }
    if attempt_replay_blocked {
        let mut receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            "fetch_replay_guard_blocked",
            Some("web_fetch_duplicate_attempt_suppressed"),
        );
        if let Some(receipt_obj) = receipt.as_object_mut() {
            receipt_obj.insert(
                "attempt_signature".to_string(),
                Value::String(fetch_attempt_signature.clone()),
            );
            receipt_obj.insert(
                "provider".to_string(),
                Value::String(selected_provider.clone()),
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
        out.insert(
            "error".to_string(),
            Value::String("web_fetch_duplicate_attempt_suppressed".to_string()),
        );
        out.insert(
            "type".to_string(),
            Value::String("web_conduit_fetch".to_string()),
        );
        out.insert(
            "requested_url".to_string(),
            Value::String(raw_requested_url.clone()),
        );
        out.insert("resolved_url".to_string(), Value::String(resolved_url.clone()));
        out.insert(
            "citation_redirect_resolved".to_string(),
            Value::Bool(redirect_resolved),
        );
        out.insert(
            "provider".to_string(),
            Value::String(selected_provider.clone()),
        );
        out.insert(
            "provider_hint".to_string(),
            Value::String(provider_hint.clone()),
        );
        out.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
        out.insert("provider_resolution".to_string(), provider_resolution);
        out.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &fetch_provider_chain),
        );
        out.insert(
            "tool_surface_status".to_string(),
            Value::String(tool_surface_status.clone()),
        );
        out.insert(
            "tool_surface_ready".to_string(),
            Value::Bool(tool_surface_ready),
        );
        out.insert(
            "tool_surface_blocking_reason".to_string(),
            Value::String(tool_surface_blocking_reason.clone()),
        );
        out.insert("tool_execution_attempted".to_string(), Value::Bool(false));
        out.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
        out.insert("meta_query_blocked".to_string(), Value::Bool(false));
        out.insert(
            "cache_status".to_string(),
            Value::String("skipped_validation".to_string()),
        );
        out.insert("cache_store_allowed".to_string(), Value::Bool(false));
        out.insert("cache_write_attempted".to_string(), Value::Bool(false));
        out.insert(
            "cache_skip_reason".to_string(),
            Value::String("replay_suppressed".to_string()),
        );
        out.insert("replay_policy".to_string(), replay_policy.clone());
        out.insert("replay_bypass".to_string(), replay_bypass.clone());
        out.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
        out.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        out.insert(
            "retry".to_string(),
            json!({
                "recommended": true,
                "strategy": "change_query_or_provider",
                "lane": replay_retry_lane,
                "retry_after_seconds": replay_retry_after_seconds
            }),
        );
        out.insert(
            "process_summary".to_string(),
            runtime_web_process_summary(
                "web_fetch",
                "replay_suppressed",
                false,
                &tool_execution_gate,
                &attempt_replay_guard,
                &json!(fetch_provider_chain.clone()),
                &selected_provider,
                Some("web_fetch_duplicate_attempt_suppressed"),
            ),
        );
        out.insert(
            "summary".to_string(),
            Value::String(
                "Repeated identical web fetch attempts were suppressed by replay guard. Adjust URL or request parameters before retrying."
                    .to_string(),
            ),
        );
        out.insert("content".to_string(), Value::String(String::new()));
        out.insert("receipt".to_string(), receipt);
        return Value::Object(out);
    }

    let fetched = fetch_with_curl_retry(
        &resolved_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        allow_rfc2544_benchmark_range,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = normalize_fetch_content_type(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let fetched_body = fetched.get("body").and_then(Value::as_str).unwrap_or("");
    let content_is_textual = content_type_is_textual(&content_type);
    let (raw_content, raw_title, content_truncated, extractor) = if content_is_textual {
        extract_fetch_content_with_extractor(fetched_body, &content_type, &extract_mode, max_chars)
    } else {
        (String::new(), None, false, "binary".to_string())
    };
    let (content, wrapped_truncated, raw_length, wrapped_length) = if content_is_textual {
        wrap_web_fetch_content(&raw_content, max_chars)
    } else {
        (String::new(), false, 0, 0)
    };
    let title = wrap_web_fetch_field(raw_title.as_deref());
    let final_url = clean_text(
        fetched
            .get("effective_url")
            .and_then(Value::as_str)
            .unwrap_or(resolved_url.as_str()),
        2200,
    );
    let summary_body = extract_fetch_summary_body(&raw_content, &extract_mode);
    let summary = if content_is_textual {
        summarize_text(&summary_body, 900)
    } else if resolved_url.is_empty() {
        format!(
            "Fetched non-text content ({}).",
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    } else {
        format!(
            "Fetched non-text content from {} ({}).",
            clean_text(&final_url, 220),
            if content_type.is_empty() {
                "binary/unknown"
            } else {
                content_type.as_str()
            }
        )
    };
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = request
        .get("materialize_artifact")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let artifact = if materialize_artifact {
        persist_artifact(root, &resolved_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && if content_is_textual {
            !content.is_empty()
        } else {
            status_code >= 200 && status_code < 400
        };
    let error_value = fetched
        .get("stderr")
        .and_then(Value::as_str)
        .map(|v| clean_text(v, 320))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            if status_code >= 400 {
                let detail = format_web_fetch_error_detail(fetched_body, &content_type, 4000);
                if detail.is_empty() {
                    String::new()
                } else {
                    wrap_web_fetch_content(&detail, 4000).0
                }
            } else {
                String::new()
            }
        });
    let receipt_reason = if matches!(
        error_value.as_str(),
        "invalid_fetch_url"
            | "blocked_hostname"
            | "blocked_private_network_target"
            | "blocked_private_network_redirect"
            | "invalid_redirect_target"
    ) {
        error_value.as_str()
    } else {
        reason.as_str()
    };
    let receipt = build_receipt(
        &raw_requested_url,
        &decision,
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        receipt_reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let mut receipt = receipt;
    if let Some(receipt_obj) = receipt.as_object_mut() {
        receipt_obj.insert(
            "attempt_signature".to_string(),
            Value::String(fetch_attempt_signature.clone()),
        );
        receipt_obj.insert(
            "provider".to_string(),
            Value::String(selected_provider.clone()),
        );
    }
    let _ = append_jsonl(&receipts_path(root), &receipt);
    let epistemic_object = json!({
        "kind": "web_document",
        "trusted": false,
        "provenance": {
            "source": "web_conduit",
            "requested_url": raw_requested_url,
            "resolved_url": resolved_url,
            "final_url": final_url,
            "response_hash": response_hash,
            "artifact_id": artifact
                .as_ref()
                .and_then(|row| row.get("artifact_id"))
                .cloned()
                .unwrap_or(Value::Null),
            "artifact_path": artifact
                .as_ref()
                .and_then(|row| row.get("path"))
                .cloned()
                .unwrap_or(Value::Null),
            "receipt_hash": receipt.get("receipt_hash").cloned().unwrap_or(Value::Null)
        },
        "verity": {
            "validated": false,
            "checks": [
                "policy_gate_passed",
                "content_hash_recorded",
                "source_marked_untrusted_until_verified"
            ]
        }
    });
    let fetch_error = if fetch_ok {
        Value::Null
    } else if error_value.is_empty() {
        if tool_surface_status == "unavailable" {
            json!("web_fetch_tool_surface_unavailable")
        } else if tool_surface_status == "degraded" && !tool_surface_ready {
            json!("web_fetch_tool_surface_degraded")
        } else {
            json!("web_conduit_fetch_failed")
        }
    } else {
        json!(error_value)
    };
    let fetch_process_summary = runtime_web_process_summary(
        "web_fetch",
        "provider_fetch_result",
        true,
        &tool_execution_gate,
        &attempt_replay_guard,
        &json!(fetch_provider_chain.clone()),
        &selected_provider,
        fetch_error.as_str(),
    );
    let fetched_ssrf_guard = fetched.get("ssrf_guard").cloned().unwrap_or(ssrf_guard);
    let content_type_value = if content_type.is_empty() {
        Value::String(String::new())
    } else {
        Value::String(content_type)
    };
    let content_value = if summary_only {
        Value::String(String::new())
    } else {
        Value::String(content.clone())
    };
    let external_content = json!({
        "untrusted": true,
        "source": "web_fetch",
        "wrapped": content_is_textual,
        "provider": selected_provider.clone(),
        "provider_chain": fetch_provider_chain.clone(),
        "tool_surface_status": tool_surface_status.clone()
    });
    let mut out_obj = serde_json::Map::<String, Value>::new();
    out_obj.insert("ok".to_string(), Value::Bool(fetch_ok));
    out_obj.insert(
        "type".to_string(),
        Value::String("web_conduit_fetch".to_string()),
    );
    out_obj.insert(
        "requested_url".to_string(),
        Value::String(raw_requested_url.clone()),
    );
    out_obj.insert(
        "resolved_url".to_string(),
        Value::String(resolved_url.clone()),
    );
    out_obj.insert("final_url".to_string(), Value::String(final_url.clone()));
    out_obj.insert(
        "citation_redirect_resolved".to_string(),
        Value::Bool(redirect_resolved),
    );
    out_obj.insert(
        "provider".to_string(),
        Value::String(selected_provider.clone()),
    );
    out_obj.insert(
        "provider_hint".to_string(),
        Value::String(provider_hint.clone()),
    );
    out_obj.insert("provider_chain".to_string(), json!(fetch_provider_chain.clone()));
    out_obj.insert("provider_resolution".to_string(), provider_resolution.clone());
    out_obj.insert(
        "tool_surface_status".to_string(),
        Value::String(tool_surface_status.clone()),
    );
    out_obj.insert(
        "tool_surface_ready".to_string(),
        Value::Bool(tool_surface_ready),
    );
    out_obj.insert(
        "tool_surface_blocking_reason".to_string(),
        Value::String(tool_surface_blocking_reason.clone()),
    );
    out_obj.insert("tool_execution_gate".to_string(), tool_execution_gate.clone());
    out_obj.insert("replay_policy".to_string(), replay_policy.clone());
    out_obj.insert("replay_bypass".to_string(), replay_bypass.clone());
    out_obj.insert("attempt_replay_guard".to_string(), attempt_replay_guard.clone());
    out_obj.insert(
        "attempt_signature".to_string(),
        Value::String(fetch_attempt_signature.clone()),
    );
    out_obj.insert("extractor".to_string(), Value::String(extractor.clone()));
    out_obj.insert("status_code".to_string(), json!(status_code));
    out_obj.insert("content_type".to_string(), content_type_value);
    out_obj.insert("extract_mode".to_string(), Value::String(extract_mode.clone()));
    out_obj.insert(
        "title".to_string(),
        Value::String(title.unwrap_or_default()),
    );
    out_obj.insert("summary".to_string(), Value::String(summary.clone()));
    let mut out = Value::Object(out_obj);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("content".to_string(), content_value);
        obj.insert(
            "content_truncated".to_string(),
            Value::Bool(content_truncated || wrapped_truncated),
        );
        obj.insert("raw_length".to_string(), json!(raw_length));
        obj.insert("wrapped_length".to_string(), json!(wrapped_length));
        obj.insert("external_content".to_string(), external_content);
        obj.insert("process_summary".to_string(), fetch_process_summary);
        obj.insert("cache_status".to_string(), json!("miss"));
        obj.insert(
            "retry_attempts".to_string(),
            fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        );
        obj.insert(
            "retry_used".to_string(),
            fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        );
        obj.insert(
            "redirect_count".to_string(),
            fetched.get("redirect_count").cloned().unwrap_or_else(|| json!(0)),
        );
        obj.insert(
            "user_agent".to_string(),
            fetched
                .get("user_agent")
                .cloned()
                .unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        );
        obj.insert(
            "accept_header".to_string(),
            fetched
                .get("accept_header")
                .cloned()
                .unwrap_or_else(|| json!(FETCH_MARKDOWN_ACCEPT_HEADER)),
        );
        obj.insert(
            "x_markdown_tokens".to_string(),
            fetched.get("x_markdown_tokens").cloned().unwrap_or(Value::Null),
        );
        obj.insert("response_hash".to_string(), json!(response_hash));
        obj.insert("artifact".to_string(), artifact.unwrap_or(Value::Null));
        obj.insert("policy_decision".to_string(), policy_eval);
        obj.insert("ssrf_guard".to_string(), fetched_ssrf_guard);
        obj.insert("receipt".to_string(), receipt);
        obj.insert("epistemic_object".to_string(), epistemic_object);
        obj.insert("error".to_string(), fetch_error);
    }
    if !fetch_ok {
        if let Some(obj) = out.as_object_mut() {
            let current_error = obj.get("error").and_then(Value::as_str).unwrap_or("");
            if current_error == "web_fetch_tool_surface_unavailable" {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Web fetch tool surface is currently unavailable. Retry after provider runtime is restored."
                            .to_string(),
                    ),
                );
            } else if current_error == "web_fetch_tool_surface_degraded" {
                obj.insert(
                    "summary".to_string(),
                    Value::String(
                        "Web fetch tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                            .to_string(),
                    ),
                );
            }
        }
    }
    let cache_status = if fetch_ok { "ok" } else { "error" };
    if cache_ttl_minutes > 0 {
        if let Some(obj) = out.as_object_mut() {
            obj.insert("cache_ttl_minutes".to_string(), json!(cache_ttl_minutes));
        }
        store_fetch_cache(
            root,
            &fetch_cache_key,
            &out,
            cache_status,
            cache_ttl_minutes,
        );
    }
    out
}
