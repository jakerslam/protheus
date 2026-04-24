use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_action_economy_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_action_economy_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_action_economy_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_ACTION_ECONOMY_GUARD_CURRENT.md";

pub fn run_action_economy_guard(args: &[String]) -> i32 {
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
    let max_unnecessary_steps =
        parse_u64_from_path(&thresholds, &["max_unnecessary_steps_per_case"], 0);
    let max_redundant_planning = parse_u64_from_path(
        &thresholds,
        &["max_redundant_planning_or_reflection_steps"],
        0,
    );
    let max_tool_call_overhead =
        parse_u64_from_path(&thresholds, &["max_tool_call_overhead_per_case"], 0);
    let max_phase_latency_ms = parse_u64_from_path(&thresholds, &["max_phase_latency_ms"], 900);
    let max_cost_per_success = parse_f64_from_path(&thresholds, &["max_cost_per_success"], 10.0);
    let max_budget_overruns = parse_u64_from_path(&thresholds, &["max_budget_overruns"], 0);
    let min_reference_efficiency =
        parse_f64_from_path(&thresholds, &["min_reference_efficiency_rate"], 1.0);

    let mut unnecessary_failures = Vec::new();
    let mut redundant_planning_failures = Vec::new();
    let mut latency_failures = Vec::new();
    let mut budget_overruns = Vec::new();
    let mut reference_failures = Vec::new();
    let mut total_cost = 0.0_f64;
    let mut successful_cases = 0_u64;
    let mut total_actual_steps = 0_u64;
    let mut total_reference_steps = 0_u64;
    let mut total_tool_calls = 0_u64;
    let mut total_reference_tool_calls = 0_u64;

    for case in cases.iter() {
        let actual_steps = array_at(case, "actual_steps");
        let reference_steps = array_at(case, "reference_steps");
        let actual_count = actual_steps.len() as u64;
        let reference_count = reference_steps.len() as u64;
        let actual_tool_calls = count_tool_calls(&actual_steps);
        let reference_tool_calls = count_tool_calls(&reference_steps);
        let unnecessary = actual_steps
            .iter()
            .filter(|step| !parse_bool_from_path(step, &["required"], true))
            .count() as u64;
        let redundant_planning = actual_steps
            .iter()
            .filter(|step| {
                let purpose = parse_string_from_path(step, &["purpose"], "");
                !parse_bool_from_path(step, &["required"], true)
                    && matches!(purpose.as_str(), "planning" | "reflection")
            })
            .count() as u64;
        let max_latency = actual_steps
            .iter()
            .map(|step| parse_u64_from_path(step, &["phase_latency_ms"], 0))
            .max()
            .unwrap_or(0);
        let cost = parse_f64_from_path(case, &["actual", "cost_units"], 0.0);
        let budget = parse_f64_from_path(case, &["budget", "max_cost_units"], f64::MAX);
        let success = parse_bool_from_path(case, &["actual", "success"], false);
        let tool_overhead = actual_tool_calls.saturating_sub(reference_tool_calls);
        let reference_efficiency = if actual_count == 0 {
            0.0
        } else {
            (reference_count as f64 / actual_count as f64).min(1.0)
        };

        total_cost += cost;
        total_actual_steps = total_actual_steps.saturating_add(actual_count);
        total_reference_steps = total_reference_steps.saturating_add(reference_count);
        total_tool_calls = total_tool_calls.saturating_add(actual_tool_calls);
        total_reference_tool_calls =
            total_reference_tool_calls.saturating_add(reference_tool_calls);
        if success {
            successful_cases = successful_cases.saturating_add(1);
        }
        if unnecessary > max_unnecessary_steps {
            unnecessary_failures.push(case_summary(case, "unnecessary_steps_over_budget"));
        }
        if redundant_planning > max_redundant_planning {
            redundant_planning_failures.push(case_summary(
                case,
                "redundant_planning_or_reflection_over_budget",
            ));
        }
        if tool_overhead > max_tool_call_overhead || reference_efficiency < min_reference_efficiency
        {
            reference_failures.push(json!({
                "id": parse_string_from_path(case, &["id"], "unknown"),
                "reason": "reference_trajectory_efficiency_failure",
                "actual_steps": actual_count,
                "reference_steps": reference_count,
                "actual_tool_calls": actual_tool_calls,
                "reference_tool_calls": reference_tool_calls,
                "reference_efficiency": reference_efficiency
            }));
        }
        if max_latency > max_phase_latency_ms {
            latency_failures.push(json!({
                "id": parse_string_from_path(case, &["id"], "unknown"),
                "reason": "phase_latency_over_budget",
                "max_phase_latency_ms": max_latency,
                "budget_ms": max_phase_latency_ms
            }));
        }
        if cost > budget {
            budget_overruns.push(json!({
                "id": parse_string_from_path(case, &["id"], "unknown"),
                "reason": "budget_overrun",
                "cost_units": cost,
                "budget_units": budget
            }));
        }
    }

    let cost_per_success = if successful_cases == 0 {
        total_cost
    } else {
        total_cost / successful_cases as f64
    };
    let reference_efficiency_rate = if total_actual_steps == 0 {
        0.0
    } else {
        (total_reference_steps as f64 / total_actual_steps as f64).min(1.0)
    };
    let action_ok = unnecessary_failures.is_empty()
        && redundant_planning_failures.is_empty()
        && latency_failures.is_empty();
    let cost_ok = cost_per_success <= max_cost_per_success
        && budget_overruns.len() as u64 <= max_budget_overruns;
    let reference_ok =
        reference_failures.is_empty() && reference_efficiency_rate >= min_reference_efficiency;
    let checks = vec![
        json!({
            "id": "action_economy_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "action_economy_scoring_contract",
            "ok": cases.len() as u64 >= min_cases && action_ok,
            "detail": format!(
                "cases={};unnecessary_failures={};redundant_planning_failures={};latency_failures={}",
                cases.len(), unnecessary_failures.len(), redundant_planning_failures.len(), latency_failures.len()
            ),
        }),
        json!({
            "id": "cost_per_success_budget_contract",
            "ok": cost_ok,
            "detail": format!(
                "cost_per_success={:.3};max_cost_per_success={:.3};budget_overruns={};max_budget_overruns={}",
                cost_per_success, max_cost_per_success, budget_overruns.len(), max_budget_overruns
            ),
        }),
        json!({
            "id": "reference_trajectory_comparison_contract",
            "ok": reference_ok,
            "detail": format!(
                "reference_efficiency_rate={:.3};min={:.3};actual_steps={};reference_steps={};tool_calls={};reference_tool_calls={}",
                reference_efficiency_rate, min_reference_efficiency, total_actual_steps, total_reference_steps, total_tool_calls, total_reference_tool_calls
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_action_economy_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "cases": cases.len(),
            "successful_cases": successful_cases,
            "total_cost_units": total_cost,
            "cost_per_success": cost_per_success,
            "budget_overruns": budget_overruns.len(),
            "total_actual_steps": total_actual_steps,
            "total_reference_steps": total_reference_steps,
            "reference_efficiency_rate": reference_efficiency_rate,
            "total_tool_calls": total_tool_calls,
            "total_reference_tool_calls": total_reference_tool_calls,
            "unnecessary_failures": unnecessary_failures.len(),
            "redundant_planning_failures": redundant_planning_failures.len(),
            "latency_failures": latency_failures.len()
        },
        "unnecessary_failures": unnecessary_failures,
        "redundant_planning_failures": redundant_planning_failures,
        "latency_failures": latency_failures,
        "budget_overruns": budget_overruns,
        "reference_failures": reference_failures,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Action Economy Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- cost_per_success: {:.3}\n- reference_efficiency_rate: {:.3}\n- budget_overruns: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        cases.len(),
        cost_per_success,
        reference_efficiency_rate,
        report
            .get("summary")
            .and_then(|node| node.get("budget_overruns"))
            .and_then(|node| node.as_u64())
            .unwrap_or(0)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more action-economy outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn array_at(value: &Value, field: &str) -> Vec<Value> {
    value
        .get(field)
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default()
}

fn count_tool_calls(steps: &[Value]) -> u64 {
    steps
        .iter()
        .filter(|step| parse_bool_from_path(step, &["tool_call"], false))
        .count() as u64
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

fn now_iso_like() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", millis)
}
