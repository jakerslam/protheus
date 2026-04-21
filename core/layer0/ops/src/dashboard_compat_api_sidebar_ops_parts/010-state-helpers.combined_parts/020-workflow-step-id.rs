
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
