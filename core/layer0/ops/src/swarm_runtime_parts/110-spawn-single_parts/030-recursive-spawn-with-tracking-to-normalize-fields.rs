
fn recursive_spawn_with_tracking(
    state: &mut SwarmState,
    parent_id: Option<&str>,
    task: &str,
    levels: u8,
    max_depth: u8,
    options: &SpawnOptions,
) -> Result<Value, String> {
    if levels == 0 {
        return Err("recursive_levels_must_be_positive".to_string());
    }

    let mut lineage = Vec::new();
    let mut current_parent = parent_id.map(ToString::to_string);
    let mut level = 0u8;
    let mut visited_parent_chain = BTreeSet::new();
    let mut loop_guard = json!({
        "triggered": false,
        "reason": "none",
    });
    let traversal_limit = (levels as usize).saturating_mul(4).max(8);
    let mut traversal_cost = 0usize;
    while level < levels {
        traversal_cost = traversal_cost.saturating_add(1);
        if traversal_cost > traversal_limit {
            loop_guard = json!({
                "triggered": true,
                "reason": "cost_guard_exceeded",
                "traversal_limit": traversal_limit,
                "traversal_cost": traversal_cost,
                "lineage_count": lineage.len(),
            });
            append_event(
                state,
                json!({
                    "type": "swarm_recursive_loop_guard_triggered",
                    "reason": "cost_guard_exceeded",
                    "task": task,
                    "parent_id": parent_id,
                    "lineage_count": lineage.len(),
                    "timestamp": now_iso(),
                }),
            );
            break;
        }
        if let Some(parent) = current_parent.as_ref() {
            if !visited_parent_chain.insert(parent.clone()) {
                loop_guard = json!({
                    "triggered": true,
                    "reason": "cycle_detected",
                    "cycle_at": parent,
                    "lineage_count": lineage.len(),
                });
                append_event(
                    state,
                    json!({
                        "type": "swarm_recursive_loop_guard_triggered",
                        "reason": "cycle_detected",
                        "cycle_at": parent,
                        "task": task,
                        "parent_id": parent_id,
                        "lineage_count": lineage.len(),
                        "timestamp": now_iso(),
                    }),
                );
                break;
            }
            if let Some(parent_guard) =
                detect_parent_lineage_loop(state, Some(parent.as_str()), traversal_limit)
            {
                loop_guard = json!({
                    "triggered": true,
                    "reason": "lineage_cycle_guard_blocked",
                    "diagnostics": parent_guard.clone(),
                });
                append_event(
                    state,
                    json!({
                        "type": "swarm_recursive_loop_guard_triggered",
                        "reason": "lineage_cycle_guard_blocked",
                        "task": task,
                        "parent_id": parent_id,
                        "diagnostics": parent_guard,
                        "lineage_count": lineage.len(),
                        "timestamp": now_iso(),
                    }),
                );
                break;
            }
        }
        let spawned = spawn_single(state, current_parent.as_deref(), task, max_depth, options)?;
        let child = spawned
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "spawn_missing_session_id".to_string())?
            .to_string();
        lineage.push(spawned);
        current_parent = Some(child);
        level = level.saturating_add(1);
    }

    Ok(json!({
        "recursive": true,
        "terminated_safely": loop_guard.get("triggered").and_then(Value::as_bool).unwrap_or(false),
        "loop_guard": loop_guard,
        "levels": levels,
        "lineage": lineage,
        "final_session_id": current_parent,
        "max_depth": max_depth
    }))
}

fn corrupted_report(corruption_type: &str, session_id: &str) -> Value {
    match corruption_type {
        "wrong_file" => json!({
            "session_id": session_id,
            "file": "FAKE.md",
            "file_size": 9999,
            "word_count": 5000,
            "first_line": "FAKE DATA HERE",
            "corrupted": true,
        }),
        _ => json!({
            "session_id": session_id,
            "file": "SOUL.md",
            "file_size": 9999,
            "word_count": 5000,
            "first_line": "FAKE DATA HERE",
            "corrupted": true,
        }),
    }
}

fn parse_reports(raw: &Value) -> Vec<AgentReport> {
    raw.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let agent_id = row
                        .get("agent_id")
                        .or_else(|| row.get("agent"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)?;

                    let mut values = BTreeMap::new();
                    if let Some(object) = row.get("values").and_then(Value::as_object) {
                        for (key, value) in object {
                            values.insert(key.to_string(), value.clone());
                        }
                    }
                    Some(AgentReport { agent_id, values })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn reports_from_state(state: &SwarmState, task_id: Option<&str>) -> Vec<AgentReport> {
    let mut reports = Vec::new();
    for session in state.sessions.values() {
        if let Some(filter) = task_id {
            if session.task != filter {
                continue;
            }
        }

        let Some(report_value) = session.report.as_ref() else {
            continue;
        };

        let mut values = BTreeMap::new();
        if let Some(object) = report_value.as_object() {
            for (key, value) in object {
                values.insert(key.to_string(), value.clone());
            }
        }

        reports.push(AgentReport {
            agent_id: session.session_id.clone(),
            values,
        });
    }
    reports
}

fn normalize_fields(fields_csv: Option<String>, reports: &[AgentReport]) -> Vec<String> {
    if let Some(raw) = fields_csv {
        let mut parsed = raw
            .split(',')
            .map(|field| field.trim())
            .filter(|field| !field.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        parsed.sort();
        parsed.dedup();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let mut keys = reports
        .iter()
        .flat_map(|report| report.values.keys().cloned())
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}
