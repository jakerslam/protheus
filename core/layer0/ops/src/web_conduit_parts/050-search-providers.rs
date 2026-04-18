fn api_search_serper(
    root: &Path,
    query: &str,
    summary_only: bool,
    human_approved: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    requested_timeout_ms: u64,
) -> Value {
    let requested_url = SERPER_SEARCH_URL.to_string();
    let (policy, _policy_path_value) = load_policy(root);
    let Some(api_key) = resolve_search_provider_credential(&policy, "serperdev") else {
        return json!({
            "ok": false,
            "error": "serper_api_key_missing",
            "requested_url": requested_url,
            "provider": "serperdev"
        });
    };
    let policy_eval = infring_layer1_security::evaluate_web_conduit_policy(
        root,
        &json!({
            "requested_url": requested_url,
            "domain": extract_domain(&requested_url),
            "human_approved": human_approved,
            "requests_last_minute": requests_last_minute(root)
        }),
        &policy,
    );
    let allow = policy_eval
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason = clean_text(
        policy_eval
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("policy_denied"),
        180,
    );
    if !allow {
        let receipt = build_receipt(
            &requested_url,
            "deny",
            None,
            0,
            &reason,
            Some("policy_denied"),
        );
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return json!({
            "ok": false,
            "error": "web_conduit_policy_denied",
            "requested_url": requested_url,
            "policy_decision": policy_eval,
            "provider": "serperdev",
            "receipt": receipt
        });
    }
    let timeout_ms = requested_timeout_ms.clamp(
        1_000,
        policy_eval
            .pointer("/policy/timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(search_default_timeout_ms(&policy)),
    );
    let max_response_bytes = policy_eval
        .pointer("/policy/max_response_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(350_000) as usize;
    let retry_attempts = policy_eval
        .pointer("/policy/retry_attempts")
        .and_then(Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let fetched = fetch_serper_with_retry(
        &api_key,
        query,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        top_k,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_serper_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let response_hash = if content.is_empty() {
        String::new()
    } else {
        sha256_hex(&content)
    };
    let materialize_artifact = true;
    let artifact = if materialize_artifact {
        persist_artifact(root, &requested_url, &response_hash, &content)
    } else {
        None
    };
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    let receipt = build_receipt(
        &requested_url,
        "allow",
        if response_hash.is_empty() {
            None
        } else {
            Some(response_hash.as_str())
        },
        status_code,
        &reason,
        if error_value.is_empty() {
            None
        } else {
            Some(error_value.as_str())
        },
    );
    let _ = append_jsonl(&receipts_path(root), &receipt);
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/json".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "response_hash": response_hash,
        "artifact": artifact.clone().unwrap_or(Value::Null),
        "policy_decision": policy_eval,
        "receipt": receipt,
        "provider": "serperdev",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("serper_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn api_search_bing_rss(
    query: &str,
    summary_only: bool,
    allowed_domains: &[String],
    exclude_subdomains: bool,
    top_k: usize,
    timeout_ms: u64,
) -> Value {
    let requested_url = web_search_bing_rss_url(query);
    let max_response_bytes = 280_000usize;
    let retry_attempts = 2usize;
    let fetched = fetch_with_curl_retry(
        &requested_url,
        timeout_ms,
        max_response_bytes,
        retry_attempts,
        false,
    );
    let status_code = fetched
        .get("status_code")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let content_type = clean_text(
        fetched
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or(""),
        180,
    );
    let parsed = render_bing_rss_payload(
        fetched.get("body").and_then(Value::as_str).unwrap_or(""),
        allowed_domains,
        exclude_subdomains,
        top_k,
        max_response_bytes,
    );
    let content = clean_text(
        parsed.get("content").and_then(Value::as_str).unwrap_or(""),
        max_response_bytes,
    );
    let summary = clean_text(
        parsed.get("summary").and_then(Value::as_str).unwrap_or(""),
        900,
    );
    let fetch_ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && parsed.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && !summary.is_empty();
    let mut error_value = clean_text(
        fetched.get("stderr").and_then(Value::as_str).unwrap_or(""),
        320,
    );
    if error_value.is_empty() {
        error_value = clean_text(
            parsed.get("error").and_then(Value::as_str).unwrap_or(""),
            220,
        );
    }
    json!({
        "ok": fetch_ok,
        "requested_url": requested_url,
        "status_code": status_code,
        "content_type": if content_type.is_empty() { Value::String("application/rss+xml".to_string()) } else { Value::String(content_type) },
        "summary": summary,
        "content": if summary_only { Value::String(String::new()) } else { Value::String(content) },
        "links": parsed.get("links").cloned().unwrap_or_else(|| json!([])),
        "content_domains": parsed.get("content_domains").cloned().unwrap_or_else(|| json!([])),
        "provider_raw_count": parsed.get("provider_raw_count").cloned().unwrap_or_else(|| json!(0)),
        "provider_filtered_count": parsed.get("provider_filtered_count").cloned().unwrap_or_else(|| json!(0)),
        "retry_attempts": fetched.get("retry_attempts").cloned().unwrap_or_else(|| json!(1)),
        "retry_used": fetched.get("retry_used").cloned().unwrap_or_else(|| json!(false)),
        "user_agent": fetched.get("user_agent").cloned().unwrap_or_else(|| json!(DEFAULT_WEB_USER_AGENTS[0])),
        "provider": "bing_rss",
        "error": if fetch_ok {
            Value::Null
        } else if error_value.is_empty() {
            Value::String("bing_rss_search_failed".to_string())
        } else {
            Value::String(error_value)
        }
    })
}

fn search_payload_usable(payload: &Value) -> bool {
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    if payload_looks_like_search_challenge(payload) || payload_looks_low_signal_search(payload) {
        return false;
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if summary.is_empty() {
        return false;
    }
    !search_summary_has_low_signal_marker(&summary)
}

fn search_query_is_meta_diagnostic(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let explicit_search_intent = ["search for ", "search the web", "web search", "find information", "finding information", "look up", "compare ", "official docs", "research online", "research on web"]
        .iter()
        .any(|marker| lowered.contains(*marker));
    if explicit_search_intent {
        return false;
    }
    if lowered.contains("did you do the web request")
        || lowered.contains("did you try it")
        || lowered.contains("why did my last prompt")
        || lowered.contains("you returned no result")
        || lowered.contains("that was just a test")
        || lowered.contains("that was a test")
        || lowered.contains("where did that come from")
    {
        return true;
    }
    let meta_hits = ["what happened", "workflow", "tool call", "web tooling", "hallucination", "hallucinated", "training data", "context issue", "answer the question", "last response", "previous response"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    if meta_hits < 2 {
        return false;
    }
    let web_intent_hits = ["site:", "http://", "https://", "latest ", "top ", "best ", "news", "framework", "docs", "recipe", "weather", "price"]
    .iter()
    .filter(|marker| lowered.contains(**marker))
    .count();
    web_intent_hits == 0
}

fn search_override_flag_enabled(value: &Value) -> bool {
    value
        .as_bool()
        .or_else(|| value.as_i64().map(|n| n != 0))
        .or_else(|| {
            value.as_str().map(|raw| {
                matches!(
                    clean_text(raw, 12).to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "y" | "on"
                )
            })
        })
        .unwrap_or(false)
}

fn search_meta_query_override(request: &Value) -> bool {
    let direct_keys = [
        "allow_meta_query_search",
        "allowMetaQuerySearch",
        "force_web_search",
        "forceWebSearch",
        "force_web_lookup",
        "forceWebLookup",
    ];
    for key in direct_keys {
        if let Some(value) = request.get(key) {
            if search_override_flag_enabled(value) {
                return true;
            }
        }
    }
    let nested_keys = [
        "/search_policy/allow_meta_query_search",
        "/searchPolicy/allowMetaQuerySearch",
        "/search_policy/force_web_search",
        "/searchPolicy/forceWebSearch",
        "/search_policy/force_web_lookup",
        "/searchPolicy/forceWebLookup",
    ];
    for pointer in nested_keys {
        if let Some(value) = request.pointer(pointer) {
            if search_override_flag_enabled(value) {
                return true;
            }
        }
    }
    false
}

fn search_early_validation_payload(
    error: &str,
    query: &str,
    summary: Option<&str>,
    provider_hint: &str,
    cache_status: &str,
    cache_skip_reason: &str,
    validation_route: &str,
    meta_query_blocked: bool,
    override_hint: Option<&str>,
    receipt: Value,
) -> Value {
    let mut out = json!({
        "ok": false,
        "error": error,
        "query": clean_text(query, 600),
        "type": "web_conduit_search",
        "provider": "none",
        "provider_hint": clean_text(provider_hint, 40).to_ascii_lowercase(),
        "cache_status": cache_status,
        "cache_store_allowed": false,
        "cache_write_attempted": false,
        "cache_skip_reason": cache_skip_reason,
        "meta_query_blocked": meta_query_blocked,
        "tool_execution_attempted": false,
        "tool_execution_skipped_reason": validation_route,
        "tool_execution_gate": {
            "should_execute": false,
            "reason": validation_route,
            "source": "early_validation"
        },
        "tool_surface_status": "not_evaluated",
        "tool_surface_ready": false,
        "tool_surface_blocking_reason": "early_validation",
        "validation_route": validation_route,
        "providers_attempted": [],
        "providers_skipped": [],
        "provider_errors": [],
        "provider_chain": [],
        "provider_resolution": {
            "status": "not_evaluated",
            "reason": validation_route,
            "source": "early_validation",
            "tool_surface_health": {
                "status": "not_evaluated",
                "selected_provider_ready": false,
                "blocking_reason": "early_validation"
            }
        },
        "provider_health": {"status": "not_evaluated", "providers": []},
        "receipt": receipt
    });
    if let Some(text) = summary {
        out["summary"] = Value::String(clean_text(text, 900));
    }
    if let Some(hint) = override_hint {
        out["override_hint"] = Value::String(clean_text(hint, 120));
    }
    out
}

fn search_early_validation_response(root: &Path, request: &Value, query: &str) -> Option<Value> {
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
    if query.is_empty() {
        let receipt = build_receipt("", "deny", None, 0, "query_required", Some("query_required"));
        let _ = append_jsonl(&receipts_path(root), &receipt);
        return Some(search_early_validation_payload(
            "query_required",
            "",
            None,
            &provider_hint,
            "skipped_validation",
            "query_required",
            "query_required",
            false,
            None,
            receipt,
        ));
    }
    if search_meta_query_override(request) {
        return None;
    }
    if !search_query_is_meta_diagnostic(query) {
        return None;
    }
    let receipt = build_receipt("", "deny", None, 0, "non_search_meta_query", Some("meta_diagnostic_query"));
    let _ = append_jsonl(&receipts_path(root), &receipt);
    Some(search_early_validation_payload(
        "non_search_meta_query",
        query,
        Some("Query appears to be workflow/tooling diagnostics rather than a web information request. Answer directly without running web search. To force web lookup for this prompt, set force_web_search=true or force_web_lookup=true."),
        &provider_hint,
        "blocked_meta_query",
        "meta_query_blocked",
        "meta_query_blocked",
        true,
        Some("force_web_search=true|force_web_lookup=true"),
        receipt,
    ))
}

fn search_query_alignment_terms(query: &str) -> Vec<String> {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    let mut terms = Vec::new();
    for token in lowered.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let candidate = token.trim();
        if candidate.len() < 3 {
            continue;
        }
        if matches!(
            candidate,
            "the"
                | "and"
                | "for"
                | "with"
                | "this"
                | "that"
                | "from"
                | "into"
                | "what"
                | "when"
                | "where"
                | "why"
                | "how"
                | "about"
                | "just"
                | "again"
                | "please"
                | "best"
                | "top"
                | "give"
                | "show"
        ) {
            continue;
        }
        if !terms.iter().any(|existing| existing == candidate) {
            terms.push(candidate.to_string());
        }
        if terms.len() >= 16 {
            break;
        }
    }
    terms
}

fn search_payload_query_aligned(payload: &Value, query: &str) -> bool {
    let terms = search_query_alignment_terms(query);
    if terms.is_empty() {
        return true;
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    )
    .to_ascii_lowercase();
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        4_000,
    )
    .to_ascii_lowercase();
    let mut combined = String::with_capacity(summary.len() + content.len() + 1);
    combined.push_str(&summary);
    combined.push('\n');
    combined.push_str(&content);
    if combined.trim().is_empty() {
        return false;
    }
    let matched_terms = terms
        .iter()
        .filter(|term| combined.contains(term.as_str()))
        .count();
    let required_hits = if terms.len() == 1 {
        1
    } else {
        2.min(terms.len())
    };
    if matched_terms >= required_hits {
        return true;
    }
    let ratio = (matched_terms as f64) / (terms.len() as f64);
    let ratio_floor = if terms.len() >= 6 { 0.40 } else { 0.34 };
    ratio >= ratio_floor
}

fn search_payload_query_mismatch(payload: &Value, query: &str) -> bool {
    !search_payload_query_aligned(payload, query)
}

fn search_payload_usable_for_query(payload: &Value, query: &str) -> bool {
    search_payload_usable(payload) && search_payload_query_aligned(payload, query)
}

fn search_payload_error_for_query(payload: &Value, query: &str) -> String {
    if !search_payload_usable(payload) {
        return search_payload_error(payload);
    }
    if !search_payload_query_aligned(payload, query) {
        return "query_result_mismatch".to_string();
    }
    "search_provider_failed".to_string()
}

fn search_payload_error(payload: &Value) -> String {
    let explicit = clean_text(
        payload.get("error").and_then(Value::as_str).unwrap_or(""),
        220,
    );
    if !explicit.is_empty() {
        return explicit;
    }
    if payload_looks_like_search_challenge(payload) {
        return "anti_bot_challenge".to_string();
    }
    if payload_looks_low_signal_search(payload) {
        return "low_signal_search_payload".to_string();
    }
    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    if search_summary_has_low_signal_marker(&summary) {
        return "low_signal_search_payload".to_string();
    }
    if !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return "search_provider_failed".to_string();
    }
    "no_usable_summary".to_string()
}

fn search_summary_has_low_signal_marker(summary: &str) -> bool {
    let lowered = clean_text(summary, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    [
        "no relevant results found for that request yet",
        "couldn't produce source-backed findings in this turn",
        "don't have usable tool findings from this turn yet",
        "this turn only produced low-signal or no-result output",
        "retry with a narrower query or one specific source url",
        "search providers returned no usable findings"
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}
