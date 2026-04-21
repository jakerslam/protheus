fn dashboard_troubleshooting_eval_recommendations(
    top_error: &str,
    top_classification: &str,
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if top_error == "web_called_during_local_intent"
        || top_classification == "local_intent_web_violation"
    {
        out.push("enforce local-intent routing: block web tools when file/workspace intent is detected and fail closed with explicit violation telemetry".to_string());
        out.push("verify response_finalization.tool_selection_authority=llm_controlled and auto_tool_calls_allowed=false for affected turns".to_string());
    }
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
    if top_error.contains("query_shape") || top_error.contains("payload_dump") {
        out.push("block malformed query blobs earlier and request concise user intent before rerunning web tooling".to_string());
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

fn dashboard_troubleshooting_parse_model_param_billion_hint(model_id: &str) -> i64 {
    let lowered = clean_text(model_id, 140).to_ascii_lowercase();
    if lowered.is_empty() {
        return 0;
    }
    let chars: Vec<char> = lowered.chars().collect();
    let mut idx = 0usize;
    while idx < chars.len() {
        if !chars[idx].is_ascii_digit() {
            idx += 1;
            continue;
        }
        let start = idx;
        while idx < chars.len() && chars[idx].is_ascii_digit() {
            idx += 1;
        }
        let has_b_suffix = idx < chars.len() && chars[idx] == 'b';
        if has_b_suffix {
            let digits: String = chars[start..idx].iter().collect();
            if let Ok(parsed) = digits.parse::<i64>() {
                if parsed > 0 {
                    return parsed;
                }
            }
        }
        idx += 1;
    }
    0
}

fn dashboard_troubleshooting_eval_model_meets_threshold(model: &str, params_billion: i64) -> bool {
    let normalized = clean_text(model, 140).to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let inferred_params = params_billion
        .max(dashboard_troubleshooting_parse_model_param_billion_hint(&normalized))
        .max(0);
    if inferred_params >= DASHBOARD_TROUBLESHOOTING_EVAL_MIN_PARAMS_BILLION {
        return true;
    }
    normalized.starts_with("gpt-5") || normalized.starts_with("o3") || normalized.starts_with("o4")
}

fn dashboard_troubleshooting_parse_i64_value(value: &Value) -> i64 {
    if let Some(v) = value.as_i64() {
        return v.max(0);
    }
    if let Some(v) = value.as_f64() {
        return (v.round() as i64).max(0);
    }
    if let Some(v) = value.as_str() {
        return clean_text(v, 32).parse::<i64>().unwrap_or(0).max(0);
    }
    0
}

fn dashboard_troubleshooting_extract_chat_scope(payload: &Value) -> String {
    [
        "/session_id",
        "/sessionId",
        "/chat_id",
        "/chatId",
        "/thread_id",
        "/threadId",
        "/conversation_id",
        "/conversationId",
        "/runtime_block/session_id",
        "/runtime_block/chat_id",
        "/runtime_block/thread_id",
        "/metadata/session_id",
        "/metadata/chat_id",
        "/metadata/thread_id",
    ]
    .iter()
    .find_map(|pointer| payload.pointer(pointer).and_then(Value::as_str))
    .map(|raw| clean_text(raw, 180))
    .unwrap_or_default()
}

fn dashboard_troubleshooting_row_matches_chat_scope(row: &Value, chat_scope: &str) -> bool {
    let expected = clean_text(chat_scope, 180);
    if expected.is_empty() {
        return true;
    }
    [
        "/chat_scope",
        "/workflow/chat_scope",
        "/session_id",
        "/sessionId",
        "/chat_id",
        "/chatId",
        "/thread_id",
        "/threadId",
        "/conversation_id",
        "/conversationId",
        "/workflow/session_id",
        "/workflow/chat_id",
        "/workflow/thread_id",
        "/metadata/session_id",
        "/metadata/chat_id",
        "/metadata/thread_id",
    ]
    .iter()
    .filter_map(|pointer| row.pointer(pointer).and_then(Value::as_str))
    .map(|raw| clean_text(raw, 180))
    .any(|candidate| candidate == expected)
}

fn dashboard_troubleshooting_recent_viable_chat_model(
    root: &Path,
    chat_scope: Option<&str>,
) -> Option<(String, i64)> {
    let scope = chat_scope.map(|raw| clean_text(raw, 180)).unwrap_or_default();

    let entries = dashboard_troubleshooting_read_recent_entries(root);
    for row in entries.iter().rev() {
        if !scope.is_empty() && !dashboard_troubleshooting_row_matches_chat_scope(row, &scope) {
            continue;
        }
        let model = [
            "/workflow/model",
            "/workflow/model_id",
            "/workflow/model_override",
            "/workflow/final_model",
            "/response_workflow/model",
            "/response_workflow/model_id",
            "/model",
        ]
        .iter()
        .find_map(|pointer| row.pointer(pointer).and_then(Value::as_str))
        .map(|raw| clean_text(raw, 140))
        .unwrap_or_default();
        if model.is_empty() {
            continue;
        }
        let params_billion = [
            "/workflow/model_params_billion",
            "/workflow/param_count_billion",
            "/workflow/model_profile/param_count_billion",
            "/workflow/model_profile/params_billion",
            "/response_workflow/model_profile/param_count_billion",
            "/response_workflow/model_profile/params_billion",
            "/model_params_billion",
            "/param_count_billion",
        ]
        .iter()
        .find_map(|pointer| row.pointer(pointer))
        .map(dashboard_troubleshooting_parse_i64_value)
        .unwrap_or(0)
        .max(dashboard_troubleshooting_parse_model_param_billion_hint(&model))
        .max(0);
        if dashboard_troubleshooting_eval_model_meets_threshold(&model, params_billion) {
            return Some((model, params_billion));
        }
    }
    None
}

fn dashboard_troubleshooting_eval_model_strength(model: &str) -> &'static str {
    if dashboard_troubleshooting_eval_model_meets_threshold(model, 0) {
        "strong"
    } else {
        "custom"
    }
}
fn dashboard_troubleshooting_resolve_eval_model(
    root: Option<&Path>,
    payload: Option<&Value>,
) -> (String, String) {
    let payload_chat_scope =
        payload.map(dashboard_troubleshooting_extract_chat_scope).filter(|v| !v.is_empty());
    if let Some(args) = payload {
        for key in ["eval_model", "evalModel", "llm_model", "llmModel", "model"] {
            let candidate = clean_text(args.get(key).and_then(Value::as_str).unwrap_or(""), 120);
            let params_billion = args
                .get("model_params_billion")
                .or_else(|| args.get("param_count_billion"))
                .map(dashboard_troubleshooting_parse_i64_value)
                .unwrap_or(0)
                .max(dashboard_troubleshooting_parse_model_param_billion_hint(&candidate))
                .max(0);
            if !candidate.is_empty()
                && dashboard_troubleshooting_eval_model_meets_threshold(&candidate, params_billion)
            {
                return (candidate, "payload".to_string());
            }
        }
    }
    if let Some(state_root) = root {
        if let Some(scope) = payload_chat_scope.as_deref() {
            if let Some((model, params_billion)) =
                dashboard_troubleshooting_recent_viable_chat_model(state_root, Some(scope))
            {
                let source = if params_billion > 0 {
                    format!("recent_viable_chat_model_scoped_{params_billion}b")
                } else {
                    "recent_viable_chat_model_scoped".to_string()
                };
                return (model, source);
            }
        } else if let Some((model, params_billion)) =
            dashboard_troubleshooting_recent_viable_chat_model(state_root, None)
        {
            let source = if params_billion > 0 {
                format!("recent_viable_chat_model_{params_billion}b")
            } else {
                "recent_viable_chat_model".to_string()
            };
            return (model, source);
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
    let mut stale_count = 0i64;
    let mut web_required_without_calls_count = 0i64;
    let mut web_called_during_local_intent_count = 0i64;
    for row in &entries {
        if dashboard_troubleshooting_exchange_failed(row) {
            failure_count += 1;
        }
        if row.get("stale").and_then(Value::as_bool).unwrap_or(false) {
            stale_count += 1;
        }
        let requires_live_web = row
            .pointer("/workflow/requires_live_web")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let tool_calls = row
            .pointer("/workflow/tool_calls")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if requires_live_web && tool_calls <= 0 {
            web_required_without_calls_count += 1;
        }
        let web_called_during_local_intent = row
            .pointer("/workflow/web_called_during_local_intent")
            .or_else(|| row.pointer("/finalization/web_invariant/web_called_during_local_intent"))
            .or_else(|| row.pointer("/response_finalization/web_invariant/web_called_during_local_intent"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if web_called_during_local_intent {
            web_called_during_local_intent_count += 1;
            *error_counts
                .entry("web_called_during_local_intent".to_string())
                .or_insert(0) += 1;
            *class_counts
                .entry("local_intent_web_violation".to_string())
                .or_insert(0) += 1;
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
        "stale_count": stale_count,
        "web_required_without_calls_count": web_required_without_calls_count,
        "web_called_during_local_intent_count": web_called_during_local_intent_count,
        "eval": {
            "engine": "troubleshooting_eval_v1",
            "model": model,
            "model_source": clean_text(eval_model_source, 80),
            "model_strength": dashboard_troubleshooting_eval_model_strength(eval_model),
            "strong_default_model": DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
            "llm_required": true
        },
        "summary": summary,
        "exchange_health": {
            "total_entries": entries.len(),
            "stale_ratio": if entries.is_empty() { 0.0 } else { (stale_count as f64) / (entries.len() as f64) },
            "web_required_without_calls_ratio": if entries.is_empty() { 0.0 } else { (web_required_without_calls_count as f64) / (entries.len() as f64) },
            "web_called_during_local_intent_ratio": if entries.is_empty() { 0.0 } else { (web_called_during_local_intent_count as f64) / (entries.len() as f64) }
        },
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
                dashboard_troubleshooting_resolve_eval_model(Some(root), None)
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
