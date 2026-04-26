use serde_json::{json, Value};
use std::collections::BTreeMap;
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
const MIN_RECURRENT_SIGNATURE_COUNT: usize = 2;

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
    for cluster in clustered_issue_rows(rows.as_slice()) {
        let row = cluster
            .representative
            .unwrap_or_else(|| rows.first().unwrap_or(&Value::Null));
        let mut candidate = issue_candidate_from_inbox_row(row);
        attach_cluster_evidence(&mut candidate, &cluster);
        let quality_failures = issue_candidate_quality_failures(&candidate);
        if quality_failures.is_empty() {
            candidates.push(candidate);
        } else {
            rejected.push(json!({
                "trace_id": str_at(row, &["trace_id"]).unwrap_or("unknown"),
                "stable_failure_signature": cluster.signature,
                "recurrence_count": cluster.rows.len(),
                "quality_failures": quality_failures,
                "candidate": candidate
            }));
        }
    }
    let drafting_ok = !candidates.is_empty();
    let quality_ok = rejected.is_empty();
    let receipt_grounding_ok = candidates.iter().all(candidate_has_grounded_evidence);
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
            "receipt_grounded": receipt_grounding_ok,
            "minimum_recurrent_signature_count": MIN_RECURRENT_SIGNATURE_COUNT
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

struct IssueRowCluster<'a> {
    signature: String,
    rows: Vec<&'a Value>,
    representative: Option<&'a Value>,
    critical_bypass: bool,
}

fn clustered_issue_rows(rows: &[Value]) -> Vec<IssueRowCluster<'_>> {
    let mut grouped: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for row in rows {
        grouped
            .entry(stable_failure_signature(row))
            .or_default()
            .push(row);
    }
    grouped
        .into_iter()
        .map(|(signature, rows)| {
            let critical_bypass = rows.iter().any(|row| is_critical_severity(row));
            let representative = rows
                .iter()
                .copied()
                .find(|row| is_critical_severity(row))
                .or_else(|| rows.first().copied());
            IssueRowCluster {
                signature,
                rows,
                representative,
                critical_bypass,
            }
        })
        .collect()
}

fn stable_failure_signature(row: &Value) -> String {
    let signal = primary_signal(row);
    let code = str_at(row, &["normalized_failure_code"]).unwrap_or("none");
    let layer = str_at(row, &["suspected_layer"])
        .or_else(|| str_at(row, &["component"]))
        .unwrap_or("surface/orchestration");
    let quality = runtime_quality_signature(row.get("runtime_quality").unwrap_or(&Value::Null));
    format!("{layer}|{signal}|{code}|{quality}")
}

fn runtime_quality_signature(runtime_quality: &Value) -> String {
    [
        (
            "candidate_count",
            runtime_quality_value(runtime_quality, "candidate_count"),
        ),
        (
            "typed_probe_contract_gap_count",
            runtime_quality_value(runtime_quality, "typed_probe_contract_gap_count"),
        ),
        (
            "heuristic_probe_source_count",
            runtime_quality_value(runtime_quality, "heuristic_probe_source_count"),
        ),
        (
            "fallback_action_count",
            runtime_quality_value(runtime_quality, "fallback_action_count"),
        ),
        (
            "zero_executable_candidates",
            runtime_quality_value(runtime_quality, "zero_executable_candidates"),
        ),
        (
            "surface_adapter_fallback",
            runtime_quality_value(runtime_quality, "surface_adapter_fallback"),
        ),
    ]
    .iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<_>>()
    .join(";")
}

fn runtime_quality_value(runtime_quality: &Value, key: &str) -> String {
    runtime_quality
        .get(key)
        .map(|value| {
            if let Some(raw) = value.as_u64() {
                raw.to_string()
            } else if let Some(raw) = value.as_bool() {
                raw.to_string()
            } else if let Some(raw) = value.as_str() {
                raw.to_string()
            } else {
                "unknown".to_string()
            }
        })
        .unwrap_or_else(|| "missing".to_string())
}

fn runtime_quality_metrics(row: &Value) -> Value {
    let runtime_quality = row.get("runtime_quality").unwrap_or(&Value::Null);
    json!({
        "candidate_count": runtime_quality.get("candidate_count").cloned().unwrap_or(Value::Null),
        "used_heuristic_probe": runtime_quality.get("used_heuristic_probe").cloned().unwrap_or(Value::Null),
        "heuristic_probe_source_count": runtime_quality.get("heuristic_probe_source_count").cloned().unwrap_or(Value::Null),
        "typed_probe_contract_gap_count": runtime_quality.get("typed_probe_contract_gap_count").cloned().unwrap_or(Value::Null),
        "fallback_action_count": runtime_quality.get("fallback_action_count").cloned().unwrap_or(Value::Null),
        "zero_executable_candidates": runtime_quality.get("zero_executable_candidates").cloned().unwrap_or(Value::Null),
        "surface_adapter_fallback": runtime_quality.get("surface_adapter_fallback").cloned().unwrap_or(Value::Null)
    })
}

fn attach_cluster_evidence(candidate: &mut Value, cluster: &IssueRowCluster<'_>) {
    let trace_ids = cluster
        .rows
        .iter()
        .filter_map(|row| str_at(row, &["trace_id"]))
        .collect::<Vec<_>>();
    let signature = json!(cluster.signature);
    let recurrence_count = json!(cluster.rows.len());
    candidate["stable_failure_signature"] = signature.clone();
    candidate["recurrence_count"] = recurrence_count.clone();
    candidate["representative_trace_ids"] = json!(trace_ids);
    candidate["critical_bypass"] = json!(cluster.critical_bypass);
    if let Some(evidence) = candidate.get_mut("evidence") {
        evidence["runtime_quality_metrics"] = cluster
            .representative
            .map(runtime_quality_metrics)
            .unwrap_or_else(|| json!({}));
        evidence["stable_failure_signature"] = signature;
        evidence["recurrence_count"] = recurrence_count;
    }
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
        "agent_id": str_at(row, &["agent_id"]).unwrap_or(""),
        "source": str_at(row, &["source"]).unwrap_or("unknown"),
        "case_id": str_at(row, &["case_id"]).unwrap_or(""),
        "turn_id": str_at(row, &["turn_id"]).unwrap_or(""),
        "severity": severity_for_row(row),
        "trace_id": trace_id,
        "symptom": symptom_for_signal(signal),
        "expected_behavior": expected_behavior_for_signal(signal),
        "actual_behavior": str_at(row, &["sanitized_assistant_text"]).unwrap_or(""),
        "evidence": {
            "trace_id": trace_id,
            "agent_id": str_at(row, &["agent_id"]).unwrap_or(""),
            "source": str_at(row, &["source"]).unwrap_or("unknown"),
            "case_id": str_at(row, &["case_id"]).unwrap_or(""),
            "turn_id": str_at(row, &["turn_id"]).unwrap_or(""),
            "component": str_at(row, &["component"]).unwrap_or(""),
            "receipt_ids": row.get("receipt_ids").cloned().unwrap_or_else(|| json!([])),
            "monitor_evidence_id": str_at(row, &["monitor_evidence_id"]).unwrap_or(""),
            "source_hash": str_at(row, &["source_hash"]).unwrap_or(""),
            "failure_signals": row.get("failure_signals").cloned().unwrap_or_else(|| json!([])),
            "normalized_failure_code": str_at(row, &["normalized_failure_code"]).unwrap_or("none"),
            "sanitized_user_text": str_at(row, &["sanitized_user_text"]).unwrap_or(""),
            "sanitized_assistant_text": str_at(row, &["sanitized_assistant_text"]).unwrap_or(""),
            "evidence_summary": str_at(row, &["evidence_summary"]).unwrap_or(""),
            "runtime_quality": row.get("runtime_quality").cloned().unwrap_or_else(|| json!({})),
            "workflow_quality": row.get("workflow_quality").cloned().unwrap_or(Value::Null)
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
    let recurrence_count = candidate
        .get("recurrence_count")
        .and_then(|node| node.as_u64())
        .unwrap_or(0) as usize;
    let critical_bypass = candidate
        .get("critical_bypass")
        .and_then(|node| node.as_bool())
        .unwrap_or(false);
    if recurrence_count < MIN_RECURRENT_SIGNATURE_COUNT && !critical_bypass {
        failures.push("insufficient_recurrent_failure_signature".to_string());
    }
    if str_at(candidate, &["stable_failure_signature"]).is_none() {
        failures.push("missing_stable_failure_signature".to_string());
    }
    if candidate
        .pointer("/evidence/runtime_quality_metrics")
        .and_then(|node| node.as_object())
        .map(|metrics| metrics.is_empty())
        .unwrap_or(true)
    {
        failures.push("missing_runtime_quality_metrics".to_string());
    }
    if str_at(candidate, &["symptom"]).is_none() {
        failures.push("missing_symptom".to_string());
    }
    if !candidate_has_grounded_evidence(candidate) {
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

fn candidate_has_grounded_evidence(candidate: &Value) -> bool {
    let has_receipts = candidate
        .pointer("/evidence/receipt_ids")
        .and_then(|node| node.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false);
    let has_monitor_evidence = str_at(candidate, &["evidence", "monitor_evidence_id"]).is_some()
        && str_at(candidate, &["evidence", "source_hash"]).is_some();
    (has_receipts || has_monitor_evidence) && str_at(candidate, &["evidence", "trace_id"]).is_some()
}

fn primary_signal(row: &Value) -> &str {
    let signals = row
        .get("failure_signals")
        .and_then(|node| node.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for preferred in [
        "wrong_tool_routing",
        "no_response",
        "repetitive_fallback",
        "user_frustration",
        "workflow_visibility",
        "action_economy",
        "retry",
        "evaluator_uncertainty",
    ] {
        if signals
            .iter()
            .any(|signal| signal.as_str() == Some(preferred))
        {
            return preferred;
        }
    }
    signals
        .first()
        .and_then(|node| node.as_str())
        .unwrap_or("unknown")
}

fn severity_for_row(row: &Value) -> &'static str {
    if is_critical_severity(row) {
        return "critical";
    }
    if str_at(row, &["severity"]) == Some("high") {
        return "high";
    }
    let signal = primary_signal(row);
    match signal {
        "wrong_tool_routing" | "no_response" | "repetitive_fallback" => "high",
        _ => "warn",
    }
}

fn is_critical_severity(row: &Value) -> bool {
    matches!(str_at(row, &["severity"]), Some("critical"))
}

fn symptom_for_signal(signal: &str) -> &'static str {
    match signal {
        "wrong_tool_routing" => "Local/tool intent was routed to the wrong tool family.",
        "no_response" => "Workflow did not produce a usable final answer.",
        "repetitive_fallback" => "Workflow finalization repeated fallback boilerplate.",
        "retry" => "Recovery required repeated tool or workflow attempts.",
        "action_economy" => "Workflow exceeded latency, stage, or response-size budget.",
        "workflow_visibility" => "Workflow visibility telemetry was missing for a live turn.",
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
        "action_economy" => "Keep simple turns within latency, stage, and token budgets.",
        "workflow_visibility" => "Emit workflow visibility payloads for every live workflow turn.",
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
        "action_economy" => "workflow path exceeded the intended action economy budget",
        "workflow_visibility" => {
            "workflow telemetry did not expose the active stage to the UI/eval monitor"
        }
        "user_frustration" => "synthesis did not prioritize direct user-facing clarification",
        "evaluator_uncertainty" => {
            "eval calibration signal was insufficient for confident judgement"
        }
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
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
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

    fn grounded_row(trace_id: &str, severity: &str) -> Value {
        json!({
            "trace_id": trace_id,
            "agent_id": "agent-5bc62b0875a9",
            "source": "synthetic_user_chat_harness:test",
            "case_id": "explicit_web_tool_request",
            "turn_id": "web_001",
            "component": "surface.orchestration.tool_routing",
            "severity": severity,
            "suspected_layer": "surface/orchestration/tool-routing",
            "confidence": 0.75,
            "receipt_ids": [],
            "monitor_evidence_id": "eval-monitor:fnv64:abc",
            "source_hash": "fnv64:abc",
            "failure_signals": ["wrong_tool_routing"],
            "normalized_failure_code": "wrong_tool_web_request_stale_php_context",
            "sanitized_user_text": "Use web search to compare frameworks.",
            "sanitized_assistant_text": "<?php class ProductController {}",
            "evidence_summary": "Explicit web-search request returned stale PHP context.",
            "runtime_quality": {
                "candidate_count": 4,
                "used_heuristic_probe": false,
                "heuristic_probe_source_count": 0,
                "zero_executable_candidates": false,
                "typed_probe_contract_gap_count": 0,
                "fallback_action_count": 0,
                "surface_adapter_fallback": false
            },
            "workflow_quality": {
                "workflow": "forge_code",
                "signals": {
                    "semantic_discovery_route_required": true
                }
            }
        })
    }

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

    #[test]
    fn issue_candidate_accepts_source_hashed_monitor_evidence() {
        let rows = vec![
            grounded_row(
                "explicit_web_tool_request:web_001:wrong_tool_web_request_stale_php_context:fnv64:abc",
                "high",
            ),
            grounded_row(
                "explicit_web_tool_request:web_002:wrong_tool_web_request_stale_php_context:fnv64:def",
                "high",
            ),
        ];
        let cluster = clustered_issue_rows(rows.as_slice())
            .into_iter()
            .next()
            .expect("cluster");
        let mut candidate = issue_candidate_from_inbox_row(cluster.representative.expect("row"));
        attach_cluster_evidence(&mut candidate, &cluster);
        assert_eq!(
            str_at(&candidate, &["agent_id"]),
            Some("agent-5bc62b0875a9")
        );
        assert_eq!(str_at(&candidate, &["severity"]), Some("high"));
        assert_eq!(
            str_at(&candidate, &["evidence", "monitor_evidence_id"]),
            Some("eval-monitor:fnv64:abc")
        );
        assert!(candidate_has_grounded_evidence(&candidate));
        assert_eq!(
            candidate.pointer("/evidence/runtime_quality/candidate_count"),
            Some(&json!(4))
        );
        assert_eq!(
            candidate.pointer("/evidence/runtime_quality_metrics/candidate_count"),
            Some(&json!(4))
        );
        assert_eq!(
            candidate
                .pointer("/evidence/workflow_quality/signals/semantic_discovery_route_required"),
            Some(&json!(true))
        );
        assert!(issue_candidate_quality_failures(&candidate).is_empty());
    }

    #[test]
    fn issue_candidate_rejects_one_off_noncritical_degradation() {
        let rows = vec![grounded_row("one-off-trace", "high")];
        let cluster = clustered_issue_rows(rows.as_slice())
            .into_iter()
            .next()
            .expect("cluster");
        let mut candidate = issue_candidate_from_inbox_row(cluster.representative.expect("row"));
        attach_cluster_evidence(&mut candidate, &cluster);

        let failures = issue_candidate_quality_failures(&candidate);
        assert!(failures.contains(&"insufficient_recurrent_failure_signature".to_string()));
    }

    #[test]
    fn issue_candidate_allows_critical_singleton_bypass() {
        let rows = vec![grounded_row("critical-trace", "critical")];
        let cluster = clustered_issue_rows(rows.as_slice())
            .into_iter()
            .next()
            .expect("cluster");
        let mut candidate = issue_candidate_from_inbox_row(cluster.representative.expect("row"));
        attach_cluster_evidence(&mut candidate, &cluster);

        assert_eq!(str_at(&candidate, &["severity"]), Some("critical"));
        assert_eq!(candidate.get("critical_bypass"), Some(&json!(true)));
        assert!(issue_candidate_quality_failures(&candidate).is_empty());
    }
}
