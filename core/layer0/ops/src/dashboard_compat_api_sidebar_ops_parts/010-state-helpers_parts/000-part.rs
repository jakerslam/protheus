// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
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

fn workflow_target(value: Option<&Value>) -> String {
    clean_id(value.and_then(Value::as_str).unwrap_or(""), 120)
}

fn workflow_targets(value: Option<&Value>) -> Vec<String> {
    let mut out = Vec::<String>::new();
    if let Some(rows) = value.and_then(Value::as_array) {
        for row in rows {
            let target = clean_id(row.as_str().unwrap_or(""), 120);
            if !target.is_empty() && !out.iter().any(|existing| existing == &target) {
                out.push(target);
            }
        }
        return out;
    }
    if let Some(raw) = value.and_then(Value::as_str) {
        for piece in raw.split(',') {
            let target = clean_id(piece, 120);
            if !target.is_empty() && !out.iter().any(|existing| existing == &target) {
                out.push(target);
            }
        }
    }
    out
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
        "prompt_template": if prompt_template.is_empty() { "{{input}}" } else { &prompt_template },
        "next": workflow_target(step.get("next")),
        "next_true": workflow_target(step.get("next_true")),
        "next_false": workflow_target(step.get("next_false")),
        "fan_targets": workflow_targets(step.get("fan_targets"))
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

fn workflow_step_id(step: &Value, idx: usize) -> String {
    clean_id(
        step.get("id")
            .and_then(Value::as_str)
            .unwrap_or(&format!("step-{}", idx + 1)),
        120,
    )
}

fn workflow_raw_targets(step: &Value, steps: &[Value], idx: usize, mode: &str) -> Vec<String> {
    let mut targets = Vec::<String>::new();
    match mode {
        "conditional" => {
            for key in ["next_true", "next_false"] {
                let target = workflow_target(step.get(key));
                if !target.is_empty() {
                    targets.push(target);
                }
            }
        }
        "fan_out" => {
            targets.extend(workflow_targets(step.get("fan_targets")));
            let next = workflow_target(step.get("next"));
            if !next.is_empty() {
                targets.push(next);
            }
        }
        _ => {
            let next = workflow_target(step.get("next"));
            if !next.is_empty() {
                targets.push(next);
            } else if let Some(next_step) = steps.get(idx + 1) {
                let fallback = workflow_step_id(next_step, idx + 1);
                if !fallback.is_empty() {
                    targets.push(fallback);
                }
            }
        }
    }
    targets.dedup();
    targets
}

fn workflow_step_ids(steps: &[Value]) -> BTreeMap<String, usize> {
    let mut ids = BTreeMap::<String, usize>::new();
    for (idx, step) in steps.iter().enumerate() {
        let step_id = workflow_step_id(step, idx);
        if step_id.is_empty() {
            continue;
        }
        ids.insert(step_id, idx);
    }
    ids
}

fn workflow_reference_index(steps: &[Value]) -> BTreeMap<String, String> {
    let mut refs = BTreeMap::<String, String>::new();
    for (idx, step) in steps.iter().enumerate() {
        let step_id = workflow_step_id(step, idx);
        if step_id.is_empty() {
            continue;
        }
        refs.insert(step_id.clone(), step_id.clone());
        let by_name = clean_id(step.get("name").and_then(Value::as_str).unwrap_or(""), 120);
        if !by_name.is_empty() {
            refs.insert(by_name, step_id);
        }
    }
    refs
}

fn resolve_workflow_target_id(raw_target: &str, refs: &BTreeMap<String, String>) -> Option<String> {
    let cleaned = clean_id(raw_target, 120);
    (!cleaned.is_empty())
        .then(|| refs.get(&cleaned).cloned())
        .flatten()
}

fn mode_or_sequential<'a>(modes: &'a BTreeMap<String, String>, step_id: &str) -> &'a str {
    modes
        .get(step_id)
        .map(String::as_str)
        .unwrap_or("sequential")
}

pub fn validate_workflow_graph(workflow: &Value) -> Value {
    let steps = workflow
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return json!({
            "ok": true,
            "valid": false,
            "errors": ["workflow_steps_required"],
            "warnings": [],
            "stats": {
                "steps": 0,
                "edges": 0,
                "roots": [],
                "terminal_nodes": [],
                "cycles": 0,
                "unreachable_nodes": []
            },
            "topological_order": []
        });
    }

    let ids = workflow_step_ids(&steps);
    let refs = workflow_reference_index(&steps);
    let mut errors = Vec::<String>::new();
    let mut warnings = Vec::<String>::new();
    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut modes = BTreeMap::<String, String>::new();
    let mut terminals = Vec::<String>::new();
    let mut edges = Vec::<(String, String)>::new();

    for (idx, step) in steps.iter().enumerate() {
        let step_id = workflow_step_id(step, idx);
        if step_id.is_empty() {
            continue;
        }
        let mode = clean_text(
            step.get("mode")
                .and_then(Value::as_str)
                .unwrap_or("sequential"),
            40,
        )
        .to_ascii_lowercase();
        modes.insert(step_id.clone(), mode.clone());
        indegree.entry(step_id.clone()).or_insert(0);

        let raw_targets = workflow_raw_targets(step, &steps, idx, mode.as_str());
        if raw_targets.is_empty() {
            terminals.push(step_id.clone());
            adjacency.insert(step_id.clone(), Vec::new());
            continue;
        }
        let mut resolved = Vec::<String>::new();
        for target in raw_targets {
            let Some(target_id) = resolve_workflow_target_id(&target, &refs) else {
                errors.push(format!("unknown_target:{}->{}", step_id, target));
                continue;
            };
            if !ids.contains_key(&target_id) {
                errors.push(format!("target_not_found:{}->{}", step_id, target_id));
                continue;
            }
            if !resolved.iter().any(|existing| existing == &target_id) {
                resolved.push(target_id.clone());
                edges.push((step_id.clone(), target_id.clone()));
                *indegree.entry(target_id).or_insert(0) += 1;
            }
        }
        adjacency.insert(step_id, resolved);
    }

    if terminals.is_empty() {
        warnings.push("no_terminal_step_detected".to_string());
    }

    let mut roots = indegree
