const CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";
const CACHE_MAX_ENTRIES: usize = 240;
const CACHE_TTL_SUCCESS_SECS: i64 = 30 * 60;
const CACHE_TTL_NO_RESULTS_SECS: i64 = 2 * 60;
const LINK_FETCH_FALLBACK_LIMIT: usize = 2;
const INTERNAL_ROUTE_HINT: &str =
    "This looks like an internal command mapping request, not a web search query. Use local route diagnostics instead of web retrieval.";

fn contains_antibot_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing",
        "error-lite@duckduckgo.com",
        "anomaly-modal",
        "captcha",
        "verify you are human",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_internal_route_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    lowered.contains("tool::")
        || lowered.contains("map `tool::")
        || lowered.contains("supported route")
        || lowered.contains("command-to-route")
}

fn looks_like_domain_list_noise(text: &str) -> bool {
    let cleaned = clean_text(text, 1_600);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 16);
    if domains.len() < 3 {
        return false;
    }
    let words = cleaned.split_whitespace().count();
    words <= (domains.len() * 3 + 10)
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_REL)
}

fn cache_key(source: &str, query: &str, aperture: &str, policy: &Value) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 1,
        "source": source,
        "query": query,
        "aperture": aperture,
        "policy": policy.get("batch_query").cloned().unwrap_or(Value::Null),
    }))
}

fn cache_ttl_for_status(status: &str) -> i64 {
    if status == "ok" || status == "partial" {
        CACHE_TTL_SUCCESS_SECS
    } else {
        CACHE_TTL_NO_RESULTS_SECS
    }
}

fn load_cached_response(root: &Path, key: &str) -> Option<Value> {
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        let stale_keys = entries
            .iter()
            .filter_map(|(entry_key, entry)| {
                let expires_at = entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
                if expires_at <= now_ts {
                    Some(entry_key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for stale_key in stale_keys {
            entries.remove(&stale_key);
            mutated = true;
        }
        if let Some(entry) = entries.get(key) {
            if let Some(response) = entry.get("response") {
                hit = Some(response.clone());
            }
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

fn store_cached_response(root: &Path, key: &str, response: &Value, status: &str) {
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entries
        .retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    let ttl = cache_ttl_for_status(status).max(30);
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": status,
            "response": response
        }),
    );
    if entries.len() > CACHE_MAX_ENTRIES {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry.get("stored_at").and_then(Value::as_i64).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, stored_at)| *stored_at);
        let drop_count = entries.len().saturating_sub(CACHE_MAX_ENTRIES);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

fn fixture_payload_for_stage_url(stage: &str, url: &str) -> Option<Value> {
    let fixtures = fixture_payload_map()?;
    let stage_key = format!("{stage}::{url}");
    fixtures
        .get(&stage_key)
        .cloned()
        .or_else(|| fixtures.get(&format!("fetch::{url}")).cloned())
        .or_else(|| fixtures.get(&format!("url::{url}")).cloned())
}

fn fixture_mode_enabled() -> bool {
    std::env::var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON")
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false)
}

fn fixture_missing_payload() -> Value {
    json!({
        "ok": false,
        "error": "fixture_missing"
    })
}

fn stage_search_payload(
    root: &Path,
    stage: Option<&str>,
    query: &str,
    provider: Option<&str>,
) -> Value {
    if let Some(stage_name) = stage {
        if let Some(payload) = fixture_payload_for_stage_query(stage_name, query) {
            return payload;
        }
    } else if let Some(payload) = fixture_payload_for_query(query) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    let mut request = json!({
        "query": query,
        "summary_only": false
    });
    if let Some(provider_name) = provider {
        request["provider"] = Value::String(provider_name.to_string());
    }
    crate::web_conduit::api_search(root, &request)
}

fn stage_fetch_payload(root: &Path, stage: &str, url: &str) -> Value {
    if let Some(payload) = fixture_payload_for_stage_url(stage, url) {
        return payload;
    }
    if fixture_mode_enabled() {
        return fixture_missing_payload();
    }
    crate::web_conduit::api_fetch(
        root,
        &json!({
            "url": url,
            "summary_only": false
        }),
    )
}

fn payload_links_for_fallback(payload: &Value, max_links: usize) -> Vec<String> {
    non_search_engine_links(payload, max_links)
}

fn query_overlap_terms(query: &str, candidate: &Candidate) -> usize {
    let query_tokens = query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() > 2)
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    if query_tokens.is_empty() {
        return 0;
    }
    let haystack = clean_text(
        &format!(
            "{} {} {}",
            candidate.title, candidate.snippet, candidate.locator
        ),
        3_200,
    )
    .to_ascii_lowercase();
    query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count()
}

fn candidate_is_substantive(query: &str, candidate: &Candidate, benchmark_intent: bool) -> bool {
    let snippet = clean_text(&candidate.snippet, 1_800);
    if snippet.is_empty() {
        return false;
    }
    if contains_antibot_marker(&snippet) || contains_antibot_marker(&candidate.title) {
        return false;
    }
    if looks_like_domain_list_noise(&snippet) {
        return false;
    }
    let word_count = snippet.split_whitespace().count();
    let overlap = query_overlap_terms(query, candidate);
    if benchmark_intent {
        if word_count < 8 && overlap < 2 {
            return false;
        }
    } else if word_count < 6 && overlap < 1 {
        return false;
    }
    true
}

fn candidate_is_synthesis_eligible(
    query: &str,
    candidate: &Candidate,
    benchmark_intent: bool,
) -> bool {
    if !candidate_passes_relevance_gate(query, candidate, benchmark_intent) {
        return false;
    }
    if looks_like_ack_only(&candidate.snippet)
        || looks_like_low_signal_search_summary(&candidate.snippet)
        || looks_like_source_only_snippet(&candidate.snippet)
    {
        return false;
    }
    if !candidate_is_substantive(query, candidate, benchmark_intent) {
        return false;
    }
    let domain = candidate_domain_hint(candidate);
    if is_search_engine_domain(&domain) {
        return false;
    }
    if benchmark_intent {
        if looks_like_definition_candidate(candidate)
            || looks_like_comparison_noise_candidate(candidate)
        {
            return false;
        }
        if !looks_like_metric_rich_text(&candidate.snippet)
            && query_overlap_terms(query, candidate) < 2
        {
            return false;
        }
    }
    true
}

fn stage_error(payload: &Value, fallback: &str) -> String {
    clean_text(
        payload
            .get("error")
            .or_else(|| payload.pointer("/result/error"))
            .and_then(Value::as_str)
            .unwrap_or(fallback),
        220,
    )
}

fn collect_candidates_from_stage_payload(
    root: &Path,
    stage: &str,
    query: &str,
    payload: &Value,
    benchmark_intent: bool,
    fetched_links: &mut HashSet<String>,
) -> (Vec<Candidate>, Vec<String>) {
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();

    match candidate_from_search_payload(query, payload) {
        Ok(candidate) => {
            if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                candidates.push(candidate);
            } else {
                issues.push(format!("{stage}:candidate_low_relevance"));
            }
        }
        Err(err) => issues.push(format!("{stage}:{err}")),
    }

    let summary = clean_text(
        payload.get("summary").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    let content = clean_text(
        payload.get("content").and_then(Value::as_str).unwrap_or(""),
        2_400,
    );
    if contains_antibot_marker(&summary) || contains_antibot_marker(&content) {
        issues.push(format!("{stage}:anti_bot_challenge"));
    }
    let should_fetch_links =
        candidates.is_empty() || looks_like_low_signal_search_summary(&summary);
    if should_fetch_links {
        for link in payload_links_for_fallback(payload, LINK_FETCH_FALLBACK_LIMIT) {
            if !fetched_links.insert(link.clone()) {
                continue;
            }
            let fetch_payload = stage_fetch_payload(root, stage, &link);
            if !fetch_payload
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                issues.push(format!(
                    "{stage}:fetch:{}",
                    stage_error(&fetch_payload, "web_fetch_failed")
                ));
                continue;
            }
            match candidate_from_search_payload(query, &fetch_payload) {
                Ok(mut candidate) => {
                    if candidate.locator.is_empty()
                        || is_search_engine_domain(&candidate_domain_hint(&candidate))
                    {
                        candidate.locator = link.clone();
                    }
                    if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                        candidates.push(candidate);
                    } else {
                        issues.push(format!("{stage}:fetch_candidate_low_relevance"));
                    }
                }
                Err(err) => issues.push(format!("{stage}:fetch_candidate:{err}")),
            }
        }
    }
    (candidates, issues)
}

fn retrieve_web_candidates_for_query(root: &Path, query: &str) -> Result<Vec<Candidate>, String> {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let mut candidates = Vec::<Candidate>::new();
    let mut issues = Vec::<String>::new();
    let mut fetched_links = HashSet::<String>::new();

    let primary_payload = stage_search_payload(root, None, query, None);
    let (primary_candidates, primary_issues) = collect_candidates_from_stage_payload(
        root,
        "primary",
        query,
        &primary_payload,
        benchmark_intent,
        &mut fetched_links,
    );
    candidates.extend(primary_candidates);
    issues.extend(primary_issues);

    if candidates.is_empty()
        && issues
            .iter()
            .any(|issue| skip_duckduckgo_fallback_for_error(issue))
    {
        return Err(issues.join("|"));
    }

    if candidates.is_empty() {
        let bing_payload = stage_search_payload(root, Some("bing_rss"), query, Some("bing"));
        let (bing_candidates, bing_issues) = collect_candidates_from_stage_payload(
            root,
            "bing_rss",
            query,
            &bing_payload,
            benchmark_intent,
            &mut fetched_links,
        );
        candidates.extend(bing_candidates);
        issues.extend(bing_issues);
    }

    if candidates.is_empty() {
        let fallback_url = duckduckgo_instant_answer_url(query);
        let fallback_payload =
            if let Some(payload) = fixture_payload_for_stage_query("duckduckgo_instant", query) {
                payload
            } else if fixture_mode_enabled() {
                fixture_missing_payload()
            } else {
                stage_fetch_payload(root, "duckduckgo_instant", &fallback_url)
            };
        match candidate_from_duckduckgo_instant_payload(query, &fallback_url, &fallback_payload) {
            Ok(candidate) => {
                if candidate_is_synthesis_eligible(query, &candidate, benchmark_intent) {
                    candidates.push(candidate);
                } else {
                    issues.push("duckduckgo_instant:candidate_low_relevance".to_string());
                }
            }
            Err(err) => issues.push(format!("duckduckgo_instant:{err}")),
        }
    }

    if candidates.is_empty() {
        if issues.is_empty() {
            Err("no_usable_summary".to_string())
        } else {
            Err(issues.join("|"))
        }
    } else {
        let mut dedup = HashSet::<String>::new();
        let mut unique = Vec::<Candidate>::new();
        for candidate in candidates {
            let key = format!(
                "{}|{}|{}",
                candidate.locator.to_ascii_lowercase(),
                candidate.title.to_ascii_lowercase(),
                candidate.excerpt_hash
            );
            if dedup.insert(key) {
                unique.push(candidate);
            }
        }
        Ok(unique)
    }
}

fn rerank_score(query: &str, candidate: &Candidate) -> f64 {
    let benchmark_intent = is_benchmark_or_comparison_intent(query);
    let query_tokens = query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(|token| token.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let haystack = format!("{} {}", candidate.title, candidate.snippet).to_ascii_lowercase();
    let overlap = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count() as f64;
    let overlap_norm = if query_tokens.is_empty() {
        0.0
    } else {
        overlap / query_tokens.len() as f64
    };
    let locator_bonus = if candidate.locator.is_empty() {
        0.0
    } else {
        0.2
    };
    let status_bonus = if (200..400).contains(&candidate.status_code) {
        0.2
    } else {
        0.0
    };
    let metric_bonus = if benchmark_intent && looks_like_metric_rich_text(&candidate.snippet) {
        0.24
    } else {
        0.0
    };
    let definition_penalty = if benchmark_intent && looks_like_definition_candidate(candidate) {
        0.72
    } else {
        0.0
    };
    let comparison_noise_penalty =
        if benchmark_intent && looks_like_comparison_noise_candidate(candidate) {
            0.65
        } else {
            0.0
        };
    let mut score = 0.6 * overlap_norm + locator_bonus + status_bonus + metric_bonus
        - definition_penalty
        - comparison_noise_penalty;
    if benchmark_intent && !looks_like_metric_rich_text(&candidate.snippet) {
        score -= 0.12;
    }
    score.clamp(0.0, 1.0)
}

fn minimum_synthesis_score(benchmark_intent: bool) -> f64 {
    if benchmark_intent {
        0.33
    } else {
        0.18
    }
}

fn retrieve_web_candidates_for_query_with_timeout(
    root: &Path,
    query: &str,
    timeout: Duration,
) -> Result<Vec<Candidate>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<Candidate>, String>>();
    let root_buf = root.to_path_buf();
    let query_buf = query.to_string();
    let spawned = thread::Builder::new()
        .name("batch-query-retrieve".to_string())
        .spawn(move || {
            let out = retrieve_web_candidates_for_query(&root_buf, &query_buf);
            let _ = tx.send(out);
        });
    if spawned.is_err() {
        return Err("query_worker_spawn_failed".to_string());
    }
    match rx.recv_timeout(timeout) {
        Ok(out) => out,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("query_timeout_ms_{}", timeout.as_millis()))
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err("query_worker_disconnected".to_string())
        }
    }
}

fn is_benign_partial_failure(detail: &str) -> bool {
    let lowered = clean_text(detail, 320).to_ascii_lowercase();
    if lowered.contains("anti_bot_challenge") {
        return false;
    }
    lowered.contains("candidate_low_relevance")
        || lowered.contains("fetch_candidate_low_relevance")
        || lowered.contains("no_usable_summary")
        || lowered.contains("fixture_missing")
}

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
    let query = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        600,
    );
    let aperture = normalize_aperture(
        request
            .get("aperture")
            .and_then(Value::as_str)
            .unwrap_or("medium"),
    );
    if query.is_empty() {
        return json!({"ok": false, "status": "blocked", "summary": "Query is required.", "evidence_refs": [], "receipt_id": "", "error": "query_required"});
    }
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
    let cache_key = cache_key(&source, &query, &aperture, &policy);
    if let Some(cached) = load_cached_response(root, &cache_key) {
        let status = clean_text(
            cached
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("no_results"),
            32,
        );
        let summary = clean_text(
            cached
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("Search returned no useful information."),
            budget.max_summary_tokens.max(60),
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
        let parallel_retrieval_used = cached
            .get("parallel_retrieval_used")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let provider_snapshot = json!({
            "id": crate::deterministic_receipt_hash(&json!({"source": source, "query": query, "cache_key": cache_key})),
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
            "query_plan": [query.clone()],
            "adapter_version": "web_conduit_v1",
            "provider_snapshot": provider_snapshot,
            "snapshot_id": provider_snapshot.get("id").cloned().unwrap_or(Value::Null),
            "candidate_count": 0,
            "dedup_count": 0,
            "evidence_count": evidence_refs.as_array().map(|rows| rows.len()).unwrap_or(0),
            "cache_status": "hit",
            "latency_ms": started.elapsed().as_millis() as u64,
            "token_usage": {"summary_tokens_estimate": summary.split_whitespace().count()},
            "parallel_retrieval_used": parallel_retrieval_used,
            "partial_failure_details": [],
            "status": status
        });
        let receipt_id = crate::deterministic_receipt_hash(&receipt);
        let mut receipt_with_id = receipt.clone();
        receipt_with_id["receipt_id"] = Value::String(receipt_id.clone());
        let _ = append_jsonl(&receipts_path(root), &receipt_with_id);
        let mut out = json!({
            "ok": status != "blocked",
            "type": "batch_query",
            "status": status,
            "source": source,
            "query": query,
            "aperture": aperture,
            "summary": summary,
            "evidence_refs": evidence_refs,
            "receipt_id": receipt_id,
            "parallel_retrieval_used": parallel_retrieval_used,
            "query_timeout_ms": query_timeout.as_millis() as u64,
            "parallel_window": parallel_window,
            "rewrite_set": rewrite_set,
            "cache_status": "hit"
        });
        if let Some(meta) = nexus_connection {
            out["nexus_connection"] = meta;
        }
        return out;
    }

    let (queries, rewrite_set, rewrite_applied) = build_query_plan(&query, budget);
    let parallel_allowed = source == "web" && rewrite_applied && queries.len() > 1;
    let mut candidates = Vec::<Candidate>::new();
    let mut partial_failures = Vec::<String>::new();
    if parallel_allowed {
        let limit = parallel_window;
        let mut offset = 0usize;
        while offset < queries.len() {
            let end = (offset + limit).min(queries.len());
            let expected = end.saturating_sub(offset);
            let (tx, rx) =
                std::sync::mpsc::channel::<(usize, String, Result<Vec<Candidate>, String>)>();
            let mut chunk_rows = std::iter::repeat_with(|| None)
                .take(expected)
                .collect::<Vec<Option<(String, Result<Vec<Candidate>, String>)>>>();
            for (local_idx, q) in queries[offset..end].iter().enumerate() {
                let tx_clone = tx.clone();
                let query_item = q.clone();
                let root_buf = root.to_path_buf();
                let spawned = thread::Builder::new()
                    .name(format!("batch-query-{local_idx}"))
                    .spawn(move || {
                        let out = retrieve_web_candidates_for_query(&root_buf, &query_item);
                        let _ = tx_clone.send((local_idx, query_item, out));
                    });
                if spawned.is_err() {
                    chunk_rows[local_idx] =
                        Some((q.clone(), Err("query_worker_spawn_failed".to_string())));
                }
            }
            drop(tx);
            let mut received = chunk_rows.iter().filter(|row| row.is_some()).count();
            while received < expected {
                match rx.recv_timeout(query_timeout) {
                    Ok((local_idx, q, out)) => {
                        if local_idx < expected && chunk_rows[local_idx].is_none() {
                            chunk_rows[local_idx] = Some((q, out));
                            received += 1;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            for local_idx in 0..expected {
                let fallback_query = clean_text(&queries[offset + local_idx], 120);
                match chunk_rows[local_idx].take() {
                    Some((q, out)) => match out {
                        Ok(mut rows) => {
                            if rows.is_empty() {
                                partial_failures
                                    .push(format!("{}:no_usable_summary", clean_text(&q, 120)));
                            } else {
                                candidates.append(&mut rows);
                            }
                        }
                        Err(err) => partial_failures.push(format!("{}:{err}", clean_text(&q, 120))),
                    },
                    None => partial_failures.push(format!(
                        "{}:query_timeout_ms_{}",
                        fallback_query,
                        query_timeout.as_millis()
                    )),
                }
            }
            offset = end;
        }
    } else {
        for q in &queries {
            match retrieve_web_candidates_for_query_with_timeout(root, q, query_timeout) {
                Ok(mut rows) => {
                    if rows.is_empty() {
                        partial_failures.push(format!("{}:no_usable_summary", clean_text(q, 120)));
                    } else {
                        candidates.append(&mut rows);
                    }
                }
                Err(err) => partial_failures.push(format!("{}:{err}", clean_text(q, 120))),
            }
        }
    }

    let before_dedup = candidates.len();
    let mut seen = HashSet::<String>::new();
    candidates.retain(|row| {
        let key = format!(
            "{}|{}|{}",
            row.locator.to_ascii_lowercase(),
            row.title.to_ascii_lowercase(),
            row.excerpt_hash
        );
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });
    candidates.truncate(budget.max_candidates);

    let rerank_query = if rewrite_applied {
        queries.last().cloned().unwrap_or_else(|| query.clone())
    } else {
        query.clone()
    };
    let benchmark_intent = is_benchmark_or_comparison_intent(&rerank_query);
    let mut ranked = candidates
        .iter()
        .cloned()
        .map(|row| {
            let score = rerank_score(&rerank_query, &row);
            (row, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.title.cmp(&b.0.title))
    });
    ranked.truncate(budget.max_evidence);

    let min_synthesis_score = minimum_synthesis_score(benchmark_intent);
    let mut actionable_ranked = ranked
        .into_iter()
        .filter(|(row, score)| {
            let snippet = clean_text(&row.snippet, 1_200);
            let domain = candidate_domain_hint(row);
            !snippet.is_empty()
                && *score >= min_synthesis_score
                && !looks_like_ack_only(&snippet)
                && !looks_like_low_signal_search_summary(&snippet)
                && !looks_like_source_only_snippet(&snippet)
                && !is_search_engine_domain(&domain)
                && candidate_passes_relevance_gate(&rerank_query, row, benchmark_intent)
                && candidate_is_substantive(&rerank_query, row, benchmark_intent)
                && !(benchmark_intent && looks_like_definition_candidate(row))
                && !(benchmark_intent && looks_like_comparison_noise_candidate(row))
        })
        .collect::<Vec<_>>();

    let comparison_entities = if benchmark_intent {
        comparison_entities_from_query(&query)
    } else {
        Vec::new()
    };
    let mut comparison_guard_summary = None::<String>;
    if comparison_entities.len() >= 2 {
        let coverage_ok = comparison_entities.iter().all(|entity| {
            actionable_ranked
                .iter()
                .any(|(row, _)| candidate_mentions_entity(row, entity))
        });
        if !coverage_ok {
            actionable_ranked.clear();
            comparison_guard_summary = Some(format!(
                "Search returned no useful comparison findings for {}.",
                comparison_entities.join(" vs ")
            ));
        }
    }

    let evidence_refs = actionable_ranked
        .iter()
        .map(|(row, score)| EvidenceRef {
            source_kind: row.source_kind.clone(),
            title: row.title.clone(),
            locator: row.locator.clone(),
            excerpt_hash: row.excerpt_hash.clone(),
            score: (*score * 100.0).round() / 100.0,
            timestamp: row.timestamp.clone(),
            permissions: row.permissions.clone(),
        })
        .collect::<Vec<_>>();

    let hard_partial_failures = partial_failures
        .iter()
        .filter(|row| !is_benign_partial_failure(row))
        .cloned()
        .collect::<Vec<_>>();
    let anti_bot_detected = hard_partial_failures.iter().any(|row| {
        clean_text(row, 320)
            .to_ascii_lowercase()
            .contains("anti_bot_challenge")
    });
    let status = if evidence_refs.is_empty() {
        "no_results"
    } else if hard_partial_failures.is_empty() {
        "ok"
    } else {
        "partial"
    };
    let summary = if evidence_refs.is_empty() {
        if anti_bot_detected {
            "Search providers returned anti-bot challenge pages before usable content was extracted. Retry with specific source URLs or alternate providers."
                .to_string()
        } else {
            comparison_guard_summary
                .unwrap_or_else(|| "Search returned no useful information.".to_string())
        }
    } else {
        let mut synthesized_insights = Vec::<String>::new();
        let mut seen_domains = HashSet::<String>::new();
        for (candidate, _) in &actionable_ranked {
            let snippet_raw = if benchmark_intent {
                extract_metric_focused_fragment(&candidate.snippet)
            } else {
                clean_text(&candidate.snippet, 1_200)
            };
            let snippet = trim_words(&snippet_raw, if benchmark_intent { 30 } else { 42 });
            if snippet.is_empty() {
                continue;
            }
            let domain = candidate_domain_hint(candidate);
            let domain_key = clean_text(&domain, 160).to_ascii_lowercase();
            if domain_key != "source" && !domain_key.is_empty() && !seen_domains.insert(domain_key)
            {
                continue;
            }
            let insight = if domain == "source" {
                snippet.clone()
            } else {
                format!("{domain}: {snippet}")
            };
            if synthesized_insights
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&insight))
            {
                continue;
            }
            synthesized_insights.push(insight);
            if synthesized_insights.len() >= budget.max_evidence.max(1) {
                break;
            }
        }
        if synthesized_insights.is_empty() {
            "Search returned no useful information.".to_string()
        } else {
            let prefix = if benchmark_intent {
                "Benchmark findings:"
            } else {
                "Key findings:"
            };
            trim_words(
                &format!("{prefix} {}", synthesized_insights.join("; ")),
                budget.max_summary_tokens,
            )
        }
    };

    let provider_snapshot = json!({
        "id": crate::deterministic_receipt_hash(&json!({"source": source, "queries": queries})),
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
        "query_plan": queries,
        "adapter_version": "web_conduit_v1",
        "provider_snapshot": provider_snapshot,
        "snapshot_id": provider_snapshot.get("id").cloned().unwrap_or(Value::Null),
        "candidate_count": before_dedup,
        "dedup_count": before_dedup.saturating_sub(candidates.len()),
        "evidence_count": evidence_refs.len(),
        "cache_status": "miss",
        "latency_ms": started.elapsed().as_millis() as u64,
        "token_usage": {"summary_tokens_estimate": summary.split_whitespace().count()},
        "parallel_retrieval_used": parallel_allowed,
        "partial_failure_details": hard_partial_failures,
        "status": status
    });
    let receipt_id = crate::deterministic_receipt_hash(&receipt);
    let mut receipt_with_id = receipt.clone();
    receipt_with_id["receipt_id"] = Value::String(receipt_id.clone());
    let _ = append_jsonl(&receipts_path(root), &receipt_with_id);

    let mut out = json!({
        "ok": status != "blocked",
        "type": "batch_query",
        "status": status,
        "source": source,
        "query": query,
        "aperture": aperture,
        "summary": summary.clone(),
        "evidence_refs": evidence_refs.clone(),
        "receipt_id": receipt_id,
        "parallel_retrieval_used": parallel_allowed,
        "query_timeout_ms": query_timeout.as_millis() as u64,
        "parallel_window": parallel_window,
        "rewrite_set": rewrite_set.clone(),
        "cache_status": "miss"
    });
    store_cached_response(
        root,
        &cache_key,
        &json!({
            "status": status,
            "summary": summary,
            "evidence_refs": evidence_refs,
            "rewrite_set": rewrite_set,
            "parallel_retrieval_used": parallel_allowed
        }),
        status,
    );
    if let Some(meta) = nexus_connection {
        out["nexus_connection"] = meta;
    }
    out
}
