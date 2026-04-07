const CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";
const CACHE_MAX_ENTRIES: usize = 240;
const CACHE_TTL_SUCCESS_SECS: i64 = 30 * 60;
const CACHE_TTL_NO_RESULTS_SECS: i64 = 2 * 60;

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

fn retrieve_web_candidate_for_query(root: &Path, query: &str) -> Result<Candidate, String> {
    let payload = fixture_payload_for_query(query).unwrap_or_else(|| {
        crate::web_conduit::api_search(root, &json!({"query": query, "summary_only": false}))
    });
    match candidate_from_search_payload(query, &payload) {
        Ok(candidate) => Ok(candidate),
        Err(primary_err) => {
            if skip_duckduckgo_fallback_for_error(&primary_err) {
                return Err(primary_err);
            }
            let bing_payload =
                fixture_payload_for_stage_query("bing_rss", query).unwrap_or_else(|| {
                    crate::web_conduit::api_search(
                        root,
                        &json!({
                            "query": query,
                            "provider": "bing",
                            "summary_only": false
                        }),
                    )
                });
            if let Ok(candidate) = candidate_from_search_payload(query, &bing_payload) {
                return Ok(candidate);
            }
            let bing_err = clean_text(
                bing_payload
                    .get("error")
                    .or_else(|| bing_payload.pointer("/result/error"))
                    .and_then(Value::as_str)
                    .unwrap_or("bing_rss_no_usable_summary"),
                220,
            );
            let fallback_url = duckduckgo_instant_answer_url(query);
            let fallback_payload = fixture_payload_for_stage_query("duckduckgo_instant", query)
                .unwrap_or_else(|| {
                    crate::web_conduit::api_fetch(
                        root,
                        &json!({
                            "url": fallback_url.clone(),
                            "summary_only": false
                        }),
                    )
                });
            match candidate_from_duckduckgo_instant_payload(query, &fallback_url, &fallback_payload)
            {
                Ok(candidate) => Ok(candidate),
                Err(fallback_err) => Err(format!(
                    "{primary_err}|bing:{bing_err}|fallback:{fallback_err}"
                )),
            }
        }
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

pub fn api_batch_query(root: &Path, request: &Value) -> Value {
    let started = Instant::now();
    let policy = load_policy(root);
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
        let limit = max_parallel_subqueries(&policy).max(1);
        let mut offset = 0usize;
        while offset < queries.len() {
            let end = (offset + limit).min(queries.len());
            let handles = queries[offset..end]
                .iter()
                .map(|q| {
                    let query_item = q.clone();
                    let root_buf = root.to_path_buf();
                    thread::spawn(move || {
                        (
                            query_item.clone(),
                            retrieve_web_candidate_for_query(&root_buf, &query_item),
                        )
                    })
                })
                .collect::<Vec<_>>();
            for handle in handles {
                if let Ok((q, out)) = handle.join() {
                    match out {
                        Ok(candidate) => candidates.push(candidate),
                        Err(err) => partial_failures.push(format!("{}:{err}", clean_text(&q, 120))),
                    }
                } else {
                    partial_failures.push("thread_join_failed".to_string());
                }
            }
            offset = end;
        }
    } else {
        for q in &queries {
            match retrieve_web_candidate_for_query(root, q) {
                Ok(candidate) => candidates.push(candidate),
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

    let mut actionable_ranked = ranked
        .into_iter()
        .filter(|(row, _)| {
            let snippet = clean_text(&row.snippet, 1_200);
            !snippet.is_empty()
                && !looks_like_ack_only(&snippet)
                && !looks_like_low_signal_search_summary(&snippet)
                && !looks_like_source_only_snippet(&snippet)
                && candidate_passes_relevance_gate(&rerank_query, row, benchmark_intent)
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

    let status = if evidence_refs.is_empty() {
        "no_results"
    } else if partial_failures.is_empty() {
        "ok"
    } else {
        "partial"
    };
    let summary = if evidence_refs.is_empty() {
        comparison_guard_summary
            .unwrap_or_else(|| "Search returned no useful information.".to_string())
    } else {
        let mut synthesized_insights = Vec::<String>::new();
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
                "Web benchmark synthesis:"
            } else {
                "From web retrieval:"
            };
            trim_words(
                &format!("{prefix} {}", synthesized_insights.join(" ")),
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
        "partial_failure_details": partial_failures,
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
