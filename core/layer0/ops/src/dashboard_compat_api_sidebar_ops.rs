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

fn normalize_job(job: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(job.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id(
                "cron",
                &json!({"name": job.get("name").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    let schedule = normalize_schedule(job.get("schedule").unwrap_or(&json!({})));
    let name = clean_text(
        job.get("name")
            .and_then(Value::as_str)
            .unwrap_or("scheduled-job"),
        180,
    );
    let agent_id = clean_text(
        job.get("agent_id").and_then(Value::as_str).unwrap_or(""),
        140,
    );
    let action_message = clean_text(
        job.pointer("/action/message")
            .and_then(Value::as_str)
            .unwrap_or("Scheduled task execution."),
        2000,
    );
    let enabled = as_bool(job.get("enabled"), true);
    let created_at = clean_text(
        job.get("created_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let updated_at = clean_text(
        job.get("updated_at").and_then(Value::as_str).unwrap_or(""),
        80,
    );
    let last_run = job.get("last_run").cloned().unwrap_or(Value::Null);
    let next_run = if enabled {
        job.get("next_run")
            .cloned()
            .filter(|v| !v.is_null())
            .unwrap_or_else(|| schedule_next_run(&schedule))
    } else {
        Value::Null
    };
    json!({
        "id": id,
        "name": if name.is_empty() {
            "scheduled-job".to_string()
        } else {
            name.clone()
        },
        "agent_id": agent_id,
        "enabled": enabled,
        "schedule": schedule,
        "action": {
            "kind": clean_text(job.pointer("/action/kind").and_then(Value::as_str).unwrap_or("agent_turn"), 40),
            "message": action_message
        },
        "delivery": {
            "kind": clean_text(job.pointer("/delivery/kind").and_then(Value::as_str).unwrap_or("last_channel"), 40)
        },
        "run_count": as_i64(job.get("run_count"), 0).max(0),
        "last_run": last_run,
        "next_run": next_run,
        "created_at": if created_at.is_empty() { &now } else { &created_at },
        "updated_at": if updated_at.is_empty() { &now } else { &updated_at }
    })
}

fn load_jobs(root: &Path) -> Vec<Value> {
    let raw = read_json(&state_path(root, CRON_JOBS_REL)).unwrap_or_else(|| json!({"jobs": []}));
    let mut rows = array_from_value(&raw, "jobs")
        .iter()
        .map(normalize_job)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_jobs(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, CRON_JOBS_REL),
        &json!({
            "type": "infring_dashboard_cron_jobs",
            "updated_at": crate::now_iso(),
            "jobs": rows
        }),
    );
}

fn normalize_trigger(trigger: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(trigger.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id(
                "trigger",
                &json!({"agent_id": trigger.get("agent_id").cloned().unwrap_or(Value::Null), "ts": now}),
            )
        } else {
            raw
        }
    };
    json!({
        "id": id,
        "agent_id": clean_text(trigger.get("agent_id").and_then(Value::as_str).unwrap_or(""), 140),
        "pattern": trigger.get("pattern").cloned().unwrap_or_else(|| json!({"all": true})),
        "prompt_template": clean_text(trigger.get("prompt_template").and_then(Value::as_str).unwrap_or(""), 2000),
        "enabled": as_bool(trigger.get("enabled"), true),
        "fire_count": as_i64(trigger.get("fire_count"), 0).max(0),
        "max_fires": as_i64(trigger.get("max_fires"), 0).max(0),
        "created_at": clean_text(trigger.get("created_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "updated_at": clean_text(trigger.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80)
    })
}

fn load_triggers(root: &Path) -> Vec<Value> {
    let raw = read_json(&state_path(root, TRIGGERS_REL)).unwrap_or_else(|| json!([]));
    let mut rows = array_from_value(&raw, "triggers")
        .iter()
        .map(normalize_trigger)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_triggers(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, TRIGGERS_REL),
        &json!({
            "type": "infring_dashboard_triggers",
            "updated_at": crate::now_iso(),
            "triggers": rows
        }),
    );
}

fn normalize_approval(row: &Value) -> Value {
    let now = crate::now_iso();
    let id = {
        let raw = clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if raw.is_empty() {
            make_id("approval", &json!({"created_at": now}))
        } else {
            raw
        }
    };
    json!({
        "id": id,
        "action": clean_text(row.get("action").and_then(Value::as_str).unwrap_or("Sensitive action"), 180),
        "description": clean_text(row.get("description").and_then(Value::as_str).unwrap_or("Approval required before continuing."), 400),
        "agent_name": clean_text(row.get("agent_name").and_then(Value::as_str).unwrap_or("runtime"), 120),
        "status": clean_text(row.get("status").and_then(Value::as_str).unwrap_or("pending"), 40).to_ascii_lowercase(),
        "created_at": clean_text(row.get("created_at").and_then(Value::as_str).unwrap_or(&now), 80),
        "updated_at": clean_text(row.get("updated_at").and_then(Value::as_str).unwrap_or(&now), 80)
    })
}

fn load_approvals(root: &Path) -> Vec<Value> {
    let raw =
        read_json(&state_path(root, APPROVALS_REL)).unwrap_or_else(|| json!({"approvals": []}));
    let mut rows = array_from_value(&raw, "approvals")
        .iter()
        .map(normalize_approval)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("created_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows
}

fn save_approvals(root: &Path, rows: &[Value]) {
    write_json(
        &state_path(root, APPROVALS_REL),
        &json!({
            "type": "infring_dashboard_approvals",
            "updated_at": crate::now_iso(),
            "approvals": rows
        }),
    );
}

fn eyes_store_path(root: &Path) -> PathBuf {
    for rel in EYES_CATALOG_STATE_PATHS {
        let path = state_path(root, rel);
        if path.exists() {
            return path;
        }
    }
    state_path(root, EYES_CATALOG_STATE_PATHS[0])
}

fn normalize_eye(eye: &Value) -> Value {
    let now = crate::now_iso();
    let mut name = clean_text(eye.get("name").and_then(Value::as_str).unwrap_or(""), 120);
    let endpoint_url = clean_text(
        eye.get("endpoint_url")
            .and_then(Value::as_str)
            .or_else(|| eye.get("url").and_then(Value::as_str))
            .unwrap_or(""),
        500,
    );
    if name.is_empty() && !endpoint_url.is_empty() {
        name = host_from_url(&endpoint_url);
    }
    if name.is_empty() {
        name = "eye".to_string();
    }
    let id = {
        let raw = clean_id(
            eye.get("id")
                .and_then(Value::as_str)
                .unwrap_or(&name.to_ascii_lowercase()),
            120,
        );
        if raw.is_empty() {
            make_id("eye", &json!({"name": name, "url": endpoint_url}))
        } else {
            raw
        }
    };
    let status = {
        let raw = clean_text(
            eye.get("status")
                .and_then(Value::as_str)
                .unwrap_or("active"),
            24,
        )
        .to_ascii_lowercase();
        match raw.as_str() {
            "active" | "paused" | "dormant" | "disabled" => raw,
            _ => "active".to_string(),
        }
    };
    let mut topics = Vec::<String>::new();
    if let Some(rows) = eye.get("topics").and_then(Value::as_array) {
        for row in rows {
            let topic = clean_text(row.as_str().unwrap_or(""), 80);
            if !topic.is_empty() {
                topics.push(topic);
            }
        }
    } else if let Some(raw) = eye.get("topics").and_then(Value::as_str) {
        for part in raw.split(',') {
            let topic = clean_text(part, 80);
            if !topic.is_empty() {
                topics.push(topic);
            }
        }
    }
    json!({
        "uid": clean_text(eye.get("uid").and_then(Value::as_str).unwrap_or(&id), 160),
        "id": id,
        "name": name,
        "status": status,
        "endpoint_url": endpoint_url,
        "endpoint_host": clean_text(
            eye.get("endpoint_host")
                .and_then(Value::as_str)
                .unwrap_or(&host_from_url(
                    eye.get("endpoint_url")
                        .and_then(Value::as_str)
                        .or_else(|| eye.get("url").and_then(Value::as_str))
                        .unwrap_or("")
                )),
            120
        ),
        "api_key_present": as_bool(eye.get("api_key_present"), eye.get("api_key_hash").is_some()),
        "api_key_hash": clean_text(eye.get("api_key_hash").and_then(Value::as_str).unwrap_or(""), 160),
        "cadence_hours": as_i64(eye.get("cadence_hours"), 4).clamp(1, 168),
        "topics": topics,
        "updated_ts": clean_text(
            eye.get("updated_ts")
                .and_then(Value::as_str)
                .or_else(|| eye.get("updated_at").and_then(Value::as_str))
                .unwrap_or(&now),
            80
        ),
        "source": clean_text(eye.get("source").and_then(Value::as_str).unwrap_or("system"), 40)
    })
}

fn load_eyes(root: &Path) -> Vec<Value> {
    let raw = read_json(&eyes_store_path(root)).unwrap_or_else(|| json!({"eyes": []}));
    let mut rows =
        if let Some(catalog_rows) = raw.pointer("/catalog/eyes").and_then(Value::as_array) {
            catalog_rows.clone()
        } else {
            array_from_value(&raw, "eyes")
        };
    if rows.is_empty() && raw.is_array() {
        rows = raw.as_array().cloned().unwrap_or_default();
    }
    let mut normalized = rows.iter().map(normalize_eye).collect::<Vec<_>>();
    normalized.sort_by(|a, b| {
        clean_text(a.get("name").and_then(Value::as_str).unwrap_or(""), 120).cmp(&clean_text(
            b.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        ))
    });
    normalized
}

fn save_eyes(root: &Path, eyes: &[Value]) {
    write_json(
        &eyes_store_path(root),
        &json!({
            "type": "eyes_catalog",
            "updated_at": crate::now_iso(),
            "eyes": eyes
        }),
    );
}

fn workflow_path_segments(path_only: &str) -> Option<Vec<String>> {
    if path_only == "/api/workflows" {
        return Some(Vec::new());
    }
    if let Some(rest) = path_only.strip_prefix("/api/workflows/") {
        let segs = rest
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        return Some(segs);
    }
    None
}

pub fn handle(
    root: &Path,
    method: &str,
    path_only: &str,
    body: &[u8],
    _snapshot: &Value,
) -> Option<CompatApiResponse> {
    if let Some(segments) = workflow_path_segments(path_only) {
        let mut workflows = load_workflows(root);
        if method == "GET" && segments.is_empty() {
            return Some(CompatApiResponse {
                status: 200,
                payload: Value::Array(workflows),
            });
        }
        if !segments.is_empty() {
            let workflow_id = clean_id(&segments[0], 120);
            if workflow_id.is_empty() {
                return Some(CompatApiResponse {
                    status: 400,
                    payload: json!({"ok": false, "error": "workflow_id_required"}),
                });
            }
            if method == "GET" && segments.len() == 1 {
                if let Some(found) = workflows
                    .iter()
                    .find(|row| {
                        clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                            == workflow_id
                    })
                    .cloned()
                {
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: found,
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "GET" && segments.len() == 2 && segments[1] == "runs" {
                let runs_state = load_workflow_runs(root);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({
                        "ok": true,
                        "workflow_id": workflow_id,
                        "runs": runs_for_workflow(&runs_state, &workflow_id)
                    }),
                });
            }
            if method == "POST" && segments.len() == 2 && segments[1] == "run" {
                if let Some(idx) = workflows.iter().position(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                        == workflow_id
                }) {
                    let request = parse_json(body);
                    let input = clean_text(
                        request.get("input").and_then(Value::as_str).unwrap_or(""),
                        10_000,
                    );
                    let started = Utc::now();
                    let (output, step_rows) = workflow_output(&input, &workflows[idx]);
                    let finished = Utc::now();
                    workflows[idx]["updated_at"] = Value::String(crate::now_iso());
                    workflows[idx]["last_run"] = Value::String(crate::now_iso());
                    save_workflows(root, &workflows);
                    let mut runs_state = load_workflow_runs(root);
                    let mut runs = runs_for_workflow(&runs_state, &workflow_id);
                    let run_id = make_id(
                        "run",
                        &json!({"workflow_id": workflow_id, "ts": crate::now_iso(), "input": input}),
                    );
                    let run = json!({
                        "run_id": run_id,
                        "workflow_id": workflow_id,
                        "status": "completed",
                        "input": input,
                        "output": output,
                        "steps": step_rows,
                        "started_at": started.to_rfc3339(),
                        "finished_at": finished.to_rfc3339(),
                        "duration_ms": (finished - started).num_milliseconds().max(1)
                    });
                    runs.push(run.clone());
                    if runs.len() > 200 {
                        let keep_from = runs.len().saturating_sub(200);
                        runs = runs.into_iter().skip(keep_from).collect::<Vec<_>>();
                    }
                    set_runs_for_workflow(&mut runs_state, &workflow_id, runs);
                    save_workflow_runs(root, runs_state);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({
                            "ok": true,
                            "status": "completed",
                            "workflow_id": workflow_id,
                            "run_id": run["run_id"].clone(),
                            "output": run["output"].clone(),
                            "run": run
                        }),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "PUT" && segments.len() == 1 {
                if let Some(idx) = workflows.iter().position(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                        == workflow_id
                }) {
                    let request = parse_json(body);
                    let mut merged = workflows[idx].clone();
                    if request.get("name").and_then(Value::as_str).is_some() {
                        merged["name"] = Value::String(clean_text(
                            request
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("workflow"),
                            160,
                        ));
                    }
                    if request.get("description").and_then(Value::as_str).is_some() {
                        merged["description"] = Value::String(clean_text(
                            request
                                .get("description")
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            1000,
                        ));
                    }
                    if request.get("steps").is_some() {
                        merged["steps"] = Value::Array(
                            array_from_value(request.get("steps").unwrap_or(&json!([])), "steps")
                                .iter()
                                .enumerate()
                                .map(|(step_idx, step)| normalize_workflow_step(step, step_idx))
                                .collect::<Vec<_>>(),
                        );
                    }
                    merged["id"] = Value::String(workflow_id.clone());
                    merged["updated_at"] = Value::String(crate::now_iso());
                    workflows[idx] = normalize_workflow(&merged);
                    save_workflows(root, &workflows);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "workflow": workflows[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "workflow_not_found"}),
                });
            }
            if method == "DELETE" && segments.len() == 1 {
                let before = workflows.len();
                workflows.retain(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                        != workflow_id
                });
                if workflows.len() == before {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "workflow_not_found"}),
                    });
                }
                save_workflows(root, &workflows);
                let mut runs_state = load_workflow_runs(root);
                set_runs_for_workflow(&mut runs_state, &workflow_id, Vec::new());
                save_workflow_runs(root, runs_state);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "workflow_id": workflow_id}),
                });
            }
        }
        if method == "POST" && segments.is_empty() {
            let request = parse_json(body);
            let mut workflow = normalize_workflow(&request);
            if workflow
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                workflow["id"] = Value::String(make_id(
                    "wf",
                    &json!({"name": workflow.get("name").cloned().unwrap_or(Value::Null), "ts": crate::now_iso()}),
                ));
            }
            workflow["created_at"] = Value::String(crate::now_iso());
            workflow["updated_at"] = Value::String(crate::now_iso());
            workflows.push(workflow.clone());
            save_workflows(root, &workflows);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "workflow": workflow}),
            });
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if method == "GET" && path_only == "/api/cron/jobs" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "jobs": load_jobs(root)}),
        });
    }

    if method == "POST" && path_only == "/api/cron/jobs" {
        let request = parse_json(body);
        let mut jobs = load_jobs(root);
        let mut row = normalize_job(&request);
        if row
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            row["id"] = Value::String(make_id(
                "cron",
                &json!({"name": row.get("name").cloned().unwrap_or(Value::Null), "ts": crate::now_iso()}),
            ));
        }
        row["created_at"] = Value::String(crate::now_iso());
        row["updated_at"] = Value::String(crate::now_iso());
        row["next_run"] = if as_bool(row.get("enabled"), true) {
            schedule_next_run(row.get("schedule").unwrap_or(&json!({})))
        } else {
            Value::Null
        };
        jobs.push(row.clone());
        save_jobs(root, &jobs);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "job": row}),
        });
    }

    if path_only.starts_with("/api/cron/jobs/") {
        let tail = path_only.trim_start_matches("/api/cron/jobs/");
        let segments = tail
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        if !segments.is_empty() {
            let job_id = clean_id(&segments[0], 120);
            let mut jobs = load_jobs(root);
            if method == "PUT" && segments.len() == 2 && segments[1] == "enable" {
                let request = parse_json(body);
                if let Some(idx) = jobs.iter().position(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) == job_id
                }) {
                    let enabled = as_bool(request.get("enabled"), true);
                    jobs[idx]["enabled"] = Value::Bool(enabled);
                    jobs[idx]["updated_at"] = Value::String(crate::now_iso());
                    jobs[idx]["next_run"] = if enabled {
                        schedule_next_run(jobs[idx].get("schedule").unwrap_or(&json!({})))
                    } else {
                        Value::Null
                    };
                    save_jobs(root, &jobs);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "job": jobs[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "job_not_found"}),
                });
            }
            if method == "DELETE" && segments.len() == 1 {
                let before = jobs.len();
                jobs.retain(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) != job_id
                });
                if before == jobs.len() {
                    return Some(CompatApiResponse {
                        status: 404,
                        payload: json!({"ok": false, "error": "job_not_found"}),
                    });
                }
                save_jobs(root, &jobs);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "deleted": true, "job_id": job_id}),
                });
            }
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if path_only.starts_with("/api/schedules/") && method == "POST" && path_only.ends_with("/run") {
        let job_id = clean_id(
            path_only
                .trim_start_matches("/api/schedules/")
                .trim_end_matches("/run")
                .trim_matches('/'),
            120,
        );
        if job_id.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "job_id_required"}),
            });
        }
        let mut jobs = load_jobs(root);
        if let Some(idx) = jobs.iter().position(|row| {
            clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) == job_id
        }) {
            let ran_at = crate::now_iso();
            jobs[idx]["last_run"] = Value::String(ran_at.clone());
            jobs[idx]["updated_at"] = Value::String(ran_at.clone());
            jobs[idx]["run_count"] = Value::from(as_i64(jobs[idx].get("run_count"), 0).max(0) + 1);
            jobs[idx]["next_run"] = if as_bool(jobs[idx].get("enabled"), true) {
                schedule_next_run(jobs[idx].get("schedule").unwrap_or(&json!({})))
            } else {
                Value::Null
            };
            let agent_id = clean_text(
                jobs[idx]
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                140,
            );
            if !agent_id.is_empty() {
                let user_text = clean_text(
                    jobs[idx]
                        .pointer("/action/message")
                        .and_then(Value::as_str)
                        .unwrap_or("Scheduled task executed."),
                    2000,
                );
                let assistant_text = "Scheduled execution logged by Rust core.";
                let _ = crate::dashboard_agent_state::append_turn(
                    root,
                    &agent_id,
                    &user_text,
                    assistant_text,
                );
            }
            save_jobs(root, &jobs);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({
                    "ok": true,
                    "status": "completed",
                    "job_id": job_id,
                    "ran_at": ran_at
                }),
            });
        }
        return Some(CompatApiResponse {
            status: 404,
            payload: json!({"ok": false, "status": "failed", "error": "job_not_found"}),
        });
    }

    if path_only == "/api/triggers" && method == "GET" {
        return Some(CompatApiResponse {
            status: 200,
            payload: Value::Array(load_triggers(root)),
        });
    }

    if path_only.starts_with("/api/triggers/") {
        let trigger_id = clean_id(path_only.trim_start_matches("/api/triggers/"), 120);
        let mut triggers = load_triggers(root);
        if method == "PUT" {
            let request = parse_json(body);
            if let Some(idx) = triggers.iter().position(|row| {
                clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) == trigger_id
            }) {
                if request.get("enabled").is_some() {
                    triggers[idx]["enabled"] = Value::Bool(as_bool(request.get("enabled"), true));
                }
                triggers[idx]["updated_at"] = Value::String(crate::now_iso());
                save_triggers(root, &triggers);
                return Some(CompatApiResponse {
                    status: 200,
                    payload: json!({"ok": true, "trigger": triggers[idx].clone()}),
                });
            }
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({"ok": false, "error": "trigger_not_found"}),
            });
        }
        if method == "DELETE" {
            let before = triggers.len();
            triggers.retain(|row| {
                clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120) != trigger_id
            });
            if before == triggers.len() {
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "trigger_not_found"}),
                });
            }
            save_triggers(root, &triggers);
            return Some(CompatApiResponse {
                status: 200,
                payload: json!({"ok": true, "deleted": true, "trigger_id": trigger_id}),
            });
        }
        return Some(CompatApiResponse {
            status: 405,
            payload: json!({"ok": false, "error": "method_not_allowed"}),
        });
    }

    if path_only == "/api/approvals" && method == "GET" {
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "approvals": load_approvals(root)}),
        });
    }

    if path_only.starts_with("/api/approvals/") && method == "POST" {
        let tail = path_only.trim_start_matches("/api/approvals/");
        let segments = tail
            .split('/')
            .filter_map(|v| {
                let cleaned = clean_text(v, 200);
                if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                }
            })
            .collect::<Vec<_>>();
        if segments.len() == 2 {
            let approval_id = clean_id(&segments[0], 120);
            let action = clean_id(&segments[1], 40);
            if action == "approve" || action == "reject" {
                let mut approvals = load_approvals(root);
                if let Some(idx) = approvals.iter().position(|row| {
                    clean_id(row.get("id").and_then(Value::as_str).unwrap_or(""), 120)
                        == approval_id
                }) {
                    approvals[idx]["status"] = Value::String(if action == "approve" {
                        "approved".to_string()
                    } else {
                        "rejected".to_string()
                    });
                    approvals[idx]["updated_at"] = Value::String(crate::now_iso());
                    save_approvals(root, &approvals);
                    return Some(CompatApiResponse {
                        status: 200,
                        payload: json!({"ok": true, "approval": approvals[idx].clone()}),
                    });
                }
                return Some(CompatApiResponse {
                    status: 404,
                    payload: json!({"ok": false, "error": "approval_not_found"}),
                });
            }
        }
        return Some(CompatApiResponse {
            status: 400,
            payload: json!({"ok": false, "error": "invalid_approval_route"}),
        });
    }

    if path_only == "/api/eyes" && method == "GET" {
        let eyes = load_eyes(root);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({
                "ok": true,
                "eyes": eyes,
                "catalog": {"eyes": eyes}
            }),
        });
    }

    if path_only == "/api/eyes" && method == "POST" {
        let request = parse_json(body);
        let name = clean_text(
            request.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        );
        let url = clean_text(
            request.get("url").and_then(Value::as_str).unwrap_or(""),
            500,
        );
        let api_key = clean_text(
            request.get("api_key").and_then(Value::as_str).unwrap_or(""),
            4000,
        );
        if name.is_empty() && url.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "name_or_url_required"}),
            });
        }
        if url.is_empty() && api_key.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({"ok": false, "error": "url_or_api_key_required"}),
            });
        }
        let mut eyes = load_eyes(root);
        let canonical_name = if name.is_empty() {
            host_from_url(&url)
        } else {
            name
        };
        let mut id = clean_id(request.get("id").and_then(Value::as_str).unwrap_or(""), 120);
        if id.is_empty() {
            id = clean_id(&canonical_name, 120);
        }
        if id.is_empty() {
            id = make_id(
                "eye",
                &json!({"name": canonical_name, "url": url, "ts": crate::now_iso()}),
            );
        }
        let now = crate::now_iso();
        let topics = clean_text(
            request.get("topics").and_then(Value::as_str).unwrap_or(""),
            600,
        );
        let topic_rows = topics
            .split(',')
            .filter_map(|v| {
                let t = clean_text(v, 80);
                if t.is_empty() {
                    None
                } else {
                    Some(Value::String(t))
                }
            })
            .collect::<Vec<_>>();
        let status = {
            let raw = clean_text(
                request
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("active"),
                24,
            )
            .to_ascii_lowercase();
            match raw.as_str() {
                "active" | "paused" | "dormant" | "disabled" => raw,
                _ => "active".to_string(),
            }
        };
        let cadence = as_i64(request.get("cadence_hours"), 4).clamp(1, 168);
        let mut row = json!({
            "uid": make_id("eyeuid", &json!({"id": id, "ts": now})),
            "id": id,
            "name": if canonical_name.is_empty() { "eye" } else { &canonical_name },
            "status": status,
            "endpoint_url": url,
            "endpoint_host": host_from_url(request.get("url").and_then(Value::as_str).unwrap_or("")),
            "api_key_present": !api_key.is_empty(),
            "api_key_hash": if api_key.is_empty() {
                Value::Null
            } else {
                Value::String(crate::deterministic_receipt_hash(&json!({"id": id, "api_key": api_key})))
            },
            "cadence_hours": cadence,
            "topics": topic_rows,
            "updated_ts": now,
            "source": "manual"
        });
        row = normalize_eye(&row);
        let existing_idx = eyes.iter().position(|eye| {
            clean_id(eye.get("id").and_then(Value::as_str).unwrap_or(""), 120) == id
                || clean_text(eye.get("name").and_then(Value::as_str).unwrap_or(""), 120)
                    .eq_ignore_ascii_case(row.get("name").and_then(Value::as_str).unwrap_or(""))
        });
        let created = if let Some(idx) = existing_idx {
            let mut merged = eyes[idx].clone();
            merged["name"] = row["name"].clone();
            merged["status"] = row["status"].clone();
            if !row["endpoint_url"].as_str().unwrap_or("").is_empty() {
                merged["endpoint_url"] = row["endpoint_url"].clone();
                merged["endpoint_host"] = row["endpoint_host"].clone();
            }
            if row.get("api_key_present").and_then(Value::as_bool) == Some(true) {
                merged["api_key_present"] = Value::Bool(true);
                merged["api_key_hash"] = row["api_key_hash"].clone();
            }
            merged["cadence_hours"] = row["cadence_hours"].clone();
            merged["topics"] = row["topics"].clone();
            merged["updated_ts"] = Value::String(crate::now_iso());
            eyes[idx] = normalize_eye(&merged);
            false
        } else {
            eyes.push(row.clone());
            true
        };
        save_eyes(root, &eyes);
        let eye = eyes
            .iter()
            .find(|eye| clean_id(eye.get("id").and_then(Value::as_str).unwrap_or(""), 120) == id)
            .cloned()
            .unwrap_or(row);
        return Some(CompatApiResponse {
            status: 200,
            payload: json!({"ok": true, "created": created, "eye": eye}),
        });
    }

    None
}
