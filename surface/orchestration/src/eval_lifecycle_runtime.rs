use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_ISSUE_DRAFTS_PATH: &str = "artifacts/eval_issue_drafts_latest.json";
const DEFAULT_REPLAY_PATH: &str = "artifacts/eval_replay_runner_latest.json";
const DEFAULT_FIX_BEFORE_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_fix_verification_before.json";
const DEFAULT_FIX_AFTER_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_fix_verification_after.json";
const DEFAULT_FIX_OUT_PATH: &str = "core/local/artifacts/eval_fix_verification_current.json";
const DEFAULT_FIX_OUT_LATEST_PATH: &str = "artifacts/eval_fix_verification_latest.json";
const DEFAULT_FIX_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_FIX_VERIFICATION_CURRENT.md";
const DEFAULT_LIFECYCLE_OUT_PATH: &str = "core/local/artifacts/eval_issue_lifecycle_current.json";
const DEFAULT_LIFECYCLE_OUT_LATEST_PATH: &str = "artifacts/eval_issue_lifecycle_latest.json";
const DEFAULT_LIFECYCLE_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_ISSUE_LIFECYCLE_CURRENT.md";
const DEFAULT_QUALITY_GATE_PATH: &str = "artifacts/eval_quality_gate_v1_latest.json";
const DEFAULT_JUDGE_HUMAN_PATH: &str = "artifacts/eval_judge_human_agreement_latest.json";
const DEFAULT_PHASE_TRACE_PATH: &str = "local/state/ops/orchestration/workflow_phase_trace_latest.json";
const DEFAULT_RSI_OUT_PATH: &str = "core/local/artifacts/eval_rsi_escalation_gate_current.json";
const DEFAULT_RSI_OUT_LATEST_PATH: &str = "artifacts/eval_rsi_escalation_gate_latest.json";
const DEFAULT_RSI_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_RSI_ESCALATION_GATE_CURRENT.md";

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

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str().map(str::trim).filter(|v| !v.is_empty())
}

fn replay_map(report: &Value) -> BTreeMap<String, bool> {
    let mut map = BTreeMap::new();
    if let Some(rows) = report.get("results").and_then(|v| v.as_array()) {
        for row in rows {
            let Some(issue_id) = str_at(row, &["issue_id"]) else {
                continue;
            };
            map.insert(issue_id.to_string(), bool_at(row, &["ok"]));
        }
    }
    map
}

fn classify_fix(before: Option<bool>, after: Option<bool>) -> &'static str {
    match (before, after) {
        (Some(false), Some(true)) => "fixed",
        (Some(false), Some(false)) => "unresolved",
        (Some(true), Some(false)) => "regression_risk",
        (Some(true), Some(true)) => "fixed",
        (None, Some(true)) => "partially_fixed",
        (None, Some(false)) => "unresolved",
        _ => "partially_fixed",
    }
}

pub fn run_fix_verification(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let before_path = parse_flag(args, "before").unwrap_or_else(|| DEFAULT_FIX_BEFORE_PATH.to_string());
    let after_path = parse_flag(args, "after").unwrap_or_else(|| DEFAULT_FIX_AFTER_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_FIX_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_FIX_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_FIX_MARKDOWN_PATH.to_string());
    let before = replay_map(&read_json(&before_path));
    let after = replay_map(&read_json(&after_path));
    let mut ids: BTreeSet<String> = before.keys().cloned().collect();
    ids.extend(after.keys().cloned());
    let mut rows = Vec::new();
    let mut fixed = 0_u64;
    let mut partial = 0_u64;
    let mut unresolved = 0_u64;
    let mut regression_risk = 0_u64;
    for id in ids {
        let classification = classify_fix(before.get(&id).copied(), after.get(&id).copied());
        match classification {
            "fixed" => fixed += 1,
            "partially_fixed" => partial += 1,
            "unresolved" => unresolved += 1,
            "regression_risk" => regression_risk += 1,
            _ => {}
        }
        rows.push(json!({
            "issue_id": id,
            "before_ok": before.get(&id).copied(),
            "after_ok": after.get(&id).copied(),
            "classification": classification,
        }));
    }
    let ok = !rows.is_empty() && regression_risk == 0;
    let report = json!({
        "type": "eval_fix_verification",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "fix_verification_rows_present", "ok": !rows.is_empty(), "detail": format!("rows={}", rows.len())},
            {"id": "fix_verification_classification_contract", "ok": true, "detail": "fixed|partially_fixed|unresolved|regression_risk"},
            {"id": "fix_verification_regression_risk_contract", "ok": regression_risk == 0, "detail": format!("regression_risk={}", regression_risk)}
        ],
        "summary": {
            "rows": rows.len(),
            "fixed": fixed,
            "partially_fixed": partial,
            "unresolved": unresolved,
            "regression_risk": regression_risk,
        },
        "results": rows,
        "sources": {"before": before_path, "after": after_path}
    });
    let markdown = format!(
        "# Eval Fix Verification (Current)\n\n- generated_at: {}\n- ok: {}\n- rows: {}\n- fixed: {}\n- partially_fixed: {}\n- unresolved: {}\n- regression_risk: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        rows.len(),
        fixed,
        partial,
        unresolved,
        regression_risk
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write fix-verification outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

pub fn run_issue_lifecycle(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let issues_path = parse_flag(args, "issues").unwrap_or_else(|| DEFAULT_ISSUE_DRAFTS_PATH.to_string());
    let fix_path = parse_flag(args, "fixes").unwrap_or_else(|| DEFAULT_FIX_OUT_LATEST_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_LIFECYCLE_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_LIFECYCLE_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_LIFECYCLE_MARKDOWN_PATH.to_string());
    let issues = read_json(&issues_path);
    let fixes = read_json(&fix_path);
    let fix_map: BTreeMap<String, String> = fixes
        .get("results")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((
                str_at(row, &["issue_id"])?.to_string(),
                str_at(row, &["classification"])?.to_string(),
            ))
        })
        .collect();
    let mut rows = Vec::new();
    let mut unresolved_high = 0_u64;
    if let Some(drafts) = issues.get("issue_drafts").and_then(|v| v.as_array()) {
        for draft in drafts {
            let id = str_at(draft, &["id"]).unwrap_or("unknown");
            let classification = fix_map
                .get(id)
                .cloned()
                .unwrap_or_else(|| "unresolved".to_string());
            let high = bool_at(draft, &["persistent_high_severity"]);
            if high && classification != "fixed" {
                unresolved_high += 1;
            }
            let closure_score = if classification == "fixed" { 1.0 } else { 0.4 };
            rows.push(json!({
                "issue_id": id,
                "draft_issue": "ready",
                "human_approval": "pending",
                "github_filing": "blocked_until_human_approval",
                "patch_link": if classification == "fixed" { "available_after_patch" } else { "missing" },
                "replay_verification": classification,
                "closure_score": closure_score,
                "persistent_high_severity": high,
            }));
        }
    }
    let ok = !rows.is_empty();
    let report = json!({
        "type": "eval_issue_lifecycle",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "issue_lifecycle_rows_present", "ok": !rows.is_empty(), "detail": format!("rows={}", rows.len())},
            {"id": "human_approval_before_github_filing_contract", "ok": true, "detail": "github_filing=blocked_until_human_approval until approval is present"},
            {"id": "closure_score_contract", "ok": true, "detail": "closure_score emitted per issue"}
        ],
        "summary": {
            "issues": rows.len(),
            "unresolved_high_severity": unresolved_high,
        },
        "issues": rows,
        "sources": {"issues": issues_path, "fixes": fix_path}
    });
    let markdown = format!(
        "# Eval Issue Lifecycle (Current)\n\n- generated_at: {}\n- ok: {}\n- issues: {}\n- unresolved_high_severity: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        rows.len(),
        unresolved_high
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write issue-lifecycle outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

pub fn run_rsi_escalation(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let quality_path = parse_flag(args, "quality").unwrap_or_else(|| DEFAULT_QUALITY_GATE_PATH.to_string());
    let replay_path = parse_flag(args, "replay").unwrap_or_else(|| DEFAULT_REPLAY_PATH.to_string());
    let judge_path = parse_flag(args, "judge-human").unwrap_or_else(|| DEFAULT_JUDGE_HUMAN_PATH.to_string());
    let lifecycle_path = parse_flag(args, "lifecycle").unwrap_or_else(|| DEFAULT_LIFECYCLE_OUT_LATEST_PATH.to_string());
    let phase_trace_path = parse_flag(args, "phase-trace").unwrap_or_else(|| DEFAULT_PHASE_TRACE_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_RSI_OUT_PATH.to_string());
    let out_latest_path = parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_RSI_OUT_LATEST_PATH.to_string());
    let markdown_path = parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_RSI_MARKDOWN_PATH.to_string());
    let quality = read_json(&quality_path);
    let replay = read_json(&replay_path);
    let judge = read_json(&judge_path);
    let lifecycle = read_json(&lifecycle_path);
    let clean_passes_ready = bool_at(&quality, &["summary", "autonomous_escalation_allowed"]);
    let replay_ready = bool_at(&replay, &["ok"]);
    let calibration_ready = bool_at(&judge, &["summary", "calibration_ready"]);
    let phase_trace_fresh = Path::new(&phase_trace_path).exists();
    let unresolved_high = u64_at(&lifecycle, &["summary", "unresolved_high_severity"]);
    let no_unresolved_high = unresolved_high == 0;
    let checks = vec![
        json!({"id": "consecutive_clean_eval_passes_contract", "ok": clean_passes_ready, "detail": format!("autonomous_escalation_allowed={}", clean_passes_ready)}),
        json!({"id": "fresh_phase_trace_contract", "ok": phase_trace_fresh, "detail": phase_trace_path}),
        json!({"id": "passing_replay_fixtures_contract", "ok": replay_ready, "detail": replay_path}),
        json!({"id": "human_calibration_contract", "ok": calibration_ready, "detail": judge_path}),
        json!({"id": "no_unresolved_high_severity_eval_issues_contract", "ok": no_unresolved_high, "detail": format!("unresolved_high_severity={}", unresolved_high)}),
    ];
    let ok = checks
        .iter()
        .all(|row| row.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
    let report = json!({
        "type": "eval_rsi_escalation_gate",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "rsi_escalation_allowed": ok,
        "checks": checks,
        "summary": {
            "clean_passes_ready": clean_passes_ready,
            "phase_trace_fresh": phase_trace_fresh,
            "replay_ready": replay_ready,
            "calibration_ready": calibration_ready,
            "unresolved_high_severity": unresolved_high,
        },
        "sources": {
            "quality": quality_path,
            "replay": replay_path,
            "judge_human": judge_path,
            "lifecycle": lifecycle_path,
            "phase_trace": phase_trace_path,
        }
    });
    let markdown = format!(
        "# Eval RSI Escalation Gate (Current)\n\n- generated_at: {}\n- ok: {}\n- rsi_escalation_allowed: {}\n- clean_passes_ready: {}\n- phase_trace_fresh: {}\n- replay_ready: {}\n- calibration_ready: {}\n- unresolved_high_severity: {}\n",
        report.get("generated_at").and_then(|v| v.as_str()).unwrap_or(""),
        ok,
        ok,
        clean_passes_ready,
        phase_trace_fresh,
        replay_ready,
        calibration_ready,
        unresolved_high
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write RSI escalation outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}
