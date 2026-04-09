fn run_route_model(policy: &Value, payload: &Value, policy_path: &Path) -> Value {
    let tags = norm_tags(payload.get("tags"));
    let default_model = clean_text(
        payload
            .get("default")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_MODEL),
        240,
    );
    let route = route_model_with_policy(policy, &tags, &default_model);
    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_route_model",
        "policy_path": policy_path.to_string_lossy().to_string(),
        "tags": tags,
        "default_model": default_model,
        "model": route.get("model").cloned().unwrap_or(Value::Null),
        "tier": route.get("tier").cloned().unwrap_or(Value::Null),
        "matched_rule_index": route.get("matched_rule_index").cloned().unwrap_or(Value::Null),
        "matched_default_rule": route.get("matched_default_rule").cloned().unwrap_or(Value::Null),
    }))
}

fn run_escalate_model(policy: &Value, payload: &Value, policy_path: &Path) -> Value {
    let tags = norm_tags(payload.get("tags"));
    let default_model = clean_text(
        payload
            .get("default")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_MODEL),
        240,
    );
    let route = route_model_with_policy(policy, &tags, &default_model);
    let base_model = clean_text(
        route
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_MODEL),
        240,
    );
    let base_tier = tier_for_model(policy, &base_model).unwrap_or_else(|| {
        clean_text(
            route.get("tier").and_then(Value::as_str).unwrap_or("tier2"),
            40,
        )
    });
    let tag_set = tags.iter().cloned().collect::<HashSet<_>>();
    let high_risk = tag_set
        .iter()
        .any(|tag| high_risk_tags().contains(tag.as_str()));

    let mut chain = Vec::<String>::new();
    if high_risk {
        let tier1 = first_model_for_tier(policy, "tier1", &base_model);
        if !tier1.is_empty() {
            chain.push(tier1);
        }
    } else {
        let ordered_tiers = match base_tier.as_str() {
            "tier3" => vec!["tier3", "tier2", "tier1"],
            "tier2" => vec!["tier2", "tier1"],
            _ => vec!["tier1"],
        };
        for tier in ordered_tiers {
            let model = first_model_for_tier(policy, tier, &base_model);
            if !model.is_empty() && !chain.iter().any(|row| row == &model) {
                chain.push(model);
            }
        }
    }
    if chain.is_empty() {
        chain.push(base_model.clone());
    }

    with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_escalate_model",
        "policy_path": policy_path.to_string_lossy().to_string(),
        "tags": tags,
        "baseTier": base_tier,
        "baseModel": base_model,
        "modelChain": chain,
    }))
}

fn run_plan_auto(payload: &Value) -> Result<Value, String> {
    let task = clean_text(
        payload.get("task").and_then(Value::as_str).unwrap_or(""),
        240,
    );
    if task.is_empty() {
        return Err("task_required".to_string());
    }
    let tags = norm_tags(payload.get("tags"));
    let high_risk = tags
        .iter()
        .any(|tag| high_risk_tags().contains(tag.as_str()));
    let steps = if high_risk {
        vec![
            "Identify exact files/commands involved and current state",
            "Backup any files that will be modified before applying changes",
            "Apply the smallest safe change that satisfies the goal",
            "If checks fail, rollback with the recorded backup path",
            "Run verification checks and summarize outcomes with evidence",
        ]
    } else {
        vec![
            "Identify exact files/commands involved and current state",
            "Apply the smallest safe change that satisfies the goal",
            "Run verification checks and summarize outcomes with evidence",
        ]
    };
    let assumptions = vec![
        "Task scope is limited to the stated request",
        "No secrets or credentials will be printed to console",
    ];
    let risks = vec![
        "Unexpected side-effects if task modifies config/files",
        "Insufficient context may cause incorrect assumptions",
    ];

    let plan = json!({
        "goal": task,
        "assumptions": assumptions,
        "risks": risks,
        "steps": steps,
        "needs_user_input": if high_risk {
            Value::Array(vec![])
        } else {
            json!(["Confirm ambiguous requirements before irreversible changes"])
        },
        "tags": if tags.len() >= 3 {
            Value::Array(tags.into_iter().map(Value::String).collect::<Vec<_>>())
        } else {
            json!(["planning", "safety", "review"])
        },
        "meta": {
            "generatedAtUtc": crate::now_iso(),
            "mode": "auto-minimal"
        }
    });

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_plan_auto",
        "payload": {
            "task": payload.get("task").cloned().unwrap_or(Value::Null),
            "tags": payload.get("tags").cloned().unwrap_or_else(|| json!([])),
            "plan": plan
        }
    })))
}

fn list_from(plan: &Value, key: &str) -> Vec<String> {
    plan.get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 400))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn run_plan_validate(payload: &Value) -> Result<Value, String> {
    let plan = if payload.get("goal").is_some() {
        payload
    } else {
        payload.get("plan").unwrap_or(&Value::Null)
    };
    if !plan.is_object() {
        return Err("plan_object_required".to_string());
    }
    let goal = clean_text(plan.get("goal").and_then(Value::as_str).unwrap_or(""), 240);
    if goal.is_empty() {
        return Err("plan_goal_required".to_string());
    }
    let assumptions = list_from(plan, "assumptions");
    let risks = list_from(plan, "risks");
    let steps = list_from(plan, "steps");
    let tags = list_from(plan, "tags")
        .into_iter()
        .map(|row| row.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if assumptions.len() < 2 {
        return Err("plan_assumptions_min_2".to_string());
    }
    if risks.len() < 2 {
        return Err("plan_risks_min_2".to_string());
    }
    if steps.len() < 3 {
        return Err("plan_steps_min_3".to_string());
    }

    let weak_prefixes = ["think", "consider", "review generally", "brainstorm"];
    let weak_steps = steps
        .iter()
        .filter(|step| {
            let lowered = step.to_ascii_lowercase();
            weak_prefixes
                .iter()
                .any(|prefix| lowered.starts_with(prefix))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !weak_steps.is_empty() {
        return Err("plan_steps_non_actionable".to_string());
    }

    let requires_rollback = tags
        .iter()
        .any(|tag| high_risk_tags().contains(tag.as_str()));
    if requires_rollback {
        let has_rollback = steps.iter().any(|step| {
            let lowered = step.to_ascii_lowercase();
            lowered.contains("rollback") || lowered.contains("revert")
        });
        if !has_rollback {
            return Err("plan_high_risk_requires_rollback".to_string());
        }
    }

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_plan_validate",
        "goal": goal,
        "steps_count": steps.len(),
        "assumptions_count": assumptions.len(),
        "risks_count": risks.len(),
        "requires_rollback": requires_rollback,
    })))
}

fn run_postflight_validate(payload: &Value) -> Result<Value, String> {
    if !payload.is_object() {
        return Err("postflight_object_required".to_string());
    }
    let required = [
        "files_touched",
        "commands_run",
        "routing_tags",
        "model_used",
        "result_summary",
    ];
    let missing = required
        .iter()
        .filter(|key| payload.get(**key).is_none())
        .map(|key| key.to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("missing_keys:{}", missing.join(",")));
    }
    if !payload
        .get("files_touched")
        .map(Value::is_array)
        .unwrap_or(false)
        || !payload
            .get("commands_run")
            .map(Value::is_array)
            .unwrap_or(false)
    {
        return Err("files_touched_and_commands_run_must_be_arrays".to_string());
    }
    let routing_tags_len = payload
        .get("routing_tags")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    if routing_tags_len < 3 {
        return Err("routing_tags_min_3".to_string());
    }
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_postflight_validate",
        "routing_tags_count": routing_tags_len
    })))
}

fn run_output_validate(payload: &Value) -> Result<Value, String> {
    let mode = clean_text(
        payload
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("query"),
        40,
    )
    .to_ascii_lowercase();
    let text = clean_text(
        payload.get("text").and_then(Value::as_str).unwrap_or(""),
        100_000,
    );
    if text.is_empty() {
        return Err("text_required".to_string());
    }
    let lowered = text.to_ascii_lowercase();
    let tags = norm_tags(payload.get("tags"));
    let mut warnings = Vec::<String>::new();

    let (must_any, must_all, max_chars) = match mode.as_str() {
        "creative" => (
            vec!["Ideas:", "Options:", "Concepts:"],
            vec!["Next:"],
            DEFAULT_OUTPUT_VALIDATE_MAX_CREATIVE,
        ),
        "governance_override" => (
            vec!["Risks:", "Assumptions:"],
            vec!["Next:"],
            DEFAULT_OUTPUT_VALIDATE_MAX_GOVERNANCE,
        ),
        _ => (
            vec!["Answer:", "Result:", "Findings:"],
            vec!["Next:"],
            DEFAULT_OUTPUT_VALIDATE_MAX_QUERY,
        ),
    };

    if text.len() > max_chars {
        warnings.push(format!(
            "text_length_exceeds_recommended:{}>{}",
            text.len(),
            max_chars
        ));
    }
    for marker in must_all {
        if !lowered.contains(&marker.to_ascii_lowercase()) {
            warnings.push(format!("missing_required_marker:{marker}"));
        }
    }
    if !must_any
        .iter()
        .any(|marker| lowered.contains(&marker.to_ascii_lowercase()))
    {
        warnings.push(format!("missing_one_of:{}", must_any.join("|")));
    }
    if tags
        .iter()
        .any(|tag| high_risk_tags().contains(tag.as_str()))
    {
        if !lowered.contains("next:") {
            warnings.push("high_risk_missing_next_marker".to_string());
        }
        let has_safety_marker = ["Risks:", "Assumptions:", "Rollback:", "Controls:"]
            .iter()
            .any(|marker| lowered.contains(&marker.to_ascii_lowercase()));
        if !has_safety_marker {
            warnings.push("high_risk_missing_safety_markers".to_string());
        }
    }

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_output_validate",
        "mode": mode,
        "errors": [],
        "warnings": warnings
    })))
}

fn get_by_dot_path<'a>(value: &'a Value, dot_path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in dot_path.split('.') {
        if segment.trim().is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

fn run_state_read(state: &Value, key_path: Option<&str>, path: &Path) -> Result<Value, String> {
    if let Some(key) = key_path {
        let key_clean = clean_text(key, 240);
        let Some(value) = get_by_dot_path(state, &key_clean) else {
            return Err("state_key_not_found".to_string());
        };
        return Ok(with_receipt(json!({
            "ok": true,
            "type": "operator_tooling_state_read",
            "state_path": path.to_string_lossy().to_string(),
            "key": key_clean,
            "value": value
        })));
    }
    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_state_read",
        "state_path": path.to_string_lossy().to_string(),
        "state": state
    })))
}

fn run_state_write(state: &mut Value, payload: &Value, path: &Path) -> Result<Value, String> {
    let now = crate::now_iso();
    let mut last_task = state
        .get("last_task")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    for key in ["task", "model", "result"] {
        if let Some(value) = payload.get(key) {
            last_task.insert(key.to_string(), value.clone());
        }
    }
    if let Some(tags) = payload.get("tags").and_then(Value::as_array) {
        last_task.insert("tags".to_string(), Value::Array(tags.clone()));
    }
    last_task.insert("ts".to_string(), json!(now));
    state["last_task"] = Value::Object(last_task.clone());

    let mut decision_appended = false;
    if let Some(decision) = payload.get("decision") {
        if !decision.is_null() {
            let mut row = Map::<String, Value>::new();
            row.insert("ts".to_string(), json!(crate::now_iso()));
            row.insert("decision".to_string(), decision.clone());
            row.insert(
                "context".to_string(),
                payload.get("context").cloned().unwrap_or_else(|| json!({})),
            );
            let mut decisions = state
                .get("decisions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            decisions.push(Value::Object(row));
            if decisions.len() > 200 {
                let keep_from = decisions.len().saturating_sub(200);
                decisions = decisions.into_iter().skip(keep_from).collect::<Vec<_>>();
            }
            state["decisions"] = Value::Array(decisions);
            decision_appended = true;
        }
    }

    write_json_file(path, state)?;

    Ok(with_receipt(json!({
        "ok": true,
        "type": "operator_tooling_state_write",
        "state_path": path.to_string_lossy().to_string(),
        "last_task": last_task,
        "decision_appended": decision_appended
    })))
}

fn append_decision_markdown(
    path: &Path,
    title: &str,
    reason: &str,
    verify: &str,
    rollback: &str,
    details: &Value,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{err}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("decision_log_open_failed:{err}"))?;
    let details_line = if details.is_null() || details == &json!({}) {
        String::new()
    } else {
        format!(
            "- details: `{}`\n",
            serde_json::to_string(details).unwrap_or_else(|_| "{}".to_string())
        )
    };
    let entry = format!(
        "## {}\n- when: {}\n{}{}{}{}\n",
        if title.trim().is_empty() {
            "Decision"
        } else {
            title.trim()
        },
        crate::now_iso(),
        if reason.trim().is_empty() {
            String::new()
        } else {
            format!("- why: {}\n", reason.trim())
        },
        if verify.trim().is_empty() {
            String::new()
        } else {
            format!("- verify: `{}`\n", verify.trim())
        },
        if rollback.trim().is_empty() {
            String::new()
        } else {
            format!("- rollback: {}\n", rollback.trim())
        },
        details_line
    );
    file.write_all(entry.as_bytes())
        .map_err(|err| format!("decision_log_write_failed:{err}"))
}
