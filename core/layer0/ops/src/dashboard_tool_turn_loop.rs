// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const TERMINAL_PERMISSION_POLICY_REL: &str =
    "client/runtime/config/terminal_command_permission_policy.json";
const TOOL_NO_FINDINGS_COPY: &str = "No relevant results found for that request yet.";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn normalize_tool_name(raw: &str) -> String {
    clean_text(raw, 80)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
}

fn extract_rule(raw: &str) -> String {
    let cleaned = clean_text(raw, 320);
    if let Some(inner) = cleaned.strip_prefix("Bash(") {
        if let Some(pattern) = inner.strip_suffix(')') {
            return clean_text(pattern, 240);
        }
    }
    clean_text(&cleaned, 240)
}

fn rules_from_value(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(extract_rule))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>()
}

fn load_permission_rules(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut deny = Vec::<String>::new();
    let mut ask = Vec::<String>::new();
    let path = root.join(TERMINAL_PERMISSION_POLICY_REL);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            deny.extend(rules_from_value(
                value
                    .get("deny_rules")
                    .or_else(|| value.pointer("/permissions/deny")),
            ));
            ask.extend(rules_from_value(
                value
                    .get("ask_rules")
                    .or_else(|| value.pointer("/permissions/ask")),
            ));
        }
    }
    deny.sort();
    deny.dedup();
    ask.sort();
    ask.dedup();
    (deny, ask)
}

fn input_confirmed(input: &Value) -> bool {
    input.get("confirm").and_then(Value::as_bool).unwrap_or(false)
        || !clean_text(
            input
                .get("approval_note")
                .and_then(Value::as_str)
                .unwrap_or(""),
            200,
        )
        .is_empty()
}

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
            let action = clean_text(input.get("action").and_then(Value::as_str).unwrap_or(""), 80)
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
            let count = input.get("count").and_then(Value::as_i64).unwrap_or(0).max(0);
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
    let (verdict, matched) = crate::command_permission_kernel::evaluate_command_permission_for_kernel(
        &command,
        &deny_rules,
        &ask_rules,
    );
    let verdict_str = verdict.as_str().to_string();
    if verdict_str == "allow" {
        return None;
    }
    if verdict_str == "ask" && input_confirmed(input) {
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
    if crate::tool_output_match_filter::matches_ack_placeholder(&cleaned) {
        return Some((
            TOOL_NO_FINDINGS_COPY.to_string(),
            "ack_placeholder_suppressed".to_string(),
        ));
    }
    if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&cleaned)
    {
        return Some((rewritten, format!("failure_placeholder_rewrite:{rule_id}")));
    }
    None
}

fn rewrite_object_key(obj: &mut serde_json::Map<String, Value>, key: &str, events: &mut Vec<String>) {
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
        for key in ["summary", "content", "result", "message", "error"] {
            rewrite_object_key(obj, key, &mut events);
        }
        if let Some(result_obj) = obj.get_mut("result").and_then(Value::as_object_mut) {
            for key in ["summary", "content", "result", "message", "error"] {
                rewrite_object_key(result_obj, key, &mut events);
            }
        }
        if let Some(receipt_obj) = obj.get_mut("receipt").and_then(Value::as_object_mut) {
            for key in ["summary", "content", "message", "error"] {
                rewrite_object_key(receipt_obj, key, &mut events);
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
        obj.insert("turn_loop_tracking".to_string(), tracking.unwrap_or(Value::Null));
    }
}

fn output_tokens_estimate(payload: &Value) -> usize {
    let mut total = 0usize;
    for key in ["summary", "content", "result", "message", "error"] {
        total += payload
            .get(key)
            .and_then(Value::as_str)
            .map(|row| row.len())
            .unwrap_or(0);
    }
    if total == 0 {
        total = payload.to_string().len().min(32_000);
    }
    (total / 4).max(1)
}

pub(crate) fn record_tool_turn_tracking(
    root: &Path,
    session_id: &str,
    tool_name: &str,
    payload: &Value,
) -> Option<Value> {
    let clean_session = clean_text(session_id, 120);
    if clean_session.is_empty() {
        return None;
    }
    let command = format!("tool::{}", normalize_tool_name(tool_name));
    let batch = json!({
        "session_id": clean_session,
        "records": [
            {
                "session_id": clean_session,
                "command": command,
                "output_tokens": output_tokens_estimate(payload)
            }
        ]
    });
    crate::session_command_tracking_kernel::record_batch_for_kernel(root, &batch).ok()
}

pub(crate) fn turn_transaction_payload(
    hydrate: &str,
    tool_execute: &str,
    synthesize: &str,
    session_persist: &str,
) -> Value {
    json!({
        "hydrate": clean_text(hydrate, 60),
        "tool_execute": clean_text(tool_execute, 60),
        "synthesize": clean_text(synthesize, 60),
        "session_persist": clean_text(session_persist, 60)
    })
}

pub(crate) fn hydration_failed_payload(agent_id: &str) -> Value {
    json!({
        "ok": false,
        "error": "context_hydration_incomplete",
        "agent_id": clean_text(agent_id, 120),
        "message": "Conversation context hydration failed closed before model execution. Retry once; if it persists, run `infringctl doctor --json` and `/context`.",
        "turn_transaction": turn_transaction_payload("failed_closed", "skipped", "skipped", "skipped")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_filter_rewrites_ack_placeholder_copy() {
        let mut payload = json!({"ok": true, "summary": "Web search completed."});
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let lowered = payload
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(lowered.contains("no relevant results"));
        assert!(!lowered.contains("web search completed"));
    }

    #[test]
    fn pre_gate_respects_confirm_for_ask_verdicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"ask_rules":["Bash(echo *)"]}"#).expect("write policy");
        let blocked = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello"}),
        )
        .expect("blocked");
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("tool_confirmation_required")
        );
        let allowed = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello","confirm":true}),
        );
        assert!(allowed.is_none());
    }
}
