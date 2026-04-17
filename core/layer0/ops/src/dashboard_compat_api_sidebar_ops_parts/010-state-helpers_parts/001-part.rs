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
