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
    if let Some(code) = no_results_error_code_from_summary(&summary) { out["error"] = Value::String(code.to_string()); }
    if let Some(meta) = nexus_connection {
        out["nexus_connection"] = meta;
    }
    out
}
