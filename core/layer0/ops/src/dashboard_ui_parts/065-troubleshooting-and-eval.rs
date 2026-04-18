const DASHBOARD_TROUBLESHOOTING_RECENT_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/recent_workflows.json";
const DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_snapshot.json";
const DASHBOARD_TROUBLESHOOTING_SNAPSHOT_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/snapshot_history.jsonl";
const DASHBOARD_TROUBLESHOOTING_EVAL_QUEUE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_queue.json";
const DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_eval_report.json";
const DASHBOARD_TROUBLESHOOTING_EVAL_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_reports.jsonl";
const DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/issue_outbox.json";
const DASHBOARD_TROUBLESHOOTING_MAX_RECENT: usize = 10;
const DASHBOARD_TROUBLESHOOTING_MAX_QUEUE: usize = 500;
const DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX: usize = 300;

fn dashboard_troubleshooting_read_items(path: &Path, key: &str) -> Vec<Value> {
    read_json_file(path)
        .and_then(|value| value.get(key).and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

fn dashboard_troubleshooting_write_items(path: &Path, key: &str, items: &[Value], kind: &str) {
    let mut out = serde_json::Map::<String, Value>::new();
    out.insert("ok".to_string(), json!(true));
    out.insert("type".to_string(), json!(kind));
    out.insert("ts".to_string(), json!(now_iso()));
    out.insert(key.to_string(), Value::Array(items.to_vec()));
    let mut row = Value::Object(out);
    row["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&row));
    write_json(path, &row);
}

fn dashboard_troubleshooting_read_recent_entries(root: &Path) -> Vec<Value> {
    dashboard_troubleshooting_read_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_RECENT_REL),
        "entries",
    )
}

fn dashboard_troubleshooting_write_recent_entries(root: &Path, entries: &[Value]) {
    dashboard_troubleshooting_write_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_RECENT_REL),
        "entries",
        entries,
        "dashboard_troubleshooting_recent_workflows",
    );
}

fn dashboard_troubleshooting_read_eval_queue(root: &Path) -> Vec<Value> {
    dashboard_troubleshooting_read_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_EVAL_QUEUE_REL),
        "items",
    )
}

fn dashboard_troubleshooting_write_eval_queue(root: &Path, items: &[Value]) {
    dashboard_troubleshooting_write_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_EVAL_QUEUE_REL),
        "items",
        items,
        "dashboard_troubleshooting_eval_queue",
    );
}

fn dashboard_troubleshooting_read_issue_outbox(root: &Path) -> Vec<Value> {
    dashboard_troubleshooting_read_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL),
        "items",
    )
}

fn dashboard_troubleshooting_write_issue_outbox(root: &Path, items: &[Value]) {
    dashboard_troubleshooting_write_items(
        &root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL),
        "items",
        items,
        "dashboard_troubleshooting_issue_outbox",
    );
}

fn dashboard_troubleshooting_web_call_count(tools: &[Value]) -> usize {
    tools.iter()
        .filter(|row| {
            let name = clean_text(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            name.contains("web") || name.contains("batch_query")
        })
        .count()
}

fn dashboard_troubleshooting_lane_error_code(lane_payload: &Value) -> String {
    clean_text(
        lane_payload
            .get("error")
            .and_then(Value::as_str)
            .or_else(|| {
                lane_payload
                    .pointer("/response_finalization/error_code")
                    .and_then(Value::as_str)
            })
            .or_else(|| {
                lane_payload
                    .pointer("/response_finalization/classification_guard/active_error_code")
                    .and_then(Value::as_str)
            })
            .unwrap_or(""),
        120,
    )
    .to_ascii_lowercase()
}

fn dashboard_troubleshooting_compact_tool_receipts(tools: &[Value]) -> Vec<Value> {
    tools.iter()
        .take(12)
        .map(|row| {
            json!({
                "name": clean_text(
                    row.get("name").or_else(|| row.get("tool")).and_then(Value::as_str).unwrap_or("unknown"),
                    80
                ),
                "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or("unknown"), 80),
                "error": clean_text(row.get("error").and_then(Value::as_str).unwrap_or(""), 160),
                "query": clean_text(row.get("query").and_then(Value::as_str).unwrap_or(""), 240)
            })
        })
        .collect()
}

fn dashboard_troubleshooting_latest_process_summary(root: &Path) -> String {
    dashboard_troubleshooting_read_recent_entries(root)
        .last()
        .and_then(|row| row.pointer("/process_summary/current").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 360))
        .unwrap_or_default()
}

fn dashboard_troubleshooting_exchange_failed(exchange: &Value) -> bool {
    if !exchange
        .get("lane_ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return true;
    }
    let error_code = clean_text(
        exchange
            .pointer("/workflow/error_code")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if !error_code.is_empty() {
        return true;
    }
    let status = clean_text(
        exchange
            .pointer("/workflow/transaction_status")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    matches!(status.as_str(), "failed" | "degraded")
}

fn dashboard_troubleshooting_capture_snapshot(root: &Path, trigger: &str, metadata: &Value) -> Value {
    let entries = dashboard_troubleshooting_read_recent_entries(root);
    let failure_count = entries
        .iter()
        .filter(|row| dashboard_troubleshooting_exchange_failed(row))
        .count() as i64;
    let snapshot_id = format!(
        "snap_{}",
        &crate::v8_kernel::sha256_hex_str(&format!(
            "{}:{}:{}",
            now_iso(),
            clean_text(trigger, 60),
            failure_count
        ))[..12]
    );
    let mut snapshot = json!({
        "ok": true,
        "type": "dashboard_troubleshooting_snapshot",
        "snapshot_id": snapshot_id,
        "trigger": clean_text(trigger, 60),
        "ts": now_iso(),
        "failure_count": failure_count,
        "entry_count": entries.len(),
        "entries": entries,
        "metadata": metadata.clone()
    });
    snapshot["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&snapshot));
    write_json(
        &root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL),
        &snapshot,
    );
    append_jsonl(
        &root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_HISTORY_REL),
        &snapshot,
    );
    snapshot
}

fn dashboard_troubleshooting_enqueue_eval(root: &Path, snapshot: &Value, reason: &str) -> Value {
    let mut queue = dashboard_troubleshooting_read_eval_queue(root);
    let item = json!({
        "id": format!(
            "evalq_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}:{}",
                now_iso(),
                snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or("unknown"),
                clean_text(reason, 60)
            ))[..12]
        ),
        "status": "queued",
        "reason": clean_text(reason, 60),
        "created_at": now_iso(),
        "snapshot": snapshot.clone()
    });
    queue.push(item.clone());
    if queue.len() > DASHBOARD_TROUBLESHOOTING_MAX_QUEUE {
        let keep_from = queue.len() - DASHBOARD_TROUBLESHOOTING_MAX_QUEUE;
        queue = queue.split_off(keep_from);
    }
    dashboard_troubleshooting_write_eval_queue(root, &queue);
    item
}

fn dashboard_troubleshooting_sorted_histogram(map: HashMap<String, i64>, key: &str) -> Vec<Value> {
    let mut rows = map.into_iter().collect::<Vec<_>>();
    rows.sort_by_key(|(_, count)| Reverse(*count));
    rows.into_iter()
        .take(8)
        .map(|(value, count)| {
            let mut out = serde_json::Map::<String, Value>::new();
            out.insert(key.to_string(), json!(value));
            out.insert("count".to_string(), json!(count));
            Value::Object(out)
        })
        .collect()
}

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

fn dashboard_troubleshooting_generate_eval_report(snapshot: &Value, source: &str) -> Value {
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
            let report = dashboard_troubleshooting_generate_eval_report(&snapshot, source);
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

fn dashboard_troubleshooting_issue_request_from_report(
    payload: &Value,
    snapshot: &Value,
    eval_report: &Value,
) -> Value {
    let source = clean_text(
        payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("dashboard_report_message"),
        80,
    );
    let title_hint = clean_text(
        payload
            .get("title")
            .and_then(Value::as_str)
            .or_else(|| eval_report.get("summary").and_then(Value::as_str))
            .unwrap_or("Dashboard troubleshooting report"),
        110,
    );
    let snapshot_id = clean_text(
        snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or("unknown"),
        80,
    );
    let report_id = clean_text(
        eval_report.get("report_id").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let eval_summary = clean_text(
        eval_report.get("summary").and_then(Value::as_str).unwrap_or(""),
        1600,
    );
    let recent_summaries = snapshot
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .rev()
                .take(5)
                .filter_map(|row| row.pointer("/process_summary/current").and_then(Value::as_str))
                .map(|raw| format!("- {}", clean_text(raw, 280)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let user_note = clean_text(
        payload
            .get("note")
            .or_else(|| payload.get("description"))
            .or_else(|| payload.get("body"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        3000,
    );
    let body = format!(
        "source: {source}\nsnapshot_id: {snapshot_id}\neval_report_id: {report_id}\n\nsummary:\n{eval_summary}\n\nrecent_process_summaries:\n{recent_summaries}\n\nuser_note:\n{user_note}"
    );
    let mut request = json!({
        "title": title_hint,
        "body": body,
        "source": source
    });
    if let Some(obj) = request.as_object_mut() {
        if let Some(owner) = payload.get("owner").and_then(Value::as_str) {
            let owner_clean = clean_text(owner, 120);
            if !owner_clean.is_empty() {
                obj.insert("owner".to_string(), json!(owner_clean));
            }
        }
        if let Some(repo) = payload.get("repo").and_then(Value::as_str) {
            let repo_clean = clean_text(repo, 120);
            if !repo_clean.is_empty() {
                obj.insert("repo".to_string(), json!(repo_clean));
            }
        }
    }
    request
}

fn dashboard_troubleshooting_enqueue_outbox(
    root: &Path,
    issue_request: &Value,
    issue_lane: &LaneResult,
    snapshot: &Value,
    eval_report: &Value,
) -> Value {
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    let outbox_row = json!({
        "id": format!(
            "outbox_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}",
                now_iso(),
                clean_text(
                    issue_request.get("title").and_then(Value::as_str).unwrap_or("issue"),
                    120
                )
            ))[..12]
        ),
        "created_at": now_iso(),
        "attempts": 0,
        "snapshot_id": clean_text(snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or(""), 80),
        "eval_report_id": clean_text(eval_report.get("report_id").and_then(Value::as_str).unwrap_or(""), 80),
        "issue_request": issue_request.clone(),
        "last_error": issue_lane.payload.clone().unwrap_or_else(|| json!({})),
        "last_status": issue_lane.status
    });
    items.push(outbox_row.clone());
    if items.len() > DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX {
        let keep_from = items.len() - DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX;
        items = items.split_off(keep_from);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &items);
    outbox_row
}

fn dashboard_payload_usize(value: &Value, key: &str, fallback: usize, min: usize, max: usize) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|raw| (raw as usize).clamp(min, max))
        .unwrap_or(fallback)
}

fn dashboard_troubleshooting_capture_chat_exchange(
    root: &Path,
    agent_id: &str,
    raw_input: &str,
    lane_payload: &Value,
    lane_ok: bool,
    requires_live_web: bool,
) -> Value {
    let previous_summary = dashboard_troubleshooting_latest_process_summary(root);
    let tools = lane_payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tool_calls = dashboard_troubleshooting_web_call_count(&tools);
    let route = clean_text(
        lane_payload
            .pointer("/response_workflow/gates/route/route")
            .and_then(Value::as_str)
            .unwrap_or(if requires_live_web { "task" } else { "info" }),
        40,
    )
    .to_ascii_lowercase();
    let classification = clean_text(
        lane_payload
            .pointer("/response_finalization/tool_transaction/classification")
            .and_then(Value::as_str)
            .or_else(|| {
                lane_payload
                    .pointer("/response_finalization/web_invariant/classification")
                    .and_then(Value::as_str)
            })
            .unwrap_or("unknown"),
        80,
    )
    .to_ascii_lowercase();
    let transaction_status = clean_text(
        lane_payload
            .pointer("/response_finalization/tool_transaction/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        80,
    )
    .to_ascii_lowercase();
    let error_code = dashboard_troubleshooting_lane_error_code(lane_payload);
    let hard_guard_applied = lane_payload
        .pointer("/response_finalization/hard_guard/applied")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let assistant = clean_text(
        lane_payload
            .get("response")
            .and_then(Value::as_str)
            .or_else(|| lane_payload.get("output").and_then(Value::as_str))
            .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
            .unwrap_or(""),
        8000,
    );
    let stage_receipts = vec![
        json!({"stage":"route","status":"ok","result":{"route":route}}),
        json!({"stage":"analysis","status":"ok","result":{"requires_live_web":requires_live_web}}),
        json!({"stage":"tool_selection","status":"ok","result":{"tool_calls":tool_calls}}),
        json!({
            "stage":"tool_execution",
            "status": if requires_live_web && tool_calls == 0 { "failed" } else { transaction_status.as_str() },
            "result":{"classification":classification}
        }),
        json!({"stage":"synthesis","status": if assistant.is_empty() { "failed" } else { "ok" }}),
        json!({"stage":"coherence_check","status": if hard_guard_applied { "failed" } else { "ok" }}),
        json!({"stage":"final_output","status": if lane_ok { "ok" } else { "failed" }}),
    ];
    let current_summary = clean_text(
        &format!(
            "route={route}; tools={tool_calls}; classification={classification}; txn_status={transaction_status}; lane_ok={lane_ok}; error={}",
            if error_code.is_empty() { "none" } else { &error_code }
        ),
        360,
    );
    let mut exchange = json!({
        "id": format!(
            "trb_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}:{}",
                now_iso(),
                clean_text(agent_id, 140),
                clean_text(raw_input, 200)
            ))[..12]
        ),
        "type": "dashboard_workflow_exchange_trace",
        "ts": now_iso(),
        "agent_id": clean_text(agent_id, 140),
        "input": clean_text(raw_input, 4000),
        "assistant": assistant,
        "lane_ok": lane_ok,
        "workflow": {
            "requires_live_web": requires_live_web,
            "tool_calls": tool_calls,
            "classification": classification,
            "transaction_status": transaction_status,
            "hard_guard_applied": hard_guard_applied,
            "error_code": error_code
        },
        "stage_receipts": stage_receipts,
        "tool_receipts": dashboard_troubleshooting_compact_tool_receipts(&tools),
        "process_summary": {
            "previous": previous_summary,
            "current": current_summary
        },
        "finalization": {
            "outcome": clean_text(
                lane_payload.pointer("/response_finalization/outcome").and_then(Value::as_str).unwrap_or(""),
                120
            ),
            "tool_transaction": lane_payload.pointer("/response_finalization/tool_transaction").cloned().unwrap_or_else(|| json!({})),
            "web_invariant": lane_payload.pointer("/response_finalization/web_invariant").cloned().unwrap_or_else(|| json!({})),
            "hard_guard": lane_payload.pointer("/response_finalization/hard_guard").cloned().unwrap_or_else(|| json!({}))
        }
    });
    exchange["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&exchange));

    let mut entries = dashboard_troubleshooting_read_recent_entries(root);
    entries.push(exchange.clone());
    if entries.len() > DASHBOARD_TROUBLESHOOTING_MAX_RECENT {
        let keep_from = entries.len() - DASHBOARD_TROUBLESHOOTING_MAX_RECENT;
        entries = entries.split_off(keep_from);
    }
    dashboard_troubleshooting_write_recent_entries(root, &entries);

    let failure_detected = dashboard_troubleshooting_exchange_failed(&exchange);
    let mut snapshot = Value::Null;
    let mut queue_item = Value::Null;
    let mut eval_drain = Value::Null;
    if failure_detected {
        snapshot = dashboard_troubleshooting_capture_snapshot(
            root,
            "auto_failure",
            &json!({
                "agent_id": clean_text(agent_id, 140),
                "input_preview": clean_text(raw_input, 220)
            }),
        );
        queue_item = dashboard_troubleshooting_enqueue_eval(root, &snapshot, "auto_failure");
        eval_drain = dashboard_troubleshooting_eval_drain_internal(root, 1, "auto_failure");
    }

    json!({
        "recorded": true,
        "exchange_id": clean_text(exchange.get("id").and_then(Value::as_str).unwrap_or(""), 80),
        "failure_detected": failure_detected,
        "snapshot_id": clean_text(snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or(""), 80),
        "process_summary": {
            "previous": exchange.pointer("/process_summary/previous").cloned().unwrap_or(Value::Null),
            "current": exchange.pointer("/process_summary/current").cloned().unwrap_or(Value::Null)
        },
        "eval_queue_item": queue_item,
        "eval_drain": eval_drain
    })
}

fn dashboard_troubleshooting_state_lane(root: &Path, payload: &Value) -> LaneResult {
    let limit = dashboard_payload_usize(payload, "limit", 10, 1, 50);
    let mut entries = dashboard_troubleshooting_read_recent_entries(root);
    if entries.len() > limit {
        let keep_from = entries.len() - limit;
        entries = entries.split_off(keep_from);
    }
    let eval_queue = dashboard_troubleshooting_read_eval_queue(root);
    let outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let latest_snapshot = read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
        .unwrap_or_else(|| json!({}));
    let latest_eval = read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
        .unwrap_or_else(|| json!({}));
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_state",
            "recent": {
                "count": entries.len(),
                "entries": entries
            },
            "latest_snapshot": latest_snapshot,
            "latest_eval_report": latest_eval,
            "eval_queue": {
                "depth": eval_queue.len(),
                "items": eval_queue.into_iter().take(limit).collect::<Vec<_>>()
            },
            "issue_outbox": {
                "depth": outbox.len(),
                "items": outbox.into_iter().take(limit).collect::<Vec<_>>()
            }
        })),
    }
}

fn dashboard_troubleshooting_eval_drain_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 5, 1, 50);
    let out = dashboard_troubleshooting_eval_drain_internal(root, max_items, "manual");
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.eval.drain".to_string()],
        payload: Some(out),
    }
}

fn dashboard_troubleshooting_outbox_flush_lane(root: &Path, payload: &Value) -> LaneResult {
    let max_items = dashboard_payload_usize(payload, "max_items", 10, 1, 50);
    let mut items = dashboard_troubleshooting_read_issue_outbox(root);
    let mut remaining = Vec::<Value>::new();
    let mut submitted = Vec::<Value>::new();
    let mut failed = Vec::<Value>::new();
    for row in items.drain(..) {
        if submitted.len() + failed.len() >= max_items {
            remaining.push(row);
            continue;
        }
        let request = row
            .get("issue_request")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let lane = run_action(root, "dashboard.github.issue.create", &request);
        if lane.ok {
            submitted.push(json!({
                "item_id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80),
                "issue": lane.payload.clone().unwrap_or_else(|| json!({}))
            }));
            dashboard_troubleshooting_clear_active_context(root, "issue_outbox_submission_succeeded");
            continue;
        }
        let mut updated = row.clone();
        if let Some(obj) = updated.as_object_mut() {
            let attempts = obj.get("attempts").and_then(Value::as_i64).unwrap_or(0) + 1;
            obj.insert("attempts".to_string(), json!(attempts));
            obj.insert("last_attempt_at".to_string(), json!(now_iso()));
            obj.insert(
                "last_error".to_string(),
                lane.payload.clone().unwrap_or_else(|| json!({})),
            );
        }
        failed.push(updated.clone());
        remaining.push(updated);
    }
    dashboard_troubleshooting_write_issue_outbox(root, &remaining);
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.outbox.flush".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_outbox_flush",
            "submitted_count": submitted.len(),
            "failed_count": failed.len(),
            "remaining_depth": remaining.len(),
            "submitted": submitted,
            "failed": failed
        })),
    }
}

fn dashboard_troubleshooting_report_message_lane(root: &Path, payload: &Value) -> LaneResult {
    let snapshot = dashboard_troubleshooting_capture_snapshot(
        root,
        "user_report",
        &json!({
            "source": clean_text(payload.get("source").and_then(Value::as_str).unwrap_or("dashboard_report_message"), 80),
            "session_id": clean_text(payload.get("session_id").or_else(|| payload.get("sessionId")).and_then(Value::as_str).unwrap_or(""), 160),
            "message_id": clean_text(payload.get("message_id").or_else(|| payload.get("messageId")).and_then(Value::as_str).unwrap_or(""), 160)
        }),
    );
    let queue_item = dashboard_troubleshooting_enqueue_eval(root, &snapshot, "user_report");
    let eval_drain = dashboard_troubleshooting_eval_drain_internal(root, 1, "user_report");
    let eval_report = eval_drain
        .get("reports")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .cloned()
        .or_else(|| read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL)))
        .unwrap_or_else(|| json!({}));
    let issue_request = dashboard_troubleshooting_issue_request_from_report(payload, &snapshot, &eval_report);
    let issue_lane = run_action(root, "dashboard.github.issue.create", &issue_request);
    if issue_lane.ok {
        dashboard_troubleshooting_clear_active_context(root, "issue_submission_succeeded");
        return LaneResult {
            ok: true,
            status: 0,
            argv: vec!["dashboard.troubleshooting.report_message".to_string()],
            payload: Some(json!({
                "ok": true,
                "type": "dashboard_troubleshooting_report",
                "submitted": true,
                "queued": false,
                "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
                "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
                "issue": issue_lane.payload.unwrap_or_else(|| json!({}))
            })),
        };
    }
    let outbox_item =
        dashboard_troubleshooting_enqueue_outbox(root, &issue_request, &issue_lane, &snapshot, &eval_report);
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.report_message".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_report",
            "submitted": false,
            "queued": true,
            "snapshot_id": snapshot.get("snapshot_id").cloned().unwrap_or(Value::Null),
            "eval_report_id": eval_report.get("report_id").cloned().unwrap_or(Value::Null),
            "queue_item": queue_item,
            "eval_drain": eval_drain,
            "issue_error": issue_lane
                .payload
                .as_ref()
                .and_then(|row| row.get("error"))
                .cloned()
                .unwrap_or_else(|| json!("github_issue_transport_error")),
            "issue_error_hint": "Issue pipeline queued locally; run dashboard.troubleshooting.outbox.flush after auth/pipeline recovery.",
            "outbox_item": outbox_item
        })),
    }
}
