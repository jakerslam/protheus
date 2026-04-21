
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
