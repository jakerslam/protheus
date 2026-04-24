use infring_orchestration_surface_v1::contracts::{
    EvalCalibrationSnapshot, EvalQualityGateHistory, EvalQualityGatePolicy, EvalQualitySignalMode,
    EvalQualitySignalSnapshot,
};
use infring_orchestration_surface_v1::eval::{
    evaluate_judge_human_agreement, evaluate_quality_gate, EvalJudgeHumanAgreementPolicy,
    EvalJudgeHumanComparableRow, EvalVerdict,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_QUALITY_PATH: &str = "artifacts/eval_quality_metrics_latest.json";
const DEFAULT_MONITOR_PATH: &str = "local/state/ops/eval_agent_chat_monitor/latest.json";
const DEFAULT_JUDGE_HUMAN_PATH: &str = "artifacts/eval_judge_human_agreement_latest.json";
const DEFAULT_HISTORY_PATH: &str = "local/state/ops/eval_quality_gate_v1/history.json";
const DEFAULT_QUALITY_OUT_PATH: &str = "core/local/artifacts/eval_quality_gate_v1_current.json";
const DEFAULT_QUALITY_OUT_LATEST_PATH: &str = "artifacts/eval_quality_gate_v1_latest.json";
const DEFAULT_QUALITY_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_QUALITY_GATE_V1_CURRENT.md";

const DEFAULT_FEEDBACK_PATH: &str =
    "local/state/ops/eval_agent_chat_monitor/reviewer_feedback.jsonl";
const DEFAULT_THRESHOLDS_PATH: &str = "tests/tooling/config/eval_quality_thresholds.json";
const DEFAULT_JUDGE_OUT_PATH: &str = "core/local/artifacts/eval_judge_human_agreement_current.json";
const DEFAULT_JUDGE_OUT_LATEST_PATH: &str = "artifacts/eval_judge_human_agreement_latest.json";
const DEFAULT_JUDGE_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_JUDGE_HUMAN_AGREEMENT_CURRENT.md";

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
    let Some(raw) = parse_flag(args, key) else {
        return default;
    };
    match raw.trim() {
        "1" | "true" | "TRUE" | "yes" | "on" => true,
        "0" | "false" | "FALSE" | "no" | "off" => false,
        _ => default,
    }
}

fn parse_u64_flag(args: &[String], key: &str, default: u64) -> u64 {
    parse_flag(args, key)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_f64_from_path(value: &Value, path: &[&str], default: f64) -> f64 {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor.as_f64().unwrap_or(default)
}

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor
        .as_u64()
        .or_else(|| {
            cursor
                .as_i64()
                .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
        })
        .unwrap_or(default)
}

fn parse_bool_from_path(value: &Value, path: &[&str], default: bool) -> bool {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default,
        };
    }
    cursor.as_bool().unwrap_or(default)
}

fn parse_string_from_path(value: &Value, path: &[&str], default: &str) -> String {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor.get(*segment) {
            Some(next) => next,
            None => return default.to_string(),
        };
    }
    cursor.as_str().unwrap_or(default).to_string()
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
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<Value>(trimmed).ok()
        })
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

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

fn as_eval_mode(raw: &str) -> EvalQualitySignalMode {
    match raw.trim().to_lowercase().as_str() {
        "scored" => EvalQualitySignalMode::Scored,
        "insufficient_signal" => EvalQualitySignalMode::InsufficientSignal,
        _ => EvalQualitySignalMode::Unknown,
    }
}

fn verdict_from_row(row: &Value, keys: &[&str]) -> Option<EvalVerdict> {
    for key in keys {
        if let Some(raw) = row.get(*key).and_then(|v| v.as_str()) {
            if let Some(verdict) = EvalVerdict::from_any(raw) {
                return Some(verdict);
            }
        }
    }
    None
}

fn run_quality_gate(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let quality_path =
        parse_flag(args, "quality").unwrap_or_else(|| DEFAULT_QUALITY_PATH.to_string());
    let monitor_path =
        parse_flag(args, "monitor").unwrap_or_else(|| DEFAULT_MONITOR_PATH.to_string());
    let judge_human_path =
        parse_flag(args, "judge-human").unwrap_or_else(|| DEFAULT_JUDGE_HUMAN_PATH.to_string());
    let history_path =
        parse_flag(args, "history").unwrap_or_else(|| DEFAULT_HISTORY_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_QUALITY_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest")
        .unwrap_or_else(|| DEFAULT_QUALITY_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown")
        .unwrap_or_else(|| DEFAULT_QUALITY_MARKDOWN_PATH.to_string());
    let required_consecutive_passes = parse_u64_flag(args, "required-consecutive-passes", 3).max(1);
    let now_iso = now_iso_like();

    let quality = read_json(&quality_path);
    let monitor = read_json(&monitor_path);
    let judge_human = read_json(&judge_human_path);
    let history = read_json(&history_path);

    let evaluation_mode_raw =
        parse_string_from_path(&quality, &["metrics", "evaluation_mode"], "unknown");
    let signal_snapshot = EvalQualitySignalSnapshot {
        quality_ok: parse_bool_from_path(&quality, &["ok"], false),
        monitor_ok: parse_bool_from_path(&monitor, &["ok"], false),
        evaluation_mode: as_eval_mode(&evaluation_mode_raw),
        predicted_non_info_samples: parse_u64_from_path(
            &quality,
            &["metrics", "predicted_non_info_samples"],
            0,
        ),
        minimum_eval_samples: parse_u64_from_path(
            &quality,
            &["metrics", "minimum_eval_samples"],
            0,
        ),
    };
    let calibration_snapshot = EvalCalibrationSnapshot {
        calibration_ready: parse_bool_from_path(
            &judge_human,
            &["summary", "calibration_ready"],
            false,
        ),
        status: parse_string_from_path(&judge_human, &["summary", "status"], "missing"),
        agreement_rate: parse_f64_from_path(&judge_human, &["summary", "agreement_rate"], 0.0),
        agreement_min: parse_f64_from_path(&judge_human, &["summary", "agreement_min"], 0.0),
        comparable_samples: parse_u64_from_path(
            &judge_human,
            &["summary", "comparable_samples"],
            0,
        ),
        minimum_samples: parse_u64_from_path(&judge_human, &["summary", "minimum_samples"], 0),
    };
    let history_snapshot = EvalQualityGateHistory {
        consecutive_passes: parse_u64_from_path(&history, &["consecutive_passes"], 0),
    };
    let policy = EvalQualityGatePolicy {
        required_consecutive_passes,
    };
    let gate_state = evaluate_quality_gate(
        &signal_snapshot,
        &calibration_snapshot,
        &history_snapshot,
        &policy,
    );

    let checks = vec![
        json!({
            "id": "quality_artifact_present",
            "ok": Path::new(&quality_path).exists(),
            "detail": quality_path,
        }),
        json!({
            "id": "monitor_artifact_present",
            "ok": Path::new(&monitor_path).exists(),
            "detail": monitor_path,
        }),
        json!({
            "id": "judge_human_artifact_present",
            "ok": Path::new(&judge_human_path).exists(),
            "detail": judge_human_path,
        }),
        json!({
            "id": "quality_threshold_contract",
            "ok": signal_snapshot.quality_ok,
            "detail": format!("quality_ok={}", signal_snapshot.quality_ok),
        }),
        json!({
            "id": "monitor_contract",
            "ok": signal_snapshot.monitor_ok,
            "detail": format!("monitor_ok={}", signal_snapshot.monitor_ok),
        }),
        json!({
            "id": "quality_signal_ready_contract",
            "ok": gate_state.quality_signal_sufficient,
            "detail": format!(
                "evaluation_mode={};predicted_non_info={};minimum_eval_samples={}",
                evaluation_mode_raw,
                signal_snapshot.predicted_non_info_samples,
                signal_snapshot.minimum_eval_samples
            ),
        }),
        json!({
            "id": "judge_human_calibration_contract",
            "ok": calibration_snapshot.calibration_ready,
            "detail": format!(
                "status={};agreement_rate={:.3};comparable_samples={}",
                calibration_snapshot.status,
                calibration_snapshot.agreement_rate,
                calibration_snapshot.comparable_samples
            ),
        }),
        json!({
            "id": "consecutive_pass_tracking_contract",
            "ok": true,
            "detail": format!(
                "consecutive={};required={};remaining={};soft_blocked={}",
                gate_state.consecutive_passes,
                gate_state.required_consecutive_passes,
                gate_state.remaining_to_unlock,
                gate_state.soft_blocked
            ),
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

    let report = json!({
        "type": "eval_quality_gate_v1",
        "schema_version": 2,
        "generated_at": now_iso,
        "ok": ok,
        "checks": checks,
        "summary": {
            "quality_ok": signal_snapshot.quality_ok,
            "monitor_ok": signal_snapshot.monitor_ok,
            "quality_signal_sufficient": gate_state.quality_signal_sufficient,
            "quality_evaluation_mode": evaluation_mode_raw,
            "predicted_non_info_samples": signal_snapshot.predicted_non_info_samples,
            "minimum_eval_samples": signal_snapshot.minimum_eval_samples,
            "calibration_ready": calibration_snapshot.calibration_ready,
            "calibration_status": calibration_snapshot.status,
            "calibration_agreement_rate": calibration_snapshot.agreement_rate,
            "calibration_comparable_samples": calibration_snapshot.comparable_samples,
            "current_pass": gate_state.current_pass,
            "consecutive_passes": gate_state.consecutive_passes,
            "required_consecutive_passes": gate_state.required_consecutive_passes,
            "autonomous_escalation_allowed": gate_state.autonomous_escalation_allowed,
            "remaining_to_unlock": gate_state.remaining_to_unlock,
        },
        "sources": {
            "quality": quality_path,
            "monitor": monitor_path,
            "judge_human": judge_human_path,
            "history": history_path,
        }
    });

    let history_out = json!({
        "type": "eval_quality_gate_v1_history",
        "updated_at": now_iso,
        "consecutive_passes": gate_state.consecutive_passes,
        "required_consecutive_passes": gate_state.required_consecutive_passes,
        "autonomous_escalation_allowed": gate_state.autonomous_escalation_allowed,
        "last_soft_blocked": gate_state.soft_blocked,
        "last_result_ok": ok,
    });
    let markdown = format!(
        "# Eval Quality Gate v1 (Current)\n\n- generated_at: {}\n- ok: {}\n- quality_ok: {}\n- monitor_ok: {}\n- quality_signal_sufficient: {}\n- calibration_ready: {}\n- calibration_status: {}\n- consecutive_passes: {}\n- required_consecutive_passes: {}\n- autonomous_escalation_allowed: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        signal_snapshot.quality_ok,
        signal_snapshot.monitor_ok,
        gate_state.quality_signal_sufficient,
        calibration_snapshot.calibration_ready,
        calibration_snapshot.status,
        gate_state.consecutive_passes,
        gate_state.required_consecutive_passes,
        gate_state.autonomous_escalation_allowed
    );

    let write_ok = write_json(&history_path, &history_out).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more quality-gate outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn run_judge_human_agreement(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let feedback_path =
        parse_flag(args, "feedback").unwrap_or_else(|| DEFAULT_FEEDBACK_PATH.to_string());
    let thresholds_path =
        parse_flag(args, "thresholds").unwrap_or_else(|| DEFAULT_THRESHOLDS_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_JUDGE_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_JUDGE_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_JUDGE_MARKDOWN_PATH.to_string());
    let window_days = parse_u64_flag(args, "window-days", 7);
    let now_iso = now_iso_like();

    let thresholds = read_json(&thresholds_path);
    let minimum_samples =
        parse_u64_from_path(&thresholds, &["global", "judge_human_min_samples"], 5);
    let agreement_min =
        parse_f64_from_path(&thresholds, &["global", "judge_human_agreement_min"], 0.7);
    let policy = EvalJudgeHumanAgreementPolicy {
        minimum_samples,
        agreement_min,
    };

    let rows = read_jsonl(&feedback_path);
    let mut malformed_rows = 0_u64;
    let mut comparable_rows = Vec::new();
    for row in rows.iter() {
        if !row.is_object() {
            malformed_rows = malformed_rows.saturating_add(1);
            continue;
        }
        let human = verdict_from_row(
            row,
            &[
                "human_verdict",
                "reviewer_verdict",
                "verdict",
                "human_label",
                "label",
            ],
        );
        let judge = verdict_from_row(
            row,
            &[
                "judge_verdict",
                "llm_verdict",
                "model_verdict",
                "predicted_verdict",
                "auto_verdict",
                "system_verdict",
            ],
        );
        let (Some(human_verdict), Some(judge_verdict)) = (human, judge) else {
            continue;
        };
        comparable_rows.push(EvalJudgeHumanComparableRow {
            ts: row
                .get("ts")
                .and_then(|v| v.as_str())
                .or_else(|| row.get("timestamp").and_then(|v| v.as_str()))
                .map(|v| v.to_string()),
            issue_id: row
                .get("issue_id")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
            human_verdict,
            judge_verdict,
            note: row
                .get("note")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string()),
        });
    }

    let result = evaluate_judge_human_agreement(&comparable_rows, &policy);
    let checks = vec![
        json!({
            "id": "feedback_rows_present",
            "ok": Path::new(&feedback_path).exists(),
            "detail": feedback_path,
        }),
        json!({
            "id": "thresholds_present",
            "ok": Path::new(&thresholds_path).exists(),
            "detail": thresholds_path,
        }),
        json!({
            "id": "judge_human_signal_coverage_contract",
            "ok": true,
            "detail": format!(
                "status={};comparable_samples={};minimum_samples={};window_days={}",
                result.summary.status, result.summary.comparable_samples, result.summary.minimum_samples, window_days
            ),
        }),
        json!({
            "id": "judge_human_agreement_threshold_contract",
            "ok": if result.summary.status == "insufficient_signal" { true } else { result.summary.agreement_rate >= result.summary.agreement_min },
            "detail": format!(
                "agreement_rate={:.3};agreement_min={:.3}",
                result.summary.agreement_rate, result.summary.agreement_min
            ),
        }),
        json!({
            "id": "feedback_shape_contract",
            "ok": malformed_rows == 0,
            "detail": format!("rows={};malformed={}", rows.len(), malformed_rows),
        }),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

    let report = json!({
        "type": "eval_judge_human_agreement_guard",
        "schema_version": 1,
        "generated_at": now_iso,
        "ok": ok,
        "checks": checks,
        "summary": {
            "window_days": window_days,
            "feedback_rows_total": rows.len(),
            "feedback_rows_window": rows.len(),
            "comparable_samples": result.summary.comparable_samples,
            "minimum_samples": result.summary.minimum_samples,
            "agreement_rate": result.summary.agreement_rate,
            "agreement_min": result.summary.agreement_min,
            "calibration_ready": result.summary.calibration_ready,
            "status": result.summary.status,
            "pair_counts": result.summary.pair_counts,
        },
        "comparisons": comparable_rows,
        "sources": {
            "feedback": feedback_path,
            "thresholds": thresholds_path,
        }
    });
    let markdown = format!(
        "# Eval Judge-Human Agreement Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- status: {}\n- agreement_rate: {:.3}\n- agreement_min: {:.3}\n- comparable_samples: {}\n- minimum_samples: {}\n- calibration_ready: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        result.summary.status,
        result.summary.agreement_rate,
        result.summary.agreement_min,
        result.summary.comparable_samples,
        result.summary.minimum_samples,
        result.summary.calibration_ready
    );

    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more judge-human outputs");
        return 2;
    }

    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn usage() {
    eprintln!(
        "usage: cargo run --manifest-path surface/orchestration/Cargo.toml --bin eval-runtime -- <quality-gate|judge-human-agreement> [--strict=0|1] [args...]"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((command, tail)) = args.split_first() else {
        usage();
        return ExitCode::from(2);
    };
    let status = match command.as_str() {
        "quality-gate" => run_quality_gate(tail),
        "judge-human-agreement" => run_judge_human_agreement(tail),
        _ => {
            usage();
            2
        }
    };
    if status == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(status as u8)
    }
}
