fn findings_by_agent(findings: &[Value]) -> BTreeMap<String, Vec<Value>> {
    let mut out: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for finding in findings {
        let agent_id = {
            let direct = to_clean_string(finding.get("agent_id"));
            if !direct.is_empty() {
                direct
            } else {
                finding
                    .get("metadata")
                    .and_then(Value::as_object)
                    .and_then(|meta| meta.get("agent_id"))
                    .map(|value| to_clean_string(Some(value)))
                    .unwrap_or_default()
            }
        };
        if agent_id.is_empty() {
            continue;
        }
        out.entry(agent_id).or_default().push(finding.clone());
    }
    out
}

fn run_coordinator(root: &Path, input: &Value) -> Value {
    let task_id = get_string_any(input, &["task_id"]);
    if task_id.is_empty() {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": "missing_task_id"
        });
    }

    let audit_id = {
        let explicit = get_string_any(input, &["audit_id"]);
        if explicit.is_empty() {
            format!("audit-{}", stable_hash_short(&task_id))
        } else {
            explicit
        }
    };

    let items = input
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let findings = input
        .get("findings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scopes = input
        .get("scopes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let agent_count = get_i64_any(input, &["agent_count"], 1).max(1);

    let root_dir_string = get_string_any(input, &["root_dir", "rootDir"]);
    let root_dir = if root_dir_string.is_empty() {
        None
    } else {
        Some(root_dir_string.as_str())
    };

    let scope_check = detect_scope_overlaps(&scopes);
    if scope_check.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": scope_check.get("reason_code").cloned().unwrap_or(Value::String("scope_overlap_detected".to_string())),
            "overlaps": scope_check.get("overlaps").cloned().unwrap_or(Value::Array(Vec::new())),
            "scope_id": scope_check.get("scope_id").cloned().unwrap_or(Value::Null)
        });
    }

    let normalized_scopes = scope_check
        .get("normalized_scopes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let partitions =
        assign_scopes_to_partitions(&partition_work(&items, agent_count), &normalized_scopes);
    let scope_by_agent = scope_map_by_agent(&partitions);

    let task_type = {
        let value = get_string_any(input, &["task_type"]);
        if value.is_empty() {
            "audit".to_string()
        } else {
            value
        }
    };
    let coordinator_session = {
        let session = get_string_any(input, &["coordinator_session"]);
        if session.is_empty() {
            Value::Null
        } else {
            Value::String(session)
        }
    };
    let agents = partitions
        .iter()
        .map(|partition| {
            json!({
                "agent_id": partition.get("agent_id").cloned().unwrap_or(Value::Null),
                "status": "running",
                "details": {
                    "scope_id": partition.get("scope").and_then(|scope| scope.get("scope_id")).cloned().unwrap_or(Value::Null)
                }
            })
        })
        .collect::<Vec<_>>();

    let task_group = ensure_task_group(
        root,
        &json!({
            "task_group_id": get_string_any(input, &["task_group_id"]),
            "task_type": task_type,
            "coordinator_session": coordinator_session,
            "agent_count": partitions.len() as i64,
            "agents": agents
        }),
        root_dir,
    );

    if task_group.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": task_group.get("reason_code").cloned().unwrap_or(Value::String("task_group_creation_failed".to_string()))
        });
    }

    let findings_with_audit = findings
        .iter()
        .map(|finding| {
            if let Value::Object(map) = finding {
                let mut row = map.clone();
                row.insert("audit_id".to_string(), Value::String(audit_id.clone()));
                Value::Object(row)
            } else {
                json!({ "audit_id": audit_id })
            }
        })
        .collect::<Vec<_>>();

    let filtered = apply_scope_filtering(&findings_with_audit, &scope_by_agent);
    let merged = merge_findings(
        &filtered
            .get("kept")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let kept_findings = filtered
        .get("kept")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let findings_by_agent = findings_by_agent(&kept_findings);

    let updated_progress = json!({
        "processed": merged
            .get("merged")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0),
        "total": items.len()
    });

    let write_progress = write_scratchpad(
        root,
        &task_id,
        &json!({ "progress": updated_progress }),
        root_dir,
    );
    if write_progress.is_err() {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": write_progress.err().unwrap_or_else(|| "scratchpad_write_failed".to_string())
        });
    }

    let merged_findings = merged
        .get("merged")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for finding in &merged_findings {
        let out = append_finding(
            root,
            &task_id,
            &json!({
                "audit_id": audit_id,
                "item_id": finding.get("item_id").cloned().unwrap_or(Value::Null),
                "severity": finding.get("severity").cloned().unwrap_or(Value::Null),
                "status": finding.get("status").cloned().unwrap_or(Value::Null),
                "location": finding.get("location").cloned().unwrap_or(Value::Null),
                "evidence": finding.get("evidence").cloned().unwrap_or(Value::Array(Vec::new())),
                "timestamp": finding.get("timestamp").cloned().unwrap_or(Value::String(now_iso())),
                "summary": finding.get("summary").cloned().unwrap_or(Value::Null),
                "agent_id": finding.get("agent_id").cloned().unwrap_or(Value::Null),
                "metadata": finding.get("metadata").cloned().unwrap_or(Value::Null)
            }),
            root_dir,
        );
        if out.get("ok").and_then(Value::as_bool) != Some(true) {
            return json!({
                "ok": false,
                "type": "orchestration_coordinator",
                "reason_code": out.get("reason_code").cloned().unwrap_or(Value::String("append_finding_failed".to_string())),
                "task_id": task_id,
                "audit_id": audit_id
            });
        }
    }

    let checkpoint = maybe_checkpoint(
        root,
        &task_id,
        &json!({
            "processed_count": updated_progress.get("processed").cloned().unwrap_or(Value::Number(serde_json::Number::from(0))),
            "total_count": updated_progress.get("total").cloned().unwrap_or(Value::Number(serde_json::Number::from(0))),
            "now_ms": Utc::now().timestamp_millis()
        }),
        root_dir,
    );
    if checkpoint.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": checkpoint.get("reason_code").cloned().unwrap_or(Value::String("checkpoint_tick_failed".to_string())),
            "task_id": task_id,
            "audit_id": audit_id
        });
    }

    let completion = track_batch_completion(
        root,
        &to_clean_string(
            task_group
                .get("task_group")
                .and_then(|v| v.get("task_group_id")),
        ),
        &partitions
            .iter()
            .map(|partition| {
                let agent_id = to_clean_string(partition.get("agent_id"));
                let partial_results = findings_by_agent
                    .get(&agent_id)
                    .cloned()
                    .unwrap_or_default();
                json!({
                    "agent_id": partition.get("agent_id").cloned().unwrap_or(Value::Null),
                    "status": "done",
                    "details": {
                        "processed_count": partition
                            .get("items")
                            .and_then(Value::as_array)
                            .map(|rows| rows.len())
                            .unwrap_or(0),
                        "partial_results_count": partial_results.len(),
                        "partial_results": partial_results,
                        "scope_id": partition
                            .get("scope")
                            .and_then(|scope| scope.get("scope_id"))
                            .cloned()
                            .unwrap_or(Value::Null)
                    }
                })
            })
            .collect::<Vec<_>>(),
        root_dir,
    );

    if completion.get("ok").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "type": "orchestration_coordinator",
            "reason_code": completion.get("reason_code").cloned().unwrap_or(Value::String("completion_tracking_failed".to_string())),
            "task_id": task_id,
            "audit_id": audit_id
        });
    }

    json!({
        "ok": true,
        "type": "orchestration_coordinator",
        "task_id": task_id,
        "audit_id": audit_id,
        "task_group_id": task_group.get("task_group").and_then(|v| v.get("task_group_id")).cloned().unwrap_or(Value::Null),
        "partition_count": partitions.len(),
        "partitions": partitions,
        "findings_total": findings.len(),
        "findings_in_scope": filtered.get("kept").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "findings_merged": merged_findings.len(),
        "findings_deduped": get_i64_any(&merged, &["deduped_count"], 0),
        "findings_dropped": merged.get("dropped").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "scope_violation_count": filtered.get("violations").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "scope_violations": filtered.get("violations").cloned().unwrap_or(Value::Array(Vec::new())),
        "checkpoint": checkpoint,
        "completion_summary": completion.get("summary").cloned().unwrap_or(Value::Null),
        "notification": completion.get("notification").cloned().unwrap_or(Value::Null),
        "report": {
            "findings": merged.get("merged").cloned().unwrap_or(Value::Array(Vec::new())),
            "dropped": merged.get("dropped").cloned().unwrap_or(Value::Array(Vec::new()))
        }
    })
}
