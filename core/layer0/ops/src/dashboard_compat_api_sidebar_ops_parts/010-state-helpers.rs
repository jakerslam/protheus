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
        .iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(id.clone()) } else { None })
        .collect::<Vec<_>>();
    if roots.is_empty() {
        roots.push(workflow_step_id(&steps[0], 0));
    }

    let mut visited = BTreeSet::<String>::new();
    let mut stack = roots.clone();
    while let Some(current) = stack.pop() {
        if current.is_empty() || !visited.insert(current.clone()) {
            continue;
        }
        if let Some(children) = adjacency.get(&current) {
            for child in children {
                stack.push(child.clone());
            }
        }
    }
    let mut unreachable = ids
        .keys()
        .filter(|id| !visited.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    unreachable.sort();
    if !unreachable.is_empty() {
        errors.push(format!("unreachable_steps:{}", unreachable.join(",")));
    }

    let mut visiting = BTreeSet::<String>::new();
    let mut visited_cycle = BTreeSet::<String>::new();
    let mut cycle_edges = Vec::<(String, String)>::new();

    fn dfs_cycle(
        node: &str,
        adjacency: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        cycle_edges: &mut Vec<(String, String)>,
    ) {
        if visited.contains(node) {
            return;
        }
        visiting.insert(node.to_string());
        if let Some(children) = adjacency.get(node) {
            for child in children {
                if visiting.contains(child) {
                    cycle_edges.push((node.to_string(), child.to_string()));
                    continue;
                }
                dfs_cycle(child, adjacency, visiting, visited, cycle_edges);
            }
        }
        visiting.remove(node);
        visited.insert(node.to_string());
    }

    for id in ids.keys() {
        dfs_cycle(
            id,
            &adjacency,
            &mut visiting,
            &mut visited_cycle,
            &mut cycle_edges,
        );
    }
    cycle_edges.sort();
    cycle_edges.dedup();
    for (from, to) in &cycle_edges {
        if mode_or_sequential(&modes, from) != "loop" {
            errors.push(format!("cycle_without_loop:{}->{}", from, to));
        } else {
            warnings.push(format!("loop_cycle:{}->{}", from, to));
        }
    }

    let mut topo_indegree = indegree.clone();
    for (from, to) in &cycle_edges {
        if mode_or_sequential(&modes, from) == "loop" {
            if let Some(degree) = topo_indegree.get_mut(to) {
                *degree = degree.saturating_sub(1);
            }
        }
    }
    let mut topo_queue = topo_indegree
        .iter()
        .filter_map(|(id, degree)| if *degree == 0 { Some(id.clone()) } else { None })
        .collect::<Vec<_>>();
    topo_queue.sort();
    let mut topo = Vec::<String>::new();
    while let Some(current) = topo_queue.pop() {
        topo.push(current.clone());
        if let Some(children) = adjacency.get(&current) {
            for child in children {
                let is_loop_edge = cycle_edges
                    .iter()
                    .any(|(from, to)| from == &current && to == child)
                    && mode_or_sequential(&modes, &current) == "loop";
                if is_loop_edge {
                    continue;
                }
                if let Some(degree) = topo_indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        topo_queue.push(child.clone());
                    }
                }
            }
        }
    }
    if topo.len() != ids.len() {
        warnings.push("topological_order_partial_due_to_cycle".to_string());
    }

    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();
    terminals.sort();
    terminals.dedup();

    json!({
        "ok": true,
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": warnings,
        "stats": {
            "steps": ids.len(),
            "edges": edges.len(),
            "roots": roots,
            "terminal_nodes": terminals,
            "cycles": cycle_edges.len(),
            "unreachable_nodes": unreachable
        },
        "topological_order": topo
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

fn summarize_evicted_runs(evicted: &[Value]) -> Value {
    if evicted.is_empty() {
        return Value::Null;
    }
    let mut status_counts = BTreeMap::<String, i64>::new();
    let mut duration_total = 0i64;
    let mut duration_count = 0i64;
    let mut samples = Vec::<String>::new();
    for row in evicted {
        let status = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            40,
        );
        *status_counts.entry(status).or_insert(0) += 1;
        if let Some(ms) = row.get("duration_ms").and_then(Value::as_i64) {
            if ms > 0 {
                duration_total += ms;
                duration_count += 1;
            }
        }
        if samples.len() < 2 {
            let sample = clean_text(row.get("output").and_then(Value::as_str).unwrap_or(""), 220);
            if !sample.is_empty() {
                samples.push(sample);
            }
        }
    }
    let status_json = status_counts
        .into_iter()
        .map(|(status, count)| (status, json!(count)))
        .collect::<Map<String, Value>>();
    json!({
        "keyframe_id": make_id("wf-kf", &json!({"ts": crate::now_iso(), "size": evicted.len()})),
        "created_at": crate::now_iso(),
        "evicted_runs": evicted.len(),
        "status_counts": Value::Object(status_json),
        "avg_duration_ms": if duration_count > 0 { duration_total / duration_count } else { 0 },
        "sample_outputs": samples
    })
}

fn set_runs_for_workflow(state: &mut Value, workflow_id: &str, mut runs: Vec<Value>) {
    let mut evicted = Vec::<Value>::new();
    if runs.len() > 200 {
        let keep_from = runs.len().saturating_sub(200);
        evicted = runs.iter().take(keep_from).cloned().collect::<Vec<_>>();
        runs = runs.into_iter().skip(keep_from).collect::<Vec<_>>();
    }
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

    let keyframe = summarize_evicted_runs(&evicted);
    if keyframe.is_null() {
        return;
    }
    if !state
        .get("keyframes_by_workflow")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["keyframes_by_workflow"] = Value::Object(Map::new());
    }
    if let Some(map) = state
        .get_mut("keyframes_by_workflow")
        .and_then(Value::as_object_mut)
    {
        let key = clean_id(workflow_id, 120);
        let mut rows = map
            .get(&key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rows.push(keyframe);
        if rows.len() > 50 {
            let keep_from = rows.len().saturating_sub(50);
            rows = rows.into_iter().skip(keep_from).collect::<Vec<_>>();
        }
        map.insert(key, Value::Array(rows));
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
