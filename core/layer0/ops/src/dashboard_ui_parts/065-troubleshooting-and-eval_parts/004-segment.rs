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
