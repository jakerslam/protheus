use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_MONITOR_PATH: &str = "local/state/ops/eval_agent_chat_monitor/latest.json";
const DEFAULT_PHASE_TRACE_PATH: &str =
    "local/state/ops/orchestration/workflow_phase_trace_latest.json";
const DEFAULT_TAXONOMY_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_issue_taxonomy_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_judge_lane_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_judge_lane_latest.json";
const DEFAULT_MIN_CONFIDENCE: f64 = 0.80;
const DEFAULT_MODEL: &str = "gpt-5.4";

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).unwrap_or_default()
        ),
    )
}

fn str_at<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn array_at<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn clean_token(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn issue_class_by_id(issue_id: &str) -> &'static str {
    match issue_id {
        "workflow_retry_macro_template_detected" => "response_loop",
        "workflow_route_automation_claim_detected" => "bad_workflow_selection",
        "auto_tool_selection_claim_detected" => "auto_tool_selection_claim",
        "policy_block_template_detected" => "policy_block_confusion",
        "file_tool_route_misdirection_detected" => "tool_output_misdirection",
        "repeated_response_loop_detected" => "response_loop",
        "unsupported_claim_detected" => "hallucination",
        "wrong_tool_selection_detected" => "wrong_tool_selection",
        "no_response_detected" => "no_response",
        _ => "unknown",
    }
}

fn critical_classes(taxonomy: &Value) -> BTreeSet<String> {
    array_at(taxonomy, "classes")
        .iter()
        .filter(|row| {
            row.get("critical")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn eligible_models() -> BTreeSet<&'static str> {
    BTreeSet::from(["gpt-5.5", "gpt-5.4", "gpt-5.3-codex", "gpt-5.3-codex-spark"])
}

fn select_model(monitor: &Value) -> Value {
    let env_model = env::var("INFRING_EVAL_JUDGE_MODEL").unwrap_or_default();
    let monitor_model = monitor
        .pointer("/summary/troubleshooting_latest_eval/model")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (candidate, source) = if !env_model.trim().is_empty() {
        (env_model.trim().to_string(), "env:INFRING_EVAL_JUDGE_MODEL")
    } else if !monitor_model.trim().is_empty() {
        (
            monitor_model.trim().to_string(),
            "monitor.troubleshooting_latest_eval.model",
        )
    } else {
        (DEFAULT_MODEL.to_string(), "default")
    };
    let eligible = eligible_models();
    let is_eligible = eligible.contains(candidate.as_str());
    json!({
        "selected_model": if is_eligible { candidate.clone() } else { DEFAULT_MODEL.to_string() },
        "candidate_model": candidate,
        "model_source": source,
        "eligible": is_eligible,
        "threshold_decision": if is_eligible { "eligible" } else { "fallback_to_default_strong_model" },
        "fallback_reason": if is_eligible { "none" } else { "candidate_model_not_in_eval_judge_allowlist" },
        "eligible_models": eligible.into_iter().collect::<Vec<_>>()
    })
}

fn phase_trace_receipt_count(phase_trace: &Value) -> usize {
    array_at(phase_trace, "observed_kernel_receipt_ids").len()
}

fn evidence_supported(issue: &Value, issue_class: &str, phase_trace: &Value) -> (bool, Vec<Value>) {
    let mut support = Vec::new();
    let evidence_rows = array_at(issue, "evidence");
    let receipt_count = phase_trace_receipt_count(phase_trace);
    if !evidence_rows.is_empty() {
        support.push(json!({
            "source": "monitor.issue.evidence",
            "count": evidence_rows.len()
        }));
    }
    if receipt_count > 0 {
        support.push(json!({
            "source": "orchestration.phase_trace.observed_kernel_receipt_ids",
            "count": receipt_count
        }));
    }
    let has_raw_turn = evidence_rows.iter().any(|row| {
        !str_at(row, "turn_id").trim().is_empty()
            && (!str_at(row, "snippet").trim().is_empty() || issue_class == "no_response")
    });
    if has_raw_turn {
        support.push(json!({
            "source": "monitor.issue.raw_turn_text",
            "count": evidence_rows.len()
        }));
    }
    (!support.is_empty() && has_raw_turn, support)
}

fn issue_ready(
    issue: &Value,
    issue_class: &str,
    known_classes: &BTreeSet<String>,
    phase_trace: &Value,
    min_confidence: f64,
) -> (bool, Vec<String>, Vec<Value>) {
    let mut blockers = Vec::new();
    if !known_classes.contains(issue_class) {
        blockers.push(format!("unknown_issue_class:{issue_class}"));
    }
    if str_at(issue, "severity").trim().is_empty() {
        blockers.push("missing_severity".to_string());
    }
    if str_at(issue, "summary").trim().is_empty() {
        blockers.push("missing_summary".to_string());
    }
    if str_at(issue, "next_action").trim().is_empty() {
        blockers.push("missing_proposed_fix".to_string());
    }
    if array_at(issue, "acceptance_criteria").is_empty() {
        blockers.push("missing_acceptance_criteria".to_string());
    }
    let confidence = issue
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if confidence < min_confidence {
        blockers.push("confidence_below_threshold".to_string());
    }
    let (supported, support) = evidence_supported(issue, issue_class, phase_trace);
    if !supported {
        blockers.push("unsupported_judge_claim".to_string());
    }
    (blockers.is_empty(), blockers, support)
}

fn judge_rows(
    monitor: &Value,
    phase_trace: &Value,
    known_classes: &BTreeSet<String>,
    min_confidence: f64,
) -> Vec<Value> {
    array_at(monitor, "issues")
        .iter()
        .map(|issue| {
            let issue_id = str_at(issue, "id");
            let issue_class = issue_class_by_id(issue_id);
            let (ready, blockers, support) = issue_ready(
                issue,
                issue_class,
                known_classes,
                phase_trace,
                min_confidence,
            );
            let evidence = array_at(issue, "evidence")
                .iter()
                .take(4)
                .map(|row| {
                    json!({
                        "turn_id": str_at(row, "turn_id"),
                        "ts": str_at(row, "ts"),
                        "raw_turn_text": str_at(row, "snippet")
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "id": issue_id,
                "verdict": if ready { "issue_ready" } else { "needs_more_evidence" },
                "issue_class": issue_class,
                "severity": clean_token(str_at(issue, "severity")),
                "summary": str_at(issue, "summary"),
                "evidence": evidence,
                "proposed_fix": str_at(issue, "next_action"),
                "confidence": issue.get("confidence").and_then(Value::as_f64).unwrap_or(0.0),
                "issue_readiness": ready,
                "blockers": blockers,
                "supported_by": support,
                "owner_component": str_at(issue, "owner_component"),
                "owner_path": str_at(issue, "owner_path"),
                "acceptance_criteria": array_at(issue, "acceptance_criteria"),
                "phase_trace_id": str_at(phase_trace, "trace_id"),
                "phase_trace_receipt_count": phase_trace_receipt_count(phase_trace)
            })
        })
        .collect()
}

fn support_summary(rows: &[Value]) -> BTreeMap<String, usize> {
    let mut summary = BTreeMap::new();
    for row in rows {
        let verdict = str_at(row, "verdict").to_string();
        *summary.entry(verdict).or_insert(0) += 1;
    }
    summary
}

fn run() -> io::Result<(bool, Value)> {
    let args: Vec<String> = env::args().skip(1).collect();
    let monitor_path =
        parse_flag(&args, "monitor").unwrap_or_else(|| DEFAULT_MONITOR_PATH.to_string());
    let phase_trace_path =
        parse_flag(&args, "phase-trace").unwrap_or_else(|| DEFAULT_PHASE_TRACE_PATH.to_string());
    let taxonomy_path =
        parse_flag(&args, "taxonomy").unwrap_or_else(|| DEFAULT_TAXONOMY_PATH.to_string());
    let out_path = parse_flag(&args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path =
        parse_flag(&args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());

    let monitor = read_json(&monitor_path);
    let phase_trace = read_json(&phase_trace_path);
    let taxonomy = read_json(&taxonomy_path);
    let known_classes = critical_classes(&taxonomy);
    let model = select_model(&monitor);
    let rows = judge_rows(
        &monitor,
        &phase_trace,
        &known_classes,
        DEFAULT_MIN_CONFIDENCE,
    );
    let unsupported = rows
        .iter()
        .filter(|row| {
            array_at(row, "blockers")
                .iter()
                .any(|blocker| blocker.as_str() == Some("unsupported_judge_claim"))
        })
        .count();
    let ready = rows
        .iter()
        .filter(|row| {
            row.get("issue_readiness")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let ok = !rows.is_empty() && unsupported == 0 && ready == rows.len();
    let report = json!({
        "type": "eval_judge_lane",
        "schema_version": 1,
        "generated_at_ms": now_ms(),
        "ok": ok,
        "owner": "surface_orchestration_control_plane",
        "model": model,
        "policy": {
            "min_confidence": DEFAULT_MIN_CONFIDENCE,
            "unsupported_claims_allowed": false,
            "requires_phase_trace_receipts": true,
            "requires_raw_turn_evidence": true
        },
        "summary": {
            "judged_issue_count": rows.len(),
            "issue_ready_count": ready,
            "unsupported_claim_count": unsupported,
            "verdict_counts": support_summary(&rows)
        },
        "judge_outputs": rows,
        "sources": {
            "monitor": monitor_path,
            "phase_trace": phase_trace_path,
            "taxonomy": taxonomy_path
        }
    });
    write_json(&out_path, &report)?;
    write_json(&latest_path, &report)?;
    Ok((ok, report))
}

fn main() -> ExitCode {
    match run() {
        Ok((ok, report)) => {
            let _ = writeln!(
                io::stdout(),
                "{}",
                serde_json::to_string(&report).unwrap_or_default()
            );
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(err) => {
            let _ = writeln!(io::stderr(), "eval judge lane failed: {err}");
            ExitCode::from(1)
        }
    }
}
