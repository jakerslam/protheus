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
fn dashboard_troubleshooting_enqueue_eval(
    root: &Path,
    snapshot: &Value,
    reason: &str,
    eval_model_hint: Option<&str>,
) -> Value {
    let mut queue = dashboard_troubleshooting_read_eval_queue(root);
    let reason_clean = clean_text(reason, 60);
    let snapshot_id = clean_text(
        snapshot
            .get("snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        120,
    );
    if let Some(existing) = queue.iter().find(|row| {
        clean_text(row.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase()
            == "queued"
            && clean_text(
                row.get("reason").and_then(Value::as_str).unwrap_or(""),
                60,
            ) == reason_clean
            && clean_text(
                row.pointer("/snapshot/snapshot_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            ) == snapshot_id
    }) {
        let mut deduped = existing.clone();
        if let Some(obj) = deduped.as_object_mut() {
            obj.insert("deduped".to_string(), json!(true));
        }
        return deduped;
    }
    let (eval_model, eval_model_source) = if let Some(raw) = eval_model_hint {
        let cleaned = clean_text(raw, 120);
        if cleaned.is_empty() {
            dashboard_troubleshooting_resolve_eval_model(Some(root), None)
        } else {
            (cleaned, "payload".to_string())
        }
    } else {
        dashboard_troubleshooting_resolve_eval_model(Some(root), None)
    };
    let queue_priority = match reason_clean.as_str() {
        "user_report" => 100,
        "auto_failure" => 80,
        _ => 50,
    };
    let item = json!({
        "id": format!(
            "evalq_{}",
            &crate::v8_kernel::sha256_hex_str(&format!(
                "{}:{}:{}",
                now_iso(),
                snapshot.get("snapshot_id").and_then(Value::as_str).unwrap_or("unknown"),
                reason_clean
            ))[..12]
        ),
        "status": "queued",
        "reason": reason_clean,
        "priority": queue_priority,
        "created_at": now_iso(),
        "snapshot": snapshot.clone(),
        "eval_model": eval_model,
        "eval_model_source": eval_model_source
    });
    queue.push(item.clone());
    queue.sort_by(|a, b| {
        let ap = a.get("priority").and_then(Value::as_i64).unwrap_or(0);
        let bp = b.get("priority").and_then(Value::as_i64).unwrap_or(0);
        bp.cmp(&ap)
    });
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
