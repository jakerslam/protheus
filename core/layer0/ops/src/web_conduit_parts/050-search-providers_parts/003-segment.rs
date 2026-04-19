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
    let early_gate = json!({
        "should_execute": false,
        "mode": "blocked",
        "reason": validation_route,
        "source": "early_validation"
    });
    let early_replay_guard = json!({
        "blocked": false,
        "reason": "not_evaluated"
    });
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
        "process_summary": runtime_web_process_summary(
            "web_search",
            validation_route,
            false,
            &early_gate,
            &early_replay_guard,
            &json!([]),
            "none",
            Some(error)
        ),
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
