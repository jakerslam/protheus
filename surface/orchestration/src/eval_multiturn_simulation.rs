use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_multiturn_simulation_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_multiturn_simulation_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_multiturn_simulation_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_MULTITURN_SIMULATION_GUARD_CURRENT.md";

pub fn run_multiturn_simulation_guard(args: &[String]) -> i32 {
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
    let min_cases = parse_u64_from_path(&thresholds, &["min_cases"], 3);
    let min_dialogue_success_rate =
        parse_f64_from_path(&thresholds, &["min_dialogue_success_rate"], 1.0);
    let min_turn_success_rate = parse_f64_from_path(&thresholds, &["min_turn_success_rate"], 1.0);
    let max_policy_violations = parse_u64_from_path(&thresholds, &["max_policy_violations"], 0);
    let min_clarification_quality =
        parse_f64_from_path(&thresholds, &["min_clarification_quality"], 0.8);
    let min_recovery_quality = parse_f64_from_path(&thresholds, &["min_recovery_quality"], 0.8);

    let mut mutable_goal_cases = 0_u64;
    let mut partial_info_turns = 0_u64;
    let mut clarification_turns = 0_u64;
    let mut frustration_turns = 0_u64;
    let mut tool_environment_cases = 0_u64;
    let mut dialogue_successes = 0_u64;
    let mut turn_successes = 0_u64;
    let mut total_turns = 0_u64;
    let mut policy_violations = Vec::new();
    let mut clarification_scores = Vec::new();
    let mut recovery_scores = Vec::new();

    for case in cases.iter() {
        let turns = case
            .get("turns")
            .and_then(|node| node.as_array())
            .cloned()
            .unwrap_or_default();
        if case
            .get("mutable_goal_events")
            .and_then(|node| node.as_array())
            .map(|events| !events.is_empty())
            .unwrap_or(false)
        {
            mutable_goal_cases = mutable_goal_cases.saturating_add(1);
        }
        if has_tool_environment(case) {
            tool_environment_cases = tool_environment_cases.saturating_add(1);
        }
        let mut case_success = !turns.is_empty();
        for turn in turns.iter() {
            total_turns = total_turns.saturating_add(1);
            if parse_bool_from_path(turn, &["user_signal", "partial_information"], false) {
                partial_info_turns = partial_info_turns.saturating_add(1);
            }
            if parse_bool_from_path(turn, &["user_signal", "clarification_needed"], false) {
                clarification_turns = clarification_turns.saturating_add(1);
                clarification_scores.push(parse_f64_from_path(
                    turn,
                    &["actual", "clarification_quality"],
                    0.0,
                ));
            }
            if parse_u64_from_path(turn, &["user_signal", "frustration_level"], 0) > 0 {
                frustration_turns = frustration_turns.saturating_add(1);
                recovery_scores.push(parse_f64_from_path(
                    turn,
                    &["actual", "recovery_quality"],
                    0.0,
                ));
            }
            let turn_ok = turn_success(turn);
            if turn_ok {
                turn_successes = turn_successes.saturating_add(1);
            } else {
                case_success = false;
            }
            if policy_violation(turn) {
                case_success = false;
                policy_violations.push(turn_summary(case, turn, "policy_violation"));
            }
        }
        if case_success {
            dialogue_successes = dialogue_successes.saturating_add(1);
        }
    }

    let dialogue_success_rate = ratio(dialogue_successes, cases.len() as u64);
    let turn_success_rate = ratio(turn_successes, total_turns);
    let clarification_quality = average(&clarification_scores);
    let recovery_quality = average(&recovery_scores);
    let stateful_ok = cases.len() as u64 >= min_cases
        && mutable_goal_cases > 0
        && partial_info_turns > 0
        && clarification_turns > 0
        && frustration_turns > 0;
    let policy_ok = tool_environment_cases as usize == cases.len()
        && policy_violations.len() as u64 <= max_policy_violations;
    let metrics_ok = dialogue_success_rate >= min_dialogue_success_rate
        && turn_success_rate >= min_turn_success_rate
        && clarification_quality >= min_clarification_quality
        && recovery_quality >= min_recovery_quality;

    let checks = vec![
        json!({
            "id": "multiturn_simulation_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "stateful_multiturn_user_simulation_contract",
            "ok": stateful_ok,
            "detail": format!(
                "cases={};min_cases={};mutable_goal_cases={};partial_info_turns={};clarification_turns={};frustration_turns={}",
                cases.len(), min_cases, mutable_goal_cases, partial_info_turns, clarification_turns, frustration_turns
            ),
        }),
        json!({
            "id": "policy_constrained_tool_environment_contract",
            "ok": policy_ok,
            "detail": format!(
                "tool_environment_cases={};cases={};policy_violations={};max_policy_violations={}",
                tool_environment_cases, cases.len(), policy_violations.len(), max_policy_violations
            ),
        }),
        json!({
            "id": "multiturn_eval_metrics_contract",
            "ok": metrics_ok,
            "detail": format!(
                "dialogue_success={:.3};turn_success={:.3};clarification_quality={:.3};recovery_quality={:.3}",
                dialogue_success_rate, turn_success_rate, clarification_quality, recovery_quality
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_multiturn_simulation_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "cases": cases.len(),
            "total_turns": total_turns,
            "dialogue_success_rate": dialogue_success_rate,
            "turn_success_rate": turn_success_rate,
            "policy_violation_count": policy_violations.len(),
            "clarification_quality": clarification_quality,
            "recovery_quality": recovery_quality,
            "mutable_goal_cases": mutable_goal_cases,
            "partial_info_turns": partial_info_turns,
            "frustration_turns": frustration_turns
        },
        "policy_violations": policy_violations,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Multi-Turn Simulation Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- total_turns: {}\n- dialogue_success_rate: {:.3}\n- turn_success_rate: {:.3}\n- policy_violation_count: {}\n- clarification_quality: {:.3}\n- recovery_quality: {:.3}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        cases.len(),
        total_turns,
        dialogue_success_rate,
        turn_success_rate,
        policy_violations.len(),
        clarification_quality,
        recovery_quality
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more multi-turn simulation outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn has_tool_environment(case: &Value) -> bool {
    case.pointer("/simulated_tool_environment/policy_constraints")
        .and_then(|node| node.as_array())
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        && case
            .pointer("/simulated_tool_environment/tools")
            .and_then(|node| node.as_array())
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
}

fn turn_success(turn: &Value) -> bool {
    let expected_tool = parse_string_from_path(turn, &["expected", "allowed_tool"], "");
    let actual_tool = parse_string_from_path(turn, &["actual", "tool_used"], "");
    let expected_policy = parse_string_from_path(turn, &["expected", "policy_action"], "");
    let actual_policy = parse_string_from_path(turn, &["actual", "policy_action"], "");
    parse_bool_from_path(turn, &["actual", "turn_success"], false)
        && expected_tool == actual_tool
        && expected_policy == actual_policy
}

fn policy_violation(turn: &Value) -> bool {
    parse_string_from_path(turn, &["expected", "policy_action"], "")
        != parse_string_from_path(turn, &["actual", "policy_action"], "")
}

fn turn_summary(case: &Value, turn: &Value, reason: &str) -> Value {
    json!({
        "case_id": parse_string_from_path(case, &["id"], "unknown"),
        "turn_id": parse_string_from_path(turn, &["turn_id"], "unknown"),
        "reason": reason,
        "expected": turn.get("expected").cloned().unwrap_or_else(|| json!({})),
        "actual": turn.get("actual").cloned().unwrap_or_else(|| json!({}))
    })
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        1.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
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
