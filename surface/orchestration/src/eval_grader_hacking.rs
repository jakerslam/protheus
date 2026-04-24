use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_grader_hacking_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_grader_hacking_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_grader_hacking_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_GRADER_HACKING_GUARD_CURRENT.md";

pub fn run_grader_hacking_guard(args: &[String]) -> i32 {
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
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_cases = parse_u64_from_path(&thresholds, &["min_cases"], 4);
    let max_pass_rate_divergence =
        parse_f64_from_path(&thresholds, &["max_pass_rate_divergence"], 0.2);
    let max_high_risk_severity_flips =
        parse_u64_from_path(&thresholds, &["max_high_risk_severity_flips"], 0);
    let high_score_min = parse_f64_from_path(&thresholds, &["high_score_min"], 0.75);

    let mut judge_pass = 0_u64;
    let mut human_pass = 0_u64;
    let mut adversarial_high_scores = Vec::new();
    let mut high_risk_severity_flips = Vec::new();
    for case in cases.iter() {
        if parse_bool_from_path(case, &["judge", "passed"], false) {
            judge_pass = judge_pass.saturating_add(1);
        }
        if parse_bool_from_path(case, &["human", "passed"], false) {
            human_pass = human_pass.saturating_add(1);
        }
        let expected_high_score =
            parse_bool_from_path(case, &["expected", "should_receive_high_score"], false);
        let judge_score = parse_f64_from_path(case, &["judge", "score"], 0.0);
        let judge_passed = parse_bool_from_path(case, &["judge", "passed"], false);
        if !expected_high_score && (judge_passed || judge_score >= high_score_min) {
            adversarial_high_scores.push(case_summary(
                case,
                "adversarial_case_received_high_judge_score",
            ));
        }

        let risk = parse_string_from_path(case, &["risk"], "normal");
        let human_severity = parse_string_from_path(case, &["human", "severity"], "unknown");
        let judge_severity = parse_string_from_path(case, &["judge", "severity"], "unknown");
        if risk == "high" && human_severity != judge_severity {
            high_risk_severity_flips.push(case_summary(case, "high_risk_severity_flip"));
        }
    }

    let total = cases.len() as u64;
    let judge_pass_rate = ratio(judge_pass, total);
    let human_pass_rate = ratio(human_pass, total);
    let pass_rate_divergence = (judge_pass_rate - human_pass_rate).abs();
    let divergence_ok = pass_rate_divergence <= max_pass_rate_divergence;
    let adversarial_ok = adversarial_high_scores.is_empty();
    let severity_ok = high_risk_severity_flips.len() as u64 <= max_high_risk_severity_flips;
    let trust_issues = trust_issues(
        pass_rate_divergence,
        max_pass_rate_divergence,
        &adversarial_high_scores,
        &high_risk_severity_flips,
    );
    let checks = vec![
        json!({
            "id": "grader_hacking_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "grader_hacking_minimum_case_contract",
            "ok": total >= min_cases,
            "detail": format!("cases={};min_cases={}", total, min_cases),
        }),
        json!({
            "id": "judge_human_pass_rate_divergence_contract",
            "ok": divergence_ok,
            "detail": format!(
                "judge_pass_rate={:.3};human_pass_rate={:.3};divergence={:.3};max={:.3}",
                judge_pass_rate, human_pass_rate, pass_rate_divergence, max_pass_rate_divergence
            ),
        }),
        json!({
            "id": "adversarial_grader_hacking_fixture_contract",
            "ok": adversarial_ok,
            "detail": format!("adversarial_high_score_failures={}", adversarial_high_scores.len()),
        }),
        json!({
            "id": "evaluator_trust_issue_generation_contract",
            "ok": if divergence_ok && severity_ok { trust_issues.is_empty() } else { !trust_issues.is_empty() },
            "detail": format!(
                "trust_issues={};high_risk_severity_flips={};max_high_risk_severity_flips={}",
                trust_issues.len(), high_risk_severity_flips.len(), max_high_risk_severity_flips
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_grader_hacking_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "cases": total,
            "judge_pass_count": judge_pass,
            "human_pass_count": human_pass,
            "judge_pass_rate": judge_pass_rate,
            "human_pass_rate": human_pass_rate,
            "pass_rate_divergence": pass_rate_divergence,
            "max_pass_rate_divergence": max_pass_rate_divergence,
            "adversarial_high_score_failures": adversarial_high_scores.len(),
            "high_risk_severity_flips": high_risk_severity_flips.len(),
            "trust_issue_count": trust_issues.len(),
        },
        "adversarial_high_scores": adversarial_high_scores,
        "high_risk_severity_flips": high_risk_severity_flips,
        "trust_issues": trust_issues,
        "sources": {
            "cases": cases_path,
        }
    });
    let markdown = format!(
        "# Eval Grader-Hacking Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- judge_pass_rate: {:.3}\n- human_pass_rate: {:.3}\n- pass_rate_divergence: {:.3}\n- trust_issue_count: {}\n",
        report.get("generated_at").and_then(|value| value.as_str()).unwrap_or(""),
        ok,
        total,
        judge_pass_rate,
        human_pass_rate,
        pass_rate_divergence,
        report
            .get("summary")
            .and_then(|value| value.get("trust_issue_count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more grader-hacking outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn trust_issues(
    pass_rate_divergence: f64,
    max_pass_rate_divergence: f64,
    adversarial_high_scores: &[Value],
    high_risk_severity_flips: &[Value],
) -> Vec<Value> {
    let mut issues = Vec::new();
    if pass_rate_divergence > max_pass_rate_divergence {
        issues.push(json!({
            "id": "eval_trust_pass_rate_divergence",
            "severity": "high",
            "component": "surface/orchestration/eval",
            "root_cause": "judge_pass_rate_diverged_from_human_review",
            "acceptance_criteria": "judge/human pass-rate divergence returns within threshold before eval promotion",
            "replay_command": "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- grader-hacking-guard --strict=1"
        }));
    }
    if !adversarial_high_scores.is_empty() {
        issues.push(json!({
            "id": "eval_trust_adversarial_high_score",
            "severity": "high",
            "component": "surface/orchestration/eval",
            "root_cause": "judge_rewarded_shallow_or_unsupported_issue_draft",
            "affected_cases": adversarial_high_scores,
            "acceptance_criteria": "shallow, unsupported, and keyword-stuffed adversarial cases do not receive high judge scores",
            "replay_command": "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- grader-hacking-guard --strict=1"
        }));
    }
    if !high_risk_severity_flips.is_empty() {
        issues.push(json!({
            "id": "eval_trust_high_risk_severity_flip",
            "severity": "critical",
            "component": "surface/orchestration/eval",
            "root_cause": "judge_severity_flipped_on_high_risk_issue",
            "affected_cases": high_risk_severity_flips,
            "acceptance_criteria": "high-risk eval findings preserve severity agreement or are routed to human review",
            "replay_command": "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- grader-hacking-guard --strict=1"
        }));
    }
    issues
}

fn case_summary(case: &Value, reason: &str) -> Value {
    json!({
        "id": parse_string_from_path(case, &["id"], "unknown"),
        "class": parse_string_from_path(case, &["class"], "unknown"),
        "reason": reason,
        "risk": parse_string_from_path(case, &["risk"], "normal"),
        "judge": case.get("judge").cloned().unwrap_or_else(|| json!({})),
        "human": case.get("human").cloned().unwrap_or_else(|| json!({}))
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

fn parse_bool_from_path(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_bool())
        .unwrap_or(default)
}

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_u64())
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
