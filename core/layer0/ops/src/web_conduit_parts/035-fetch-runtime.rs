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
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
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
            "requested_url": raw_requested_url,
            "requested_provider": unknown_provider,
            "fetch_provider_catalog": fetch_provider_catalog_snapshot(root, &policy),
            "receipt": receipt
        });
    }
    let (provider_resolution, fetch_provider_chain, selected_provider) =
        resolved_fetch_provider_selection(root, &policy, request, &provider_hint);
    let allow_rfc2544_benchmark_range = request
        .pointer("/ssrf_policy/allow_rfc2544_benchmark_range")
        .and_then(Value::as_bool)
        .or_else(|| {
            policy
                .pointer("/web_conduit/ssrf_policy/allow_rfc2544_benchmark_range")
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
            "requested_url": raw_requested_url,
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider,
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain,
            "provider_resolution": provider_resolution,
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
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            &reason,
            Some(if approval.is_some() {
                "approval_required"
            } else {
                "policy_denied"
            }),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": raw_requested_url,
            "resolved_url": resolved_url,
            "citation_redirect_resolved": redirect_resolved,
            "provider": selected_provider,
            "provider_hint": provider_hint,
            "provider_chain": fetch_provider_chain,
            "provider_resolution": provider_resolution,
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
        }
        return cached;
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
        json!("web_conduit_fetch_failed")
    } else {
        json!(error_value)
    };
    let fetched_ssrf_guard = fetched.get("ssrf_guard").cloned().unwrap_or(ssrf_guard);
    let mut out = json!({
        "ok": fetch_ok,
        "type": "web_conduit_fetch",
        "requested_url": raw_requested_url,
        "resolved_url": resolved_url,
        "final_url": final_url,
        "citation_redirect_resolved": redirect_resolved,
        "provider": selected_provider,
        "provider_hint": provider_hint,
        "provider_chain": fetch_provider_chain,
        "provider_resolution": provider_resolution,
        "extractor": extractor,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String(String::new()) } else { Value::String(content_type) },
        "extract_mode": extract_mode,
        "title": title.unwrap_or_default(),
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content.clone()) },
        "content_truncated": content_truncated || wrapped_truncated,
        "raw_length": raw_length,
        "wrapped_length": wrapped_length,
        "external_content": {
            "untrusted": true,
            "source": "web_fetch",
            "wrapped": content_is_textual
        },
        "cache_status": "miss",
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "redirect_count": fetched.get("redirect_count").cloned().unwrap_or_else(|| json!(0)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "accept_header": fetched.get("accept_header").cloned().unwrap_or_else(|| json!(FETCH_MARKDOWN_ACCEPT_HEADER)),
        "x_markdown_tokens": fetched.get("x_markdown_tokens").cloned().unwrap_or(Value::Null),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "ssrf_guard": fetched_ssrf_guard,
        "receipt": receipt,
        "epistemic_object": epistemic_object,
        "error": fetch_error
    });
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
