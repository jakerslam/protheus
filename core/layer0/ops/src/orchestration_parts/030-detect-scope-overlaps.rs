fn detect_scope_overlaps(scopes: &[Value]) -> Value {
    let mut normalized: Vec<Value> = Vec::new();
    let mut seen_scope_ids: HashMap<String, usize> = HashMap::new();
    for (index, scope) in scopes.iter().enumerate() {
        let out = normalize_scope(scope, index);
        if out.get("ok").and_then(Value::as_bool) != Some(true) {
            return json!({
                "ok": false,
                "reason_code": out.get("reason_code").cloned().unwrap_or(Value::String("scope_invalid".to_string())),
                "scope_id": out.get("scope_id").cloned().unwrap_or(Value::Null),
                "overlaps": []
            });
        }
        let normalized_scope = out
            .get("scope")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        let scope_id = to_clean_string(normalized_scope.get("scope_id"));
        if let Some(previous_index) = seen_scope_ids.insert(scope_id.clone(), index) {
            return json!({
                "ok": false,
                "reason_code": "scope_duplicate_scope_id",
                "scope_id": scope_id,
                "normalized_scopes": normalized,
                "overlaps": [{
                    "left_scope_id": normalized[previous_index]
                        .get("scope_id")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "right_scope_id": normalized_scope.get("scope_id").cloned().unwrap_or(Value::Null),
                    "reason_code": "scope_duplicate_scope_id"
                }]
            });
        }
        normalized.push(normalized_scope);
    }

    let mut overlaps = Vec::new();
    for left_index in 0..normalized.len() {
        for right_index in (left_index + 1)..normalized.len() {
            let left = &normalized[left_index];
            let right = &normalized[right_index];

            let left_series = left
                .get("series")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| to_clean_string(Some(&v)))
                .collect::<HashSet<_>>();

            let right_series = right
                .get("series")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| to_clean_string(Some(&v)))
                .collect::<HashSet<_>>();

            let overlapping_series = left_series
                .intersection(&right_series)
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>();

            let left_paths = left
                .get("paths")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| to_clean_string(Some(&v)))
                .collect::<Vec<_>>();
            let right_paths = right
                .get("paths")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|v| to_clean_string(Some(&v)))
                .collect::<Vec<_>>();

            let mut overlapping_paths = Vec::new();
            for left_path in &left_paths {
                for right_path in &right_paths {
                    if path_pattern_overlaps(left_path, right_path) {
                        overlapping_paths.push(json!({
                            "left": left_path,
                            "right": right_path
                        }));
                    }
                }
            }

            if !overlapping_series.is_empty() || !overlapping_paths.is_empty() {
                overlaps.push(json!({
                    "left_scope_id": to_clean_string(left.get("scope_id")),
                    "right_scope_id": to_clean_string(right.get("scope_id")),
                    "overlapping_series": overlapping_series,
                    "overlapping_paths": overlapping_paths
                }));
            }
        }
    }

    json!({
        "ok": overlaps.is_empty(),
        "reason_code": if overlaps.is_empty() { "scope_non_overlap_ok" } else { "scope_overlap_detected" },
        "normalized_scopes": normalized,
        "overlaps": overlaps
    })
}

fn finding_in_scope(finding: &Value, scope: &Value) -> Value {
    let normalized_scope = normalize_scope(scope, 0);
    if normalized_scope.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "reason_code": normalized_scope.get("reason_code").cloned().unwrap_or(Value::String("scope_invalid".to_string())),
            "in_scope": false,
            "scope_id": normalized_scope.get("scope_id").cloned().unwrap_or(Value::Null)
        });
    }

    let scope_data = normalized_scope
        .get("scope")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let series = scope_data
        .get("series")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|v| to_clean_string(Some(&v)))
        .collect::<Vec<_>>();
    let paths = scope_data
        .get("paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|v| to_clean_string(Some(&v)))
        .collect::<Vec<_>>();

    let matches_series = finding_matches_series_scope(finding, &series);
    let matches_paths = finding_matches_path_scope(finding, &paths);
    let in_scope = matches_series && matches_paths;

    json!({
        "ok": true,
        "reason_code": if in_scope { "finding_in_scope" } else { "finding_out_of_scope" },
        "in_scope": in_scope,
        "scope_id": scope_data.get("scope_id").cloned().unwrap_or(Value::Null),
        "matches_series": matches_series,
        "matches_paths": matches_paths
    })
}

fn classify_findings_by_scope(findings: &[Value], scope: &Value, agent_id: &str) -> Value {
    let mut in_scope = Vec::new();
    let mut out_of_scope = Vec::new();
    let mut violations = Vec::new();

    let normalized_agent_id = if agent_id.trim().is_empty() {
        Value::Null
    } else {
        Value::String(agent_id.trim().to_string())
    };

    for finding in findings {
        let verdict = finding_in_scope(finding, scope);
        if verdict.get("ok").and_then(Value::as_bool) != Some(true) {
            out_of_scope.push(finding.clone());
            violations.push(json!({
                "reason_code": verdict.get("reason_code").cloned().unwrap_or(Value::String("scope_classification_failed".to_string())),
                "item_id": finding.get("item_id").cloned().unwrap_or(Value::Null),
                "location": finding.get("location").cloned().unwrap_or(Value::Null),
                "agent_id": normalized_agent_id,
                "scope_id": verdict.get("scope_id").cloned().unwrap_or(Value::Null)
            }));
            continue;
        }

        if verdict.get("in_scope").and_then(Value::as_bool) == Some(true) {
            in_scope.push(finding.clone());
            continue;
        }

        out_of_scope.push(finding.clone());
        violations.push(json!({
            "reason_code": "out_of_scope_finding",
            "item_id": finding.get("item_id").cloned().unwrap_or(Value::Null),
            "location": finding.get("location").cloned().unwrap_or(Value::Null),
            "agent_id": normalized_agent_id,
            "scope_id": verdict.get("scope_id").cloned().unwrap_or(Value::Null),
            "matches_series": verdict.get("matches_series").cloned().unwrap_or(Value::Bool(false)),
            "matches_paths": verdict.get("matches_paths").cloned().unwrap_or(Value::Bool(false))
        }));
    }

    json!({
        "ok": true,
        "type": "orchestration_scope_classification",
        "in_scope": in_scope,
        "out_of_scope": out_of_scope,
        "violations": violations
    })
}

fn slug(raw: &str, fallback: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.trim().to_ascii_lowercase().chars() {
        let mapped =
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '_' | '.' | '-') {
                ch
            } else {
                '-'
            };
        if mapped == '-' {
            if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        } else {
            out.push(mapped);
            prev_dash = false;
        }
        if out.len() >= max_len {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed
    }
}

fn timestamp_token(now_ms: i64) -> String {
    let date = chrono::DateTime::<Utc>::from_timestamp_millis(now_ms).unwrap_or_else(Utc::now);
    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        date.year(),
        date.month(),
        date.day(),
        date.hour(),
        date.minute(),
        date.second()
    )
}

fn nonce_token(length: usize) -> String {
    let width = length.max(4);
    let mut bytes = vec![0u8; width];
    rand::thread_rng().fill_bytes(&mut bytes);
    let hex = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    hex.chars().take(width).collect()
}

fn generate_task_group_id(task_type: &str, now_ms: i64, nonce: &str) -> String {
    let nonce_value = if nonce.trim().is_empty() {
        nonce_token(6)
    } else {
        nonce.trim().to_ascii_lowercase()
    };
    let out = format!(
        "{}-{}-{}",
        slug(task_type, "task", 48),
        timestamp_token(now_ms),
        slug(&nonce_value, &nonce_token(6), 24)
    );
    out.chars().take(127).collect()
}

fn allowed_agent_status(status: &str) -> bool {
    matches!(
        status,
        "pending" | "running" | "done" | "failed" | "timeout"
    )
}

fn terminal_agent_status(status: &str) -> bool {
    matches!(status, "done" | "failed" | "timeout")
}

fn normalize_agent_id(raw: &str, index: usize) -> Result<String, String> {
    let id = if raw.trim().is_empty() {
        format!("agent-{}", index + 1)
    } else {
        raw.trim().to_string()
    };
    if validate_agent_id(&id) {
        Ok(id)
    } else {
        Err(format!("invalid_agent_id:{id}"))
    }
}

fn normalize_agents(input_agents: &[Value], fallback_count: i64) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for (index, row) in input_agents.iter().enumerate() {
        let row_object = row.as_object().cloned().unwrap_or_default();
        let raw_agent_id = row_object
            .get("agent_id")
            .or_else(|| row_object.get("agentId"))
            .map(|v| to_clean_string(Some(v)))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| to_clean_string(Some(row)));
        let agent_id = normalize_agent_id(&raw_agent_id, index)?;
        if !seen.insert(agent_id.clone()) {
            continue;
        }
        let status = to_clean_string(row_object.get("status")).to_ascii_lowercase();
        let normalized_status = if allowed_agent_status(&status) {
            status
        } else {
            "pending".to_string()
        };
        let details = row_object
            .get("details")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        out.push(json!({
            "agent_id": agent_id,
            "status": normalized_status,
            "updated_at": now_iso(),
            "details": details
        }));
    }

    let desired_count = fallback_count.max(1) as usize;
    while out.len() < desired_count {
        let next_id = normalize_agent_id("", out.len())?;
        if !seen.insert(next_id.clone()) {
            continue;
        }
        out.push(json!({
            "agent_id": next_id,
            "status": "pending",
            "updated_at": now_iso(),
            "details": {}
        }));
    }

    Ok(out)
}

fn status_counts(task_group: &Value) -> Value {
    let mut pending = 0;
    let mut running = 0;
    let mut done = 0;
    let mut failed = 0;
    let mut timeout = 0;

    let agents = task_group
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for agent in agents {
        let status = to_clean_string(agent.get("status")).to_ascii_lowercase();
        match status.as_str() {
            "pending" => pending += 1,
            "running" => running += 1,
            "done" => done += 1,
            "failed" => failed += 1,
            "timeout" => timeout += 1,
            _ => {}
        }
    }

    let total = pending + running + done + failed + timeout;
    json!({
        "pending": pending,
        "running": running,
        "done": done,
        "failed": failed,
        "timeout": timeout,
        "total": total
    })
}

fn derive_group_status(task_group: &Value) -> String {
    let counts = status_counts(task_group);
    let total = get_i64_any(&counts, &["total"], 0);
    let pending = get_i64_any(&counts, &["pending"], 0);
    let running = get_i64_any(&counts, &["running"], 0);
    let done = get_i64_any(&counts, &["done"], 0);
    let failed = get_i64_any(&counts, &["failed"], 0);
    let timeout = get_i64_any(&counts, &["timeout"], 0);

    if total == 0 || pending == total {
        "pending".to_string()
    } else if running > 0 || pending > 0 {
        "running".to_string()
    } else if failed > 0 && done == 0 && timeout == 0 {
        "failed".to_string()
    } else if timeout > 0 && done == 0 && failed == 0 {
        "timeout".to_string()
    } else if done == total {
        "done".to_string()
    } else if done + failed + timeout == total {
        "completed".to_string()
    } else {
        "running".to_string()
    }
}

fn default_task_group(task_group_id: &str, input: &Value) -> Result<Value, String> {
    let agent_count = get_i64_any(input, &["agent_count", "agentCount"], 1).max(1);
    let agents_source = input
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let agents = normalize_agents(&agents_source, agent_count)?;

    let coordinator_session = {
        let value = get_string_any(input, &["coordinator_session", "coordinatorSession"]);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };

    Ok(json!({
        "schema_version": TASKGROUP_SCHEMA_VERSION,
        "task_group_id": task_group_id,
        "task_type": slug(&get_string_any(input, &["task_type", "taskType"]), "task", 48),
        "coordinator_session": coordinator_session,
        "created_at": now_iso(),
        "updated_at": now_iso(),
        "agent_count": agents.len(),
        "status": "pending",
        "agents": agents,
        "history": []
    }))
}

#[derive(Debug, Clone)]
struct LoadedTaskGroup {
    exists: bool,
    file_path: PathBuf,
    task_group: Value,
}
