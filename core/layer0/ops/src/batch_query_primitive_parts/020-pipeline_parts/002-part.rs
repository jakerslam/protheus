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
            "query_plan_source": query_plan_source,
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
            "query_plan": cached
                .get("query_plan")
                .cloned()
                .unwrap_or_else(|| json!(query_plan.queries)),
            "query_plan_source": cached
                .get("query_plan_source")
                .cloned()
                .unwrap_or_else(|| json!(query_plan.query_plan_source)),
            "partial_failure_details": partial_failure_details,
            "cache_status": "hit"
        });
        if let Some(meta) = nexus_connection {
            out["nexus_connection"] = meta;
        }
        return out;
        }
    }

    let queries = query_plan.queries.clone();
    let rewrite_set = query_plan.rewrite_set.clone();
    let parallel_allowed = source == "web" && query_plan.rewrite_applied && queries.len() > 1;
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

    let rerank_query = query_plan.rerank_query.clone();
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
                "Search did not produce enough source coverage to compare {} in this turn. This is a retrieval-quality miss, not proof the systems are equivalent. Retry with named competitors or one specific source URL per side.",
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
    let status = if evidence_refs.is_empty() {
        "no_results"
    } else if hard_partial_failures.is_empty() {
        "ok"
    } else {
        "partial"
    };
    let summary = if evidence_refs.is_empty() {
        let partial_failure_value = Value::Array(
            hard_partial_failures
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        );
        no_results_summary_for_batch_query(
            &query,
            &source,
            &partial_failure_value,
            comparison_guard_summary,
        )
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
        if is_framework_catalog_intent(&query) {
            let fallback_insights =
                framework_catalog_fallback_insights(&actionable_ranked, budget.max_evidence);
            let synthesized_joined = synthesized_insights.join(" ");
            let fallback_joined = fallback_insights.join(" ");
            if framework_name_hits(&synthesized_joined) < 2
                && framework_name_hits(&fallback_joined)
                    > framework_name_hits(&synthesized_joined)
            {
                synthesized_insights = fallback_insights.clone();
            }
            if framework_summary_contains_low_signal_sources(&synthesized_insights.join(" "))
                && !fallback_insights.is_empty()
            {
                synthesized_insights = fallback_insights;
            }
        }
        if synthesized_insights.is_empty() {
            if source == "web" {
                "Web retrieval ran, but only low-signal snippets were available for synthesis in this turn. Retry with a narrower query or one specific source URL for source-backed findings."
                    .to_string()
            } else {
                crate::tool_output_match_filter::no_findings_user_copy().to_string()
            }
        } else {
            let prefix = if is_benchmark_or_comparison_intent(&query) {
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
        "query_plan_source": query_plan.query_plan_source,
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
        "query_plan": queries.clone(),
        "query_plan_source": query_plan.query_plan_source,
        "partial_failure_details": hard_partial_failures.clone(),
        "cache_status": "miss"
    });
    store_cached_response(
        root,
        &cache_key_primary,
        &json!({
            "status": status,
            "summary": summary,
            "evidence_refs": evidence_refs,
            "rewrite_set": rewrite_set,
            "query_plan": queries,
            "query_plan_source": query_plan.query_plan_source,
            "partial_failure_details": hard_partial_failures,
            "parallel_retrieval_used": parallel_allowed
        }),
        status,
    );
    if let Some(meta) = nexus_connection {
        out["nexus_connection"] = meta;
    }
    out
}
