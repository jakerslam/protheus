use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

const DEFAULT_TRACE_PATH: &str = "local/state/ops/orchestration/workflow_phase_trace_latest.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/orchestration_phase_trace_guard_current.json";

const REQUIRED_STRING_FIELDS: &[&str] = &[
    "type",
    "owner",
    "trace_id",
    "user_intent",
    "selected_workflow",
    "tool_decision",
    "tool_family",
    "tool_result_summary",
    "finalization_status",
    "fallback_path",
    "workflow_template",
    "active_stage",
    "receipt_hash",
];

const REQUIRED_PRESENT_FIELDS: &[&str] = &[
    "schema_version",
    "generated_at_ms",
    "selected_model",
    "latency_ms",
    "decision_trace",
    "observed_kernel_outcome_refs",
    "expected_kernel_contract_ids",
    "normalized_failure_codes",
    "issue_signals",
];

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

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn read_json(path: &str) -> io::Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn require_string(trace: &Value, field: &str, issues: &mut Vec<String>) {
    match trace.get(field).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        Some(_) => issues.push(format!("empty_required_field:{field}")),
        None => issues.push(format!("missing_required_field:{field}")),
    }
}

fn require_present(trace: &Value, field: &str, issues: &mut Vec<String>) {
    if trace.get(field).is_none() {
        issues.push(format!("missing_required_field:{field}"));
    }
}

fn require_non_empty_array(trace: &Value, field: &str, issues: &mut Vec<String>) {
    match trace.get(field).and_then(Value::as_array) {
        Some(values) if !values.is_empty() => {}
        Some(_) => issues.push(format!("empty_required_array:{field}")),
        None => issues.push(format!("missing_required_array:{field}")),
    }
}

fn validate_phase_fields(trace: &Value, issues: &mut Vec<String>) {
    let Some(phases) = trace.get("phases").and_then(Value::as_array) else {
        return;
    };
    for (idx, phase) in phases.iter().enumerate() {
        for field in ["phase", "status", "owner", "note"] {
            match phase.get(field).and_then(Value::as_str) {
                Some(value) if !value.trim().is_empty() => {}
                Some(_) => issues.push(format!("phase_{idx}_empty_field:{field}")),
                None => issues.push(format!("phase_{idx}_missing_field:{field}")),
            }
        }
        if phase.get("eval_visible").and_then(Value::as_bool).is_none() {
            issues.push(format!("phase_{idx}_missing_field:eval_visible"));
        }
    }
}

fn validate_decision_trace(trace: &Value, issues: &mut Vec<String>) {
    let Some(decision) = trace.get("decision_trace") else {
        return;
    };
    match decision.get("chosen").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        Some(_) => issues.push("decision_trace_empty_field:chosen".to_string()),
        None => issues.push("decision_trace_missing_field:chosen".to_string()),
    }
    match decision.get("rationale").and_then(Value::as_array) {
        Some(values) if !values.is_empty() => {}
        Some(_) => issues.push("decision_trace_empty_array:rationale".to_string()),
        None => issues.push("decision_trace_missing_array:rationale".to_string()),
    }
}

fn validate_trace(trace: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    for field in REQUIRED_STRING_FIELDS {
        require_string(trace, field, &mut issues);
    }
    for field in REQUIRED_PRESENT_FIELDS {
        require_present(trace, field, &mut issues);
    }
    if trace.get("type").and_then(Value::as_str) != Some("orchestration_workflow_phase_trace") {
        issues.push("invalid_type".to_string());
    }
    if trace.get("schema_version").and_then(Value::as_u64) != Some(1) {
        issues.push("invalid_schema_version".to_string());
    }
    require_non_empty_array(trace, "phases", &mut issues);
    require_non_empty_array(trace, "collectors", &mut issues);
    require_non_empty_array(trace, "observed_kernel_receipt_ids", &mut issues);
    validate_phase_fields(trace, &mut issues);
    validate_decision_trace(trace, &mut issues);
    issues.sort();
    issues.dedup();
    issues
}

fn run() -> io::Result<(bool, Value)> {
    let args: Vec<String> = env::args().skip(1).collect();
    let trace_path = parse_flag(&args, "trace").unwrap_or_else(|| DEFAULT_TRACE_PATH.to_string());
    let out_path = parse_flag(&args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let trace = read_json(&trace_path)?;
    let issues = validate_trace(&trace);
    let ok = issues.is_empty();
    let report = json!({
        "type": "orchestration_phase_trace_completeness_guard",
        "schema_version": 1,
        "ok": ok,
        "trace_path": trace_path,
        "issue_count": issues.len(),
        "issues": issues,
        "checked_contract": "planes/contracts/orchestration/workflow_phase_trace_v1.json",
        "owner": "surface_orchestration_control_plane"
    });
    ensure_parent(&out_path)?;
    fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        ),
    )?;
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
            let _ = writeln!(io::stderr(), "workflow phase trace guard failed: {err}");
            ExitCode::from(1)
        }
    }
}
