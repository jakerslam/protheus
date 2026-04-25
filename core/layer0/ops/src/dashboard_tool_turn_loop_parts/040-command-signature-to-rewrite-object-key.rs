
fn tool_command_signature(tool_name: &str, input: &Value) -> String {
    let normalized = normalize_tool_name(tool_name);
    match normalized.as_str() {
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec" => clean_text(
            input
                .get("command")
                .or_else(|| input.get("cmd"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            3000,
        ),
        "manage_agent" | "agent_action" => {
            let action = clean_text(
                input.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            let agent = clean_text(
                input
                    .get("agent_id")
                    .or_else(|| input.get("target_agent_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            );
            clean_text(format!("manage_agent {action} {agent}").trim(), 400)
        }
        "spawn_subagents" | "spawn_swarm" | "agent_spawn" | "sessions_spawn" => {
            let count = input
                .get("count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            clean_text(
                format!(
                    "spawn_subagents count={} objective={}",
                    count,
                    clean_text(
                        input
                            .get("objective")
                            .or_else(|| input.get("task"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        120
                    )
                )
                .trim(),
                500,
            )
        }
        _ => String::new(),
    }
}

pub(crate) fn pre_tool_permission_gate(
    root: &Path,
    tool_name: &str,
    input: &Value,
) -> Option<Value> {
    let decision = pre_tool_permission_decision(root, tool_name, input);
    let error = decision
        .get("blocked_error")
        .and_then(Value::as_str)
        .unwrap_or("");
    if error.is_empty() {
        return None;
    }
    Some(json!({
        "ok": false,
        "error": error,
        "type": "tool_pre_gate_blocked",
        "tool": decision.get("tool").cloned().unwrap_or_else(|| Value::String(normalize_tool_name(tool_name))),
        "fail_closed": true,
        "permission_gate": decision,
        "hint": if error == "tool_confirmation_required" {
            "Confirmation required before this tool can run."
        } else {
            "Tool blocked by command permission policy."
        }
    }))
}

pub(crate) fn pre_tool_permission_decision(root: &Path, tool_name: &str, input: &Value) -> Value {
    let normalized = normalize_tool_name(tool_name);
    let command = tool_command_signature(&normalized, input);
    if command.is_empty() {
        return json!({
            "tool": normalized,
            "configured": false,
            "verdict": "not_applicable",
            "effective_verdict": "allow",
            "matched": Value::Null,
            "explicit_confirmation": input_confirmed(input),
            "autonomous_spawn_tool": tool_is_autonomous_spawn(normalized.as_str()),
            "auto_confirmed": false,
            "auto_confirm_reason": Value::Null,
            "blocked_error": Value::Null,
            "command_signature": ""
        });
    }
    let (deny_rules, ask_rules) = load_permission_rules(root);
    let (verdict, matched) =
        crate::command_permission_kernel::evaluate_command_permission_for_kernel(
            &command,
            &deny_rules,
            &ask_rules,
        );
    let verdict_str = verdict.as_str().to_string();
    let explicit_confirmation = input_confirmed(input);
    let autonomous_spawn_tool = tool_is_autonomous_spawn(normalized.as_str());
    let auto_confirmed =
        verdict_str == "ask" && (explicit_confirmation || autonomous_spawn_tool);
    let auto_confirm_reason = if verdict_str == "ask" && explicit_confirmation {
        Some("input_confirmed")
    } else if verdict_str == "ask" && autonomous_spawn_tool {
        Some("autonomous_spawn_tool")
    } else {
        None
    };
    let effective_verdict = if verdict_str == "ask" && auto_confirmed {
        "allow"
    } else {
        verdict_str.as_str()
    };
    let blocked_error = if effective_verdict == "deny" {
        Some("tool_permission_denied")
    } else if effective_verdict == "ask" {
        Some("tool_confirmation_required")
    } else {
        None
    };
    let mut decision = json!({
        "tool": normalized,
        "configured": true,
        "verdict": verdict_str,
        "effective_verdict": effective_verdict,
        "matched": matched,
        "deny_rules_count": deny_rules.len(),
        "ask_rules_count": ask_rules.len(),
        "command_signature": command,
        "explicit_confirmation": explicit_confirmation,
        "autonomous_spawn_tool": autonomous_spawn_tool,
        "auto_confirmed": auto_confirmed,
        "auto_confirm_reason": auto_confirm_reason,
        "blocked_error": blocked_error,
    });
    if verdict_str == "ask"
        && autonomous_spawn_tool
        && !explicit_confirmation
    {
        decision["spawn_autonomy_contract"] = json!({
            "mode": "ask_verdict_auto_allowed",
            "deny_rules_still_fail_closed": true,
            "confirmation_loop_suppressed": true
        });
    }
    decision
}

fn rewrite_text_for_post_filter(value: &str) -> Option<(String, String)> {
    let cleaned = clean_text(value, 32_000);
    if cleaned.is_empty() {
        return None;
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_raw_payload_dump(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_unsynthesized_web_dump(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_repetitive_thinking_chatter(&cleaned)
    {
        return Some((rewritten, rule_id));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&cleaned)
    {
        return Some((rewritten, format!("failure_placeholder_rewrite:{rule_id}")));
    }
    if crate::tool_output_match_filter::matches_ack_placeholder(&cleaned) {
        return Some((
            crate::tool_output_match_filter::no_findings_user_copy().to_string(),
            "ack_placeholder_suppressed".to_string(),
        ));
    }
    None
}

fn rewrite_object_key(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    events: &mut Vec<String>,
) {
    let original = obj
        .get(key)
        .and_then(Value::as_str)
        .map(|row| row.to_string())
        .unwrap_or_default();
    if original.is_empty() {
        return;
    }
    if let Some((rewritten, event)) = rewrite_text_for_post_filter(&original) {
        obj.insert(key.to_string(), Value::String(rewritten));
        events.push(format!("{key}:{event}"));
    }
}

pub(crate) fn apply_post_tool_output_filter(payload: &mut Value) -> Value {
    let mut events = Vec::<String>::new();
    if let Some(obj) = payload.as_object_mut() {
        for key in [
            "summary", "content", "result", "message", "error", "response", "details", "hint",
            "text",
        ] {
            rewrite_object_key(obj, key, &mut events);
        }
        if let Some(result_obj) = obj.get_mut("result").and_then(Value::as_object_mut) {
            for key in [
                "summary", "content", "result", "message", "error", "response", "details", "hint",
                "text",
            ] {
                rewrite_object_key(result_obj, key, &mut events);
            }
        }
        if let Some(receipt_obj) = obj.get_mut("receipt").and_then(Value::as_object_mut) {
            for key in [
                "summary", "content", "message", "error", "response", "details", "hint", "text",
            ] {
                rewrite_object_key(receipt_obj, key, &mut events);
            }
        }
        if let Some(finalization_obj) = obj
            .get_mut("response_finalization")
            .and_then(Value::as_object_mut)
        {
            for key in ["response", "message", "error", "details", "text"] {
                rewrite_object_key(finalization_obj, key, &mut events);
            }
        }
    }
    let report = json!({
        "applied": !events.is_empty(),
        "events": events
    });
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("turn_loop_post_filter".to_string(), report.clone());
    }
    report
}

pub(crate) fn annotate_tool_payload_tracking(
    root: &Path,
    session_id: &str,
    tool_name: &str,
    payload: &mut Value,
) {
    let post_filter_report = apply_post_tool_output_filter(payload);
    let tracking = record_tool_turn_tracking(root, session_id, tool_name, payload);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("turn_loop_post_filter".to_string(), post_filter_report);
        obj.insert(
            "turn_loop_tracking".to_string(),
            tracking.unwrap_or(Value::Null),
        );
    }
}
