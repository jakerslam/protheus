    let (requested_url_input, requested_url_source) = fetch_url_source_and_input(request);
    let requested_url_source_kind = fetch_url_source_kind(requested_url_source);
    let requested_url_source_confidence = fetch_url_source_confidence(requested_url_source_kind);
    let requested_url_source_recovery_mode =
        fetch_url_source_recovery_mode(requested_url_source);
    let requested_url_source_lineage = fetch_url_source_lineage(
        requested_url_source,
        requested_url_source_kind,
        requested_url_source_confidence,
    );
    let raw_requested_url = normalize_fetch_requested_url_input(&requested_url_input);
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
    let fetch_url_override_source = fetch_url_shape_override_source(&policy, request);
    let fetch_url_override_used = fetch_url_shape_override(&policy, request);
    let fetch_url_shape_error = fetch_url_shape_error_code(&raw_requested_url);
    if fetch_url_shape_error != "none" && !fetch_url_override_used {
        let receipt = build_receipt(
            &raw_requested_url,
            "deny",
            None,
            0,
            fetch_url_shape_error,
            Some(fetch_url_shape_error),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let summary = if fetch_url_shape_error == "fetch_url_payload_dump_detected" {
            Some("Requested URL appears to be pasted output/log content instead of a valid URL.")
        } else if fetch_url_shape_error == "fetch_url_invalid_scheme" {
            Some("Requested URL must start with http:// or https://.")
        } else if fetch_url_shape_error == "fetch_url_required" {
            Some("Requested URL is required for web fetch.")
        } else {
            Some("Requested URL shape is invalid for web fetch.")
        };
        let mut out = fetch_early_validation_payload(
            fetch_url_shape_error,
            &raw_requested_url,
            &provider_hint,
            "skipped_validation",
            fetch_url_shape_error,
            fetch_url_shape_error,
            summary,
            Some("submit a concise http(s) URL; set force_fetch_url_shape_override=true only for controlled diagnostics"),
            None,
            None,
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
            obj.insert("fetch_url_shape_blocked".to_string(), json!(true));
            obj.insert("fetch_url_shape_error".to_string(), json!(fetch_url_shape_error));
            obj.insert(
                "fetch_url_shape_stats".to_string(),
                fetch_url_shape_stats(&raw_requested_url),
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
            obj.insert("fetch_url_shape_override_allowed".to_string(), json!(false));
            obj.insert("fetch_url_shape_override_used".to_string(), json!(false));
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
                    false,
                    fetch_url_override_source,
                ),
            );
            obj.insert(
                "retry".to_string(),
                json!({
                    "recommended": true,
                    "strategy": "provide_valid_http_or_https_url",
                    "lane": "web_fetch",
                    "retry_after_seconds": 0
                }),
            );
        }
        return out;
    }
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
        let mut out = fetch_early_validation_payload(
            "non_fetch_meta_query",
            &raw_requested_url,
            &provider_hint,
            "blocked_meta_query",
            "meta_query_blocked",
            "meta_query_blocked",
            Some("Requested fetch URL appears to be conversational/tooling diagnostics rather than a valid web URL. Answer directly without running web fetch. To force fetch evaluation, set force_web_fetch=true or force_web_search=true."),
            Some("force_web_fetch=true|force_web_search=true"),
            None,
            None,
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
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
                "retry".to_string(),
                json!({
                    "recommended": true,
                    "strategy": "answer_directly_or_set_force_web_fetch",
                    "lane": "web_fetch",
                    "retry_after_seconds": 0
                }),
            );
        }
        return out;
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
        let mut out = fetch_early_validation_payload(
            "unknown_fetch_provider",
            &raw_requested_url,
            &provider_hint,
            "skipped_validation",
            "unknown_fetch_provider",
            "request_contract_blocked",
            None,
            None,
            Some(&unknown_provider),
            Some(fetch_provider_catalog_snapshot(root, &policy)),
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
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
                "retry".to_string(),
                json!({
                    "recommended": true,
                    "strategy": "use_supported_provider_or_auto",
                    "lane": "web_fetch",
                    "retry_after_seconds": 0
                }),
            );
        }
        return out;
    }
    let (provider_resolution, fetch_provider_chain, selected_provider) =
        resolved_fetch_provider_selection(root, &policy, request, &provider_hint);
    let tool_surface_status = provider_resolution
        .get("tool_surface_status")
        .or_else(|| provider_resolution.pointer("/tool_surface_health/status"))
        .and_then(Value::as_str)
        .unwrap_or("unavailable")
        .to_string();
