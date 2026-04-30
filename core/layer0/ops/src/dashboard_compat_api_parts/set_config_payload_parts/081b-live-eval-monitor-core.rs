fn live_eval_monitor_enabled(root: &Path) -> bool {
    if let Ok(raw) = std::env::var("INFRING_LIVE_EVAL_MONITOR") {
        let v = raw.trim().to_ascii_lowercase();
        if matches!(v.as_str(), "0" | "false" | "off" | "disabled") {
            return false;
        }
        if matches!(v.as_str(), "1" | "true" | "on" | "enabled") {
            return true;
        }
    }
    for rel in [
        "local/state/ops/eval_live_monitor/config.json",
        "validation/evals/policies/live_eval_monitor_policy.json",
    ] {
        if let Some(config) = read_json(&root.join(rel)) {
            if let Some(enabled) = config.get("enabled").and_then(Value::as_bool) {
                return enabled;
            }
        }
    }
    true
}

fn live_eval_issue_event(
    agent_id: &str,
    issue_class: &str,
    severity: &str,
    summary: &str,
    message: &str,
    response: &str,
) -> Value {
    let issue_id = format!("{issue_class}_live");
    let seed =
        json!({"agent_id": agent_id, "issue": issue_id, "message": message, "response": response});
    json!({
        "ts": crate::now_iso(),
        "source": format!("agent:{agent_id}"),
        "source_type": "live_eval_turn_issue",
        "severity": severity,
        "summary": clean_text(summary, 260),
        "attention_key": format!("agent:{agent_id}:live_eval:{issue_class}:{}", crate::deterministic_receipt_hash(&seed).chars().take(20).collect::<String>()),
        "raw_event": {
            "agent_id": agent_id,
            "issue_class": issue_class,
            "issue_id": issue_id,
            "user_text": clean_text(message, 700),
            "assistant_text": clean_text(response, 700),
            "expected_fix": "Preserve LLM-authored chat output and record diagnostics without injecting system text."
        }
    })
}

fn live_eval_monitor_turn(
    root: &Path,
    agent_id: &str,
    message: &str,
    response: &str,
    previous_assistant: &str,
    previous_user: &str,
    response_finalization: &Value,
) -> Value {
    let id = clean_agent_id(agent_id);
    let enabled = live_eval_monitor_enabled(root);
    if !enabled {
        return json!({
            "ok": true,
            "enabled": false,
            "type": "live_eval_turn_monitor",
            "agent_id": id,
            "generated_at": crate::now_iso(),
            "issue_count": 0,
            "issues": [],
            "stream_path": "local/state/ops/eval_live_monitor/events.jsonl",
            "queue_receipts": [],
            "chat_injection_allowed": false
        });
    }
    let final_text = clean_text(response, 2_400);
    let prev_sig = normalize_placeholder_signature(previous_assistant);
    let final_sig = normalize_placeholder_signature(&final_text);
    let previous_user_sig = normalize_placeholder_signature(previous_user);
    let current_user_sig = normalize_placeholder_signature(message);
    let repeated_user_request =
        !previous_user_sig.is_empty() && previous_user_sig == current_user_sig;
    let final_ack_only = response_finalization
        .get("final_ack_only")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| response_looks_like_tool_ack_without_findings(&final_text));
    let system_fallback = response_finalization
        .get("workflow_system_fallback_used")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let route_failure_code = clean_text(
        response_finalization
            .pointer("/route_failure/error_code")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    let final_empty_is_pending_tool_request =
        final_text.is_empty() && response_finalization.get("pending_tool_request").is_some();
    let repeated_response = !prev_sig.is_empty() && prev_sig == final_sig;
    let repeated_response_has_failure_shape = system_fallback
        || final_ack_only
        || response_looks_like_tool_ack_without_findings(&final_text)
        || response_looks_like_raw_web_artifact_dump(&final_text)
        || response_looks_like_unsynthesized_web_snippet_dump(&final_text)
        || visible_response_looks_like_internal_deliberation(&final_text)
        || visible_response_looks_like_json_response_wrapper(&final_text);
    let mut events = Vec::<Value>::new();
    if final_text.is_empty() && !route_failure_code.is_empty() {
        events.push(live_eval_issue_event(
            &id,
            "message_route_error",
            "warn",
            &format!("Live eval saw a structured message route error: {route_failure_code}."),
            message,
            response,
        ));
    } else if final_text.is_empty() && !final_empty_is_pending_tool_request {
        events.push(live_eval_issue_event(
            &id,
            "no_response",
            "high",
            "Live eval saw an empty finalized assistant response.",
            message,
            response,
        ));
    } else if repeated_response && (!repeated_user_request || repeated_response_has_failure_shape) {
        events.push(live_eval_issue_event(
            &id,
            "repeated_response_loop",
            "high",
            "Live eval saw a repeated assistant response.",
            message,
            response,
        ));
    } else if final_ack_only && !final_empty_is_pending_tool_request {
        events.push(live_eval_issue_event(
            &id,
            "ack_only_final_response",
            "warn",
            "Live eval saw an ack-only final response.",
            message,
            response,
        ));
    }
    if visible_response_looks_like_internal_deliberation(&final_text) {
        events.push(live_eval_issue_event(
            &id,
            "visible_internal_deliberation",
            "high",
            "Live eval saw internal deliberation exposed in the visible assistant response.",
            message,
            response,
        ));
    }
    if visible_response_looks_like_json_response_wrapper(&final_text) {
        events.push(live_eval_issue_event(
            &id,
            "visible_json_response_wrapper",
            "warn",
            "Live eval saw a JSON response wrapper exposed in the visible assistant response.",
            message,
            response,
        ));
    }
    events.extend(live_eval_workflow_issue_events(
        &id,
        message,
        response,
        previous_assistant,
        response_finalization,
        system_fallback,
    ));
    let stream_path = root.join("local/state/ops/eval_live_monitor/events.jsonl");
    let latest_path = root.join("local/state/ops/eval_live_monitor/latest.json");
    let mut queue_receipts = Vec::<Value>::new();
    for event in &events {
        append_jsonl_row(&stream_path, event);
        append_jsonl_row(
            &root
                .join("local/state/ops/eval_agent_feedback")
                .join(format!("{id}.attention.jsonl")),
            event,
        );
        queue_receipts.push(enqueue_attention_event_best_effort(
            root,
            "dashboard_live_eval_monitor",
            event,
        ));
    }
    let latest = json!({
        "ok": true,
        "enabled": true,
        "type": "live_eval_turn_monitor",
        "agent_id": id,
        "generated_at": crate::now_iso(),
        "issue_count": events.len(),
        "issues": events,
        "stream_path": "local/state/ops/eval_live_monitor/events.jsonl",
        "queue_receipts": queue_receipts,
        "chat_injection_allowed": false
    });
    write_json_pretty(&latest_path, &latest);
    latest
}
