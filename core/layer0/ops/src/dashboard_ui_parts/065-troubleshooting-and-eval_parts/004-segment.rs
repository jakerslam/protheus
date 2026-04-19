// Layer ownership: core/layer0/ops (dashboard_ui_parts troubleshooting)
fn dashboard_troubleshooting_workflow_signal(
    lane_ok: bool,
    error_code: &str,
    hard_guard_applied: bool,
    assistant: &str,
) -> &'static str {
    if !lane_ok || !error_code.is_empty() || hard_guard_applied {
        "error"
    } else if !assistant.trim().is_empty() {
        "completion"
    } else {
        "pending"
    }
}

fn dashboard_troubleshooting_exchange_signature(
    agent_id: &str,
    raw_input: &str,
    classification: &str,
    transaction_status: &str,
    error_code: &str,
    tool_calls: usize,
    workflow_signal: &str,
) -> String {
    crate::v8_kernel::sha256_hex_str(&format!(
        "{}|{}|{}|{}|{}|{}|{}",
        clean_text(agent_id, 140),
        clean_text(raw_input, 300),
        clean_text(classification, 80),
        clean_text(transaction_status, 80),
        clean_text(error_code, 120),
        tool_calls,
        clean_text(workflow_signal, 32),
    ))
}

fn dashboard_troubleshooting_window_slice(
    total_count: usize,
    limit: usize,
    offset_from_latest: usize,
) -> (usize, usize) {
    if total_count == 0 {
        return (0, 0);
    }
    let bounded_offset = offset_from_latest.min(total_count.saturating_sub(1));
    let end_exclusive = total_count.saturating_sub(bounded_offset);
    let bounded_limit = limit.max(1).min(end_exclusive.max(1));
    let start = end_exclusive.saturating_sub(bounded_limit);
    (start, end_exclusive)
}

fn dashboard_troubleshooting_window_contract(
    total_count: usize,
    visible_start: usize,
    visible_end: usize,
    requested_limit: usize,
    offset_from_latest: usize,
) -> Value {
    let visible_count = visible_end.saturating_sub(visible_start);
    let hidden_top_count = visible_start;
    let hidden_bottom_count = total_count.saturating_sub(visible_end);
    let next_offset_from_latest = (offset_from_latest + visible_count)
        .min(total_count.saturating_sub(1));
    let previous_offset_from_latest = offset_from_latest.saturating_sub(visible_count);
    json!({
        "total_count": total_count,
        "requested_limit": requested_limit,
        "offset_from_latest": offset_from_latest,
        "visible_start": visible_start,
        "visible_end_exclusive": visible_end,
        "visible_count": visible_count,
        "hidden_top_count": hidden_top_count,
        "hidden_bottom_count": hidden_bottom_count,
        "hidden_count": hidden_top_count + hidden_bottom_count,
        "show_top_indicator": hidden_top_count > 0,
        "show_bottom_indicator": hidden_bottom_count > 0,
        "next_offset_from_latest": next_offset_from_latest,
        "previous_offset_from_latest": previous_offset_from_latest
    })
}

const DASHBOARD_TROUBLESHOOTING_LOOP_WARNING_REPEAT_COUNT: i64 = 3;
const DASHBOARD_TROUBLESHOOTING_LOOP_CRITICAL_REPEAT_COUNT: i64 = 6;

fn dashboard_troubleshooting_loop_lane(error_code: &str) -> &'static str {
    let normalized = clean_text(error_code, 120).to_ascii_lowercase();
    if normalized.starts_with("web_") {
        "tool_completion"
    } else if normalized.starts_with("gateway_") {
        "lifecycle"
    } else {
        "general"
    }
}

fn dashboard_troubleshooting_loop_recovery_hint(
    level: &str,
    loop_lane: &str,
    error_code: &str,
) -> &'static str {
    if level == "none" {
        return "none";
    }
    if loop_lane == "tool_completion" {
        return if level == "critical" {
            "pause_tool_calls_and_run_targeted_web_tooling_retry_audit"
        } else {
            "narrow_tool_inputs_and_retry_once_with_diagnostics"
        };
    }
    if clean_text(error_code, 120)
        .to_ascii_lowercase()
        .contains("policy")
    {
        return "check_policy_contract_then_retry_with_explicit_override_if_allowed";
    }
    if level == "critical" {
        "capture_snapshot_and_enqueue_eval_before_retry"
    } else {
        "retry_with_single_guarded_attempt"
    }
}

fn dashboard_troubleshooting_loop_detection(
    repeat_count: i64,
    workflow_signal: &str,
    error_code: &str,
) -> Value {
    let normalized_repeat_count = repeat_count.max(1);
    let is_error_lane = workflow_signal == "error" || !error_code.trim().is_empty();
    if !is_error_lane || normalized_repeat_count < DASHBOARD_TROUBLESHOOTING_LOOP_WARNING_REPEAT_COUNT {
        let loop_lane = dashboard_troubleshooting_loop_lane(error_code);
        return json!({
            "detected": false,
            "level": "none",
            "detector": "none",
            "count": normalized_repeat_count,
            "lane": loop_lane,
            "message": "no loop risk detected",
            "recovery_hint": dashboard_troubleshooting_loop_recovery_hint("none", loop_lane, error_code),
            "restart_workflow": false
        });
    }
    let critical = normalized_repeat_count >= DASHBOARD_TROUBLESHOOTING_LOOP_CRITICAL_REPEAT_COUNT;
    let level = if critical { "critical" } else { "warning" };
    let loop_lane = dashboard_troubleshooting_loop_lane(error_code);
    json!({
        "detected": true,
        "level": level,
        "detector": "generic_repeat",
        "count": normalized_repeat_count,
        "lane": loop_lane,
        "message": if critical {
            "repeated failing workflow signature exceeded critical threshold"
        } else {
            "repeated failing workflow signature exceeded warning threshold"
        },
        "recovery_hint": dashboard_troubleshooting_loop_recovery_hint(level, loop_lane, error_code),
        "restart_workflow": critical
    })
}

fn dashboard_troubleshooting_completion_signal(
    lane_ok: bool,
    error_code: &str,
    hard_guard_applied: bool,
    assistant: &str,
) -> (&'static str, &'static str) {
    if !error_code.trim().is_empty() || hard_guard_applied || !lane_ok {
        ("api_req_failed", "error_or_guard")
    } else if !assistant.trim().is_empty() {
        ("completion_result", "assistant_response_present")
    } else {
        ("pending", "awaiting_assistant_output")
    }
}

fn dashboard_troubleshooting_tool_call_key(tool: &Value, fallback_index: usize) -> String {
    let id = clean_text(
        tool.pointer("/result/id")
            .and_then(Value::as_str)
            .or_else(|| tool.pointer("/id").and_then(Value::as_str))
            .unwrap_or(""),
        140,
    );
    if !id.is_empty() {
        return id;
    }
    let name = clean_text(
        tool.pointer("/tool").and_then(Value::as_str).unwrap_or("unknown_tool"),
        80,
    )
    .to_ascii_lowercase();
    let call_index = clean_text(
        tool.pointer("/result/index")
            .and_then(Value::as_i64)
            .map(|v| v.to_string())
            .unwrap_or_default()
            .as_str(),
        20,
    );
    if call_index.is_empty() {
        format!("{name}:{fallback_index}")
    } else {
        format!("{name}:{call_index}")
    }
}

fn dashboard_troubleshooting_tool_call_dedupe_summary(
    tools: &[Value],
) -> (Vec<String>, usize, usize) {
    let mut keys = Vec::<String>::new();
    let mut unique = std::collections::BTreeSet::<String>::new();
    for (idx, tool) in tools.iter().enumerate() {
        let key = dashboard_troubleshooting_tool_call_key(tool, idx);
        keys.push(key.clone());
        unique.insert(key);
    }
    let unique_count = unique.len();
    let duplicate_count = keys.len().saturating_sub(unique_count);
    (keys, unique_count, duplicate_count)
}

fn dashboard_troubleshooting_retry_after_hint_seconds(lane_payload: &Value) -> i64 {
    lane_payload
        .pointer("/response_finalization/tool_transaction/retry/retry_after_seconds")
        .or_else(|| {
            lane_payload.pointer("/response_finalization/web_invariant/retry/retry_after_seconds")
        })
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
}

fn dashboard_troubleshooting_credential_source_hint(lane_payload: &Value) -> &'static str {
    let configured = lane_payload
        .pointer("/response_finalization/tool_transaction/provider_auth_present")
        .or_else(|| lane_payload.pointer("/response_finalization/web_invariant/provider_auth_present"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if configured { "config_or_env" } else { "missing" }
}

fn dashboard_troubleshooting_capture_chat_exchange(
    root: &Path,
    agent_id: &str,
    raw_input: &str,
    lane_payload: &Value,
    lane_ok: bool,
    requires_live_web: bool,
) -> Value {
    let mut entries = dashboard_troubleshooting_read_recent_entries(root);
    let previous_summary = entries
        .last()
        .and_then(|row| row.pointer("/process_summary/current").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 360))
        .unwrap_or_default();
    let prior_sequence = entries
        .last()
        .and_then(|row| row.get("source_sequence"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let tools = lane_payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tool_calls = dashboard_troubleshooting_web_call_count(&tools);
    let (tool_call_keys, tool_call_unique_count, tool_call_duplicate_count) =
        dashboard_troubleshooting_tool_call_dedupe_summary(&tools);
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
    let source_sequence = lane_payload
        .pointer("/response_finalization/source_sequence")
        .or_else(|| lane_payload.pointer("/runtime_block/source_sequence"))
        .and_then(Value::as_i64)
        .unwrap_or(prior_sequence + 1)
        .max(1);
    let age_seconds = lane_payload
        .pointer("/response_finalization/age_seconds")
        .or_else(|| lane_payload.pointer("/runtime_block/age_seconds"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .max(0.0);
    let stale = lane_payload
        .pointer("/response_finalization/stale")
        .or_else(|| lane_payload.pointer("/runtime_block/stale"))
        .and_then(Value::as_bool)
        .unwrap_or(age_seconds >= 15.0);
    let assistant = clean_text(
        lane_payload
            .get("response")
            .and_then(Value::as_str)
            .or_else(|| lane_payload.get("output").and_then(Value::as_str))
            .or_else(|| lane_payload.pointer("/turn/assistant").and_then(Value::as_str))
            .unwrap_or(""),
        8000,
    );
    let workflow_signal =
        dashboard_troubleshooting_workflow_signal(lane_ok, &error_code, hard_guard_applied, &assistant);
    let (completion_signal, completion_signal_reason) = dashboard_troubleshooting_completion_signal(
        lane_ok,
        &error_code,
        hard_guard_applied,
        &assistant,
    );
    let retry_after_hint_seconds = dashboard_troubleshooting_retry_after_hint_seconds(lane_payload);
    let credential_resolution_source = dashboard_troubleshooting_credential_source_hint(lane_payload);
    let workflow_signal_signature = dashboard_troubleshooting_exchange_signature(
        agent_id,
        raw_input,
        &classification,
        &transaction_status,
        &error_code,
        tool_calls,
        workflow_signal,
    );
    let stage_receipts = vec![
        json!({"stage":"route","status":"ok","result":{"route":route, "source_sequence": source_sequence, "age_seconds": age_seconds, "stale": stale}}),
        json!({"stage":"analysis","status":"ok","result":{"requires_live_web":requires_live_web, "source_sequence": source_sequence, "age_seconds": age_seconds, "stale": stale}}),
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
            "route={route}; tools={tool_calls}; tool_dupes={tool_call_duplicate_count}; classification={classification}; txn_status={transaction_status}; lane_ok={lane_ok}; signal={workflow_signal}; completion={completion_signal}; error={}",
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
        "source_sequence": source_sequence,
        "age_seconds": age_seconds,
        "stale": stale,
        "workflow": {
            "requires_live_web": requires_live_web,
            "tool_calls": tool_calls,
            "tool_call_unique_count": tool_call_unique_count,
            "tool_call_duplicate_count": tool_call_duplicate_count,
            "tool_call_keys": tool_call_keys,
            "classification": classification,
            "transaction_status": transaction_status,
            "workflow_signal": workflow_signal,
            "workflow_signal_signature": workflow_signal_signature,
            "completion_signal": completion_signal,
            "completion_signal_reason": completion_signal_reason,
            "hard_guard_applied": hard_guard_applied,
            "error_code": error_code,
            "retry_after_hint_seconds": retry_after_hint_seconds,
            "credential_resolution_source": credential_resolution_source,
            "source_sequence": source_sequence,
            "age_seconds": age_seconds,
            "stale": stale
        },
        "workflow_signal": workflow_signal,
        "workflow_signal_signature": workflow_signal_signature,
        "completion_signal": completion_signal,
        "completion_signal_reason": completion_signal_reason,
        "repeat_count": 1,
        "stage_receipts": stage_receipts,
        "workflow_gate_trace": lane_payload
            .pointer("/response_workflow/gates")
            .cloned()
            .unwrap_or_else(|| json!({})),
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
    let mut deduped = false;
    if let Some(last) = entries.last_mut() {
        let last_signature = last
            .get("workflow_signal_signature")
            .and_then(Value::as_str)
            .unwrap_or("");
        if last_signature == workflow_signal_signature {
            let next_repeat = last
                .get("repeat_count")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .saturating_add(1);
            if let Some(obj) = last.as_object_mut() {
                obj.insert("repeat_count".to_string(), json!(next_repeat));
                obj.insert("ts_last_seen".to_string(), json!(now_iso()));
                obj.insert("source_sequence".to_string(), json!(source_sequence));
                obj.insert("age_seconds".to_string(), json!(age_seconds));
                obj.insert("stale".to_string(), json!(stale));
            }
            last["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(last));
            exchange = last.clone();
            deduped = true;
        }
    }
    let exchange_repeat_count = exchange
        .get("repeat_count")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1);
    let loop_detection = dashboard_troubleshooting_loop_detection(
        exchange_repeat_count,
        &workflow_signal,
        &error_code,
    );
    if let Some(obj) = exchange.as_object_mut() {
        obj.insert("loop_detection".to_string(), loop_detection.clone());
        if let Some(workflow_obj) = obj.get_mut("workflow").and_then(Value::as_object_mut) {
            workflow_obj.insert("loop_detection".to_string(), loop_detection);
        }
    }
    exchange["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&exchange));
    if deduped {
        if let Some(last) = entries.last_mut() {
            *last = exchange.clone();
        }
    }
    if !deduped {
        entries.push(exchange.clone());
    }
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
        let (eval_model, _) = dashboard_troubleshooting_resolve_eval_model(None);
        queue_item = dashboard_troubleshooting_enqueue_eval(
            root,
            &snapshot,
            "auto_failure",
            Some(&eval_model),
        );
        eval_drain = dashboard_troubleshooting_eval_drain_internal(root, 1, "auto_failure");
    }

    json!({
        "recorded": true,
        "deduped": deduped,
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
    let recent_offset_from_latest =
        dashboard_payload_usize(payload, "offset_from_latest", 0, 0, 10_000);
    let entries_all = dashboard_troubleshooting_read_recent_entries(root);
    let recent_total_count = entries_all.len();
    let (recent_start, recent_end) =
        dashboard_troubleshooting_window_slice(recent_total_count, limit, recent_offset_from_latest);
    let entries = if recent_total_count == 0 {
        Vec::<Value>::new()
    } else {
        entries_all[recent_start..recent_end].to_vec()
    };
    let failure_count_recent = entries
        .iter()
        .filter(|row| dashboard_troubleshooting_exchange_failed(row))
        .count();
    let web_required_without_calls_count = entries
        .iter()
        .filter(|row| {
            row.pointer("/workflow/requires_live_web")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && row
                    .pointer("/workflow/tool_calls")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    <= 0
        })
        .count();
    let last_error_code = entries
        .last()
        .and_then(|row| row.pointer("/workflow/error_code").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 120))
        .unwrap_or_default();
    let stale_count = entries
        .iter()
        .filter(|row| row.get("stale").and_then(Value::as_bool).unwrap_or(false))
        .count();
    let max_source_sequence = entries
        .iter()
        .filter_map(|row| row.get("source_sequence").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    let loop_detection = entries
        .last()
        .and_then(|row| row.get("loop_detection"))
        .cloned()
        .unwrap_or_else(|| {
            dashboard_troubleshooting_loop_detection(1, "pending", "")
        });
    let eval_queue = dashboard_troubleshooting_read_eval_queue(root);
    let outbox = dashboard_troubleshooting_read_issue_outbox(root);
    let now_epoch = dashboard_troubleshooting_now_epoch_seconds();
    let outbox_ready_count = outbox
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                <= now_epoch
        })
        .count();
    let outbox_cooldown_blocked = outbox
        .iter()
        .filter(|row| {
            row.get("next_retry_after_epoch_s")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > now_epoch
        })
        .count();
    let outbox_next_retry_epoch = outbox
        .iter()
        .filter_map(|row| row.get("next_retry_after_epoch_s").and_then(Value::as_i64))
        .filter(|epoch| *epoch > now_epoch)
        .min()
        .unwrap_or(0);
    let latest_snapshot = read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL))
        .unwrap_or_else(|| json!({}));
    let latest_eval = read_json_file(&root.join(DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL))
        .unwrap_or_else(|| json!({}));
    let deadletter_all = dashboard_troubleshooting_read_deadletter_all(root);
    let deadletter = if deadletter_all.len() > limit {
        deadletter_all[deadletter_all.len().saturating_sub(limit)..].to_vec()
    } else {
        deadletter_all.clone()
    };
    let deadletter_depth = deadletter_all.len();
    let deadletter_reason_histogram = dashboard_troubleshooting_reason_histogram(&deadletter_all);
    let outbox_error_histogram = dashboard_troubleshooting_outbox_reason_histogram(&outbox);
    let outbox_total_count = outbox.len();
    let outbox_offset_from_latest = dashboard_payload_usize(
        payload,
        "outbox_offset_from_latest",
        recent_offset_from_latest,
        0,
        10_000,
    );
    let (outbox_start, outbox_end) =
        dashboard_troubleshooting_window_slice(outbox_total_count, limit, outbox_offset_from_latest);
    let outbox_visible_items = if outbox_total_count == 0 {
        Vec::<Value>::new()
    } else {
        outbox[outbox_start..outbox_end].to_vec()
    };
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_state",
            "recent": {
                "count": entries.len(),
                "window": dashboard_troubleshooting_window_contract(
                    recent_total_count,
                    recent_start,
                    recent_end,
                    limit,
                    recent_offset_from_latest
                ),
                "failure_count": failure_count_recent,
                "web_required_without_calls_count": web_required_without_calls_count,
                "last_error_code": last_error_code,
                "stale_count": stale_count,
                "max_source_sequence": max_source_sequence,
                "loop_detection": loop_detection,
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
                "window": dashboard_troubleshooting_window_contract(
                    outbox_total_count,
                    outbox_start,
                    outbox_end,
                    limit,
                    outbox_offset_from_latest
                ),
                "ready_count": outbox_ready_count,
                "cooldown_blocked_count": outbox_cooldown_blocked,
                "next_retry_after_epoch_s": outbox_next_retry_epoch,
                "error_histogram": outbox_error_histogram,
                "items": outbox_visible_items
            },
            "issue_deadletter": {
                "depth": deadletter_depth,
                "reason_histogram": deadletter_reason_histogram,
                "items": deadletter
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
