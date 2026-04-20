    let (query_raw, query_source) = search_query_and_source(request);
    let query = search_strip_invisible_unicode(&clean_text(&query_raw, 600));
    let _query_invisible_unicode_removed_count =
        search_invisible_unicode_removed_count(&query_raw);
    let _query_invisible_unicode_stripped = _query_invisible_unicode_removed_count > 0;
    let query_source_kind = search_query_source_kind(query_source);
    let query_source_confidence = search_query_source_confidence(query_source_kind);
    let query_source_recovery_mode = search_query_source_recovery_mode(query_source);
    let query_source_lineage =
        search_query_source_lineage(query_source, query_source_kind, query_source_confidence);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("search_provider"))
            .or_else(|| request.get("searchProvider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    let (policy, _policy_path_value) = load_policy(root);
    let query_shape_override = search_query_shape_override(&policy, request);
    let query_shape_override_source = search_query_shape_override_source(&policy, request);
    let query_shape_error = search_query_shape_error_code(&query);
    let query_shape_fetch_url_candidate = search_query_fetch_url_candidate(&query).unwrap_or_default();
    let query_shape_fetch_url_candidate_kind = search_query_fetch_url_candidate_kind(&query);
    if query_shape_error != "none" && !query_shape_override {
        let reason = query_shape_error;
        let receipt = build_receipt("", "deny", None, 0, reason, Some(reason));
        let _ = append_jsonl(&receipts_path(root), &receipt);
        let summary = if reason == "query_payload_dump_detected" {
            "Query looks like pasted output/log content instead of a concise web request. Submit a short intent-focused query."
        } else if reason == "query_prefers_fetch_url" {
            "Query is a direct URL. Use web fetch for this input instead of web search."
        } else {
            "Query shape is invalid for web search. Submit concise query text with clear keywords."
        };
        let mut out = search_early_validation_payload(
            reason,
            &query,
            Some(summary),
            &provider_hint,
            "skipped_validation",
            reason,
            reason,
            false,
            Some("submit concise query text (recommended <= 300 chars)"),
            receipt,
        );
        if let Some(obj) = out.as_object_mut() {
            obj.insert("query_shape_blocked".to_string(), json!(true));
            obj.insert("query_shape_error".to_string(), json!(reason));
            obj.insert("query_shape_stats".to_string(), search_query_shape_stats(&query));
            obj.insert(
                "query_shape_fetch_url_candidate".to_string(),
                json!(query_shape_fetch_url_candidate.clone()),
            );
            obj.insert(
                "query_shape_fetch_url_candidate_kind".to_string(),
                json!(query_shape_fetch_url_candidate_kind),
            );
            obj.insert("query_shape_override_allowed".to_string(), json!(false));
            obj.insert("query_shape_override_used".to_string(), json!(false));
            obj.insert(
                "query_shape_override_source".to_string(),
                json!(query_shape_override_source),
            );
            obj.insert(
                "query_shape_category".to_string(),
                json!(search_query_shape_category(reason)),
            );
            obj.insert(
                "query_shape_recommended_action".to_string(),
                json!(search_query_shape_recommended_action(reason)),
            );
            obj.insert(
                "query_shape_route_hint".to_string(),
                json!(search_query_shape_route_hint(reason)),
            );
            obj.insert(
                "query_shape".to_string(),
                search_query_shape_contract(&query, reason, false, query_shape_override_source),
            );
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
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
            obj.insert(
                "suggested_next_action".to_string(),
                search_query_shape_suggested_next_action(&query, reason),
            );
            obj.insert(
                "retry".to_string(),
                search_retry_envelope_for_error(reason),
            );
        }
        return out;
    }
    if let Some(mut early) = search_early_validation_response(root, request, &query) {
        let early_error = clean_text(
            early.get("error").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if let Some(obj) = early.as_object_mut() {
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
            obj.insert("query_source_lineage".to_string(), query_source_lineage.clone());
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
                "retry".to_string(),
                search_retry_envelope_for_error(&early_error),
            );
        }
        return early;
    }
    let normalized_filters = normalized_search_filters(request);
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let raw_freshness = clean_text(
        request
            .get("freshness")
            .or_else(|| request.get("search_recency_filter"))
            .or_else(|| request.get("searchRecencyFilter"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        60,
    );
    let raw_date_after = clean_text(
        request
            .get("date_after")
            .or_else(|| request.get("dateAfter"))
            .or_else(|| request.get("search_after_date"))
            .or_else(|| request.get("searchAfterDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    let raw_date_before = clean_text(
        request
            .get("date_before")
            .or_else(|| request.get("dateBefore"))
            .or_else(|| request.get("search_before_date"))
            .or_else(|| request.get("searchBeforeDate"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    );
    if !raw_freshness.is_empty() && (!raw_date_after.is_empty() || !raw_date_before.is_empty()) {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "conflicting_time_filters",
            Some("conflicting_time_filters"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "conflicting_time_filters",
            "query": query,
            "query_source": query_source,
            "query_source_kind": query_source_kind,
            "query_source_confidence": query_source_confidence,
            "query_source_recovery_mode": query_source_recovery_mode,
            "query_source_lineage": query_source_lineage,
            "query_shape_fetch_url_candidate": query_shape_fetch_url_candidate,
            "query_shape_fetch_url_candidate_kind": query_shape_fetch_url_candidate_kind,
            "query_shape_error": query_shape_error,
            "query_shape_category": search_query_shape_category(query_shape_error),
            "query_shape_recommended_action": search_query_shape_recommended_action(query_shape_error),
            "query_shape_route_hint": search_query_shape_route_hint(query_shape_error),
            "query_shape": search_query_shape_contract(
                &query,
                query_shape_error,
                query_shape_override,
                query_shape_override_source
            ),
            "suggested_next_action": search_query_shape_suggested_next_action(&query, query_shape_error),
            "freshness": raw_freshness,
            "date_after": raw_date_after,
            "date_before": raw_date_before,
            "summary": "freshness cannot be combined with date_after/date_before. Use either freshness or an explicit date range.",
            "filters": normalized_filters.clone(),
            "provider_hint": provider_hint.clone(),
            "tool_execution_attempted": false,
            "tool_execution_gate": {
                "should_execute": false,
                "mode": "blocked",
                "reason": "conflicting_time_filters",
                "source": "request_contract"
            },
            "meta_query_blocked": false,
            "cache_status": "skipped_validation",
            "cache_store_allowed": false,
            "cache_write_attempted": false,
            "cache_skip_reason": "conflicting_time_filters",
            "retry": search_retry_envelope_for_error("conflicting_time_filters"),
            "provider_catalog": provider_catalog_snapshot(root, &policy),
            "process_summary": runtime_web_process_summary(
                "web_search",
                "request_contract_blocked",
                false,
                &json!({
                    "should_execute": false,
                    "mode": "blocked",
                    "reason": "conflicting_time_filters",
                    "source": "request_contract"
                }),
                &json!({
                    "blocked": false,
                    "reason": "not_evaluated"
                }),
                &json!([]),
                "none",
                Some("conflicting_time_filters")
            ),
            "receipt": receipt
        });
    }
