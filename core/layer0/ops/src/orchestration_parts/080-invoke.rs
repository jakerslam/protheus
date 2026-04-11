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
        "taskgroup.path" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let root_dir = payload_root_dir(payload);
            match taskgroup_path(root, &task_group_id, root_dir.as_deref()) {
                Ok(file_path) => json!({
                    "ok": true,
                    "type": "orchestration_taskgroup_path",
                    "task_group_id": task_group_id.to_ascii_lowercase(),
                    "file_path": file_path
                }),
                Err(err) => json!({
                    "ok": false,
                    "type": "orchestration_taskgroup_path",
                    "reason_code": err,
                    "task_group_id": task_group_id.to_ascii_lowercase()
                }),
            }
        }
        "taskgroup.ensure" => {
            let root_dir = payload_root_dir(payload);
            ensure_task_group(root, payload, root_dir.as_deref())
        }
        "taskgroup.query" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let root_dir = payload_root_dir(payload);
            query_task_group(root, &task_group_id, root_dir.as_deref())
        }
        "taskgroup.update_status" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let agent_id = get_string_any(payload, &["agent_id", "agentId"]);
            let status = get_string_any(payload, &["status"]);
            let details = payload
                .get("details")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let root_dir = payload_root_dir(payload);
            update_agent_status(
                root,
                &task_group_id,
                &agent_id,
                &status,
                &details,
                root_dir.as_deref(),
            )
        }
        "completion.status" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let root_dir = payload_root_dir(payload);
            ensure_and_summarize(root, &task_group_id, root_dir.as_deref())
        }
        "completion.track" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let update = payload
                .get("update")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            let root_dir = payload_root_dir(payload);
            track_agent_completion(root, &task_group_id, &update, root_dir.as_deref())
        }
        "completion.batch" => {
            let task_group_id = get_string_any(payload, &["task_group_id", "taskGroupId", "id"]);
            let updates = array_from_payload_value(
                payload
                    .get("updates")
                    .or_else(|| payload.get("updates_json"))
                    .or_else(|| payload.get("updatesJson")),
            );
            let root_dir = payload_root_dir(payload);
            track_batch_completion(root, &task_group_id, &updates, root_dir.as_deref())
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
        "coordinator.run" => run_coordinator(root, payload),
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
