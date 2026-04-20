
pub fn api_batch_query(root: &Path, request: &Value) -> Value {
    let started = Instant::now();
    let policy = load_policy(root);
    let parallel_window = max_parallel_subqueries(&policy).max(1);
    let query_timeout = query_timeout(&policy);
    let source = normalize_source(
        request
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("web"),
    );
    let aperture = normalize_aperture(
        request
            .get("aperture")
            .and_then(Value::as_str)
            .unwrap_or("medium"),
    );
    let query = request_query_text(request, 600);
    if source == "web" && looks_like_internal_route_query(&query) {
        return json!({
            "ok": true,
            "status": "no_results",
            "summary": INTERNAL_ROUTE_HINT,
            "evidence_refs": [],
            "receipt_id": "",
            "error": "internal_route_query_requires_local_diagnostics"
        });
    }
    if !enabled_sources(&policy).iter().any(|row| row == &source) {
        return json!({"ok": false, "status": "blocked", "summary": format!("Source `{source}` is not allowed by policy."), "evidence_refs": [], "receipt_id": "", "error": "source_blocked"});
    }
    if aperture == "large" && !allow_large(&policy) {
        return json!({"ok": false, "status": "blocked", "summary": "Aperture `large` is blocked by policy.", "evidence_refs": [], "receipt_id": "", "error": "aperture_blocked"});
    }
    let budget = match aperture_budget(&aperture) {
        Some(value) => value,
        None => {
            return json!({"ok": false, "status": "blocked", "summary": "Unsupported aperture.", "evidence_refs": [], "receipt_id": "", "error": "aperture_unsupported"})
        }
    };
    if query.is_empty() {
        return json!({"ok": false, "status": "blocked", "summary": "Query is required.", "evidence_refs": [], "receipt_id": "", "error": "query_required"});
    }
    if source == "web" && is_local_subject_comparison_query(&query) {
        return json!({
            "ok": true,
            "status": "no_results",
            "summary": local_subject_comparison_summary(&query),
            "evidence_refs": [],
            "receipt_id": "",
            "error": "local_subject_requires_workspace_analysis"
        });
    }
    let nexus_connection =
        match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus("batch_query")
        {
            Ok(meta) => meta,
            Err(err) => {
                return json!({
                    "ok": false,
                    "type": "batch_query",
                    "status": "blocked",
                    "source": source,
                    "query": query,
                    "aperture": aperture,
                    "summary": "Batch query blocked by hierarchical nexus ingress policy.",
                    "evidence_refs": [],
                    "receipt_id": "",
                    "error": "batch_query_nexus_delivery_denied",
                    "nexus_error": clean_text(&err, 240)
                })
            }
        };
    let query_plan = resolve_query_plan(request, &query, budget);
    let cache_key_primary =
        cache_key_with_query_plan(&source, &query, &aperture, &policy, &query_plan.queries);
    let legacy_cache_key = cache_key(&source, &query, &aperture, &policy);
    let (cached_response, cache_lookup_key) = if let Some(cached) = load_cached_response(root, &cache_key_primary)
    {
        (Some(cached), cache_key_primary.clone())
    } else if cache_key_primary != legacy_cache_key {
        if let Some(cached) = load_cached_response(root, &legacy_cache_key) {
            (Some(cached), legacy_cache_key.clone())
        } else {
            (None, cache_key_primary.clone())
        }
    } else {
        (None, cache_key_primary.clone())
    };
    if let Some(cached) = cached_response {
        let cached_status = clean_text(
            cached
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("no_results"),
            32,
        );
        let query_plan_value = cached
            .get("query_plan")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|row| clean_text(row, 600))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|rows| !rows.is_empty())
            .unwrap_or_else(|| query_plan.queries.clone());
        let query_plan_source = clean_text(
            cached
                .get("query_plan_source")
                .and_then(Value::as_str)
                .unwrap_or(query_plan.query_plan_source),
            64,
        );
        let evidence_refs = cached
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let rewrite_set = cached
            .get("rewrite_set")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let partial_failure_details = cached
            .get("partial_failure_details")
            .and_then(Value::as_array)
            .cloned()
            .map(Value::Array)
            .unwrap_or_else(|| json!([]));
        let raw_summary = clean_text(
            cached
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or(crate::tool_output_match_filter::no_findings_user_copy()),
            budget.max_summary_tokens.max(60),
        );
        let cached_summary_requires_refresh = source == "web"
            && cached_status == "ok"
            && cached_framework_summary_requires_refresh(&query, &raw_summary, &evidence_refs);
        if cached_summary_requires_refresh {
            // Recompute instead of replaying stale forum-led framework summaries.
        } else {
        let status = if cached_status == "ok" && looks_like_low_signal_search_summary(&raw_summary) {
            "no_results".to_string()
        } else {
            cached_status
        };
        let summary = rewrite_cached_batch_query_summary(
            &query,
            &source,
            &raw_summary,
            &evidence_refs,
            &partial_failure_details,
        );
        let parallel_retrieval_used = cached
            .get("parallel_retrieval_used")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let provider_snapshot = json!({
            "id": crate::deterministic_receipt_hash(&json!({"source": source, "query": query, "cache_key": cache_lookup_key})),
            "source": source,
            "adapter_version": "web_conduit_v1",
            "disposable": true
        });
        let receipt = json!({
            "type": "batch_query_receipt",
            "ts": crate::now_iso(),
            "source": source,
            "query": query,
            "aperture": aperture,
            "query_timeout_ms": query_timeout.as_millis() as u64,
            "parallel_window": parallel_window,
            "rewrite_set": rewrite_set,
            "query_plan": query_plan_value,
