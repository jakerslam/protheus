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

