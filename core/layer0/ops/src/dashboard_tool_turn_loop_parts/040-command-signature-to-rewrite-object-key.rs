
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
    let normalized = normalize_tool_name(tool_name);
    let command = tool_command_signature(&normalized, input);
    if command.is_empty() {
        return None;
    }
    let (deny_rules, ask_rules) = load_permission_rules(root);
    let (verdict, matched) =
        crate::command_permission_kernel::evaluate_command_permission_for_kernel(
            &command,
            &deny_rules,
            &ask_rules,
        );
    let verdict_str = verdict.as_str().to_string();
    if verdict_str == "allow" {
        return None;
    }
    if verdict_str == "ask"
        && (input_confirmed(input) || tool_is_autonomous_spawn(normalized.as_str()))
    {
        return None;
    }
    let error = if verdict_str == "deny" {
        "tool_permission_denied"
    } else {
        "tool_confirmation_required"
    };
    Some(json!({
        "ok": false,
        "error": error,
        "type": "tool_pre_gate_blocked",
        "tool": normalized,
        "fail_closed": true,
        "permission_gate": {
            "verdict": verdict_str,
            "matched": matched,
            "deny_rules_count": deny_rules.len(),
            "ask_rules_count": ask_rules.len(),
            "command_signature": command
        },
        "hint": if error == "tool_confirmation_required" {
            "Confirmation required before this tool can run."
        } else {
            "Tool blocked by command permission policy."
        }
    }))
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
