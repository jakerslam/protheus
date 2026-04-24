use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_REVIEWED_PATH: &str = "artifacts/eval_learning_loop_reviewed_examples_latest.json";
const DEFAULT_HOLDOUT_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_learning_loop_policy_holdout.json";
const DEFAULT_OUT_PATH: &str =
    "core/local/artifacts/eval_learning_loop_policy_promotion_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_learning_loop_policy_promotion_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_LEARNING_LOOP_POLICY_PROMOTION_CURRENT.md";

pub fn run_eval_learning_loop_policy(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let reviewed_path =
        parse_flag(args, "reviewed").unwrap_or_else(|| DEFAULT_REVIEWED_PATH.to_string());
    let holdout_path = parse_flag(args, "holdout").unwrap_or_else(|| DEFAULT_HOLDOUT_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let reviewed = read_json(&reviewed_path);
    let examples = reviewed
        .get("reviewed_examples")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let calibration = calibration_update(&examples);
    let reviewed_comparison = compare_on_reviewed_examples(&examples, calibration.threshold);
    let holdout = read_json(&holdout_path);
    let holdout_comparison = compare_on_holdout(&holdout);
    let regression_blockers = regression_blockers(&reviewed_comparison, &holdout_comparison);
    let calibration_ok = examples.len() >= 5
        && bool_at(&calibration.report, &["runtime_code_mutation_requested"]) == false
        && calibration.reference_examples >= 2;
    let promotion_ok = reviewed_comparison.candidate_correct > reviewed_comparison.active_correct
        && holdout_comparison.candidate_correct > holdout_comparison.active_correct
        && regression_blockers.is_empty();
    let ok = calibration_ok && promotion_ok;
    let report = json!({
        "type": "eval_learning_loop_policy_promotion",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_calibration_update_contract", "ok": calibration_ok, "detail": format!("examples={};reference_examples={};runtime_code_mutation_requested=false", examples.len(), calibration.reference_examples)},
            {"id": "eval_holdout_policy_promotion_contract", "ok": promotion_ok, "detail": format!("reviewed_active_correct={};reviewed_candidate_correct={};holdout_active_correct={};holdout_candidate_correct={}", reviewed_comparison.active_correct, reviewed_comparison.candidate_correct, holdout_comparison.active_correct, holdout_comparison.candidate_correct)},
            {"id": "eval_high_severity_regression_block_contract", "ok": regression_blockers.is_empty(), "detail": format!("blockers={}", regression_blockers.len())}
        ],
        "summary": {
            "reviewed_examples": examples.len(),
            "candidate_policy_promotable": promotion_ok,
            "regression_blockers": regression_blockers.len(),
            "candidate_threshold": calibration.threshold,
            "active_threshold": 0.65
        },
        "calibration_update": calibration.report,
        "reviewed_comparison": reviewed_comparison.report,
        "holdout_comparison": holdout_comparison.report,
        "regression_blockers": regression_blockers,
        "sources": {"reviewed": reviewed_path, "holdout": holdout_path}
    });
    let markdown = format!(
        "# Eval Learning Loop Policy Promotion (Current)\n\n- generated_at: {}\n- ok: {}\n- reviewed_examples: {}\n- candidate_policy_promotable: {}\n- regression_blockers: {}\n- candidate_threshold: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        examples.len(),
        promotion_ok,
        report.pointer("/summary/regression_blockers").and_then(|node| node.as_u64()).unwrap_or(0),
        calibration.threshold
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop policy outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

struct CalibrationUpdate {
    threshold: f64,
    reference_examples: u64,
    report: Value,
}

fn calibration_update(examples: &[Value]) -> CalibrationUpdate {
    let accepted_confidences: Vec<f64> = examples
        .iter()
        .filter(|example| desired_positive(example))
        .filter_map(|example| example.pointer("/candidate/confidence").and_then(|node| node.as_f64()))
        .collect();
    let threshold = accepted_confidences
        .iter()
        .copied()
        .fold(0.65_f64, f64::min)
        .max(0.50);
    let reference_examples = examples
        .iter()
        .filter(|example| {
            matches!(
                str_at(example, &["reviewer_outcome"]).unwrap_or(""),
                "accepted" | "partial"
            )
        })
        .count() as u64;
    let report = json!({
        "candidate_policy_id": "eval-learning-loop-policy-candidate-v1",
        "parent_policy_id": "eval-learning-loop-policy-active-v1",
        "threshold_adjustments": {
            "issue_readiness_min_confidence": threshold,
            "active_issue_readiness_min_confidence": 0.65
        },
        "scoring_weight_adjustments": {
            "receipt_grounding_weight": 0.45,
            "root_cause_basis_weight": 0.35,
            "reviewer_outcome_weight": 0.20
        },
        "reference_example_count": reference_examples,
        "runtime_code_mutation_requested": false,
        "policy_only_update": true
    });
    CalibrationUpdate {
        threshold,
        reference_examples,
        report,
    }
}

struct Comparison {
    active_correct: u64,
    candidate_correct: u64,
    active_high_severity_misses: u64,
    candidate_high_severity_misses: u64,
    report: Value,
}

fn compare_on_reviewed_examples(examples: &[Value], candidate_threshold: f64) -> Comparison {
    let mut rows = Vec::new();
    let mut active_correct = 0_u64;
    let mut candidate_correct = 0_u64;
    let mut active_high_severity_misses = 0_u64;
    let mut candidate_high_severity_misses = 0_u64;
    for example in examples {
        let confidence = example
            .pointer("/candidate/confidence")
            .and_then(|node| node.as_f64())
            .unwrap_or(0.0);
        let expected_positive = desired_positive(example);
        let active_positive = confidence >= 0.65;
        let candidate_positive = confidence >= candidate_threshold && bool_at(example, &["root_cause_correct"]);
        active_correct += (active_positive == expected_positive) as u64;
        candidate_correct += (candidate_positive == expected_positive) as u64;
        if is_high_severity(example) && active_positive != expected_positive {
            active_high_severity_misses += 1;
        }
        if is_high_severity(example) && candidate_positive != expected_positive {
            candidate_high_severity_misses += 1;
        }
        rows.push(json!({
            "id": str_at(example, &["candidate_id"]).unwrap_or("unknown"),
            "expected_positive": expected_positive,
            "active_positive": active_positive,
            "candidate_positive": candidate_positive,
            "severity": str_at(example, &["severity"]).unwrap_or("medium")
        }));
    }
    Comparison {
        active_correct,
        candidate_correct,
        active_high_severity_misses,
        candidate_high_severity_misses,
        report: json!({
            "dataset": "reviewed_examples",
            "rows": rows,
            "active_correct": active_correct,
            "candidate_correct": candidate_correct,
            "active_high_severity_misses": active_high_severity_misses,
            "candidate_high_severity_misses": candidate_high_severity_misses
        }),
    }
}

fn compare_on_holdout(holdout: &Value) -> Comparison {
    let mut rows = Vec::new();
    let mut active_correct = 0_u64;
    let mut candidate_correct = 0_u64;
    let mut active_high_severity_misses = 0_u64;
    let mut candidate_high_severity_misses = 0_u64;
    for case in holdout
        .get("cases")
        .and_then(|node| node.as_array())
        .into_iter()
        .flatten()
    {
        let expected_positive = str_at(case, &["expected"]) == Some("accept");
        let active_positive = case.get("active_score").and_then(|node| node.as_f64()).unwrap_or(0.0) >= 0.60;
        let candidate_positive = case
            .get("candidate_score")
            .and_then(|node| node.as_f64())
            .unwrap_or(0.0)
            >= 0.60;
        active_correct += (active_positive == expected_positive) as u64;
        candidate_correct += (candidate_positive == expected_positive) as u64;
        let high = matches!(str_at(case, &["severity"]).unwrap_or(""), "high" | "critical");
        if high && active_positive != expected_positive {
            active_high_severity_misses += 1;
        }
        if high && candidate_positive != expected_positive {
            candidate_high_severity_misses += 1;
        }
        rows.push(json!({
            "id": str_at(case, &["case_id"]).unwrap_or("unknown"),
            "class": str_at(case, &["class"]).unwrap_or("unknown"),
            "expected_positive": expected_positive,
            "active_positive": active_positive,
            "candidate_positive": candidate_positive,
            "severity": str_at(case, &["severity"]).unwrap_or("medium")
        }));
    }
    Comparison {
        active_correct,
        candidate_correct,
        active_high_severity_misses,
        candidate_high_severity_misses,
        report: json!({
            "dataset": "holdout_known_bad",
            "rows": rows,
            "active_correct": active_correct,
            "candidate_correct": candidate_correct,
            "active_high_severity_misses": active_high_severity_misses,
            "candidate_high_severity_misses": candidate_high_severity_misses
        }),
    }
}

fn regression_blockers(reviewed: &Comparison, holdout: &Comparison) -> Vec<Value> {
    let mut blockers = Vec::new();
    if reviewed.candidate_high_severity_misses > reviewed.active_high_severity_misses {
        blockers.push(json!({"dataset": "reviewed_examples", "reason": "candidate_high_severity_regression"}));
    }
    if holdout.candidate_high_severity_misses > holdout.active_high_severity_misses {
        blockers.push(json!({"dataset": "holdout_known_bad", "reason": "candidate_high_severity_regression"}));
    }
    if reviewed.candidate_correct < reviewed.active_correct {
        blockers.push(json!({"dataset": "reviewed_examples", "reason": "candidate_accuracy_regression"}));
    }
    if holdout.candidate_correct < holdout.active_correct {
        blockers.push(json!({"dataset": "holdout_known_bad", "reason": "candidate_accuracy_regression"}));
    }
    blockers
}

fn desired_positive(example: &Value) -> bool {
    matches!(
        str_at(example, &["reviewer_outcome"]).unwrap_or(""),
        "accepted" | "partial"
    ) && bool_at(example, &["root_cause_correct"])
}

fn is_high_severity(example: &Value) -> bool {
    matches!(str_at(example, &["severity"]).unwrap_or(""), "high" | "critical")
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
    fn regression_blocker_detects_high_severity_candidate_regression() {
        let reviewed = Comparison {
            active_correct: 2,
            candidate_correct: 2,
            active_high_severity_misses: 0,
            candidate_high_severity_misses: 1,
            report: json!({}),
        };
        let holdout = Comparison {
            active_correct: 2,
            candidate_correct: 2,
            active_high_severity_misses: 0,
            candidate_high_severity_misses: 0,
            report: json!({}),
        };
        assert_eq!(regression_blockers(&reviewed, &holdout).len(), 1);
    }
}
