// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::CompatApiResponse;

const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const WORKFLOWS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/workflows.json";
const WORKFLOW_RUNS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/workflow_runs.json";
const CRON_JOBS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/cron_jobs.json";
const TRIGGERS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/triggers.json";
const EYES_CATALOG_STATE_PATHS: [&str; 3] = [
    "client/runtime/local/state/ui/infring_dashboard/eyes_catalog.json",
    "client/runtime/local/state/eyes/catalog.json",
    "client/runtime/local/state/ui/eyes/catalog.json",
];

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn clean_id(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, max_len).to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn parse_json(body: &[u8]) -> Value {
    serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
}

fn array_from_value(value: &Value, key: &str) -> Vec<Value> {
    if value.is_array() {
        return value.as_array().cloned().unwrap_or_default();
    }
    value
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn make_id(prefix: &str, seed: &Value) -> String {
    let hash = crate::deterministic_receipt_hash(seed);
    format!(
        "{}-{}",
        clean_id(prefix, 24),
        hash.chars().take(10).collect::<String>()
    )
}

fn host_from_url(url: &str) -> String {
    let cleaned = clean_text(url, 400);
    if cleaned.is_empty() {
        return String::new();
    }
    let no_scheme = cleaned
        .split("://")
        .nth(1)
        .map(|v| v.to_string())
        .unwrap_or(cleaned);
    clean_text(no_scheme.split('/').next().unwrap_or(""), 120)
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn as_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn now_plus(minutes: i64) -> String {
    (Utc::now() + Duration::minutes(minutes.max(1))).to_rfc3339()
}

fn normalize_workflow_step(step: &Value, idx: usize) -> Value {
    let name = clean_text(
        step.get("name")
            .and_then(Value::as_str)
            .unwrap_or(&format!("step-{}", idx + 1)),
        120,
    );
    let agent_name = clean_text(
        step.get("agent_name")
            .and_then(Value::as_str)
            .or_else(|| step.pointer("/agent/name").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    );
    let mode_raw = clean_text(
        step.get("mode")
            .and_then(Value::as_str)
            .or_else(|| step.get("type").and_then(Value::as_str))
            .unwrap_or("sequential"),
        40,
    )
    .to_ascii_lowercase();
    let mode = match mode_raw.as_str() {
        "fan_out" | "parallel" => "fan_out",
        "conditional" => "conditional",
        "loop" => "loop",
        _ => "sequential",
    };
    let prompt_template = clean_text(
        step.get("prompt")
            .and_then(Value::as_str)
            .or_else(|| step.get("prompt_template").and_then(Value::as_str))
            .unwrap_or("{{input}}"),
        4000,
    );
    json!({
        "id": clean_id(
            step.get("id")
                .and_then(Value::as_str)
                .unwrap_or(&format!("step-{}", idx + 1)),
            80
        ),
        "name": if name.is_empty() { format!("step-{}", idx + 1) } else { name },
        "agent": {
            "name": agent_name
        },
        "mode": mode,
        "prompt_template": if prompt_template.is_empty() { "{{input}}" } else { &prompt_template }
    })
}

fn normalize_workflow(workflow: &Value) -> Value {
    let now = crate::now_iso();
    let name = clean_text(
        workflow
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("workflow"),
        160,
    );
    let id = {
        let provided = clean_id(
            workflow.get("id").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if provided.is_empty() {
            make_id(
                "wf",
                &json!({"name": name, "ts": now, "seed": workflow.get("created_at").cloned().unwrap_or(Value::Null)}),
            )
        } else {
            provided
        }
    };
    let description = clean_text(
        workflow
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        1000,
    );
    let steps = array_from_value(workflow.get("steps").unwrap_or(&json!([])), "steps")
        .iter()
        .enumerate()
        .map(|(idx, step)| normalize_workflow_step(step, idx))
        .collect::<Vec<_>>();
    let created_at = clean_text(
        workflow
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or(&now),
        80,
    );
    let updated_at = clean_text(
        workflow
            .get("updated_at")
            .and_then(Value::as_str)
            .unwrap_or(&created_at),
        80,
    );
    json!({
        "id": id,
        "name": if name.is_empty() { "workflow" } else { &name },
        "description": description,
        "steps": steps,
        "created_at": if created_at.is_empty() { &now } else { &created_at },
        "updated_at": if updated_at.is_empty() { &now } else { &updated_at },
        "last_run": workflow.get("last_run").cloned().unwrap_or(Value::Null)
    })
}

fn load_workflows(root: &Path) -> Vec<Value> {
    let path = state_path(root, WORKFLOWS_REL);
    let raw = read_json(&path).unwrap_or_else(|| json!({"workflows": []}));
    let mut rows = array_from_value(&raw, "workflows")
        .iter()
        .map(normalize_workflow)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_workflows(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, WORKFLOWS_REL),
        &json!({
            "type": "infring_dashboard_workflows",
            "updated_at": crate::now_iso(),
            "workflows": rows
        }),
    );
}

fn load_workflow_runs(root: &Path) -> Value {
    read_json(&state_path(root, WORKFLOW_RUNS_REL)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_workflow_runs",
            "updated_at": crate::now_iso(),
            "runs_by_workflow": {}
        })
    })
}

fn save_workflow_runs(root: &Path, mut state: Value) {
    state["updated_at"] = Value::String(crate::now_iso());
    write_json(&state_path(root, WORKFLOW_RUNS_REL), &state);
}

fn runs_for_workflow(state: &Value, workflow_id: &str) -> Vec<Value> {
    state
        .pointer(&format!("/runs_by_workflow/{}", clean_id(workflow_id, 120)))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn set_runs_for_workflow(state: &mut Value, workflow_id: &str, runs: Vec<Value>) {
    if !state
        .get("runs_by_workflow")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["runs_by_workflow"] = Value::Object(Map::new());
    }
    if let Some(map) = state
        .get_mut("runs_by_workflow")
        .and_then(Value::as_object_mut)
    {
        map.insert(clean_id(workflow_id, 120), Value::Array(runs));
    }
}

fn workflow_output(input: &str, workflow: &Value) -> (String, Vec<Value>) {
    let steps = workflow
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut current = clean_text(input, 10_000);
    let mut rows = Vec::<Value>::new();
    for (idx, step) in steps.iter().enumerate() {
        let name = clean_text(
            step.get("name")
                .and_then(Value::as_str)
                .unwrap_or(&format!("step-{}", idx + 1)),
            120,
        );
        let prompt = clean_text(
            step.get("prompt_template")
                .and_then(Value::as_str)
                .unwrap_or("{{input}}"),
            4000,
        );
        let rendered = if prompt.contains("{{input}}") {
            prompt.replace("{{input}}", &current)
        } else if current.is_empty() {
            prompt
        } else {
            format!("{prompt}\n\nInput:\n{current}")
        };
        let output = clean_text(&rendered, 16_000);
        rows.push(json!({
            "step": if name.is_empty() { format!("step-{}", idx + 1) } else { name },
            "output": output
        }));
        current = output;
    }
    (current, rows)
}

fn normalize_schedule(schedule: &Value) -> Value {
    if let Some(kind) = schedule.get("kind").and_then(Value::as_str) {
        if kind == "cron" {
            return json!({
                "kind": "cron",
                "expr": clean_text(schedule.get("expr").and_then(Value::as_str).unwrap_or("* * * * *"), 120)
            });
        }
        if kind == "every" {
            return json!({
                "kind": "every",
                "every_secs": as_i64(schedule.get("every_secs"), 300).max(30)
            });
        }
        if kind == "at" {
            return json!({
                "kind": "at",
                "at": clean_text(schedule.get("at").and_then(Value::as_str).unwrap_or(""), 120)
            });
        }
    }
    if let Some(expr) = schedule.get("expr").and_then(Value::as_str) {
        return json!({"kind": "cron", "expr": clean_text(expr, 120)});
    }
    if let Some(expr) = schedule.as_str() {
        return json!({"kind": "cron", "expr": clean_text(expr, 120)});
    }
    json!({"kind": "cron", "expr": "* * * * *"})
}

fn schedule_next_run(schedule: &Value) -> Value {
    let kind = clean_text(
        schedule
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("cron"),
        24,
    )
    .to_ascii_lowercase();
    if kind == "at" {
        let at = clean_text(
            schedule.get("at").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        if !at.is_empty() {
            return Value::String(at);
        }
        return Value::Null;
    }
    if kind == "every" {
        let secs = as_i64(schedule.get("every_secs"), 300).max(30);
        return Value::String((Utc::now() + Duration::seconds(secs)).to_rfc3339());
    }
    let expr = clean_text(
        schedule.get("expr").and_then(Value::as_str).unwrap_or(""),
        120,
    );
    if expr.is_empty() || expr == "* * * * *" {
        return Value::String(now_plus(1));
    }
    if expr.starts_with("*/") {
        let mins = expr
            .trim_start_matches("*/")
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(5);
        return Value::String(now_plus(mins));
    }
    if expr == "0 * * * *" {
        return Value::String(now_plus(60));
    }
    if expr.starts_with("0 */") {
        let hours = expr
            .trim_start_matches("0 */")
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1);
        return Value::String(now_plus(hours * 60));
    }
    Value::String(now_plus(15))
}
