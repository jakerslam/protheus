use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_trajectory_scoring_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_trajectory_scoring_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_trajectory_scoring_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_TRAJECTORY_SCORING_GUARD_CURRENT.md";

pub fn run_trajectory_scoring_guard(args: &[String]) -> i32 {
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
    let min_tool_choice_rate = parse_f64_from_path(&thresholds, &["min_tool_choice_rate"], 1.0);
    let min_tool_order_rate = parse_f64_from_path(&thresholds, &["min_tool_order_rate"], 1.0);
    let min_parameter_rate = parse_f64_from_path(&thresholds, &["min_parameter_rate"], 1.0);
    let min_stop_continue_rate = parse_f64_from_path(&thresholds, &["min_stop_continue_rate"], 1.0);
    let max_redundant_per_turn =
        parse_u64_from_path(&thresholds, &["max_redundant_tool_calls_per_turn"], 0);
    let max_loop_workflows =
        parse_u64_from_path(&thresholds, &["max_repeated_call_loop_workflows"], 0);
    let max_ungrounded_outputs = parse_u64_from_path(&thresholds, &["max_ungrounded_outputs"], 0);

    let mut tool_choice_success = 0_u64;
    let mut tool_order_success = 0_u64;
    let mut parameter_success = 0_u64;
    let mut stop_continue_success = 0_u64;
    let mut comparable_steps = 0_u64;
    let mut redundant_tool_failures = Vec::new();
    let mut repeated_loop_failures = Vec::new();
    let mut ungrounded_outputs = Vec::new();

    for case in cases.iter() {
        let expected = case
            .get("expected_steps")
            .and_then(|node| node.as_array())
            .cloned()
            .unwrap_or_default();
        let actual = case
            .get("actual_steps")
            .and_then(|node| node.as_array())
            .cloned()
            .unwrap_or_default();
        comparable_steps = comparable_steps.saturating_add(expected.len() as u64);
        if order_matches(&expected, &actual) {
            tool_order_success = tool_order_success.saturating_add(expected.len() as u64);
        }
        for (index, expected_step) in expected.iter().enumerate() {
            let actual_step = actual.get(index).unwrap_or(&Value::Null);
            if tool_choice_matches(expected_step, actual_step) {
                tool_choice_success = tool_choice_success.saturating_add(1);
            }
            if parameters_match(expected_step, actual_step) {
                parameter_success = parameter_success.saturating_add(1);
            }
            if stop_continue_matches(expected_step, actual_step) {
                stop_continue_success = stop_continue_success.saturating_add(1);
            }
        }

        let redundant = redundant_tool_calls_by_turn(&actual);
        let max_case_redundant = redundant.values().copied().max().unwrap_or(0);
        if max_case_redundant > max_redundant_per_turn {
            redundant_tool_failures.push(case_summary(case, "redundant_tool_calls_per_turn"));
        }
        if has_repeated_call_loop(&actual) {
            repeated_loop_failures.push(case_summary(case, "repeated_tool_call_loop"));
        }
        for output in case
            .get("outputs")
            .and_then(|node| node.as_array())
            .into_iter()
            .flatten()
        {
            if !output_is_grounded(output) {
                ungrounded_outputs.push(json!({
                    "case_id": parse_string_from_path(case, &["id"], "unknown"),
                    "output_id": parse_string_from_path(output, &["id"], "unknown"),
                    "reason": "missing_required_receipt_citation"
                }));
            }
        }
    }

    let tool_choice_rate = ratio(tool_choice_success, comparable_steps);
    let tool_order_rate = ratio(tool_order_success, comparable_steps);
    let parameter_rate = ratio(parameter_success, comparable_steps);
    let stop_continue_rate = ratio(stop_continue_success, comparable_steps);
    let trajectory_ok = tool_choice_rate >= min_tool_choice_rate
        && tool_order_rate >= min_tool_order_rate
        && parameter_rate >= min_parameter_rate
        && stop_continue_rate >= min_stop_continue_rate;
    let redundancy_ok = redundant_tool_failures.is_empty()
        && repeated_loop_failures.len() as u64 <= max_loop_workflows;
    let grounding_ok = ungrounded_outputs.len() as u64 <= max_ungrounded_outputs;
    let checks = vec![
        json!({
            "id": "trajectory_scoring_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "trajectory_scoring_minimum_case_contract",
            "ok": cases.len() as u64 >= min_cases,
            "detail": format!("cases={};min_cases={}", cases.len(), min_cases),
        }),
        json!({
            "id": "trajectory_tool_scoring_contract",
            "ok": trajectory_ok,
            "detail": format!(
                "tool_choice={:.3};tool_order={:.3};parameters={:.3};stop_continue={:.3}",
                tool_choice_rate, tool_order_rate, parameter_rate, stop_continue_rate
            ),
        }),
        json!({
            "id": "trajectory_redundant_tool_loop_contract",
            "ok": redundancy_ok,
            "detail": format!(
                "redundant_turn_failures={};repeated_loop_workflows={};max_loop_workflows={}",
                redundant_tool_failures.len(), repeated_loop_failures.len(), max_loop_workflows
            ),
        }),
        json!({
            "id": "trajectory_intermediate_output_grounding_contract",
            "ok": grounding_ok,
            "detail": format!(
                "ungrounded_outputs={};max_ungrounded_outputs={}",
                ungrounded_outputs.len(), max_ungrounded_outputs
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_trajectory_scoring_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "cases": cases.len(),
            "comparable_steps": comparable_steps,
            "tool_choice_rate": tool_choice_rate,
            "tool_order_rate": tool_order_rate,
            "parameter_correctness_rate": parameter_rate,
            "stop_continue_rate": stop_continue_rate,
            "redundant_tool_failures": redundant_tool_failures.len(),
            "repeated_loop_failures": repeated_loop_failures.len(),
            "ungrounded_outputs": ungrounded_outputs.len()
        },
        "redundant_tool_failures": redundant_tool_failures,
        "repeated_loop_failures": repeated_loop_failures,
        "ungrounded_outputs": ungrounded_outputs,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Trajectory Scoring Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- comparable_steps: {}\n- tool_choice_rate: {:.3}\n- tool_order_rate: {:.3}\n- parameter_correctness_rate: {:.3}\n- stop_continue_rate: {:.3}\n- ungrounded_outputs: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        cases.len(),
        comparable_steps,
        tool_choice_rate,
        tool_order_rate,
        parameter_rate,
        stop_continue_rate,
        report
            .get("summary")
            .and_then(|node| node.get("ungrounded_outputs"))
            .and_then(|node| node.as_u64())
            .unwrap_or(0)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more trajectory-scoring outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn order_matches(expected: &[Value], actual: &[Value]) -> bool {
    expected.len() <= actual.len()
        && expected
            .iter()
            .zip(actual.iter())
            .all(|(expected_step, actual_step)| tool_choice_matches(expected_step, actual_step))
}

fn tool_choice_matches(expected: &Value, actual: &Value) -> bool {
    parse_string_from_path(expected, &["tool_family"], "")
        == parse_string_from_path(actual, &["tool_family"], "")
        && parse_string_from_path(expected, &["tool_name"], "")
            == parse_string_from_path(actual, &["tool_name"], "")
}

fn parameters_match(expected: &Value, actual: &Value) -> bool {
    let Some(expected_params) = expected
        .get("parameters_subset")
        .and_then(|node| node.as_object())
    else {
        return true;
    };
    let Some(actual_params) = actual.get("parameters").and_then(|node| node.as_object()) else {
        return false;
    };
    expected_params
        .iter()
        .all(|(key, value)| actual_params.get(key) == Some(value))
}

fn stop_continue_matches(expected: &Value, actual: &Value) -> bool {
    parse_string_from_path(expected, &["decision"], "")
        == parse_string_from_path(actual, &["decision"], "")
}

fn redundant_tool_calls_by_turn(actual: &[Value]) -> BTreeMap<String, u64> {
    let mut signatures: BTreeMap<String, u64> = BTreeMap::new();
    for step in actual {
        let turn_id = parse_string_from_path(step, &["turn_id"], "unknown");
        let signature = format!(
            "{}:{}:{}:{}",
            turn_id,
            parse_string_from_path(step, &["tool_family"], ""),
            parse_string_from_path(step, &["tool_name"], ""),
            canonical_json(step.get("parameters").unwrap_or(&Value::Null))
        );
        signatures
            .entry(signature)
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
    }
    let mut redundant: BTreeMap<String, u64> = BTreeMap::new();
    for (signature, count) in signatures {
        if count > 1 {
            let turn = signature.split(':').next().unwrap_or("unknown").to_string();
            redundant
                .entry(turn)
                .and_modify(|value| *value = (*value).max(count.saturating_sub(1)))
                .or_insert(count.saturating_sub(1));
        }
    }
    redundant
}

fn has_repeated_call_loop(actual: &[Value]) -> bool {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for step in actual {
        let signature = format!(
            "{}:{}:{}",
            parse_string_from_path(step, &["tool_family"], ""),
            parse_string_from_path(step, &["tool_name"], ""),
            canonical_json(step.get("parameters").unwrap_or(&Value::Null))
        );
        counts
            .entry(signature)
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
    }
    counts.values().any(|count| *count >= 3)
}

fn output_is_grounded(output: &Value) -> bool {
    let depends_on = string_array(output, "depends_on_receipts");
    let cites = string_array(output, "cites_receipts");
    !depends_on.is_empty() && depends_on.iter().all(|receipt| cites.contains(receipt))
}

fn string_array(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(|node| node.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_str().map(|text| text.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn case_summary(case: &Value, reason: &str) -> Value {
    json!({
        "id": parse_string_from_path(case, &["id"], "unknown"),
        "workflow": parse_string_from_path(case, &["workflow"], "unknown"),
        "reason": reason
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
