use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_FEEDBACK_PATH: &str =
    "local/state/ops/eval_agent_chat_monitor/reviewer_feedback.jsonl";
const DEFAULT_POLICY_PATH: &str =
    "surface/orchestration/config/eval_authority_calibration_policy.json";
const DEFAULT_HISTORY_PATH: &str = "local/state/ops/eval_authority_calibration/history.jsonl";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_authority_calibration_current.json";
const DEFAULT_LATEST_PATH: &str = "artifacts/eval_authority_calibration_latest.json";
const DEFAULT_REPORT_PATH: &str =
    "local/workspace/reports/EVAL_AUTHORITY_CALIBRATION_CURRENT.md";

pub fn run_eval_authority_calibration(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let require_closed_loop_ready = parse_bool_flag(args, "require-closed-loop-ready", false);
    let closed_loop_approved = parse_bool_flag(args, "closed-loop-approved", false)
        || parse_bool_flag(args, "operator-approved", false);
    let feedback_path =
        parse_flag(args, "feedback").unwrap_or_else(|| DEFAULT_FEEDBACK_PATH.to_string());
    let policy_path = parse_flag(args, "policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let history_path =
        parse_flag(args, "history").unwrap_or_else(|| DEFAULT_HISTORY_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_LATEST_PATH.to_string());
    let report_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_REPORT_PATH.to_string());

    let policy = read_json(&policy_path);
    let feedback_rows = read_jsonl(&feedback_path);
    let history_rows = read_jsonl(&history_path);
    let metrics = calibration_metrics(&feedback_rows);
    let thresholds = threshold_summary(&policy);
    let thresholds_met = thresholds_met(&metrics, &thresholds);
    let previous_clean_windows = consecutive_clean_windows(&history_rows);
    let consecutive_clean_windows = if thresholds_met {
        previous_clean_windows.saturating_add(1)
    } else {
        0
    };
    let minimum_windows = u64_field(&thresholds, "minimum_consecutive_passing_windows", 4);
    let stability_ready = consecutive_clean_windows >= minimum_windows;
    let calibration_pass = thresholds_met && stability_ready;
    let observation_authority_allowed =
        bool_field(&thresholds, "observation_authority_allowed", true);
    let requires_operator_approval = bool_field(&thresholds, "requires_operator_approval", true);
    let closed_loop_autonomy_allowed =
        bool_field(&thresholds, "closed_loop_autonomy_allowed", false);
    let closed_loop_change_ready =
        calibration_pass && (!requires_operator_approval || closed_loop_approved);
    let closed_loop_auto_change_allowed =
        closed_loop_change_ready && closed_loop_autonomy_allowed;
    let authority_mode = if closed_loop_auto_change_allowed {
        "closed_loop_auto_change_allowed"
    } else if closed_loop_change_ready {
        "supervised_change_pipeline_ready"
    } else if calibration_pass {
        "closed_loop_operator_approval_required"
    } else {
        "authoritative_observations_only"
    };
    let calibration_checks = vec![
        json!({
            "id": "eval_closed_loop_feedback_sample_floor_contract",
            "ok": u64_field(&metrics, "reviewed_samples", 0)
                >= u64_field(&thresholds, "minimum_reviewed_samples", 50),
            "detail": format!(
                "reviewed_samples={};minimum={}",
                u64_field(&metrics, "reviewed_samples", 0),
                u64_field(&thresholds, "minimum_reviewed_samples", 50)
            ),
        }),
        json!({
            "id": "eval_closed_loop_precision_contract",
            "ok": f64_field(&metrics, "precision", 0.0)
                >= f64_field(&thresholds, "minimum_precision", 0.90),
            "detail": format!("precision={:.3}", f64_field(&metrics, "precision", 0.0)),
        }),
        json!({
            "id": "eval_closed_loop_recall_contract",
            "ok": f64_field(&metrics, "recall", 0.0)
                >= f64_field(&thresholds, "minimum_recall", 0.85),
            "detail": format!("recall={:.3}", f64_field(&metrics, "recall", 0.0)),
        }),
        json!({
            "id": "eval_closed_loop_error_ceiling_contract",
            "ok": f64_field(&metrics, "false_positive_rate", 1.0)
                <= f64_field(&thresholds, "maximum_false_positive_rate", 0.05)
                && f64_field(&metrics, "false_negative_rate", 1.0)
                    <= f64_field(&thresholds, "maximum_false_negative_rate", 0.10),
            "detail": format!(
                "false_positive_rate={:.3};false_negative_rate={:.3}",
                f64_field(&metrics, "false_positive_rate", 1.0),
                f64_field(&metrics, "false_negative_rate", 1.0)
            ),
        }),
        json!({
            "id": "eval_closed_loop_longitudinal_stability_contract",
            "ok": stability_ready,
            "detail": format!(
                "consecutive_clean_windows={};minimum={}",
                consecutive_clean_windows, minimum_windows
            ),
        }),
    ];
    let mut boundary_checks = vec![
        json!({
            "id": "eval_observation_authority_allowed_contract",
            "ok": observation_authority_allowed,
            "detail": format!(
                "observation_authority_allowed={}",
                observation_authority_allowed
            ),
        }),
        json!({
            "id": "eval_closed_loop_operator_approval_boundary_contract",
            "ok": !closed_loop_change_ready || !requires_operator_approval || closed_loop_approved,
            "detail": format!(
                "closed_loop_change_ready={};requires_operator_approval={};closed_loop_approved={}",
                closed_loop_change_ready, requires_operator_approval, closed_loop_approved
            ),
        }),
        json!({
            "id": "eval_no_closed_loop_auto_change_before_calibration_contract",
            "ok": !closed_loop_autonomy_allowed || closed_loop_change_ready,
            "detail": format!(
                "closed_loop_autonomy_allowed={};closed_loop_change_ready={}",
                closed_loop_autonomy_allowed, closed_loop_change_ready
            ),
        }),
    ];
    if require_closed_loop_ready {
        boundary_checks.push(json!({
            "id": "eval_closed_loop_ready_required_contract",
            "ok": closed_loop_change_ready,
            "detail": format!(
                "closed_loop_change_ready={};require_closed_loop_ready=true",
                closed_loop_change_ready
            ),
        }));
    }
    let ok = boundary_checks
        .iter()
        .all(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false));
    let report = json!({
        "type": "eval_authority_calibration_guard",
        "schema_version": 2,
        "generated_at": now_iso_like(),
        "ok": ok,
        "calibration_pass": calibration_pass,
        "observation_authority_allowed": observation_authority_allowed,
        "authority_allowed": observation_authority_allowed,
        "closed_loop_change_ready": closed_loop_change_ready,
        "closed_loop_auto_change_allowed": closed_loop_auto_change_allowed,
        "authority_mode": authority_mode,
        "calibration_checks": calibration_checks,
        "boundary_checks": boundary_checks,
        "summary": {
            "metrics": metrics,
            "thresholds": thresholds,
            "thresholds_met": thresholds_met,
            "previous_clean_windows": previous_clean_windows,
            "consecutive_clean_windows": consecutive_clean_windows,
            "minimum_consecutive_passing_windows": minimum_windows,
            "stability_ready": stability_ready,
            "closed_loop_approved": closed_loop_approved,
            "requires_operator_approval": requires_operator_approval,
            "authority_policy": "authoritative_observations_now_closed_loop_changes_after_calibration"
        },
        "sources": {
            "feedback": feedback_path,
            "policy": policy_path,
            "history": history_path
        }
    });
    let history_row = json!({
        "generated_at": report.get("generated_at").cloned().unwrap_or_else(|| json!("")),
        "ok": ok,
        "calibration_pass": calibration_pass,
        "observation_authority_allowed": observation_authority_allowed,
        "closed_loop_change_ready": closed_loop_change_ready,
        "metrics": report.pointer("/summary/metrics").cloned().unwrap_or_else(|| json!({})),
    });
    let markdown = markdown(&report);
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&latest_path, &report).is_ok()
        && append_jsonl(&history_path, &history_row).is_ok()
        && write_text(&report_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_authority_calibration: failed to write one or more outputs");
        return 2;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    );
    if strict && !ok {
        return 1;
    }
    0
}

fn calibration_metrics(rows: &[Value]) -> Value {
    let mut accepted = 0_u64;
    let mut rejected = 0_u64;
    let mut partial = 0_u64;
    let mut missed = 0_u64;
    let mut unknown = 0_u64;
    for row in rows {
        match feedback_category(row).as_str() {
            "accepted" => accepted = accepted.saturating_add(1),
            "rejected" => rejected = rejected.saturating_add(1),
            "partial" => partial = partial.saturating_add(1),
            "missed" => missed = missed.saturating_add(1),
            _ => unknown = unknown.saturating_add(1),
        }
    }
    let predicted_positive = accepted.saturating_add(rejected).saturating_add(partial);
    let actual_positive = accepted.saturating_add(partial).saturating_add(missed);
    let reviewed_samples = accepted
        .saturating_add(rejected)
        .saturating_add(partial)
        .saturating_add(missed);
    let precision = ratio(accepted, predicted_positive);
    let recall = ratio(accepted, actual_positive);
    json!({
        "feedback_rows": rows.len() as u64,
        "reviewed_samples": reviewed_samples,
        "accepted": accepted,
        "rejected": rejected,
        "partial": partial,
        "missed": missed,
        "unknown": unknown,
        "precision": precision,
        "recall": recall,
        "false_positive_rate": ratio(rejected, predicted_positive),
        "false_negative_rate": ratio(missed, actual_positive),
    })
}

fn threshold_summary(policy: &Value) -> Value {
    json!({
        "minimum_reviewed_samples": u64_path(policy, &["thresholds", "minimum_reviewed_samples"], 50),
        "minimum_precision": f64_path(policy, &["thresholds", "minimum_precision"], 0.90),
        "minimum_recall": f64_path(policy, &["thresholds", "minimum_recall"], 0.85),
        "maximum_false_positive_rate": f64_path(policy, &["thresholds", "maximum_false_positive_rate"], 0.05),
        "maximum_false_negative_rate": f64_path(policy, &["thresholds", "maximum_false_negative_rate"], 0.10),
        "minimum_consecutive_passing_windows": u64_path(policy, &["thresholds", "minimum_consecutive_passing_windows"], 4),
        "requires_operator_approval": bool_path(policy, &["authority", "requires_operator_approval"], true),
        "observation_authority_allowed": bool_path(policy, &["authority", "observation_authority_allowed"], true),
        "closed_loop_autonomy_allowed": bool_path(policy, &["authority", "closed_loop_autonomy_allowed"], false),
    })
}

fn thresholds_met(metrics: &Value, thresholds: &Value) -> bool {
    u64_field(metrics, "reviewed_samples", 0)
        >= u64_field(thresholds, "minimum_reviewed_samples", 50)
        && u64_field(metrics, "unknown", 0) == 0
        && f64_field(metrics, "precision", 0.0) >= f64_field(thresholds, "minimum_precision", 0.90)
        && f64_field(metrics, "recall", 0.0) >= f64_field(thresholds, "minimum_recall", 0.85)
        && f64_field(metrics, "false_positive_rate", 1.0)
            <= f64_field(thresholds, "maximum_false_positive_rate", 0.05)
        && f64_field(metrics, "false_negative_rate", 1.0)
            <= f64_field(thresholds, "maximum_false_negative_rate", 0.10)
}

fn consecutive_clean_windows(rows: &[Value]) -> u64 {
    let mut count = 0_u64;
    for row in rows.iter().rev() {
        if row.get("calibration_pass").and_then(Value::as_bool) == Some(true) {
            count = count.saturating_add(1);
        } else {
            break;
        }
    }
    count
}

fn feedback_category(row: &Value) -> String {
    for key in [
        "reviewer_outcome",
        "reviewer_status",
        "feedback_outcome",
        "verdict",
        "human_verdict",
        "human_label",
        "label",
    ] {
        let raw = row
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let category = match raw.as_str() {
            "accepted" | "accept" | "approved" | "correct" | "true_positive" | "tp" => "accepted",
            "rejected" | "reject" | "incorrect" | "false_positive" | "fp" => "rejected",
            "partial" | "partially_accepted" | "partially_correct" | "mixed" => "partial",
            "missed" | "false_negative" | "fn" => "missed",
            _ => "",
        };
        if !category.is_empty() {
            return category.to_string();
        }
    }
    "unknown".to_string()
}

fn markdown(report: &Value) -> String {
    format!(
        "# Eval Authority Calibration\n\n- ok: {}\n- authority_mode: {}\n- calibration_pass: {}\n- authority_allowed: {}\n- reviewed_samples: {}\n- precision: {:.3}\n- recall: {:.3}\n- false_positive_rate: {:.3}\n- false_negative_rate: {:.3}\n- consecutive_clean_windows: {}\n",
        report.get("ok").and_then(Value::as_bool).unwrap_or(false),
        report.get("authority_mode").and_then(Value::as_str).unwrap_or("unknown"),
        report.get("calibration_pass").and_then(Value::as_bool).unwrap_or(false),
        report.get("observation_authority_allowed").and_then(Value::as_bool).unwrap_or(false),
        report.get("closed_loop_change_ready").and_then(Value::as_bool).unwrap_or(false),
        report.get("closed_loop_auto_change_allowed").and_then(Value::as_bool).unwrap_or(false),
        report.pointer("/summary/metrics/reviewed_samples").and_then(Value::as_u64).unwrap_or(0),
        report.pointer("/summary/metrics/precision").and_then(Value::as_f64).unwrap_or(0.0),
        report.pointer("/summary/metrics/recall").and_then(Value::as_f64).unwrap_or(0.0),
        report.pointer("/summary/metrics/false_positive_rate").and_then(Value::as_f64).unwrap_or(1.0),
        report.pointer("/summary/metrics/false_negative_rate").and_then(Value::as_f64).unwrap_or(1.0),
        report.pointer("/summary/consecutive_clean_windows").and_then(Value::as_u64).unwrap_or(0),
    )
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn read_jsonl(path: &str) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
        ),
    )
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn append_jsonl(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
    )
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("--{}=", name);
    args.iter()
        .enumerate()
        .find_map(|(idx, arg)| {
            arg.strip_prefix(&prefix)
                .map(str::to_string)
                .or_else(|| (arg == &format!("--{name}")).then(|| args.get(idx + 1).cloned()).flatten())
        })
}

fn parse_bool_flag(args: &[String], name: &str, default: bool) -> bool {
    parse_flag(args, name)
        .map(|raw| matches!(raw.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

fn bool_field(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn u64_field(value: &Value, key: &str, default: u64) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(default)
}

fn f64_field(value: &Value, key: &str, default: f64) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn bool_path(value: &Value, path: &[&str], default: bool) -> bool {
    path.iter()
        .try_fold(value, |cursor, segment| cursor.get(*segment))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn u64_path(value: &Value, path: &[&str], default: u64) -> u64 {
    path.iter()
        .try_fold(value, |cursor, segment| cursor.get(*segment))
        .and_then(Value::as_u64)
        .unwrap_or(default)
}

fn f64_path(value: &Value, path: &[&str], default: f64) -> f64 {
    path.iter()
        .try_fold(value, |cursor, segment| cursor.get(*segment))
        .and_then(Value::as_f64)
        .unwrap_or(default)
}
