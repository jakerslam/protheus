fn dashboard_troubleshooting_eval_recommendations(
    top_error: &str,
    top_classification: &str,
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if top_error == "web_tool_not_invoked" || top_classification == "tool_not_invoked" {
        out.push("tighten tool gate: require at least one recorded web tool call when requires_live_web=true".to_string());
    }
    if top_error.contains("low_signal") || top_classification == "low_signal" {
        out.push("narrow query and retry once with explicit source/provider preference before final synthesis".to_string());
    }
    if top_error.contains("query_result_mismatch") {
        out.push("treat retrieval as mismatched-to-intent and force fail-closed copy; do not surface raw dump content".to_string());
        out.push("apply query/result alignment scoring before accepting fallback web summaries".to_string());
    }
    if top_error.contains("policy") || top_classification == "policy_blocked" {
        out.push("surface policy-block reason directly and avoid hidden retries; request approval/elevation path".to_string());
    }
    if top_error.contains("auth_missing") {
        out.push("prompt operator to configure server-side github token before report pipeline retry".to_string());
    }
    if out.is_empty() {
        out.push("continue collecting stage receipts and compare consecutive snapshots for drift".to_string());
    }
    out
}
fn dashboard_troubleshooting_eval_model_strength(model: &str) -> &'static str {
    let lowered = clean_text(model, 120).to_ascii_lowercase();
    if lowered.starts_with("gpt-5") || lowered.starts_with("o3") || lowered.starts_with("o4") {
        "strong"
    } else {
        "custom"
    }
}
fn dashboard_troubleshooting_resolve_eval_model(payload: Option<&Value>) -> (String, String) {
    if let Some(args) = payload {
        for key in ["eval_model", "evalModel", "llm_model", "llmModel", "model"] {
            let candidate = clean_text(args.get(key).and_then(Value::as_str).unwrap_or(""), 120);
            if !candidate.is_empty() {
                return (candidate, "payload".to_string());
            }
        }
    }
    (
        DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL.to_string(),
        "default_strong".to_string(),
    )
}
fn dashboard_troubleshooting_generate_eval_report(
    snapshot: &Value,
    source: &str,
    eval_model: &str,
    eval_model_source: &str,
) -> Value {
    let entries = snapshot
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut error_counts = HashMap::<String, i64>::new();
    let mut class_counts = HashMap::<String, i64>::new();
    let mut failure_count = 0i64;
    for row in &entries {
        if dashboard_troubleshooting_exchange_failed(row) {
            failure_count += 1;
        }
        let error_code = clean_text(
            row.pointer("/workflow/error_code")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !error_code.is_empty() {
            *error_counts.entry(error_code).or_insert(0) += 1;
        }
        let class = clean_text(
            row.pointer("/workflow/classification")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            80,
        )
        .to_ascii_lowercase();
        *class_counts.entry(class).or_insert(0) += 1;
    }
    let error_hist = dashboard_troubleshooting_sorted_histogram(error_counts, "error_code");
    let class_hist = dashboard_troubleshooting_sorted_histogram(class_counts, "classification");
    let top_error = error_hist
        .first()
        .and_then(|row| row.get("error_code"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let top_class = class_hist
        .first()
        .and_then(|row| row.get("classification"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let severity = if failure_count >= 4 {
        "critical"
    } else if failure_count >= 1 {
        "elevated"
    } else {
        "stable"
    };
    let summary = if failure_count == 0 {
        "No failing workflow exchanges were detected in the current troubleshooting window."
            .to_string()
    } else {
        format!(
            "Detected {failure_count} failing exchanges; top_error={top_error}; top_classification={top_class}."
        )
    };
    let model = clean_text(eval_model, 120);
    let mut report = json!({
        "ok": true,
        "type": "dashboard_workflow_eval_report",
        "report_id": format!(
            "eval_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}:{}",
                now_iso(),
                snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or("unknown"),
                clean_text(source, 60)
            ))[..12]
        ),
        "source": clean_text(source, 60),
        "snapshot_id": clean_text(snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or(""), 80),
        "generated_at": now_iso(),
        "severity": severity,
        "failure_count": failure_count,
        "eval": {
            "engine": "troubleshooting_eval_v1",
            "model": model,
            "model_source": clean_text(eval_model_source, 80),
            "model_strength": dashboard_troubleshooting_eval_model_strength(eval_model),
            "strong_default_model": DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
            "llm_required": true
        },
        "summary": summary,
        "error_histogram": error_hist,
        "classification_histogram": class_hist,
        "recommendations": dashboard_troubleshooting_eval_recommendations(top_error, top_class)
    });
    report["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&report));
    report
}
fn dashboard_troubleshooting_eval_drain_internal(
    root: &Path,
    max_items: usize,
    source: &str,
) -> Value {
    let mut queue = dashboard_troubleshooting_read_eval_queue(root);
    let mut remaining = Vec::<Value>::new();
    let mut reports = Vec::<Value>::new();
    let cap = max_items.clamp(1, 50);
    for row in queue.drain(..) {
        let status = clean_text(row.get("status").and_then(Value::as_str).unwrap_or("queued"), 40)
            .to_ascii_lowercase();
        if reports.len() < cap && status == "queued" {
            let snapshot = row
                .get("snapshot")
                .cloned()
                .or_else(|| read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL)))
                .unwrap_or_else(|| json!({}));
            let queue_eval_model =
                clean_text(row.get("eval_model").and_then(Value::as_str).unwrap_or(""), 120);
            let queue_eval_model_source = clean_text(
                row.get("eval_model_source")
                    .and_then(Value::as_str)
                    .unwrap_or("queue_item"),
                80,
            );
            let (eval_model, eval_model_source) = if queue_eval_model.is_empty() {
                dashboard_troubleshooting_resolve_eval_model(None)
            } else {
                (queue_eval_model, queue_eval_model_source)
            };
            let report = dashboard_troubleshooting_generate_eval_report(
                &snapshot,
                source,
                &eval_model,
                &eval_model_source,
            );
            append_jsonl(
                &root.join(DASHBOARD_TROUBLESHOOTING_EVAL_HISTORY_REL),
                &report,
            );
            write_json(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL), &report);
            reports.push(report);
        } else {
            remaining.push(row);
        }
    }
    dashboard_troubleshooting_write_eval_queue(root, &remaining);
    json!({
        "ok": true,
        "type": "dashboard_troubleshooting_eval_drain",
        "processed_count": reports.len(),
        "queue_depth_after": remaining.len(),
        "reports": reports
    })
}
fn dashboard_troubleshooting_clear_active_context(root: &Path, reason: &str) {
    dashboard_troubleshooting_write_recent_entries(root, &[]);
    dashboard_troubleshooting_write_eval_queue(root, &[]);
    dashboard_troubleshooting_write_issue_outbox(root, &[]);
    let mut marker = json!({
        "ok": true,
        "type": "dashboard_troubleshooting_clear_marker",
        "ts": now_iso(),
        "reason": clean_text(reason, 120)
    });
    marker["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&marker));
    append_jsonl(
        &root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_HISTORY_REL),
        &marker,
    );
}
