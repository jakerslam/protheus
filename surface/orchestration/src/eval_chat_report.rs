use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const CHAT_MONITOR_ISSUES_REL: &str =
    "local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json";
const MANUAL_REPORTS_REL: &str =
    "local/state/ops/eval_agent_chat_monitor/manual_issue_reports.jsonl";
const EVAL_FEEDBACK_DIR_REL: &str = "local/state/ops/eval_agent_feedback";
const SRS_EVAL_CHAT_REPORT_ID: &str = "V12-EVAL-CHAT-REPORT-001";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect()
}

fn clean_agent_id(raw: &str) -> String {
    clean_text(raw, 160)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

fn stable_hash_hex(value: &Value) -> String {
    let raw = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(raw);
    hasher
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(value) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()));
    }
}

fn session_messages(root: &Path, agent_id: &str) -> Vec<Value> {
    let path = root
        .join(AGENT_SESSIONS_DIR_REL)
        .join(format!("{}.json", clean_agent_id(agent_id)));
    let state = read_json(&path);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let sessions = state
        .get("sessions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = sessions
        .iter()
        .find(|session| {
            clean_text(
                session
                    .get("session_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120,
            ) == active_id
        })
        .or_else(|| sessions.first());
    row.and_then(|session| session.get("messages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn message_role(row: &Value) -> String {
    let raw = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24).to_lowercase();
    if raw == "assistant" {
        "agent".to_string()
    } else if !raw.is_empty() {
        raw
    } else if row.get("user").and_then(Value::as_bool).unwrap_or(false) {
        "user".to_string()
    } else {
        "agent".to_string()
    }
}

fn should_skip_message(row: &Value) -> bool {
    row.get("thinking")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || row
            .get("streaming")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || row
            .get("terminal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || row
            .get("is_notice")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || message_role(row) == "system"
}

fn normalized_context_row(row: &Value, index: usize) -> Option<Value> {
    if should_skip_message(row) {
        return None;
    }
    let text = clean_text(row.get("text").and_then(Value::as_str).unwrap_or(""), 1_200);
    let tools = row
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if text.is_empty() && tools.is_empty() && row.get("meta").is_none() && row.get("ts").is_none() {
        return None;
    }
    let tool_names = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 80))
        .filter(|raw| !raw.is_empty())
        .take(8)
        .collect::<Vec<_>>();
    Some(json!({
        "index": index,
        "id": clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 120),
        "role": message_role(row),
        "text": text,
        "ts": clean_text(row.get("ts").and_then(Value::as_str).unwrap_or(""), 80),
        "tool_names": tool_names,
        "tool_count": tools.len()
    }))
}

fn resolve_target_index(messages: &[Value], request: &Value) -> Option<usize> {
    let requested_id = clean_text(
        request
            .get("message_id")
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    );
    if !requested_id.is_empty() {
        if let Some((idx, _)) = messages.iter().enumerate().find(|(_, row)| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) == requested_id
        }) {
            return Some(idx);
        }
    }
    request
        .get("message_index")
        .and_then(Value::as_i64)
        .and_then(|idx| usize::try_from(idx).ok())
        .filter(|idx| *idx < messages.len())
}

fn collect_report_context(
    root: &Path,
    agent_id: &str,
    request: &Value,
) -> (Option<Value>, Vec<Value>) {
    let messages = session_messages(root, agent_id);
    if let Some(target_index) = resolve_target_index(&messages, request) {
        let start = target_index.saturating_sub(14);
        let context = (start..=target_index)
            .filter_map(|idx| normalized_context_row(&messages[idx], idx))
            .collect::<Vec<_>>();
        return (
            normalized_context_row(&messages[target_index], target_index),
            context,
        );
    }
    let fallback_reported = request.get("reported_message").cloned();
    let fallback_context = request
        .get("context_messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(15)
        .collect::<Vec<_>>();
    (fallback_reported, fallback_context)
}

fn append_issue_draft(root: &Path, issue: &Value) -> PathBuf {
    let path = root.join(CHAT_MONITOR_ISSUES_REL);
    let mut state = read_json(&path);
    if !state
        .get("issue_drafts")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        state["issue_drafts"] = Value::Array(Vec::new());
    }
    if let Some(rows) = state.get_mut("issue_drafts").and_then(Value::as_array_mut) {
        let issue_id = issue.get("id").and_then(Value::as_str).unwrap_or("");
        rows.retain(|row| row.get("id").and_then(Value::as_str).unwrap_or("") != issue_id);
        rows.insert(0, issue.clone());
        rows.truncate(100);
    }
    state["type"] = json!("eval_agent_chat_monitor_issue_drafts");
    state["updated_at"] = json!(now_iso_like());
    write_json(&path, &state);
    path
}

fn feedback_item(issue: &Value) -> Value {
    json!({
        "id": issue.get("id").cloned().unwrap_or_else(|| json!("manual_chat_eval_report")),
        "source_kind": "eval_agent_chat_monitor_issue",
        "title": issue.get("title").cloned().unwrap_or_else(|| json!("Chat eval review requested")),
        "severity": issue.get("severity").cloned().unwrap_or_else(|| json!("warn")),
        "owner_component": issue.get("owner_component").cloned().unwrap_or_else(|| json!("control_plane.eval_agent_feedback")),
        "issue_class": "manual_chat_eval_report",
        "related_agent_id": issue.get("agent_id").cloned().unwrap_or_else(|| json!("")),
        "expected_fix": issue.get("next_action").cloned().unwrap_or_else(|| json!("Examine the flagged chat context.")),
        "suggested_test": issue.get("acceptance_criteria").and_then(Value::as_array).and_then(|rows| rows.first()).cloned().unwrap_or_else(|| json!("Eval review records whether the flagged chat turn is actionable.")),
        "replay_command": issue.get("replay_command").cloned().unwrap_or_else(|| json!("")),
        "evidence_summary": issue.pointer("/exact_evidence/reported_text").cloned().unwrap_or_else(|| json!("")),
        "acceptance_criteria": issue.get("acceptance_criteria").cloned().unwrap_or_else(|| json!([]))
    })
}

fn upsert_attention_state(root: &Path, agent_id: &str, issue: &Value, event: &Value) -> PathBuf {
    let agent = clean_agent_id(agent_id);
    let dir = root.join(EVAL_FEEDBACK_DIR_REL);
    let path = dir.join(format!("{agent}.json"));
    let mut state = read_json(&path);
    if !state
        .get("visible_feedback_items")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        state["visible_feedback_items"] = Value::Array(Vec::new());
    }
    if let Some(rows) = state
        .get_mut("visible_feedback_items")
        .and_then(Value::as_array_mut)
    {
        let item = feedback_item(issue);
        let issue_id = item.get("id").and_then(Value::as_str).unwrap_or("");
        rows.retain(|row| row.get("id").and_then(Value::as_str).unwrap_or("") != issue_id);
        rows.insert(0, item);
        rows.truncate(50);
    }
    if !state
        .get("attention_events")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        state["attention_events"] = Value::Array(Vec::new());
    }
    if let Some(rows) = state
        .get_mut("attention_events")
        .and_then(Value::as_array_mut)
    {
        rows.insert(0, event.clone());
        rows.truncate(50);
    }
    state["type"] = json!("eval_agent_feedback_attention_state");
    state["schema_version"] = json!(1);
    state["generated_at"] = json!(now_iso_like());
    state["agent_id"] = json!(agent.clone());
    state["scope_agent_ids"] = json!([agent.clone()]);
    state["visibility_rule"] = json!("agent_may_see_self_and_descendant_eval_feedback_only");
    state["hidden_unscoped_count"] = json!(0);
    write_json(&path, &state);
    append_jsonl(&dir.join(format!("{agent}.attention.jsonl")), event);
    path
}

pub fn stage_dashboard_chat_eval_issue_report(
    root: &Path,
    agent_id: &str,
    request: &Value,
) -> Value {
    let agent = clean_agent_id(agent_id);
    if agent.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let (reported, context_messages) = collect_report_context(root, &agent, request);
    let reported = reported.unwrap_or_else(|| json!({}));
    let reported_text = clean_text(
        reported.get("text").and_then(Value::as_str).unwrap_or(""),
        1_200,
    );
    let message_id = clean_text(
        request
            .get("message_id")
            .and_then(Value::as_str)
            .or_else(|| reported.get("id").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    );
    if message_id.is_empty() && reported_text.is_empty() {
        return json!({"ok": false, "error": "reported_message_required"});
    }
    let seed = json!({"agent_id": agent, "message_id": message_id, "reported_text": reported_text, "context_messages": context_messages});
    let issue_id = format!("manual_chat_eval_report_{}", stable_hash_hex(&seed));
    let acceptance = vec![
        "Eval review classifies whether the flagged chat turn is actionable.",
        "If actionable, the review routes ownership without creating a public GitHub issue.",
        "The bounded context includes the flagged message plus the recent chat-suggestion-sized context.",
    ];
    let evidence = context_messages
        .iter()
        .map(|row| json!({
            "agent_id": agent,
            "turn_id": format!("agent:{}:chat:{}", agent, row.get("index").and_then(Value::as_i64).unwrap_or(0)),
            "snippet": clean_text(row.get("text").and_then(Value::as_str).unwrap_or(""), 240),
            "role": row.get("role").cloned().unwrap_or_else(|| json!("agent")),
            "message_id": row.get("id").cloned().unwrap_or_else(|| json!(""))
        }))
        .collect::<Vec<_>>();
    let issue = json!({
        "id": issue_id,
        "severity": "warn",
        "owner_component": "control_plane.eval_agent_feedback",
        "agent_id": agent,
        "source_agent_id": agent,
        "source_kind": "dashboard_chat_metadata_eval_report",
        "title": "[WARN][manual_chat_eval_report] Operator flagged a chat turn for eval review.",
        "body": "Next action:\nExamine the flagged message and bounded recent context. Classify whether this is a shell, orchestration, core, or model-output issue. Do not file a public GitHub issue from this path.\n\nAcceptance criteria:\n- Eval review classifies whether the flagged chat turn is actionable.\n- If actionable, the review routes ownership without creating a public GitHub issue.\n- The bounded context includes the flagged message plus the recent chat-suggestion-sized context.",
        "next_action": "Examine the flagged message and bounded recent context; classify and route any actionable defect internally.",
        "acceptance_criteria": acceptance,
        "exact_evidence": {
            "agent_id": agent,
            "message_id": message_id,
            "message_index": reported.get("index").cloned().unwrap_or_else(|| request.get("message_index").cloned().unwrap_or_else(|| json!(0))),
            "reported_text": reported_text,
            "context_messages": context_messages,
            "context_policy": {
                "source": "chat_metadata_hazard_button",
                "max_previous_messages": 14,
                "approximates_last_exchanges": 7
            }
        },
        "evidence": evidence,
        "replay_command": "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- agent-feedback --agent-id=<agent-id> --strict=1"
    });
    let event = json!({
        "ts": now_iso_like(),
        "source": format!("agent:{agent}"),
        "source_type": "eval_issue_feedback",
        "severity": "warn",
        "summary": "Eval review requested for a flagged chat turn.",
        "attention_key": format!("eval_feedback:{agent}:{}", issue.get("id").and_then(Value::as_str).unwrap_or("manual_chat_eval_report")),
        "raw_event": {
            "agent_id": agent,
            "related_agent_id": agent,
            "issue_id": issue.get("id").cloned().unwrap_or_else(|| json!("manual_chat_eval_report")),
            "issue_class": "manual_chat_eval_report",
            "source_kind": "eval_agent_chat_monitor_issue",
            "owner_component": "control_plane.eval_agent_feedback",
            "expected_fix": issue.get("next_action").cloned().unwrap_or_else(|| json!("Examine the flagged chat context.")),
            "suggested_test": acceptance.first().copied().unwrap_or_default(),
            "replay_command": issue.get("replay_command").cloned().unwrap_or_else(|| json!("")),
            "terms": ["manual", "chat", "eval", "report"]
        }
    });
    let issue_path = append_issue_draft(root, &issue);
    append_jsonl(&root.join(MANUAL_REPORTS_REL), &issue);
    let state_path = upsert_attention_state(root, &agent, &issue, &event);
    json!({
        "ok": true,
        "type": "dashboard_chat_eval_issue_report",
        "srs_id": SRS_EVAL_CHAT_REPORT_ID,
        "authority": "surface/orchestration",
        "agent_id": agent,
        "issue_id": issue.get("id").cloned().unwrap_or_else(|| json!("")),
        "message_id": message_id,
        "context_message_count": context_messages.len(),
        "github_issue_created": false,
        "issue_drafts_path": issue_path.to_string_lossy().to_string(),
        "attention_state_path": state_path.to_string_lossy().to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_issue_uses_recent_context_without_github() {
        let root =
            std::env::temp_dir().join(format!("infring_eval_chat_report_{}", now_iso_like()));
        let agent = "agent-report";
        let session_path = root
            .join(AGENT_SESSIONS_DIR_REL)
            .join(format!("{agent}.json"));
        let messages = (0..18)
            .map(|idx| json!({"id": format!("m{idx}"), "role": if idx % 2 == 0 { "user" } else { "assistant" }, "text": format!("message {idx}")}))
            .collect::<Vec<_>>();
        write_json(
            &session_path,
            &json!({"active_session_id": "default", "sessions": [{"session_id": "default", "messages": messages}]}),
        );
        let result = stage_dashboard_chat_eval_issue_report(
            &root,
            agent,
            &json!({"message_id": "m17", "message_index": 17}),
        );
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            result.get("github_issue_created").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            result.get("authority").and_then(Value::as_str),
            Some("surface/orchestration")
        );
        assert_eq!(
            result.get("context_message_count").and_then(Value::as_u64),
            Some(15)
        );
        assert!(root.join(CHAT_MONITOR_ISSUES_REL).exists());
        assert!(root
            .join(EVAL_FEEDBACK_DIR_REL)
            .join(format!("{agent}.json"))
            .exists());
        let state = read_json(
            &root
                .join(EVAL_FEEDBACK_DIR_REL)
                .join(format!("{agent}.json")),
        );
        assert_eq!(state.get("agent_id").and_then(Value::as_str), Some(agent));
        let _ = fs::remove_dir_all(root);
    }
}
