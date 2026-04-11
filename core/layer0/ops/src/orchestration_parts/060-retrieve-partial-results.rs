fn retrieve_partial_results(root: &Path, input: &Value) -> Value {
    let task_id = get_string_any(input, &["task_id", "taskId"]);
    if task_id.is_empty() {
        return json!({
            "ok": false,
            "type": "orchestration_partial_retrieval",
            "reason_code": "missing_task_id"
        });
    }

    let session_history = input
        .get("session_history")
        .or_else(|| input.get("sessionHistory"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let from_sessions = from_session_history(&session_history);
    if from_sessions.get("ok").and_then(Value::as_bool) == Some(true) {
        return json!({
            "ok": true,
            "type": "orchestration_partial_retrieval",
            "source": from_sessions.get("source").cloned().unwrap_or(Value::Null),
            "task_id": task_id,
            "items_completed": from_sessions.get("items_completed").cloned().unwrap_or(Value::Null),
            "findings_sofar": from_sessions.get("findings_sofar").cloned().unwrap_or(Value::Array(Vec::new())),
            "checkpoint_path": from_sessions.get("checkpoint_path").cloned().unwrap_or(Value::Null),
            "source_session_id": from_sessions.get("source_session_id").cloned().unwrap_or(Value::Null),
            "decision": normalize_decision(&get_string_any(input, &["decision"]), true)
        });
    }

    let root_dir_value = get_string_any(input, &["root_dir", "rootDir"]);
    let root_dir = if root_dir_value.is_empty() {
        None
    } else {
        Some(root_dir_value.as_str())
    };

    let task_group_id = get_string_any(input, &["task_group_id", "taskGroupId", "id"]);
    if !task_group_id.is_empty() {
        let task_group_partial = latest_partial_results_from_task_group(
            root,
            &task_group_id,
            root_dir,
        );
        if task_group_partial.get("ok").and_then(Value::as_bool) == Some(true) {
            return json!({
                "ok": true,
                "type": "orchestration_partial_retrieval",
                "source": task_group_partial.get("source").cloned().unwrap_or(Value::Null),
                "task_id": task_id,
                "task_group_id": task_group_partial.get("task_group_id").cloned().unwrap_or(Value::Null),
                "items_completed": task_group_partial.get("items_completed").cloned().unwrap_or(Value::Null),
                "findings_sofar": task_group_partial.get("findings_sofar").cloned().unwrap_or(Value::Array(Vec::new())),
                "checkpoint_path": task_group_partial.get("checkpoint_path").cloned().unwrap_or(Value::Null),
                "source_agent_ids": task_group_partial.get("source_agent_ids").cloned().unwrap_or(Value::Array(Vec::new())),
                "decision": normalize_decision(&get_string_any(input, &["decision"]), true)
            });
        }
    }
    let checkpoint = latest_checkpoint_from_scratchpad(root, &task_id, root_dir);

    if checkpoint.get("ok").and_then(Value::as_bool) != Some(true) {
        let attempted_sources = if task_group_id.is_empty() {
            vec!["session_history", "checkpoint"]
        } else {
            vec!["session_history", "task_group", "checkpoint"]
        };
        return json!({
            "ok": false,
            "type": "orchestration_partial_retrieval",
            "reason_code": "partial_results_unavailable",
            "task_id": task_id,
            "attempted_sources": attempted_sources,
            "checkpoint_reason": checkpoint.get("reason_code").cloned().unwrap_or(Value::Null)
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_partial_retrieval",
        "source": checkpoint.get("source").cloned().unwrap_or(Value::Null),
        "task_id": task_id,
        "items_completed": checkpoint.get("items_completed").cloned().unwrap_or(Value::Null),
        "findings_sofar": checkpoint.get("findings_sofar").cloned().unwrap_or(Value::Array(Vec::new())),
        "checkpoint_path": checkpoint.get("checkpoint_path").cloned().unwrap_or(Value::Null),
        "retry_allowed": checkpoint.get("retry_allowed").cloned().unwrap_or(Value::Bool(false)),
        "decision": normalize_decision(&get_string_any(input, &["decision"]), true)
    })
}

fn latest_partial_results_from_task_group(
    root: &Path,
    task_group_id: &str,
    root_dir: Option<&str>,
) -> Value {
    let query = query_task_group(root, task_group_id, root_dir);
    if query.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_partial_taskgroup_fallback",
            "reason_code": query.get("reason_code").cloned().unwrap_or(Value::String("task_group_query_failed".to_string())),
            "task_group_id": task_group_id.trim().to_ascii_lowercase()
        });
    }

    let agents = query
        .get("task_group")
        .and_then(|value| value.get("agents"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut findings_sofar = Vec::new();
    let mut source_agent_ids = Vec::new();
    let mut items_completed = 0i64;

    for agent in agents {
        let details = agent
            .get("details")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        let partial_results = details
            .get("partial_results")
            .or_else(|| details.get("partialResults"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if partial_results.is_empty() {
            continue;
        }
        items_completed += get_i64_any(
            &details,
            &["items_completed", "processed_count", "partial_results_count"],
            partial_results.len() as i64,
        )
        .max(partial_results.len() as i64);
        findings_sofar.extend(partial_results);

        let agent_id = to_clean_string(agent.get("agent_id"));
        if !agent_id.is_empty() {
            source_agent_ids.push(Value::String(agent_id));
        }
    }

    if findings_sofar.is_empty() {
        return json!({
            "ok": false,
            "type": "orchestration_partial_taskgroup_fallback",
            "reason_code": "task_group_no_partial_results",
            "task_group_id": task_group_id.trim().to_ascii_lowercase()
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_partial_taskgroup_fallback",
        "source": "task_group",
        "task_group_id": task_group_id.trim().to_ascii_lowercase(),
        "items_completed": items_completed,
        "findings_sofar": findings_sofar,
        "checkpoint_path": Value::Null,
        "source_agent_ids": source_agent_ids
    })
}

fn partition_work(items: &[Value], agent_count: i64) -> Vec<Value> {
    let count = agent_count.max(1) as usize;
    let mut partitions = (0..count)
        .map(|idx| {
            json!({
                "agent_id": format!("agent-{}", idx + 1),
                "items": []
            })
        })
        .collect::<Vec<_>>();

    for (index, item) in items.iter().enumerate() {
        if let Some(rows) = partitions
            .get_mut(index % count)
            .and_then(|partition| partition.get_mut("items"))
            .and_then(Value::as_array_mut)
        {
            rows.push(item.clone());
        }
    }

    partitions
}

fn merge_evidence(rows: &[Value]) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for row in rows {
        if !row.is_object() {
            continue;
        }
        let key = format!(
            "{}:{}:{}",
            to_clean_string(row.get("type")),
            to_clean_string(row.get("value")),
            to_clean_string(row.get("source"))
        );
        if seen.insert(key) {
            merged.push(row.clone());
        }
    }
    merged
}

fn merge_findings(findings: &[Value]) -> Value {
    let mut buckets: BTreeMap<String, Value> = BTreeMap::new();
    let mut dropped = Vec::new();

    for raw in findings {
        let normalized = normalize_finding(raw);
        let (ok, reason) = validate_finding(&normalized);
        if !ok {
            dropped.push(json!({
                "reason_code": reason,
                "finding": normalized
            }));
            continue;
        }

        let item_id = to_clean_string(normalized.get("item_id"));
        if item_id.is_empty() {
            dropped.push(json!({
                "reason_code": "finding_invalid_item_id",
                "finding": normalized
            }));
            continue;
        }

        if let Some(existing) = buckets.get_mut(&item_id) {
            let existing_severity = to_clean_string(existing.get("severity")).to_ascii_lowercase();
            let existing_status = to_clean_string(existing.get("status")).to_ascii_lowercase();
            let next_severity = to_clean_string(normalized.get("severity")).to_ascii_lowercase();
            let next_status = to_clean_string(normalized.get("status")).to_ascii_lowercase();

            if severity_order(&next_severity) > severity_order(&existing_severity) {
                if let Value::Object(map) = existing {
                    map.insert("severity".to_string(), Value::String(next_severity));
                }
            }
            if status_order(&next_status) > status_order(&existing_status) {
                if let Value::Object(map) = existing {
                    map.insert("status".to_string(), Value::String(next_status));
                }
            }

            let evidence = [
                existing
                    .get("evidence")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                normalized
                    .get("evidence")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            ]
            .concat();
            let existing_ts = to_clean_string(existing.get("timestamp"));
            let existing_summary = to_clean_string(existing.get("summary"));
            if let Value::Object(map) = existing {
                map.insert(
                    "evidence".to_string(),
                    Value::Array(merge_evidence(&evidence)),
                );

                let next_ts = to_clean_string(normalized.get("timestamp"));
                let max_ts = if existing_ts > next_ts {
                    existing_ts
                } else {
                    next_ts
                };
                map.insert("timestamp".to_string(), Value::String(max_ts));

                let summary = [existing_summary, to_clean_string(normalized.get("summary"))]
                    .into_iter()
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !summary.is_empty() {
                    map.insert("summary".to_string(), Value::String(summary));
                }
            }
        } else {
            buckets.insert(item_id, normalized);
        }
    }

    let mut merged = buckets.values().cloned().collect::<Vec<_>>();
    merged.sort_by(|left, right| {
        let left_severity = to_clean_string(left.get("severity")).to_ascii_lowercase();
        let right_severity = to_clean_string(right.get("severity")).to_ascii_lowercase();
        let severity_cmp = severity_order(&right_severity).cmp(&severity_order(&left_severity));
        if severity_cmp != std::cmp::Ordering::Equal {
            return severity_cmp;
        }
        let left_id = to_clean_string(left.get("item_id"));
        let right_id = to_clean_string(right.get("item_id"));
        left_id.cmp(&right_id)
    });

    json!({
        "merged": merged,
        "dropped": dropped,
        "deduped_count": (findings.len() as i64) - (merged.len() as i64) - (dropped.len() as i64)
    })
}

fn assign_scopes_to_partitions(partitions: &[Value], normalized_scopes: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for (index, partition) in partitions.iter().enumerate() {
        let scope = if normalized_scopes.is_empty() {
            Value::Null
        } else {
            normalized_scopes[index % normalized_scopes.len()].clone()
        };
        let mut row = get_object(partition);
        row.insert("scope".to_string(), scope);
        out.push(Value::Object(row));
    }
    out
}

fn scope_map_by_agent(partitions: &[Value]) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for partition in partitions {
        let agent_id = to_clean_string(partition.get("agent_id"));
        let scope = partition.get("scope").cloned().unwrap_or(Value::Null);
        if !agent_id.is_empty() && scope.is_object() {
            out.insert(agent_id, scope);
        }
    }
    out
}

fn apply_scope_filtering(findings: &[Value], scope_by_agent: &HashMap<String, Value>) -> Value {
    let mut kept = Vec::new();
    let mut violations = Vec::new();

    for raw in findings {
        let finding = normalize_finding(raw);
        let agent_id = {
            let direct = to_clean_string(finding.get("agent_id"));
            if !direct.is_empty() {
                direct
            } else {
                finding
                    .get("metadata")
                    .and_then(Value::as_object)
                    .and_then(|meta| meta.get("agent_id"))
                    .map(|v| to_clean_string(Some(v)))
                    .unwrap_or_default()
            }
        };

        if agent_id.is_empty() || !scope_by_agent.contains_key(&agent_id) {
            kept.push(finding);
            continue;
        }

        let scope = scope_by_agent
            .get(&agent_id)
            .cloned()
            .unwrap_or(Value::Null);
        let classified = classify_findings_by_scope(&[finding.clone()], &scope, &agent_id);
        if classified.get("ok").and_then(Value::as_bool) != Some(true) {
            violations.push(json!({
                "reason_code": "scope_classification_failed",
                "agent_id": agent_id,
                "item_id": finding.get("item_id").cloned().unwrap_or(Value::Null),
                "location": finding.get("location").cloned().unwrap_or(Value::Null)
            }));
            continue;
        }

        if let Some(rows) = classified.get("in_scope").and_then(Value::as_array) {
            if let Some(first) = rows.first() {
                kept.push(first.clone());
            }
        }

        if let Some(rows) = classified.get("violations").and_then(Value::as_array) {
            for row in rows {
                violations.push(row.clone());
            }
        }
    }

    json!({
        "kept": kept,
        "violations": violations
    })
}

fn stable_hash_short(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
        .chars()
        .take(12)
        .collect::<String>()
}
