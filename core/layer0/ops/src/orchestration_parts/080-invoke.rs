fn array_from_payload_value(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(rows)) => rows.clone(),
        Some(Value::String(text)) => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|parsed| parsed.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn invoke(root: &Path, op: &str, payload: &Value) -> Value {
    match op {
        "schema.validate_finding" => {
            let finding = payload
                .get("finding")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            let normalized = normalize_finding(&finding);
            let (ok, reason_code) = validate_finding(&normalized);
            json!({
                "ok": ok,
                "type": "orchestration_schema_validate_finding",
                "reason_code": reason_code,
                "finding": normalized
            })
        }
        "schema.normalize_finding" => {
            let finding = payload
                .get("finding")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            json!({
                "ok": true,
                "type": "orchestration_schema_normalize_finding",
                "finding": normalize_finding(&finding)
            })
        }
        "scope.detect_overlaps" => {
            let scopes = payload
                .get("scopes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut out = detect_scope_overlaps(&scopes);
            if let Value::Object(map) = &mut out {
                map.insert(
                    "type".to_string(),
                    Value::String("orchestration_scope_validate".to_string()),
                );
            }
            out
        }
        "scope.classify_findings" => {
            let findings = payload
                .get("findings")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let scope = payload
                .get("scope")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let agent_id = get_string_any(payload, &["agent_id", "agentId"]);
            classify_findings_by_scope(&findings, &scope, &agent_id)
        }
        "scope.normalize" => {
            let scope = payload
                .get("scope")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            normalize_scope(&scope, 0)
        }
        "scope.finding_in_scope" => {
            let finding = payload
                .get("finding")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let scope = payload
                .get("scope")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            finding_in_scope(&finding, &scope)
        }
        "scratchpad.path" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let root_dir = payload_root_dir(payload);
            let out = scratchpad_path(root, &task_id, root_dir.as_deref());
            match out {
                Ok(file_path) => json!({
                    "ok": true,
                    "type": "orchestration_scratchpad_path",
                    "task_id": task_id,
                    "file_path": file_path
                }),
                Err(err) => json!({
                    "ok": false,
                    "type": "orchestration_scratchpad_path",
                    "reason_code": err,
                    "task_id": task_id
                }),
            }
        }
        "scratchpad.status" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let root_dir = payload_root_dir(payload);
            match load_scratchpad(root, &task_id, root_dir.as_deref()) {
                Ok(loaded) => json!({
                    "ok": true,
                    "type": "orchestration_scratchpad_status",
                    "task_id": task_id,
                    "file_path": loaded.file_path,
                    "exists": loaded.exists,
                    "scratchpad": loaded.scratchpad
                }),
                Err(err) => json!({
                    "ok": false,
                    "type": "orchestration_scratchpad_status",
                    "reason_code": err,
                    "task_id": task_id
                }),
            }
        }
        "scratchpad.write" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let patch = payload
                .get("patch")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            let root_dir = payload_root_dir(payload);
            match write_scratchpad(root, &task_id, &patch, root_dir.as_deref()) {
                Ok(value) => value,
                Err(err) => json!({
                    "ok": false,
                    "type": "orchestration_scratchpad_write",
                    "reason_code": err,
                    "task_id": task_id
                }),
            }
        }
        "scratchpad.append_finding" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let finding = payload
                .get("finding")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let root_dir = payload_root_dir(payload);
            append_finding(root, &task_id, &finding, root_dir.as_deref())
        }
        "scratchpad.append_checkpoint" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let checkpoint = payload
                .get("checkpoint")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let root_dir = payload_root_dir(payload);
            append_checkpoint(root, &task_id, &checkpoint, root_dir.as_deref())
        }
        "scratchpad.cleanup" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let root_dir = payload_root_dir(payload);
            cleanup_scratchpad(root, &task_id, root_dir.as_deref())
        }
        "checkpoint.should" => {
            let state = payload
                .get("state")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let metrics = payload
                .get("metrics")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let options = payload
                .get("options")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            json!({
                "ok": true,
                "type": "orchestration_checkpoint_should",
                "should_checkpoint": should_checkpoint(&state, &metrics, &options)
            })
        }
        "checkpoint.tick" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let metrics = payload
                .get("metrics")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            let root_dir = payload_root_dir(payload);
            maybe_checkpoint(root, &task_id, &metrics, root_dir.as_deref())
        }
        "checkpoint.timeout" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let metrics = payload
                .get("metrics")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            let root_dir = payload_root_dir(payload);
            handle_timeout(root, &task_id, &metrics, root_dir.as_deref())
        }
        "taskgroup.list_agents" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let root_dir = payload_root_dir(payload);
            let queried = query_task_group(root, &task_group_id, root_dir.as_deref());
            if queried.get("ok").and_then(Value::as_bool) != Some(true) {
                queried
            } else {
                json!({
                    "ok": true,
                    "type": "orchestration_taskgroup_list_agents",
                    "task_group_id": queried
                        .get("task_group")
                        .and_then(|value| value.get("task_group_id"))
                        .cloned()
                        .unwrap_or(Value::Null),
                    "agents": queried
                        .get("task_group")
                        .and_then(|value| value.get("agents"))
                        .cloned()
                        .unwrap_or(Value::Array(Vec::new())),
                    "counts": queried.get("counts").cloned().unwrap_or(Value::Null)
                })
            }
        }
        op if op.starts_with("taskgroup.") || op.starts_with("completion.") => {
            invoke_taskgroup_completion_ops(root, op, payload).unwrap_or_else(|| {
                json!({
                    "ok": false,
                    "type": "orchestration_invoke",
                    "reason_code": format!("unsupported_op:{op}")
                })
            })
        }
        "partial.normalize_decision" => {
            let decision = get_string_any(payload, &["decision"]);
            let has_partial_results = payload
                .get("has_partial_results")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            json!({
                "ok": true,
                "type": "orchestration_partial_normalize_decision",
                "decision": normalize_decision(&decision, has_partial_results)
            })
        }
        "partial.fetch" => retrieve_partial_results(root, payload),
        "partial.from_session_history" => {
            let history = payload
                .get("session_history")
                .or_else(|| payload.get("sessionHistory"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            from_session_history(&history)
        }
        "partial.latest_checkpoint" => {
            let task_id = get_string_any(payload, &["task_id", "taskId"]);
            let root_dir = payload_root_dir(payload);
            latest_checkpoint_from_scratchpad(root, &task_id, root_dir.as_deref())
        }
        "coordinator.partition" => {
            let items = payload
                .get("items")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let scopes = payload
                .get("scopes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let agent_count = get_i64_any(payload, &["agent_count", "agentCount"], 1).max(1);

            let scope_check = detect_scope_overlaps(&scopes);
            if scope_check.get("ok").and_then(Value::as_bool) != Some(true) {
                return json!({
                    "ok": false,
                    "type": "orchestration_partition",
                    "reason_code": scope_check.get("reason_code").cloned().unwrap_or(Value::String("scope_overlap_detected".to_string())),
                    "overlaps": scope_check.get("overlaps").cloned().unwrap_or(Value::Array(Vec::new()))
                });
            }
            let normalized_scopes = scope_check
                .get("normalized_scopes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let partitions = assign_scopes_to_partitions(
                &partition_work(&items, agent_count),
                &normalized_scopes,
            );
            json!({
                "ok": true,
                "type": "orchestration_partition",
                "partitions": partitions
            })
        }
        "coordinator.merge_findings" => {
            let findings = payload
                .get("findings")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let merged = merge_findings(&findings);
            json!({
                "ok": true,
                "type": "orchestration_merge_findings",
                "merged": merged.get("merged").cloned().unwrap_or(Value::Array(Vec::new())),
                "dropped": merged.get("dropped").cloned().unwrap_or(Value::Array(Vec::new())),
                "deduped_count": merged.get("deduped_count").cloned().unwrap_or(Value::Number(serde_json::Number::from(0)))
            })
        }
        "coordinator.timeout" => {
            let task_id = get_string_any(payload, &["task_id", "taskId", "id"]);
            if task_id.is_empty() {
                json!({
                    "ok": false,
                    "type": "orchestration_coordinator_timeout",
                    "reason_code": "missing_task_id"
                })
            } else {
                let items_total = payload
                    .get("items")
                    .and_then(Value::as_array)
                    .map(|rows| rows.len() as i64)
                    .unwrap_or(0);
                let findings = payload
                    .get("findings")
                    .or_else(|| payload.get("partial_results"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let metrics = json!({
                    "processed_count": get_i64_any(payload, &["processed_count", "processed"], 0),
                    "total_count": get_i64_any(payload, &["total_count", "total"], items_total),
                    "partial_results": findings,
                    "retry_count": get_i64_any(payload, &["retry_count", "retryCount"], 0),
                    "now_ms": get_i64_any(payload, &["now_ms", "nowMs"], Utc::now().timestamp_millis())
                });
                let root_dir = payload_root_dir(payload);
                handle_timeout(root, &task_id, &metrics, root_dir.as_deref())
            }
        }
        "coordinator.run" => run_coordinator(root, payload),
        "coordinator.status" => {
            let task_id = get_string_any(payload, &["task_id", "taskId", "id"]);
            if task_id.is_empty() {
                json!({
                    "ok": false,
                    "type": "orchestration_coordinator_status",
                    "reason_code": "missing_task_id"
                })
            } else {
                let root_dir = payload_root_dir(payload);
                match load_scratchpad(root, &task_id, root_dir.as_deref()) {
                    Ok(loaded) => {
                        let progress = loaded
                            .scratchpad
                            .get("progress")
                            .cloned()
                            .unwrap_or_else(|| json!({ "processed": 0, "total": 0 }));
                        let finding_count = loaded
                            .scratchpad
                            .get("findings")
                            .and_then(Value::as_array)
                            .map(|rows| rows.len())
                            .unwrap_or(0);
                        let checkpoint_count = loaded
                            .scratchpad
                            .get("checkpoints")
                            .and_then(Value::as_array)
                            .map(|rows| rows.len())
                            .unwrap_or(0);
                        json!({
                            "ok": true,
                            "type": "orchestration_coordinator_status",
                            "task_id": task_id,
                            "scratchpad_exists": loaded.exists,
                            "scratchpad_path": loaded.file_path,
                            "progress": progress,
                            "finding_count": finding_count,
                            "checkpoint_count": checkpoint_count
                        })
                    }
                    Err(err) => json!({
                        "ok": false,
                        "type": "orchestration_coordinator_status",
                        "reason_code": err,
                        "task_id": task_id
                    }),
                }
            }
        }
        _ => json!({
            "ok": false,
            "type": "orchestration_invoke",
            "reason_code": format!("unsupported_op:{op}")
        }),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "invoke".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    if command != "invoke" {
        usage();
        let payload = json!({
            "ok": false,
            "type": "orchestration_command",
            "reason_code": format!("unsupported_command:{command}"),
            "commands": ["invoke", "help"]
        });
        print_json_line(&payload);
        return 1;
    }

    let op = parsed
        .flags
        .get("op")
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if op.is_empty() {
        let payload = json!({
            "ok": false,
            "type": "orchestration_invoke",
            "reason_code": "missing_op"
        });
        print_json_line(&payload);
        return 1;
    }

    let payload_raw = parsed
        .flags
        .get("payload-json")
        .or_else(|| parsed.flags.get("payload_json"))
        .cloned()
        .unwrap_or_else(|| "{}".to_string());
    let payload = match serde_json::from_str::<Value>(&payload_raw) {
        Ok(value) => value,
        Err(_) => {
            let out = json!({
                "ok": false,
                "type": "orchestration_invoke",
                "reason_code": "invalid_payload_json"
            });
            print_json_line(&out);
            return 1;
        }
    };

    let out = invoke(root, &op, &payload);
    print_json_line(&out);
    if out.get("ok").and_then(Value::as_bool) == Some(true) {
        0
    } else {
        1
    }
}
