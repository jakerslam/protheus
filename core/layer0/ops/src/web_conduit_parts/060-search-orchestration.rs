pub fn api_search(root: &Path, request: &Value) -> Value {
    let query = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    if query.is_empty() {
        let receipt = build_receipt(
            "",
            "deny",
            None,
            0,
            "query_required",
            Some("query_required"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "query_required",
            "query": "",
            "receipt": receipt
        });
    }
    let (policy, _policy_path_value) = load_policy(root);
    let allowed_domains =
        normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
    let exclude_subdomains = request
        .get("exclude_subdomains")
        .or_else(|| request.get("exact_domain_only"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let provider_hint = clean_text(
        request
            .get("provider")
            .or_else(|| request.get("source"))
            .or_else(|| request.get("search_provider"))
            .and_then(Value::as_str)
            .unwrap_or("auto"),
        40,
    )
    .to_ascii_lowercase();
    let top_k = request
        .get("top_k")
        .or_else(|| request.get("max_results"))
        .or_else(|| request.get("num"))
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 12) as usize;
    let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
    let summary_only = request
        .get("summary_only")
        .or_else(|| request.get("summary"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let human_approved = request
        .get("human_approved")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let approval_id = request
        .get("approval_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let provider_chain = provider_chain_from_request(&provider_hint, request, &policy);
    let cache_key = search_cache_key(
        &query,
        &scoped_query,
        &allowed_domains,
        exclude_subdomains,
        top_k,
        summary_only,
        &provider_chain,
    );
    if let Some(mut cached) = load_search_cache(root, &cache_key) {
        if let Some(obj) = cached.as_object_mut() {
            obj.insert(
                "type".to_string(),
                Value::String("web_conduit_search".to_string()),
            );
            obj.insert("query".to_string(), Value::String(query.clone()));
            obj.insert(
                "effective_query".to_string(),
                Value::String(scoped_query.clone()),
            );
            obj.insert("allowed_domains".to_string(), json!(allowed_domains));
            obj.insert("exclude_subdomains".to_string(), json!(exclude_subdomains));
            obj.insert("top_k".to_string(), json!(top_k));
            obj.insert(
                "provider_hint".to_string(),
                Value::String(provider_hint.clone()),
            );
            obj.insert("provider_chain".to_string(), json!(provider_chain));
            obj.insert("cache_status".to_string(), json!("hit"));
        }
        return cached;
    }
    let primary_url = web_search_url(&scoped_query);
    let lite_url = web_search_lite_url(&scoped_query);
    let mut selected_provider = String::new();
    let mut selected = Value::Null;
    let mut attempted = Vec::<String>::new();
    let mut skipped = Vec::<Value>::new();
    let mut provider_errors = Vec::<Value>::new();
    let mut last_payload = None::<Value>;

    for provider in &provider_chain {
        if let Some(open_until) = provider_circuit_open_until(root, provider, &policy) {
            skipped.push(json!({
                "provider": provider,
                "reason": "circuit_open",
                "open_until": open_until
            }));
            continue;
        }
        attempted.push(provider.clone());
        let candidate = match provider.as_str() {
            "serperdev" => api_search_serper(
                root,
                &scoped_query,
                summary_only,
                human_approved,
                &allowed_domains,
                exclude_subdomains,
                top_k,
            ),
            "duckduckgo_lite" => api_fetch(
                root,
                &json!({
                    "url": lite_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id
                }),
            ),
            "bing_rss" => api_search_bing_rss(
                &scoped_query,
                summary_only,
                &allowed_domains,
                exclude_subdomains,
                top_k,
            ),
            _ => api_fetch(
                root,
                &json!({
                    "url": primary_url,
                    "summary_only": summary_only,
                    "human_approved": human_approved,
                    "approval_id": approval_id
                }),
            ),
        };
        if search_payload_usable(&candidate) {
            record_provider_attempt(root, provider, true, "", &policy);
            selected_provider = provider.clone();
            selected = candidate;
            break;
        }
        let reason = search_payload_error(&candidate);
        record_provider_attempt(root, provider, false, &reason, &policy);
        provider_errors.push(json!({
            "provider": provider,
            "error": reason,
            "challenge": payload_looks_like_search_challenge(&candidate),
            "low_signal": payload_looks_low_signal_search(&candidate),
            "status_code": candidate.get("status_code").and_then(Value::as_i64).unwrap_or(0)
        }));
        last_payload = Some(candidate);
    }

    let mut out = if !selected.is_null() {
        selected
    } else {
        last_payload.unwrap_or_else(|| {
            json!({
                "ok": false,
                "error": "search_providers_exhausted",
                "summary": "Search providers returned no usable findings. Retry with narrower query or explicit source URLs.",
                "content": ""
            })
        })
    };
    if out
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty()
    {
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "provider".to_string(),
                if selected_provider.is_empty() {
                    Value::String("none".to_string())
                } else {
                    Value::String(selected_provider.clone())
                },
            );
        }
    }
    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        if let Some(obj) = out.as_object_mut() {
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
    let used_lite_fallback = selected_provider == "duckduckgo_lite";
    let used_bing_fallback = selected_provider == "bing_rss";
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "type".to_string(),
            Value::String("web_conduit_search".to_string()),
        );
        obj.insert("query".to_string(), Value::String(query.clone()));
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
        obj.insert("provider_chain".to_string(), json!(provider_chain.clone()));
        obj.insert("providers_attempted".to_string(), json!(attempted));
        obj.insert("providers_skipped".to_string(), json!(skipped));
        obj.insert("provider_errors".to_string(), json!(provider_errors));
        obj.insert(
            "provider_health".to_string(),
            provider_health_snapshot(root, &provider_chain),
        );
        obj.insert(
            "search_lite_fallback".to_string(),
            json!(used_lite_fallback),
        );
        obj.insert(
            "search_bing_fallback".to_string(),
            json!(used_bing_fallback),
        );
        obj.insert("provider_hint".to_string(), Value::String(provider_hint));
        obj.insert("cache_status".to_string(), json!("miss"));
    }
    let cache_status = if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        "ok"
    } else {
        "no_results"
    };
    let search_url = match selected_provider.as_str() {
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
    let _ = append_jsonl(&receipts_path(root), &receipt);
    if let Some(obj) = out.as_object_mut() {
        obj.insert("receipt".to_string(), receipt);
    }
    store_search_cache(root, &cache_key, &out, cache_status);
    out
}

