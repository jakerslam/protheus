
fn replay_task_lineage_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let task_id = as_str(payload.get("task_id"));
    if task_id.is_empty() {
        return Err("task_id_required".to_string());
    }
    let trace_id = {
        let value = as_str(payload.get("trace_id"));
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    };
    let limit = parse_lineage_limit(payload);
    let scan_root = parse_scan_root(root, payload);

    let explicit_sources = source_paths_from_payload(&scan_root, payload);
    let sources_override = !explicit_sources.is_empty();
    let mut source_paths = if sources_override {
        explicit_sources
    } else {
        let mut discovered = known_lineage_paths(&scan_root);
        discovered.extend(discover_lineage_paths(&scan_root));
        discovered
    };
    let mut dedupe = BTreeSet::<PathBuf>::new();
    source_paths.retain(|path| dedupe.insert(path.clone()));
    let source_candidates_count = source_paths.len();

    let mut task_events = Vec::<Value>::new();
    let mut tool_calls = Vec::<Value>::new();
    let mut evidence_cards = Vec::<Value>::new();
    let mut claims = Vec::<Value>::new();
    let mut memory_mutations = Vec::<Value>::new();
    let mut assimilation_steps = Vec::<Value>::new();
    let mut scanned_files = 0usize;
    let mut scanned_rows = 0usize;
    let mut seen_result_ids = HashSet::<String>::new();
    let mut seen_evidence_ids = HashSet::<String>::new();
    let mut seen_claim_ids = HashSet::<String>::new();
    let mut seen_memory_receipts = HashSet::<String>::new();
    let mut seen_assimilation_receipts = HashSet::<String>::new();

    for path in source_paths {
        let rows = read_jsonl_rows(&path, limit);
        if rows.is_empty() {
            continue;
        }
        scanned_files = scanned_files.saturating_add(1);
        scanned_rows = scanned_rows.saturating_add(rows.len());
        let is_protocol_steps = path
            .file_name()
            .and_then(|v| v.to_str())
            .map(|name| name.eq_ignore_ascii_case("protocol_step_receipts.jsonl"))
            .unwrap_or(false);
        for (idx, row) in rows {
            if !row_matches_task_or_trace(&row, &task_id, trace_id.as_deref()) {
                continue;
            }
            let type_compact = lower_compact_type(&row);
            if type_compact.contains("task_")
                || row
                    .pointer("/payload/task_id")
                    .and_then(Value::as_str)
                    .is_some()
            {
                task_events.push(json!({
                    "source_file": path.to_string_lossy(),
                    "line_index": idx,
                    "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                    "type": row.get("type").cloned().unwrap_or(Value::Null),
                    "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                    "payload": row.get("payload").cloned().unwrap_or(Value::Null)
                }));
            }

            let mut pipelines = Vec::<Value>::new();
            collect_tool_pipeline_objects(&row, &mut pipelines, 16);
            for pipeline in pipelines {
                let normalized = pipeline
                    .get("normalized_result")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result_id = as_str(normalized.get("result_id"));
                if !result_id.is_empty() && !seen_result_ids.insert(result_id.clone()) {
                    continue;
                }
                if !result_id.is_empty() || !normalized.is_null() {
                    tool_calls.push(normalized.clone());
                }
                let evidence = pipeline
                    .get("evidence_cards")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for card in evidence {
                    let evidence_id = as_str(card.get("evidence_id"));
                    if evidence_id.is_empty() || seen_evidence_ids.insert(evidence_id) {
                        evidence_cards.push(card);
                    }
                }
                let claim_rows = pipeline
                    .get("claim_bundle")
                    .and_then(Value::as_object)
                    .and_then(|bundle| bundle.get("claims"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for claim in claim_rows {
                    let claim_id = as_str(claim.get("claim_id"));
                    if claim_id.is_empty() || seen_claim_ids.insert(claim_id) {
                        claims.push(claim);
                    }
                }
            }

            if type_compact.contains("memory_") || type_compact.contains("|memory") {
                let receipt_hash = as_str(row.get("receipt_hash"));
                if receipt_hash.is_empty() || seen_memory_receipts.insert(receipt_hash) {
                    memory_mutations.push(json!({
                        "source_file": path.to_string_lossy(),
                        "line_index": idx,
                        "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                        "type": row.get("type").cloned().unwrap_or(Value::Null),
                        "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                        "payload": row.get("payload").cloned().unwrap_or(Value::Null),
                    }));
                }
            }

            if is_protocol_steps || type_compact.contains("assimilation") {
                let receipt_hash = as_str(row.get("receipt_hash"));
                if receipt_hash.is_empty() || seen_assimilation_receipts.insert(receipt_hash) {
                    assimilation_steps.push(json!({
                        "source_file": path.to_string_lossy(),
                        "line_index": idx,
                        "receipt_hash": row.get("receipt_hash").cloned().unwrap_or(Value::Null),
                        "step_id": row.get("step_id").cloned().unwrap_or(Value::Null),
                        "type": row.get("type").cloned().unwrap_or(Value::Null),
                        "event_type": row.get("event_type").cloned().unwrap_or(Value::Null),
                        "payload": row.get("payload").cloned().unwrap_or(Value::Null),
                    }));
                }
            }
        }
    }

    let evidence_ids = evidence_cards
        .iter()
        .map(|row| as_str(row.get("evidence_id")))
        .filter(|row| !row.is_empty())
        .collect::<HashSet<_>>();
    let mut claims_without_evidence = Vec::<Value>::new();
    for claim in &claims {
        let claim_id = as_str(claim.get("claim_id"));
        let evidence_refs = claim
            .get("evidence_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|v| as_str(Some(&v)))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if evidence_refs.is_empty() || evidence_refs.iter().any(|id| !evidence_ids.contains(id)) {
            claims_without_evidence.push(json!({
                "claim_id": claim_id,
                "evidence_ids": evidence_refs
            }));
        }
    }

    Ok(json!({
        "ok": true,
        "task_id": task_id,
        "trace_id": trace_id,
        "lineage": {
            "task": task_events,
            "tool_call": tool_calls,
            "evidence": evidence_cards,
            "claim": claims,
            "memory_mutation": memory_mutations,
            "assimilation_step": assimilation_steps
        },
        "validation": {
            "claims_without_evidence": claims_without_evidence,
            "claim_evidence_integrity_ok": claims_without_evidence.is_empty()
        },
        "stats": {
            "sources_override": sources_override,
            "source_candidates_count": source_candidates_count,
            "scanned_files": scanned_files,
            "scanned_rows": scanned_rows
        }
    }))
}

pub fn query_task_lineage(
    root: &Path,
    task_id: &str,
    trace_id: Option<&str>,
    limit: usize,
    scan_root: Option<&Path>,
    sources_csv: Option<&str>,
) -> Result<Value, String> {
    let scan_root_value = scan_root
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut payload = json!({
        "task_id": task_id,
        "trace_id": trace_id.unwrap_or_default(),
        "limit": limit,
        "scan_root": scan_root_value
    });
    if let Some(sources) = sources_csv
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("sources".to_string(), Value::String(sources.to_string()));
        }
    }
    let obj = payload
        .as_object()
        .cloned()
        .unwrap_or_else(Map::<String, Value>::new);
    replay_task_lineage_value(root, &obj)
}
