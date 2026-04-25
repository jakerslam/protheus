use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "eval_learning_loop_issue_candidates.rs"]
mod eval_learning_loop_issue_candidates;
pub use eval_learning_loop_issue_candidates::run_eval_learning_loop_issue_candidates;

const DEFAULT_SOURCE_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_learning_loop_traces.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_learning_loop_inbox_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_learning_loop_inbox_latest.json";
const DEFAULT_INBOX_PATH: &str = "local/state/ops/eval_learning_loop/inbox.jsonl";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_LEARNING_LOOP_INBOX_CURRENT.md";

pub fn run_eval_learning_loop_ingest(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let source_path = parse_flag(args, "source").unwrap_or_else(|| DEFAULT_SOURCE_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let inbox_path =
        parse_flag(args, "out-inbox").unwrap_or_else(|| DEFAULT_INBOX_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let source = read_json(&source_path);
    let traces = source
        .get("traces")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut signal_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut leak_count = 0_u64;
    for trace in traces.iter() {
        let row = inbox_row(trace);
        if row_has_private_leak(&row) {
            leak_count = leak_count.saturating_add(1);
        }
        for signal in row
            .get("failure_signals")
            .and_then(|node| node.as_array())
            .into_iter()
            .flatten()
            .filter_map(|node| node.as_str())
        {
            *signal_counts.entry(signal.to_string()).or_insert(0) += 1;
        }
        rows.push(row);
    }
    let seed_fixture =
        source.get("type").and_then(Value::as_str) == Some("eval_learning_loop_trace_seed");
    let required_signals = if seed_fixture {
        vec![
            "wrong_tool_routing",
            "no_response",
            "repetitive_fallback",
            "retry",
            "user_frustration",
            "evaluator_uncertainty",
        ]
    } else {
        Vec::new()
    };
    let missing_required_signals: Vec<String> = required_signals
        .iter()
        .filter(|signal| !signal_counts.contains_key(**signal))
        .map(|signal| signal.to_string())
        .collect();
    let unclassified_rows = rows
        .iter()
        .filter(|row| {
            row.get("failure_signals")
                .and_then(Value::as_array)
                .map(Vec::is_empty)
                .unwrap_or(true)
        })
        .count();
    let ingest_ok = !rows.is_empty();
    let redaction_ok = leak_count == 0;
    let detector_ok = missing_required_signals.is_empty() && unclassified_rows == 0;
    let ok = ingest_ok && redaction_ok && detector_ok;
    let report = json!({
        "type": "eval_learning_loop_inbox",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_trace_inbox_ingest_contract", "ok": ingest_ok, "detail": format!("rows={}", rows.len())},
            {"id": "eval_trace_redaction_contract", "ok": redaction_ok, "detail": format!("private_leaks={}", leak_count)},
            {"id": "eval_failure_signal_detector_contract", "ok": detector_ok, "detail": format!("missing_required_signals={};unclassified_rows={}", missing_required_signals.len(), unclassified_rows)}
        ],
        "summary": {
            "source_traces": traces.len(),
            "inbox_rows": rows.len(),
            "private_leaks": leak_count,
            "unclassified_rows": unclassified_rows,
            "failure_signal_counts": signal_counts,
            "missing_required_signals": missing_required_signals
        },
        "inbox_path": inbox_path,
        "sources": {"trace_source": source_path},
        "rows": rows
    });
    let markdown = format!(
        "# Eval Learning Loop Inbox (Current)\n\n- generated_at: {}\n- ok: {}\n- inbox_rows: {}\n- private_leaks: {}\n- missing_required_signals: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        report.pointer("/summary/inbox_rows").and_then(|node| node.as_u64()).unwrap_or(0),
        leak_count,
        report.pointer("/summary/missing_required_signals").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0)
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_jsonl(&inbox_path, &rows).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop inbox outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn inbox_row(trace: &Value) -> Value {
    let user_text = trace_user_text(trace);
    let assistant_text = trace_assistant_text(trace);
    let failure_signals = detect_failure_signals(trace);
    let trace_id = trace_id(trace);
    let source_hash = stable_hash_hex(&format!(
        "{}\n{}\n{}\n{}\n{}",
        trace_source(trace).unwrap_or("unknown"),
        trace_id,
        failure_code(trace),
        user_text,
        assistant_text
    ));
    json!({
        "trace_id": trace_id,
        "agent_id": str_at(trace, &["agent_id"])
            .or_else(|| str_at(trace, &["evidence", "agent_id"]))
            .unwrap_or(""),
        "source": trace_source(trace).unwrap_or("unknown"),
        "case_id": str_at(trace, &["case_id"]).unwrap_or(""),
        "turn_id": str_at(trace, &["turn_id"]).unwrap_or(""),
        "component": str_at(trace, &["component"]).unwrap_or(""),
        "workflow": trace_workflow(trace),
        "phase": trace_phase(trace),
        "tool_family": trace_tool_family(trace),
        "normalized_failure_code": failure_code(trace),
        "receipt_ids": trace_receipt_ids(trace),
        "monitor_evidence_id": format!("eval-monitor:{source_hash}"),
        "failure_signals": failure_signals,
        "suspected_layer": suspected_layer(trace),
        "confidence": signal_confidence(trace),
        "sanitized_user_text": redact_text(user_text),
        "sanitized_assistant_text": redact_text(assistant_text),
        "evidence_summary": str_at(trace, &["summary"])
            .or_else(|| str_at(trace, &["tool_result_summary"]))
            .unwrap_or(""),
        "source_hash": source_hash,
        "raw_text_excluded": true,
        "private_content_redacted": true
    })
}

fn detect_failure_signals(trace: &Value) -> Vec<String> {
    let mut signals = Vec::new();
    let code = lower(failure_code(trace));
    let tool_family = lower(trace_tool_family(trace));
    let user_text = lower(trace_user_text(trace));
    let assistant_text = lower(trace_assistant_text(trace));
    let finalization = lower(str_at(trace, &["finalization_status"]).unwrap_or(""));
    let component = lower(str_at(trace, &["component"]).unwrap_or(""));
    if code.contains("wrong_tool")
        || code.contains("routed_to_web")
        || code.contains("stale_php_context")
        || code.contains("stale_context")
        || (tool_family.contains("web") && mentions_local_workspace(&user_text))
    {
        signals.push("wrong_tool_routing".to_string());
    }
    if finalization.contains("no_response")
        || code.contains("no_response")
        || assistant_text.trim().is_empty()
    {
        signals.push("no_response".to_string());
    }
    if finalization.contains("fallback_loop")
        || assistant_text.contains("completed the workflow gate")
        || assistant_text.contains("please retry")
    {
        signals.push("repetitive_fallback".to_string());
    }
    if u64_at(trace, &["retry_count"]) > 0 {
        signals.push("retry".to_string());
    }
    if code.contains("latency_over_budget")
        || code.contains("stage_count_over_budget")
        || code.contains("too_many_workflow_stages")
        || code.contains("tokens_over_budget")
    {
        signals.push("action_economy".to_string());
    }
    if code.contains("workflow_visibility") {
        signals.push("workflow_visibility".to_string());
    }
    if user_text.contains("what's going on")
        || user_text.contains("whats going on")
        || user_text.contains("just answer")
        || user_text.contains("hardlocked")
        || user_text.contains("why is")
        || user_text.contains("what? why")
        || user_text.contains("why are you")
    {
        signals.push("user_frustration".to_string());
    }
    if bool_at(trace, &["evaluator_uncertainty"])
        || code.contains("uncertain")
        || component.contains("telemetry")
    {
        signals.push("evaluator_uncertainty".to_string());
    }
    signals.sort();
    signals.dedup();
    signals
}

fn suspected_layer(trace: &Value) -> &'static str {
    let workflow = lower(trace_workflow(trace));
    let phase = lower(trace_phase(trace));
    let component = lower(str_at(trace, &["component"]).unwrap_or(""));
    if workflow.contains("tool") || phase.contains("tool") {
        "surface/orchestration/tool-routing"
    } else if phase.contains("final") || phase.contains("recovery") {
        "surface/orchestration/workflow-finalization"
    } else if component.contains("telemetry") {
        "surface/orchestration/telemetry"
    } else if workflow.contains("eval") {
        "surface/orchestration/eval"
    } else {
        "surface/orchestration"
    }
}

fn signal_confidence(trace: &Value) -> f64 {
    let count = detect_failure_signals(trace).len() as f64;
    (0.45 + (count * 0.1)).min(0.95)
}

fn trace_id(trace: &Value) -> String {
    if let Some(id) = str_at(trace, &["trace_id"]) {
        return id.to_string();
    }
    let source = trace_source(trace).unwrap_or("unknown");
    let case_id = str_at(trace, &["case_id"]).unwrap_or("case");
    let turn_id = str_at(trace, &["turn_id"]).unwrap_or("turn");
    let code = failure_code(trace);
    format!("{case_id}:{turn_id}:{code}:{}", stable_hash_hex(source))
}

fn trace_source(trace: &Value) -> Option<&str> {
    str_at(trace, &["source"])
}

fn trace_user_text(trace: &Value) -> &str {
    str_at(trace, &["user_text"])
        .or_else(|| str_at(trace, &["user_message_preview"]))
        .or_else(|| str_at(trace, &["evidence", "user_message_preview"]))
        .unwrap_or("")
}

fn trace_assistant_text(trace: &Value) -> &str {
    str_at(trace, &["assistant_text"])
        .or_else(|| str_at(trace, &["assistant_response_preview"]))
        .or_else(|| str_at(trace, &["evidence", "assistant_response_preview"]))
        .unwrap_or("")
}

fn failure_code(trace: &Value) -> &str {
    str_at(trace, &["normalized_failure_code"])
        .or_else(|| str_at(trace, &["code"]))
        .unwrap_or("none")
}

fn trace_workflow(trace: &Value) -> &str {
    str_at(trace, &["workflow"])
        .or_else(|| str_at(trace, &["component"]))
        .unwrap_or("unknown")
}

fn trace_phase(trace: &Value) -> &str {
    str_at(trace, &["phase"])
        .or_else(|| str_at(trace, &["turn_id"]))
        .or_else(|| str_at(trace, &["case_id"]))
        .unwrap_or("unknown")
}

fn trace_tool_family(trace: &Value) -> &str {
    str_at(trace, &["tool_family"])
        .or_else(|| str_at(trace, &["evidence", "expected_route"]))
        .or_else(|| {
            let code = failure_code(trace);
            if code.contains("web") {
                Some("web_search")
            } else if code.contains("workspace") {
                Some("workspace_search")
            } else if code.contains("tool") {
                Some("tool_route")
            } else {
                None
            }
        })
        .unwrap_or("unknown")
}

fn trace_receipt_ids(trace: &Value) -> Value {
    for path in [
        ["receipt_ids"].as_slice(),
        ["evidence", "receipt_ids"].as_slice(),
    ] {
        if let Some(ids) = value_at(trace, path).and_then(Value::as_array) {
            if !ids.is_empty() {
                return Value::Array(ids.clone());
            }
        }
    }
    json!([])
}

fn redact_text(raw: &str) -> String {
    raw.split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| ch == '"' || ch == '\'' || ch == ',' || ch == ';');
    if trimmed.starts_with("github_pat_") || trimmed.starts_with("ghp_") {
        return "[redacted:token]".to_string();
    }
    if trimmed.contains('@') && trimmed.contains('.') {
        return "[redacted:email]".to_string();
    }
    if trimmed.starts_with("/Users/") || trimmed.starts_with("C:\\Users\\") {
        return "[redacted:local_path]".to_string();
    }
    token.to_string()
}

fn row_has_private_leak(row: &Value) -> bool {
    let payload = serde_json::to_string(row).unwrap_or_default();
    payload.contains("github_pat_")
        || payload.contains("ghp_")
        || payload.contains("/Users/")
        || payload.contains("C:\\Users\\")
}

fn mentions_local_workspace(text: &str) -> bool {
    text.contains("local")
        || text.contains("file")
        || text.contains("directory")
        || text.contains("repo")
        || text.contains("workspace")
}

fn stable_hash_hex(raw: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn lower(raw: &str) -> String {
    raw.to_ascii_lowercase()
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline_prefix = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            if let Some(value) = args.get(idx + 1) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    match parse_flag(args, key).as_deref() {
        Some("1" | "true" | "TRUE" | "yes" | "on") => true,
        Some("0" | "false" | "FALSE" | "no" | "off") => false,
        _ => default,
    }
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn u64_at(value: &Value, path: &[&str]) -> u64 {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return 0;
        };
        cursor = next;
    }
    cursor.as_u64().unwrap_or(0)
}

fn bool_at(value: &Value, path: &[&str]) -> bool {
    value_at(value, path)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    Some(cursor)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn write_jsonl(path: &str, rows: &[Value]) -> io::Result<()> {
    ensure_parent(path)?;
    let mut payload = String::new();
    for row in rows {
        payload.push_str(&serde_json::to_string(row).unwrap_or_else(|_| "{}".to_string()));
        payload.push('\n');
    }
    fs::write(path, payload)
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_removes_tokens_paths_and_email() {
        let redacted = redact_text(
            "github_pat_secret ghp_secret /Users/jay/private C:\\Users\\Owner\\secret jay@example.com",
        );
        assert!(!redacted.contains("github_pat_"));
        assert!(!redacted.contains("ghp_"));
        assert!(!redacted.contains("/Users/"));
        assert!(!redacted.contains("C:\\Users\\"));
        assert!(!redacted.contains("jay@example.com"));
    }

    #[test]
    fn detectors_classify_wrong_tool_and_frustration() {
        let trace = json!({
            "tool_family": "web_search",
            "user_text": "What's going on with local file tooling?",
            "assistant_text": "Please retry.",
            "finalization_status": "fallback_loop",
            "normalized_failure_code": "local_file_intent_routed_to_web",
            "retry_count": 1
        });
        let signals = detect_failure_signals(&trace);
        assert!(signals.contains(&"wrong_tool_routing".to_string()));
        assert!(signals.contains(&"user_frustration".to_string()));
        assert!(signals.contains(&"repetitive_fallback".to_string()));
        assert!(signals.contains(&"retry".to_string()));
    }

    #[test]
    fn live_synthetic_trace_schema_preserves_monitor_metadata() {
        // SRS: V12-EVAL-MONITOR-FEEDBACK-001
        let trace = json!({
            "schema_version": 1,
            "source": "synthetic_user_chat_harness:round13",
            "agent_id": "agent-5bc62b0875a9",
            "case_id": "explicit_web_tool_request",
            "turn_id": "web_001",
            "component": "surface.orchestration.tool_routing",
            "code": "wrong_tool_web_request_stale_php_context",
            "severity": "high",
            "summary": "Explicit web-search request returned stale PHP context.",
            "evidence": {
                "agent_id": "agent-5bc62b0875a9",
                "user_message_preview": "Use web search to compare frameworks.",
                "assistant_response_preview": "<?php namespace App\\Http\\Controllers; class ProductController {}",
                "expected_route": "web_search",
                "failures": ["missing_required_visible_text:web search"]
            }
        });

        let row = inbox_row(&trace);
        assert_eq!(str_at(&row, &["agent_id"]), Some("agent-5bc62b0875a9"));
        assert_eq!(
            str_at(&row, &["normalized_failure_code"]),
            Some("wrong_tool_web_request_stale_php_context")
        );
        assert_eq!(str_at(&row, &["tool_family"]), Some("web_search"));
        assert_eq!(
            str_at(&row, &["sanitized_user_text"]),
            Some("Use web search to compare frameworks.")
        );
        assert!(str_at(&row, &["monitor_evidence_id"])
            .unwrap_or("")
            .starts_with("eval-monitor:fnv64:"));
        let signals = row
            .get("failure_signals")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(signals.iter().any(|signal| signal == "wrong_tool_routing"));
        assert!(!signals.iter().any(|signal| signal == "no_response"));
    }
}
