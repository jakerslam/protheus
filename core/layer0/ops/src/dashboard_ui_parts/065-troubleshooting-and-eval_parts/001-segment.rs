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
fn dashboard_troubleshooting_bootstrap_entry_from_action(
    action_row: &Value,
    source_sequence: i64,
    previous_summary: &str,
) -> Value {
    let ts = clean_text(
        action_row.get("ts").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let lane_ok = action_row
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let lane_status = action_row
        .get("lane_status")
        .and_then(Value::as_i64)
        .unwrap_or(if lane_ok { 0 } else { 1 });
    let payload = action_row.get("payload").cloned().unwrap_or_else(|| json!({}));
    let input = clean_text(
        payload
            .get("input")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.get("prompt"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        2_000,
    );
    let argv_joined = action_row
        .get("lane_argv")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|raw| clean_text(raw, 120))
                .filter(|raw| !raw.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let agent_id = payload
        .get("agent_id")
        .or_else(|| payload.get("agentId"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 140))
        .filter(|raw| !raw.is_empty())
        .or_else(|| {
            argv_joined
                .split_whitespace()
                .find_map(|token| token.strip_prefix("--session-id="))
                .map(|raw| clean_text(raw, 140))
                .filter(|raw| !raw.is_empty())
        })
        .unwrap_or_else(|| "chat-ui-default-agent".to_string());
    let normalized_error = if lane_ok {
        String::new()
    } else {
        "bootstrap_action_lane_failed".to_string()
    };
    let mut out = json!({
        "ok": true,
        "type": "dashboard_troubleshooting_recent_workflow_entry",
        "ts": if ts.is_empty() { now_iso() } else { ts.clone() },
        "source": "action_history_bootstrap",
        "source_sequence": source_sequence.max(1),
        "age_seconds": 0.0,
        "stale": false,
        "agent_id": agent_id,
        "input": input,
        "lane_ok": lane_ok,
        "requires_live_web": false,
        "workflow": {
            "route": "task",
            "classification": "bootstrap_from_action_receipt",
            "transaction_status": if lane_ok { "completed" } else { "failed" },
            "error_code": normalized_error,
            "signal": if lane_ok { "completion" } else { "error" },
            "completion_signal": if lane_ok { "completion_result" } else { "api_req_failed" },
            "completion_signal_reason": if lane_ok { "bootstrap_action_receipt" } else { "bootstrap_action_failure_receipt" }
        },
        "tooling": {
            "tool_call_count": 0,
            "tool_call_unique_count": 0,
            "tool_call_duplicate_count": 0,
            "tool_call_keys": [],
            "tool_receipts": []
        },
        "process_summary": {
            "previous": clean_text(previous_summary, 360),
            "current": if lane_ok {
                "Bootstrap captured an app.chat workflow receipt for eval/troubleshooting continuity."
            } else {
                "Bootstrap captured a failed app.chat workflow receipt; eval troubleshooting is queued."
            },
            "delta": if lane_ok {
                "Bootstrap seeded troubleshooting context from action history."
            } else {
                "Bootstrap detected a failed app.chat action receipt."
            }
        },
        "loop_detection": dashboard_troubleshooting_loop_detection(
            if lane_ok { 1 } else { DASHBOARD_TROUBLESHOOTING_LOOP_WARNING_REPEAT_COUNT },
            if lane_ok { "completion" } else { "error" },
            if lane_ok { "" } else { "bootstrap_action_lane_failed" }
        ),
        "lineage": {
            "action_receipt_hash": clean_text(
                action_row.get("receipt_hash").and_then(Value::as_str).unwrap_or(""),
                160
            )
        }
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
fn dashboard_troubleshooting_seed_recent_from_action_history(root: &Path) {
    let recent_path = root.join(DASHBOARD_TROUBLESHOOTING_RECENT_REL);
    if recent_path.exists() {
        return;
    }
    let action_history_path = root.join(ACTION_HISTORY_REL);
    let raw = fs::read_to_string(&action_history_path).unwrap_or_default();
    let mut receipts = Vec::<Value>::new();
    for line in raw.lines().rev() {
        let parsed = parse_json_loose(line.trim()).unwrap_or_else(|| json!({}));
        let action = clean_text(
            parsed.get("action").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if action != "app.chat" {
            continue;
        }
        receipts.push(parsed);
        if receipts.len() >= DASHBOARD_TROUBLESHOOTING_MAX_RECENT {
            break;
        }
    }
    receipts.reverse();
    let mut entries = Vec::<Value>::new();
    let mut previous_summary = String::new();
    for (idx, row) in receipts.iter().enumerate() {
        let entry = dashboard_troubleshooting_bootstrap_entry_from_action(
            row,
            (idx as i64) + 1,
            &previous_summary,
        );
        previous_summary = clean_text(
            entry
                .pointer("/process_summary/current")
                .and_then(Value::as_str)
                .unwrap_or(""),
            360,
        );
        entries.push(entry);
    }
    dashboard_troubleshooting_write_recent_entries(root, &entries);
}
fn dashboard_troubleshooting_write_idle_eval_report_if_missing(root: &Path, reason: &str) {
    let path = root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL);
    if path.exists() {
        return;
    }
    let entries = dashboard_troubleshooting_read_recent_entries(root);
    let mut out = json!({
        "ok": true,
        "type": "dashboard_troubleshooting_eval_report",
        "ts": now_iso(),
        "status": "idle",
        "reason": clean_text(reason, 120),
        "model": DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
        "model_source": "strong_default_bootstrap",
        "strong_default_model": DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL,
        "entry_count": entries.len(),
        "issues": [],
        "summary": "Eval runtime is initialized and waiting for failure snapshots."
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    write_json(&path, &out);
}
fn dashboard_troubleshooting_bootstrap_runtime_activation(root: &Path, snapshot: &Value) {
    dashboard_troubleshooting_seed_recent_from_action_history(root);
    let eval_queue_path = root.join(DASHBOARD_TROUBLESHOOTING_EVAL_QUEUE_REL);
    if !eval_queue_path.exists() {
        dashboard_troubleshooting_write_eval_queue(root, &[]);
    }
    let outbox_path = root.join(DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL);
    if !outbox_path.exists() {
        dashboard_troubleshooting_write_issue_outbox(root, &[]);
    }
    let latest_snapshot_path = root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL);
    if !latest_snapshot_path.exists() {
        let entries = dashboard_troubleshooting_read_recent_entries(root);
        let failure_count = entries
            .iter()
            .filter(|row| dashboard_troubleshooting_exchange_failed(row))
            .count();
        let mut boot = json!({
            "ok": true,
            "type": "dashboard_troubleshooting_snapshot",
            "snapshot_id": format!(
                "snap_{}",
                &crate::v8_kernel::sha256_hex_str(&format!("{}:{}", now_iso(), entries.len()))[..12]
            ),
            "trigger": "runtime_bootstrap",
            "ts": now_iso(),
            "failure_count": failure_count,
            "entry_count": entries.len(),
            "entries": entries,
            "metadata": {
                "source": "snapshot_writer_bootstrap",
                "snapshot_receipt_hash": clean_text(
                    snapshot.get("receipt_hash").and_then(Value::as_str).unwrap_or(""),
                    160
                )
            }
        });
        boot["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&boot));
        write_json(&latest_snapshot_path, &boot);
        append_jsonl(
            &root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_HISTORY_REL),
            &boot,
        );
        if failure_count > 0 {
            let (eval_model, _eval_model_source) =
                dashboard_troubleshooting_resolve_eval_model(Some(root), Some(snapshot));
            let _ = dashboard_troubleshooting_enqueue_eval(
                root,
                &boot,
                "bootstrap_seed_failure",
                Some(eval_model.as_str()),
            );
            let _ = dashboard_troubleshooting_eval_drain_internal(root, 1, "bootstrap_seed");
        }
    }
    dashboard_troubleshooting_write_idle_eval_report_if_missing(root, "runtime_bootstrap");
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
