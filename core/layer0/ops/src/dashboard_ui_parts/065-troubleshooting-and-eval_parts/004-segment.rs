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
        "source_sequence": source_sequence,
        "age_seconds": age_seconds,
        "stale": stale,
        "workflow": {
            "requires_live_web": requires_live_web,
            "tool_calls": tool_calls,
            "classification": classification,
            "transaction_status": transaction_status,
            "hard_guard_applied": hard_guard_applied,
            "error_code": error_code,
            "source_sequence": source_sequence,
            "age_seconds": age_seconds,
            "stale": stale
        },
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
    LaneResult {
        ok: true,
        status: 0,
        argv: vec!["dashboard.troubleshooting.state".to_string()],
        payload: Some(json!({
            "ok": true,
            "type": "dashboard_troubleshooting_state",
            "recent": {
                "count": entries.len(),
                "failure_count": failure_count_recent,
                "web_required_without_calls_count": web_required_without_calls_count,
                "last_error_code": last_error_code,
                "stale_count": stale_count,
                "max_source_sequence": max_source_sequence,
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
                "ready_count": outbox_ready_count,
                "cooldown_blocked_count": outbox_cooldown_blocked,
                "next_retry_after_epoch_s": outbox_next_retry_epoch,
                "error_histogram": outbox_error_histogram,
                "items": outbox.into_iter().take(limit).collect::<Vec<_>>()
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
