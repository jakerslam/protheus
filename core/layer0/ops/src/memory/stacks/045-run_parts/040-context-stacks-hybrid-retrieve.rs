
fn context_stacks_hybrid_retrieve(root: &Path, parsed: &crate::ParsedArgs) -> Value {
    let mut state = load_context_stacks_state(root);
    let stack_id = stack_id_from(parsed);
    if stack_id.is_empty() {
        return json!({"ok": false, "status": "blocked", "error": "stack_id_required"});
    }
    if find_manifest_index(&state, &stack_id).is_none() {
        return json!({"ok": false, "status": "blocked", "error": "stack_not_found", "stack_id": stack_id});
    }
    let top_k = parse_usize_flag(parsed, "top-k", 5).max(1);
    let query = clean(
        parsed
            .flags
            .get("query")
            .map(String::as_str)
            .unwrap_or(""),
        500,
    );
    let vector_rows = parse_json_value(parsed.flags.get("vector-json"))
        .as_array()
        .cloned()
        .unwrap_or_default();
    let edge_rows = parse_json_value(parsed.flags.get("edges-json"))
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut vector_scores = std::collections::BTreeMap::<String, f64>::new();
    for row in vector_rows {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            let score = row
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            vector_scores.insert(clean(id, 160), score);
        }
    }
    let mut edge_scores = std::collections::BTreeMap::<String, f64>::new();
    for row in edge_rows {
        if let Some(id) = row.get("id").and_then(Value::as_str) {
            let score = row
                .get("edge_confidence")
                .or_else(|| row.get("score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            edge_scores.insert(clean(id, 160), score);
        }
    }
    let mut candidate_ids = vector_scores
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for id in edge_scores.keys() {
        candidate_ids.insert(id.clone());
    }
    let mut ranked = candidate_ids
        .into_iter()
        .map(|id| {
            let vector = vector_scores.get(&id).copied().unwrap_or(0.0);
            let edge = edge_scores.get(&id).copied().unwrap_or(0.0);
            let combined = ((vector * 0.65) + (edge * 0.35)).clamp(0.0, 1.0);
            json!({
                "id": id,
                "vector_score": vector,
                "edge_confidence": edge,
                "combined_score": combined
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        let av = a
            .get("combined_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let bv = b
            .get("combined_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(top_k);

    let event = json!({
        "ts": now_iso(),
        "stack_id": stack_id,
        "query": query,
        "top_k": top_k,
        "results": ranked
    });
    state.hybrid_retrieval_events.push(event.clone());
    let _ = persist_context_stacks_state(root, &state);

    let receipt = receipt_with_common_fields(
        "context_stack_hybrid_retrieve",
        &stack_id,
        "hybrid_retrieval_ok",
        None,
        None,
    );
    let _ = append_context_stacks_receipt(root, &receipt);
    let _ = append_context_stacks_digestion_log(
        root,
        &stack_id,
        &[format!("hybrid_retrieve top_k={top_k} query={query}")],
    );
    json!({
        "ok": true,
        "type": "context_stacks_hybrid_retrieve",
        "stack_id": stack_id,
        "query": query,
        "results": event.get("results").cloned().unwrap_or_else(|| json!([])),
        "receipt_id": receipt.get("receipt_id").cloned().unwrap_or(Value::Null)
    })
}
