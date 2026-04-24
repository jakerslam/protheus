use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_ISSUE_SOURCE_PATH: &str = "artifacts/eval_learning_loop_inbox_latest.json";
const DEFAULT_ISSUE_OUT_PATH: &str =
    "core/local/artifacts/eval_learning_loop_issue_candidates_current.json";
const DEFAULT_ISSUE_OUT_LATEST_PATH: &str =
    "artifacts/eval_learning_loop_issue_candidates_latest.json";
const DEFAULT_ISSUE_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_LEARNING_LOOP_ISSUE_CANDIDATES_CURRENT.md";

pub fn run_eval_learning_loop_issue_candidates(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let source_path =
        parse_flag(args, "source").unwrap_or_else(|| DEFAULT_ISSUE_SOURCE_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_ISSUE_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_ISSUE_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_ISSUE_MARKDOWN_PATH.to_string());
    let source = read_json(&source_path);
    let rows = source
        .get("rows")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let mut candidates = Vec::new();
    let mut rejected = Vec::new();
    for row in rows.iter() {
        let candidate = issue_candidate_from_inbox_row(row);
        let quality_failures = issue_candidate_quality_failures(&candidate);
        if quality_failures.is_empty() {
            candidates.push(candidate);
        } else {
            rejected.push(json!({
                "trace_id": str_at(row, &["trace_id"]).unwrap_or("unknown"),
                "quality_failures": quality_failures,
                "candidate": candidate
            }));
        }
    }
    let drafting_ok = !candidates.is_empty();
    let quality_ok = rejected.is_empty();
    let receipt_grounding_ok = candidates.iter().all(candidate_has_receipt_evidence);
    let ok = drafting_ok && quality_ok && receipt_grounding_ok;
    let report = json!({
        "type": "eval_learning_loop_issue_candidates",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_issue_candidate_drafting_contract", "ok": drafting_ok, "detail": format!("candidates={}", candidates.len())},
            {"id": "eval_issue_candidate_quality_gate_contract", "ok": quality_ok, "detail": format!("rejected={}", rejected.len())},
            {"id": "eval_issue_candidate_receipt_grounding_contract", "ok": receipt_grounding_ok, "detail": "every accepted candidate has receipt-backed evidence"}
        ],
        "summary": {
            "source_rows": rows.len(),
            "accepted_candidates": candidates.len(),
            "rejected_candidates": rejected.len(),
            "receipt_grounded": receipt_grounding_ok
        },
        "sources": {"inbox": source_path},
        "candidates": candidates,
        "rejected": rejected
    });
    let markdown = format!(
        "# Eval Learning Loop Issue Candidates (Current)\n\n- generated_at: {}\n- ok: {}\n- accepted_candidates: {}\n- rejected_candidates: {}\n- receipt_grounded: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        report.pointer("/summary/accepted_candidates").and_then(|node| node.as_u64()).unwrap_or(0),
        report.pointer("/summary/rejected_candidates").and_then(|node| node.as_u64()).unwrap_or(0),
        receipt_grounding_ok
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop issue candidate outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn issue_candidate_from_inbox_row(row: &Value) -> Value {
    let trace_id = str_at(row, &["trace_id"]).unwrap_or("unknown");
    let signal = primary_signal(row);
    let suspected_layer = str_at(row, &["suspected_layer"]).unwrap_or("surface/orchestration");
    let confidence = row
        .get("confidence")
        .and_then(|node| node.as_f64())
        .unwrap_or(0.5);
    json!({
        "id": format!("eval-learning-{trace_id}"),
        "trace_id": trace_id,
        "symptom": symptom_for_signal(signal),
        "expected_behavior": expected_behavior_for_signal(signal),
        "actual_behavior": str_at(row, &["sanitized_assistant_text"]).unwrap_or(""),
        "evidence": {
            "trace_id": trace_id,
            "receipt_ids": row.get("receipt_ids").cloned().unwrap_or_else(|| json!([])),
            "failure_signals": row.get("failure_signals").cloned().unwrap_or_else(|| json!([])),
            "normalized_failure_code": str_at(row, &["normalized_failure_code"]).unwrap_or("none"),
            "sanitized_user_text": str_at(row, &["sanitized_user_text"]).unwrap_or(""),
            "sanitized_assistant_text": str_at(row, &["sanitized_assistant_text"]).unwrap_or("")
        },
        "suspected_layer": suspected_layer,
        "suspected_root_cause": root_cause_for_signal(signal),
        "root_cause_basis": row.get("failure_signals").cloned().unwrap_or_else(|| json!([])),
        "owner_component": suspected_layer,
        "confidence": confidence,
        "repro_path": format!("cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- learning-loop-issues --source={} --strict=1", DEFAULT_ISSUE_SOURCE_PATH),
        "acceptance_criteria": [
            format!("A replay of {trace_id} no longer emits the {signal} failure signal"),
            "The fix preserves receipt-grounded evidence in the eval learning-loop issue candidate",
            "The quality gate accepts the candidate with zero unsupported root-cause failures"
        ],
        "suggested_test": format!("Add or update an eval learning-loop fixture covering {signal}"),
        "issue_readiness": "candidate_ready"
    })
}

fn issue_candidate_quality_failures(candidate: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    if str_at(candidate, &["symptom"]).is_none() {
        failures.push("missing_symptom".to_string());
    }
    if !candidate_has_receipt_evidence(candidate) {
        failures.push("missing_receipt_backed_evidence".to_string());
    }
    if str_at(candidate, &["suspected_root_cause"]).is_none()
        || candidate
            .get("root_cause_basis")
            .and_then(|node| node.as_array())
            .map(|rows| rows.is_empty())
            .unwrap_or(true)
    {
        failures.push("unsupported_root_cause".to_string());
    }
    if str_at(candidate, &["repro_path"]).is_none() {
        failures.push("missing_repro_path".to_string());
    }
    if str_at(candidate, &["owner_component"]).is_none() {
        failures.push("missing_owner_component".to_string());
    }
    let acceptance_ok = candidate
        .get("acceptance_criteria")
        .and_then(|node| node.as_array())
        .map(|rows| rows.len() >= 3)
        .unwrap_or(false);
    if !acceptance_ok {
        failures.push("missing_acceptance_criteria".to_string());
    }
    failures
}

fn candidate_has_receipt_evidence(candidate: &Value) -> bool {
    candidate
        .pointer("/evidence/receipt_ids")
        .and_then(|node| node.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        && str_at(candidate, &["evidence", "trace_id"]).is_some()
}

fn primary_signal(row: &Value) -> &str {
    row.get("failure_signals")
        .and_then(|node| node.as_array())
        .and_then(|rows| rows.first())
        .and_then(|node| node.as_str())
        .unwrap_or("unknown")
}

fn symptom_for_signal(signal: &str) -> &'static str {
    match signal {
        "wrong_tool_routing" => "Local/tool intent was routed to the wrong tool family.",
        "no_response" => "Workflow did not produce a usable final answer.",
        "repetitive_fallback" => "Workflow finalization repeated fallback boilerplate.",
        "retry" => "Recovery required repeated tool or workflow attempts.",
        "user_frustration" => "User-visible recovery failed to answer direct frustration.",
        "evaluator_uncertainty" => "Evaluator confidence was too weak for promotion.",
        _ => "Eval learning-loop trace requires triage.",
    }
}

fn expected_behavior_for_signal(signal: &str) -> &'static str {
    match signal {
        "wrong_tool_routing" => "Route local/workspace intent only to workspace-capable tools.",
        "no_response" => "Return a direct final answer or explicit grounded fallback.",
        "repetitive_fallback" => "Break fallback loops and answer from current workflow state.",
        "retry" => "Explain retry cause and bounded recovery path with receipts.",
        "user_frustration" => "Respond directly to the user's confusion before tool recovery.",
        "evaluator_uncertainty" => "Block promotion and request more reviewed evidence.",
        _ => "Triage should preserve trace evidence and classify a concrete owner.",
    }
}

fn root_cause_for_signal(signal: &str) -> &'static str {
    match signal {
        "wrong_tool_routing" => "tool-route classification selected an incompatible tool family",
        "no_response" => "workflow finalization failed to synthesize a user-visible answer",
        "repetitive_fallback" => "fallback recovery did not detect repeated boilerplate",
        "retry" => "recovery path exceeded the intended action economy budget",
        "user_frustration" => "synthesis did not prioritize direct user-facing clarification",
        "evaluator_uncertainty" => "eval calibration signal was insufficient for confident judgement",
        _ => "learning-loop classifier produced an unknown signal",
    }
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
    cursor.as_str().map(str::trim).filter(|value| !value.is_empty())
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
    fn issue_quality_gate_rejects_ungrounded_candidates() {
        let candidate = json!({
            "symptom": "ungrounded",
            "evidence": {"trace_id": "trace-1", "receipt_ids": []},
            "suspected_root_cause": "unsupported",
            "root_cause_basis": [],
            "owner_component": "surface/orchestration",
            "repro_path": "cargo run --example replay",
            "acceptance_criteria": ["one", "two", "three"]
        });
        let failures = issue_candidate_quality_failures(&candidate);
        assert!(failures.contains(&"missing_receipt_backed_evidence".to_string()));
        assert!(failures.contains(&"unsupported_root_cause".to_string()));
    }
}
