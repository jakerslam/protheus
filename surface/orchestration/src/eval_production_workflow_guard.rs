use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_TELEMETRY_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_production_workflow_telemetry.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_production_workflow_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_production_workflow_guard_latest.json";
const DEFAULT_REPLAY_PATH: &str =
    "local/state/ops/eval_replay_fixtures/workflow_specific_replay_pack_latest.json";
const DEFAULT_DASHBOARD_PATH: &str = "core/local/artifacts/eval_workflow_reliability_current.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_PRODUCTION_WORKFLOW_RELIABILITY_CURRENT.md";

pub fn run_production_workflow_guard(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let telemetry_path =
        parse_flag(args, "telemetry").unwrap_or_else(|| DEFAULT_TELEMETRY_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let replay_path =
        parse_flag(args, "out-replay").unwrap_or_else(|| DEFAULT_REPLAY_PATH.to_string());
    let dashboard_path =
        parse_flag(args, "out-dashboard").unwrap_or_else(|| DEFAULT_DASHBOARD_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let input = read_json(&telemetry_path);
    let events = input
        .get("events")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_events = parse_u64_from_path(&thresholds, &["min_events"], 5);
    let min_candidate_count = parse_u64_from_path(&thresholds, &["min_candidate_count"], 5);
    let required_workflows = string_array(&input, "required_workflows");

    let mut candidates = Vec::new();
    let mut replay_entries = Vec::new();
    let mut workflow_rows: BTreeMap<String, WorkflowStats> = BTreeMap::new();
    for event in events.iter() {
        let workflow = parse_string_from_path(event, &["workflow"], "unknown");
        let success = parse_bool_from_path(event, &["success"], false);
        workflow_rows
            .entry(workflow.clone())
            .and_modify(|row| row.observe(success))
            .or_insert_with(|| WorkflowStats::new(success));
        if should_propose_candidate(event) {
            candidates.push(candidate_from_event(event));
            replay_entries.push(replay_from_event(event));
        }
    }

    let candidate_leaks = candidates
        .iter()
        .filter(|candidate| candidate_contains_private_source(candidate))
        .count() as u64;
    let covered_workflows = replay_entries
        .iter()
        .filter_map(|entry| entry.get("workflow").and_then(|node| node.as_str()))
        .map(|workflow| workflow.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let missing_workflows: Vec<String> = required_workflows
        .iter()
        .filter(|workflow| !covered_workflows.contains(*workflow))
        .cloned()
        .collect();
    let workflow_report = workflow_report(&workflow_rows);
    let dashboard = json!({
        "type": "eval_workflow_reliability_report",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "sections": {
            "production_workflows": workflow_report,
            "generic_benchmark_isolated": {
                "rows": [],
                "note": "generic benchmark performance is intentionally separated from InfRing production workflow reliability"
            }
        }
    });
    let replay_pack = json!({
        "type": "eval_workflow_specific_replay_pack",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "entries": replay_entries,
        "required_workflows": required_workflows,
        "missing_workflows": missing_workflows
    });

    let telemetry_ok = events.len() as u64 >= min_events
        && candidates.len() as u64 >= min_candidate_count
        && candidate_leaks == 0;
    let replay_ok = replay_pack
        .get("missing_workflows")
        .and_then(|node| node.as_array())
        .map(|rows| rows.is_empty())
        .unwrap_or(false);
    let dashboard_ok = dashboard
        .pointer("/sections/production_workflows")
        .and_then(|node| node.as_array())
        .map(|rows| rows.len() >= 5)
        .unwrap_or(false)
        && dashboard
            .pointer("/sections/generic_benchmark_isolated/rows")
            .and_then(|node| node.as_array())
            .map(|rows| rows.is_empty())
            .unwrap_or(false);
    let checks = vec![
        json!({
            "id": "production_telemetry_miner_contract",
            "ok": telemetry_ok,
            "detail": format!(
                "events={};min_events={};candidates={};min_candidates={};private_content_leaks={}",
                events.len(), min_events, candidates.len(), min_candidate_count, candidate_leaks
            ),
        }),
        json!({
            "id": "workflow_specific_replay_pack_contract",
            "ok": replay_ok,
            "detail": format!(
                "replay_entries={};missing_workflows={}",
                replay_pack.pointer("/entries").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0),
                replay_pack.pointer("/missing_workflows").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0)
            ),
        }),
        json!({
            "id": "workflow_specific_reliability_report_contract",
            "ok": dashboard_ok,
            "detail": "production workflow reliability report is separated from generic benchmark rows",
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_production_workflow_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "events": events.len(),
            "candidate_count": candidates.len(),
            "candidate_private_content_leaks": candidate_leaks,
            "replay_entries": replay_pack.pointer("/entries").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0),
            "missing_replay_workflows": replay_pack.pointer("/missing_workflows").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0),
            "production_workflow_rows": dashboard.pointer("/sections/production_workflows").and_then(|node| node.as_array()).map(|rows| rows.len()).unwrap_or(0),
            "generic_benchmark_rows": 0
        },
        "proposed_gold_candidates": candidates,
        "replay_pack_path": replay_path,
        "dashboard_path": dashboard_path,
        "sources": {
            "telemetry": telemetry_path
        }
    });
    let markdown = format!(
        "# Eval Production Workflow Reliability (Current)\n\n- generated_at: {}\n- ok: {}\n- candidates: {}\n- replay_entries: {}\n- production_workflow_rows: {}\n- generic_benchmark_rows: 0\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        report.pointer("/summary/candidate_count").and_then(|node| node.as_u64()).unwrap_or(0),
        report.pointer("/summary/replay_entries").and_then(|node| node.as_u64()).unwrap_or(0),
        report.pointer("/summary/production_workflow_rows").and_then(|node| node.as_u64()).unwrap_or(0)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_json(&replay_path, &replay_pack).is_ok()
        && write_json(&dashboard_path, &dashboard).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more production workflow outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

#[derive(Clone, Copy)]
struct WorkflowStats {
    total: u64,
    successes: u64,
}

impl WorkflowStats {
    fn new(success: bool) -> Self {
        Self {
            total: 1,
            successes: if success { 1 } else { 0 },
        }
    }

    fn observe(&mut self, success: bool) {
        self.total = self.total.saturating_add(1);
        if success {
            self.successes = self.successes.saturating_add(1);
        }
    }
}

fn workflow_report(rows: &BTreeMap<String, WorkflowStats>) -> Vec<Value> {
    rows.iter()
        .map(|(workflow, stats)| {
            json!({
                "workflow": workflow,
                "events": stats.total,
                "successes": stats.successes,
                "failures": stats.total.saturating_sub(stats.successes),
                "success_rate": ratio(stats.successes, stats.total)
            })
        })
        .collect()
}

fn should_propose_candidate(event: &Value) -> bool {
    !parse_bool_from_path(event, &["success"], false)
        || parse_u64_from_path(event, &["recurrence_count"], 0) >= 2
        || matches!(
            parse_string_from_path(event, &["severity"], "low").as_str(),
            "high" | "critical"
        )
}

fn candidate_from_event(event: &Value) -> Value {
    json!({
        "id": format!("candidate-{}", parse_string_from_path(event, &["event_id"], "unknown")),
        "workflow": parse_string_from_path(event, &["workflow"], "unknown"),
        "component": parse_string_from_path(event, &["component"], "unknown"),
        "failure_class": parse_string_from_path(event, &["failure_class"], "unknown"),
        "normalized_failure_code": parse_string_from_path(event, &["normalized_failure_code"], "unknown"),
        "severity": parse_string_from_path(event, &["severity"], "medium"),
        "sanitized_summary": parse_string_from_path(event, &["sanitized_summary"], ""),
        "source_hash": parse_string_from_path(event, &["source_hash"], ""),
        "private_content_redacted": true,
        "raw_prompt_excluded": true
    })
}

fn replay_from_event(event: &Value) -> Value {
    json!({
        "workflow": parse_string_from_path(event, &["workflow"], "unknown"),
        "fixture_id": format!("replay-{}", parse_string_from_path(event, &["event_id"], "unknown")),
        "expected_failure_code": parse_string_from_path(event, &["normalized_failure_code"], "unknown"),
        "receipts": event.get("receipt_ids").cloned().unwrap_or_else(|| json!([])),
        "replay_command": format!(
            "cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin eval_runtime -- production-workflow-guard --telemetry={} --strict=1",
            DEFAULT_TELEMETRY_PATH
        )
    })
}

fn candidate_contains_private_source(candidate: &Value) -> bool {
    if candidate.get("raw_user_text").is_some() || candidate.get("private_content").is_some() {
        return true;
    }
    let serialized = serde_json::to_string(candidate).unwrap_or_default();
    ["C:\\Users\\", "/Users/", "secret="]
        .iter()
        .any(|needle| serialized.contains(needle))
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
