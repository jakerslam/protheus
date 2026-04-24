use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_trace_localization_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_trace_localization_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_trace_localization_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_TRACE_LOCALIZATION_GUARD_CURRENT.md";

pub fn run_trace_localization_guard(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_cases = parse_u64_from_path(&thresholds, &["min_cases"], 4);
    let min_exact_or_near_rate =
        parse_f64_from_path(&thresholds, &["min_exact_or_near_span_rate"], 0.75);
    let max_wrong_phase = parse_u64_from_path(&thresholds, &["max_wrong_phase"], 0);
    let max_unsupported_cause = parse_u64_from_path(&thresholds, &["max_unsupported_cause"], 0);

    let mut schema_failures = Vec::new();
    let mut exact_span_matches = Vec::new();
    let mut near_span_matches = Vec::new();
    let mut wrong_phase = Vec::new();
    let mut unsupported_cause = Vec::new();
    for case in cases.iter() {
        if !has_trace_fields(case, "expected") || !has_trace_fields(case, "predicted") {
            schema_failures.push(case_summary(
                case,
                "missing_trace_localization_schema_fields",
            ));
            continue;
        }
        let expected_phase = parse_string_from_path(case, &["expected", "first_bad_phase"], "");
        let predicted_phase = parse_string_from_path(case, &["predicted", "first_bad_phase"], "");
        let expected_start = parse_i64_from_path(case, &["expected", "failure_span", "start"], -1);
        let expected_end = parse_i64_from_path(case, &["expected", "failure_span", "end"], -1);
        let predicted_start =
            parse_i64_from_path(case, &["predicted", "failure_span", "start"], -1);
        let predicted_end = parse_i64_from_path(case, &["predicted", "failure_span", "end"], -1);

        if expected_phase != predicted_phase {
            wrong_phase.push(case_summary(case, "wrong_phase"));
        }
        if expected_start == predicted_start && expected_end == predicted_end {
            exact_span_matches.push(case_summary(case, "exact_span_match"));
        } else if expected_phase == predicted_phase
            && (expected_start - predicted_start).abs() <= 1
            && (expected_end - predicted_end).abs() <= 1
        {
            near_span_matches.push(case_summary(case, "near_span_match"));
        }
        if !cause_is_supported(case) {
            unsupported_cause.push(case_summary(case, "unsupported_cause"));
        }
    }

    let total = cases.len() as u64;
    let exact_or_near =
        (exact_span_matches.len() as u64).saturating_add(near_span_matches.len() as u64);
    let exact_or_near_rate = ratio(exact_or_near, total);
    let schema_ok = schema_failures.is_empty();
    let fixture_ok = total >= min_cases
        && cases.iter().all(|case| {
            parse_string_from_path(case, &["source_kind"], "") == "real_workflow_failure"
        });
    let scoring_ok = exact_or_near_rate >= min_exact_or_near_rate
        && wrong_phase.len() as u64 <= max_wrong_phase
        && unsupported_cause.len() as u64 <= max_unsupported_cause;
    let checks = vec![
        json!({
            "id": "trace_localization_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "trace_localization_schema_fields_contract",
            "ok": schema_ok,
            "detail": format!(
                "schema_failures={};required=first_bad_phase,first_bad_tool_call,affected_receipt,failure_origin_confidence,failure_span",
                schema_failures.len()
            ),
        }),
        json!({
            "id": "trace_localization_gold_fixture_contract",
            "ok": fixture_ok,
            "detail": format!("cases={};min_cases={};source_kind=real_workflow_failure", total, min_cases),
        }),
        json!({
            "id": "trace_localization_scoring_contract",
            "ok": scoring_ok,
            "detail": format!(
                "exact_or_near_rate={:.3};min={:.3};wrong_phase={};max_wrong_phase={};unsupported_cause={};max_unsupported_cause={}",
                exact_or_near_rate,
                min_exact_or_near_rate,
                wrong_phase.len(),
                max_wrong_phase,
                unsupported_cause.len(),
                max_unsupported_cause
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_trace_localization_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "cases": total,
            "exact_span_matches": exact_span_matches.len(),
            "near_span_matches": near_span_matches.len(),
            "exact_or_near_span_rate": exact_or_near_rate,
            "wrong_phase": wrong_phase.len(),
            "unsupported_cause": unsupported_cause.len(),
            "schema_failures": schema_failures.len()
        },
        "schema_failures": schema_failures,
        "exact_span_matches": exact_span_matches,
        "near_span_matches": near_span_matches,
        "wrong_phase": wrong_phase,
        "unsupported_cause": unsupported_cause,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Trace Localization Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- exact_or_near_span_rate: {:.3}\n- wrong_phase: {}\n- unsupported_cause: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        total,
        exact_or_near_rate,
        wrong_phase.len(),
        unsupported_cause.len()
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more trace-localization outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn has_trace_fields(case: &Value, side: &str) -> bool {
    [
        "first_bad_phase",
        "first_bad_tool_call",
        "affected_receipt",
        "failure_origin_confidence",
    ]
    .iter()
    .all(|field| case.pointer(&format!("/{}/{}", side, field)).is_some())
        && case
            .pointer(&format!("/{}/failure_span/start", side))
            .is_some()
        && case
            .pointer(&format!("/{}/failure_span/end", side))
            .is_some()
}

fn cause_is_supported(case: &Value) -> bool {
    let predicted_receipt = parse_string_from_path(case, &["predicted", "affected_receipt"], "");
    let supporting = case
        .get("predicted")
        .and_then(|node| node.get("supporting_receipts"))
        .and_then(|node| node.as_array())
        .map(|rows| {
            rows.iter()
                .any(|row| row.as_str().unwrap_or("") == predicted_receipt)
        })
        .unwrap_or(false);
    let evidence_has_receipt = case
        .get("evidence")
        .and_then(|node| node.get("receipts"))
        .and_then(|node| node.as_array())
        .map(|rows| {
            rows.iter()
                .any(|row| row.as_str().unwrap_or("") == predicted_receipt)
        })
        .unwrap_or(false);
    supporting && evidence_has_receipt
}

fn case_summary(case: &Value, reason: &str) -> Value {
    json!({
        "id": parse_string_from_path(case, &["id"], "unknown"),
        "failure_class": parse_string_from_path(case, &["failure_class"], "unknown"),
        "reason": reason,
        "expected": case.get("expected").cloned().unwrap_or_else(|| json!({})),
        "predicted": case.get("predicted").cloned().unwrap_or_else(|| json!({}))
    })
}

fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("--{}=", name);
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(|value| value.to_string()))
}

fn parse_bool_flag(args: &[String], name: &str, default: bool) -> bool {
    parse_flag(args, name)
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, format!("{}\n", content))
}

fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn print_structured(value: &Value) {
    match serde_json::to_string(value) {
        Ok(content) => {
            let _ = writeln!(io::stdout(), "{}", content);
        }
        Err(err) => {
            let _ = writeln!(io::stderr(), "failed to serialize report: {}", err);
        }
    }
}

fn parse_string_from_path(value: &Value, path: &[&str], default: &str) -> String {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_str())
        .unwrap_or(default)
        .to_string()
}

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_u64())
        .unwrap_or(default)
}

fn parse_i64_from_path(value: &Value, path: &[&str], default: i64) -> i64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_i64())
        .unwrap_or(default)
}

fn parse_f64_from_path(value: &Value, path: &[&str], default: f64) -> f64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_f64())
        .unwrap_or(default)
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn now_iso_like() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", millis)
}
