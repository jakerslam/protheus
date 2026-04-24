use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CANDIDATES_PATH: &str = "artifacts/eval_learning_loop_issue_candidates_latest.json";
const DEFAULT_LABELS_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_learning_loop_review_labels.jsonl";
const DEFAULT_OUT_PATH: &str =
    "core/local/artifacts/eval_learning_loop_reviewed_examples_current.json";
const DEFAULT_OUT_LATEST_PATH: &str =
    "artifacts/eval_learning_loop_reviewed_examples_latest.json";
const DEFAULT_STORE_PATH: &str = "local/state/ops/eval_learning_loop/reviewed_examples.jsonl";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_LEARNING_LOOP_REVIEWED_EXAMPLES_CURRENT.md";

pub fn run_eval_learning_loop_review(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let candidates_path =
        parse_flag(args, "candidates").unwrap_or_else(|| DEFAULT_CANDIDATES_PATH.to_string());
    let labels_path = parse_flag(args, "labels").unwrap_or_else(|| DEFAULT_LABELS_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let store_path = parse_flag(args, "store").unwrap_or_else(|| DEFAULT_STORE_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let candidates = candidate_map(&read_json(&candidates_path));
    let labels = read_jsonl(&labels_path);
    let mut examples = Vec::new();
    let mut missing_candidates = Vec::new();
    for label in labels.iter() {
        let candidate_id = str_at(label, &["candidate_id"]).unwrap_or("unknown");
        let Some(candidate) = candidates.get(candidate_id) else {
            missing_candidates.push(candidate_id.to_string());
            continue;
        };
        examples.push(reviewed_example(candidate, label, &candidates_path, &labels_path));
    }

    let coverage = label_coverage(&examples);
    let labels_ok = !labels.is_empty()
        && missing_candidates.is_empty()
        && coverage.accepted > 0
        && coverage.rejected > 0
        && coverage.duplicate > 0
        && coverage.partial > 0
        && coverage.severity_adjusted > 0
        && coverage.root_cause_correct > 0
        && coverage.root_cause_incorrect > 0;
    let store_ok = !examples.is_empty() && examples.iter().all(reviewed_example_store_contract_ok);
    let ok = labels_ok && store_ok;
    let report = json!({
        "type": "eval_learning_loop_reviewed_examples",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_reviewer_label_ingest_contract", "ok": labels_ok, "detail": format!("labels={};missing_candidates={}", labels.len(), missing_candidates.len())},
            {"id": "eval_reviewed_example_store_contract", "ok": store_ok, "detail": format!("reviewed_examples={}", examples.len())}
        ],
        "summary": {
            "labels": labels.len(),
            "reviewed_examples": examples.len(),
            "missing_candidates": missing_candidates,
            "accepted": coverage.accepted,
            "rejected": coverage.rejected,
            "duplicate": coverage.duplicate,
            "partial": coverage.partial,
            "severity_adjusted": coverage.severity_adjusted,
            "root_cause_correct": coverage.root_cause_correct,
            "root_cause_incorrect": coverage.root_cause_incorrect
        },
        "store_path": store_path,
        "sources": {"candidates": candidates_path, "labels": labels_path},
        "reviewed_examples": examples
    });
    let markdown = format!(
        "# Eval Learning Loop Reviewed Examples (Current)\n\n- generated_at: {}\n- ok: {}\n- reviewed_examples: {}\n- accepted: {}\n- rejected: {}\n- duplicate: {}\n- partial: {}\n- severity_adjusted: {}\n- root_cause_incorrect: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        examples.len(),
        coverage.accepted,
        coverage.rejected,
        coverage.duplicate,
        coverage.partial,
        coverage.severity_adjusted,
        coverage.root_cause_incorrect
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_jsonl(&store_path, &examples).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop review outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn candidate_map(report: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    if let Some(rows) = report.get("candidates").and_then(|node| node.as_array()) {
        for row in rows {
            if let Some(id) = str_at(row, &["id"]) {
                out.insert(id.to_string(), row.clone());
            }
        }
    }
    out
}

fn reviewed_example(candidate: &Value, label: &Value, candidates_path: &str, labels_path: &str) -> Value {
    let candidate_id = str_at(candidate, &["id"]).unwrap_or("unknown");
    let reviewer_outcome = normalize_outcome(str_at(label, &["reviewer_outcome"]).unwrap_or(""));
    json!({
        "type": "eval_learning_loop_reviewed_example",
        "schema_version": 1,
        "example_id": format!("reviewed-{candidate_id}"),
        "candidate_id": candidate_id,
        "trace_id": str_at(candidate, &["trace_id"]).unwrap_or("unknown"),
        "source_workflow": str_at(candidate, &["suspected_layer"]).unwrap_or("surface/orchestration"),
        "reviewer_outcome": reviewer_outcome,
        "severity": str_at(label, &["severity"]).unwrap_or("medium"),
        "severity_adjusted": bool_at(label, &["severity_adjusted"]),
        "root_cause_correct": bool_at(label, &["root_cause_correct"]),
        "reviewer_id": str_at(label, &["reviewer_id"]).unwrap_or("unknown"),
        "reviewer_type": str_at(label, &["reviewer_type"]).unwrap_or("unknown"),
        "note": str_at(label, &["note"]).unwrap_or(""),
        "candidate": candidate,
        "provenance": {
            "candidate_artifact": candidates_path,
            "label_artifact": labels_path,
            "candidate_trace_id": str_at(candidate, &["trace_id"]).unwrap_or("unknown")
        },
        "timestamp": now_iso_like(),
        "retention": {
            "class": "calibration",
            "raw_private_content_excluded": true,
            "review_required_before_training": true
        }
    })
}

fn reviewed_example_store_contract_ok(example: &Value) -> bool {
    str_at(example, &["example_id"]).is_some()
        && str_at(example, &["candidate_id"]).is_some()
        && str_at(example, &["source_workflow"]).is_some()
        && str_at(example, &["reviewer_id"]).is_some()
        && str_at(example, &["reviewer_type"]).is_some()
        && str_at(example, &["timestamp"]).is_some()
        && example.get("provenance").and_then(|node| node.as_object()).is_some()
        && example.get("retention").and_then(|node| node.as_object()).is_some()
}

#[derive(Default)]
struct LabelCoverage {
    accepted: u64,
    rejected: u64,
    duplicate: u64,
    partial: u64,
    severity_adjusted: u64,
    root_cause_correct: u64,
    root_cause_incorrect: u64,
}

fn label_coverage(examples: &[Value]) -> LabelCoverage {
    let mut coverage = LabelCoverage::default();
    let mut outcomes = BTreeSet::new();
    for example in examples {
        let outcome = str_at(example, &["reviewer_outcome"]).unwrap_or("unknown");
        outcomes.insert(outcome.to_string());
        if bool_at(example, &["severity_adjusted"]) {
            coverage.severity_adjusted += 1;
        }
        if bool_at(example, &["root_cause_correct"]) {
            coverage.root_cause_correct += 1;
        } else {
            coverage.root_cause_incorrect += 1;
        }
    }
    coverage.accepted = outcomes.contains("accepted") as u64;
    coverage.rejected = outcomes.contains("rejected") as u64;
    coverage.duplicate = outcomes.contains("duplicate") as u64;
    coverage.partial = outcomes.contains("partial") as u64;
    coverage
}

fn normalize_outcome(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "accepted" | "accept" | "approved" => "accepted",
        "rejected" | "reject" | "denied" => "rejected",
        "duplicate" | "dupe" => "duplicate",
        "partial" | "partially_accepted" => "partial",
        _ => "unknown",
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

fn bool_at(value: &Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return false;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(false)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn read_jsonl(path: &str) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect()
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
    fn reviewed_example_contract_requires_provenance_and_retention() {
        let candidate = json!({
            "id": "candidate-1",
            "trace_id": "trace-1",
            "suspected_layer": "surface/orchestration/eval"
        });
        let label = json!({
            "reviewer_outcome": "accepted",
            "severity": "high",
            "root_cause_correct": true,
            "reviewer_id": "reviewer",
            "reviewer_type": "human"
        });
        let example = reviewed_example(&candidate, &label, "candidates.json", "labels.jsonl");
        assert!(reviewed_example_store_contract_ok(&example));
        assert_eq!(str_at(&example, &["reviewer_outcome"]), Some("accepted"));
    }
}
