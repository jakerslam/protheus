fn task_group_desired_agent_count(map: &Map<String, Value>) -> i64 {
    map.get("agent_count")
        .and_then(Value::as_i64)
        .or_else(|| map.get("agent_count").and_then(Value::as_u64).map(|v| v as i64))
        .unwrap_or(1)
        .max(1)
}

fn normalize_task_group_shape(
    map: &mut Map<String, Value>,
    task_group_id: Option<&str>,
) -> Result<(), String> {
    map.insert(
        "schema_version".to_string(),
        Value::String(TASKGROUP_SCHEMA_VERSION.to_string()),
    );
    if let Some(id) = task_group_id {
        map.insert(
            "task_group_id".to_string(),
            Value::String(id.trim().to_ascii_lowercase()),
        );
    }

    let agents_source = map
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let agents = normalize_agents(&agents_source, task_group_desired_agent_count(map))?;
    let normalized_agent_count = agents.len() as i64;
    map.insert("agents".to_string(), Value::Array(agents));
    map.insert(
        "agent_count".to_string(),
        Value::Number(serde_json::Number::from(normalized_agent_count)),
    );
    if !map.get("history").map(Value::is_array).unwrap_or(false) {
        map.insert("history".to_string(), Value::Array(Vec::new()));
    }
    let status = derive_group_status(&Value::Object(map.clone()));
    map.insert("status".to_string(), Value::String(status));
    Ok(())
}

fn load_task_group(
    root: &Path,
    task_group_id: &str,
    root_dir: Option<&str>,
) -> Result<LoadedTaskGroup, String> {
    let file_path = taskgroup_path(root, task_group_id, root_dir)?;
    if !file_path.exists() {
        return Ok(LoadedTaskGroup {
            exists: false,
            file_path,
            task_group: Value::Null,
        });
    }

    let raw = fs::read_to_string(&file_path)
        .map_err(|err| format!("taskgroup_read_failed:{}:{err}", file_path.display()))?;
    let mut parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("taskgroup_parse_failed:{}:{err}", file_path.display()))?;
    if !parsed.is_object() {
        return Err("invalid_taskgroup_payload".to_string());
    }

    if let Value::Object(map) = &mut parsed {
        normalize_task_group_shape(map, Some(task_group_id))?;
    }

    Ok(LoadedTaskGroup {
        exists: true,
        file_path,
        task_group: parsed,
    })
}

fn save_task_group(root: &Path, task_group: &Value, root_dir: Option<&str>) -> Value {
    if !task_group.is_object() {
        return json!({
            "ok": false,
            "type": "orchestration_taskgroup_save",
            "reason_code": "invalid_taskgroup"
        });
    }

    let task_group_id = to_clean_string(task_group.get("task_group_id")).to_ascii_lowercase();
    let file_path = match taskgroup_path(root, &task_group_id, root_dir) {
        Ok(path) => path,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_save",
                "reason_code": err
            });
        }
    };

    let mut next = task_group.clone();
    if let Value::Object(map) = &mut next {
        if let Err(err) = normalize_task_group_shape(map, None) {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_save",
                "reason_code": err
            });
        }
        map.insert("updated_at".to_string(), Value::String(now_iso()));
        if to_clean_string(map.get("created_at")).is_empty() {
            map.insert("created_at".to_string(), Value::String(now_iso()));
        }
    }

    if let Some(parent) = file_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_save",
                "reason_code": format!("taskgroup_create_parent_failed:{}:{err}", parent.display())
            });
        }
    }

    let payload = match serde_json::to_string_pretty(&next) {
        Ok(text) => text + "\n",
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_save",
                "reason_code": format!("taskgroup_encode_failed:{err}")
            });
        }
    };

    if let Err(err) = fs::write(&file_path, payload) {
        return json!({
            "ok": false,
            "type": "orchestration_taskgroup_save",
            "reason_code": format!("taskgroup_write_failed:{}:{err}", file_path.display())
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_taskgroup_save",
        "file_path": file_path,
        "task_group": next,
        "counts": status_counts(&next)
    })
}

fn ensure_task_group(root: &Path, input: &Value, root_dir: Option<&str>) -> Value {
    let requested = get_string_any(input, &["task_group_id", "taskGroupId"]).to_ascii_lowercase();
    let task_group_id = if requested.is_empty() {
        let task_type = get_string_any(input, &["task_type", "taskType"]);
        let now_ms = get_i64_any(input, &["now_ms"], Utc::now().timestamp_millis());
        let nonce = get_string_any(input, &["nonce"]);
        generate_task_group_id(
            if task_type.is_empty() {
                "task"
            } else {
                &task_type
            },
            now_ms,
            &nonce,
        )
    } else {
        requested
    };

    let loaded = match load_task_group(root, &task_group_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_ensure",
                "reason_code": err
            });
        }
    };

    if loaded.exists {
        return json!({
            "ok": true,
            "type": "orchestration_taskgroup_ensure",
            "created": false,
            "file_path": loaded.file_path,
            "task_group": loaded.task_group,
            "counts": status_counts(&loaded.task_group)
        });
    }

    let mut seed = input.clone();
    if let Value::Object(map) = &mut seed {
        map.insert(
            "task_group_id".to_string(),
            Value::String(task_group_id.clone()),
        );
    }
    let created = match default_task_group(&task_group_id, &seed) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_ensure",
                "reason_code": err
            });
        }
    };

    let saved = save_task_group(root, &created, root_dir);
    if saved.get("ok").and_then(Value::as_bool) != Some(true) {
        return saved;
    }

    json!({
        "ok": true,
        "type": "orchestration_taskgroup_ensure",
        "created": true,
        "file_path": saved.get("file_path").cloned().unwrap_or(Value::Null),
        "task_group": saved.get("task_group").cloned().unwrap_or(Value::Null),
        "counts": saved.get("counts").cloned().unwrap_or(Value::Null)
    })
}

fn update_agent_status(
    root: &Path,
    task_group_id: &str,
    agent_id: &str,
    status: &str,
    details: &Value,
    root_dir: Option<&str>,
) -> Value {
    let ensure = ensure_task_group(root, &json!({ "task_group_id": task_group_id }), root_dir);
    if ensure.get("ok").and_then(Value::as_bool) != Some(true) {
        return ensure;
    }

    let normalized_agent_id = match normalize_agent_id(agent_id, 0) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_update_status",
                "reason_code": err
            });
        }
    };
    let normalized_status = status.trim().to_ascii_lowercase();
    if !allowed_agent_status(&normalized_status) {
        return json!({
            "ok": false,
            "type": "orchestration_taskgroup_update_status",
            "reason_code": format!("invalid_agent_status:{}", if status.trim().is_empty() { "<empty>" } else { status })
        });
    }

    let mut group = ensure
        .get("task_group")
        .cloned()
        .unwrap_or(Value::Object(Map::new()));
    let now = now_iso();

    let mut previous_status = "pending".to_string();
    if let Some(agents) = group.get_mut("agents").and_then(Value::as_array_mut) {
        let mut found_index = None;
        for (index, row) in agents.iter().enumerate() {
            if to_clean_string(row.get("agent_id")) == normalized_agent_id {
                found_index = Some(index);
                break;
            }
        }

        if let Some(index) = found_index {
            if let Some(agent) = agents.get_mut(index) {
                previous_status = to_clean_string(agent.get("status")).to_ascii_lowercase();
                if let Value::Object(agent_map) = agent {
                    agent_map.insert(
                        "status".to_string(),
                        Value::String(normalized_status.clone()),
                    );
                    agent_map.insert("updated_at".to_string(), Value::String(now.clone()));

                    let mut next_details = agent_map
                        .get("details")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    if let Some(new_details) = details.as_object() {
                        for (k, v) in new_details {
                            next_details.insert(k.clone(), v.clone());
                        }
                    }
                    agent_map.insert("details".to_string(), Value::Object(next_details));
                }
            }
        } else {
            agents.push(json!({
                "agent_id": normalized_agent_id,
                "status": normalized_status,
                "updated_at": now,
                "details": details.as_object().cloned().unwrap_or_default()
            }));
        }
    }

    if let Value::Object(map) = &mut group {
        let count = map
            .get("agents")
            .and_then(Value::as_array)
            .map(|rows| rows.len() as i64)
            .unwrap_or(1);
        map.insert(
            "agent_count".to_string(),
            Value::Number(serde_json::Number::from(count)),
        );

        let mut history = map
            .get("history")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        history.push(json!({
            "event": "agent_status_update",
            "at": now_iso(),
            "agent_id": normalized_agent_id,
            "previous_status": previous_status,
            "status": normalized_status,
            "terminal": terminal_agent_status(&normalized_status),
            "details": details.as_object().cloned().unwrap_or_default()
        }));
        map.insert("history".to_string(), Value::Array(history));
    }

    let saved = save_task_group(root, &group, root_dir);
    if saved.get("ok").and_then(Value::as_bool) != Some(true) {
        return saved;
    }

    json!({
        "ok": true,
        "type": "orchestration_taskgroup_update_status",
        "task_group_id": saved.get("task_group").and_then(|v| v.get("task_group_id")).cloned().unwrap_or(Value::Null),
        "agent_id": normalized_agent_id,
        "status": normalized_status,
        "previous_status": previous_status,
        "file_path": saved.get("file_path").cloned().unwrap_or(Value::Null),
        "task_group": saved.get("task_group").cloned().unwrap_or(Value::Null),
        "counts": saved.get("counts").cloned().unwrap_or(Value::Null)
    })
}

fn query_task_group(root: &Path, task_group_id: &str, root_dir: Option<&str>) -> Value {
    let loaded = match load_task_group(root, task_group_id, root_dir) {
        Ok(value) => value,
        Err(err) => {
            return json!({
                "ok": false,
                "type": "orchestration_taskgroup_query",
                "reason_code": err,
                "task_group_id": task_group_id.trim().to_ascii_lowercase()
            });
        }
    };

    if !loaded.exists {
        return json!({
            "ok": false,
            "type": "orchestration_taskgroup_query",
            "reason_code": "task_group_not_found",
            "task_group_id": task_group_id.trim().to_ascii_lowercase()
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_taskgroup_query",
        "file_path": loaded.file_path,
        "task_group": loaded.task_group,
        "counts": status_counts(&loaded.task_group)
    })
}

fn build_checkpoint(task_id: &str, metrics: &Value, reason: &str) -> Value {
    json!({
        "task_id": task_id,
        "reason": reason,
        "processed_count": get_i64_any(metrics, &["processed_count", "processed"], 0),
        "total_count": get_i64_any(metrics, &["total_count", "total"], 0),
        "now_ms": get_i64_any(metrics, &["now_ms"], Utc::now().timestamp_millis()),
        "partial_results": metrics.get("partial_results").and_then(Value::as_array).cloned().unwrap_or_default(),
        "retry_count": get_i64_any(metrics, &["retry_count"], 0)
    })
}

fn should_checkpoint(state: &Value, metrics: &Value, options: &Value) -> bool {
    let item_interval =
        get_i64_any(options, &["itemInterval", "item_interval"], ITEM_INTERVAL).max(1);
    let time_interval_ms = get_i64_any(
        options,
        &["timeIntervalMs", "time_interval_ms"],
        TIME_INTERVAL_MS,
    )
    .max(1);
    let now_ms = get_i64_any(metrics, &["now_ms"], Utc::now().timestamp_millis());
    let processed = get_i64_any(metrics, &["processed_count", "processed"], 0).max(0);

    let checkpoints = state
        .get("checkpoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if checkpoints.is_empty() {
        return processed > 0;
    }

    let last = checkpoints.last().cloned().unwrap_or(Value::Null);
    let last_processed = get_i64_any(&last, &["processed_count"], 0);
    let last_now_ms = get_i64_any(&last, &["now_ms"], now_ms);

    let item_delta = processed - last_processed;
    let time_delta = now_ms - last_now_ms;
    item_delta >= item_interval || time_delta >= time_interval_ms
}
