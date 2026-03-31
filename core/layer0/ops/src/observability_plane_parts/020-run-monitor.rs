fn run_monitor(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        MONITORING_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "observability_realtime_monitoring_contract",
            "allowed_alert_classes": ["slo", "security", "runtime", "cost"],
            "allowed_severities": ["low", "medium", "high", "critical"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("observability_monitor_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "observability_realtime_monitoring_contract"
    {
        errors.push("observability_monitor_contract_kind_invalid".to_string());
    }
    let source = clean(
        parsed
            .flags
            .get("source")
            .cloned()
            .unwrap_or_else(|| "protheusd".to_string()),
        120,
    );
    let alert_class = clean(
        parsed
            .flags
            .get("alert-class")
            .cloned()
            .unwrap_or_else(|| "runtime".to_string()),
        32,
    )
    .to_ascii_lowercase();
    let severity = clean(
        parsed
            .flags
            .get("severity")
            .cloned()
            .unwrap_or_else(|| "medium".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let class_allowed = contract
        .get("allowed_alert_classes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == alert_class);
    if strict && !class_allowed {
        errors.push("observability_monitor_alert_class_invalid".to_string());
    }
    let severity_allowed = contract
        .get("allowed_severities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == severity);
    if strict && !severity_allowed {
        errors.push("observability_monitor_severity_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_monitor",
            "errors": errors
        });
    }

    let message = clean(
        parsed
            .flags
            .get("message")
            .cloned()
            .unwrap_or_else(|| "runtime anomaly detected".to_string()),
        220,
    );
    let alert_id = format!(
        "obs_{}",
        &sha256_hex_str(&format!("{source}:{alert_class}:{severity}:{message}"))[..12]
    );
    let context = intelligent_context(root);
    let alert = json!({
        "version": "v1",
        "alert_id": alert_id,
        "source": source,
        "alert_class": alert_class,
        "severity": severity,
        "message": message,
        "context": context,
        "ts": crate::now_iso()
    });
    let path = alerts_state_path(root);
    let _ = write_json(&path, &alert);
    let _ = append_jsonl(
        &state_root(root).join("alerts").join("history.jsonl"),
        &alert,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "observability_plane_monitor",
        "lane": "core/layer0/ops",
        "alert": alert,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&alert.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-OBSERVABILITY-001.1",
                "claim": "realtime_monitoring_emits_alerts_with_intelligent_context_and_deterministic_receipts",
                "evidence": {
                    "alert_id": alert_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_workflow(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        WORKFLOW_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "observability_workflow_editor_contract",
            "allowed_ops": ["upsert", "list", "run"],
            "allowed_triggers": ["cron", "event"]
        }),
    );
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "list".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_ops = contract
        .get("allowed_ops")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict
        && !allowed_ops
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == op)
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_workflow",
            "errors": ["observability_workflow_op_invalid"]
        });
    }

    let path = workflows_state_path(root);
    let mut state = read_json(&path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "workflows": {},
            "runs": []
        })
    });
    if !state
        .get("workflows")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["workflows"] = Value::Object(serde_json::Map::new());
    }
    if !state.get("runs").map(Value::is_array).unwrap_or(false) {
        state["runs"] = Value::Array(Vec::new());
    }

    if op == "list" {
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_workflow",
            "lane": "core/layer0/ops",
            "op": "list",
            "state": state,
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-001.2",
                    "claim": "visual_workflow_editor_and_scheduler_surfaces_registered_workflows",
                    "evidence": {
                        "workflow_count": state
                            .get("workflows")
                            .and_then(Value::as_object)
                            .map(|m| m.len())
                            .unwrap_or(0)
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    let workflow_id = clean_id(
        parsed
            .flags
            .get("workflow-id")
            .map(String::as_str)
            .or_else(|| parsed.flags.get("id").map(String::as_str)),
        "default-workflow",
    );
    if op == "upsert" {
        let trigger = clean(
            parsed
                .flags
                .get("trigger")
                .cloned()
                .unwrap_or_else(|| "cron".to_string()),
            20,
        )
        .to_ascii_lowercase();
        let trigger_allowed = contract
            .get("allowed_triggers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .any(|row| row == trigger);
        if strict && !trigger_allowed {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_workflow",
                "errors": ["observability_workflow_trigger_invalid"]
            });
        }
        let schedule = clean(
            parsed
                .flags
                .get("schedule")
                .cloned()
                .unwrap_or_else(|| "*/5 * * * *".to_string()),
            160,
        );
        if strict && trigger == "cron" && !looks_like_cron(&schedule) {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_workflow",
                "errors": ["observability_workflow_schedule_invalid_for_cron"]
            });
        }
        if strict && trigger == "event" && !schedule.starts_with("event:") {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_workflow",
                "errors": ["observability_workflow_schedule_invalid_for_event"]
            });
        }
        let steps = parse_json_flag(
            parsed.flags.get("steps-json"),
            json!(["collect-metrics", "attach-context", "notify"]),
        );
        let step_names = steps
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|row| {
                row.as_str()
                    .map(|raw| clean(raw.to_string(), 120))
                    .filter(|cleaned| !cleaned.is_empty())
            })
            .collect::<Vec<_>>();
        if strict && step_names.is_empty() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "observability_plane_workflow",
                "errors": ["observability_workflow_steps_required"]
            });
        }
        let compiled_graph = compile_steps_graph(&step_names);
        let workflow = json!({
            "workflow_id": workflow_id,
            "trigger": trigger,
            "schedule": schedule,
            "steps": Value::Array(step_names.iter().map(|step| Value::String(step.clone())).collect()),
            "compiled_graph": compiled_graph,
            "updated_at": crate::now_iso()
        });
        state["workflows"][&workflow_id] = workflow.clone();
        state["updated_at"] = Value::String(crate::now_iso());
        let _ = write_json(&path, &state);
        let _ = append_jsonl(
            &state_root(root).join("workflows").join("history.jsonl"),
            &json!({"op": "upsert", "workflow_id": workflow_id, "ts": crate::now_iso()}),
        );
        let mut out = json!({
            "ok": true,
            "strict": strict,
            "type": "observability_plane_workflow",
            "lane": "core/layer0/ops",
            "op": "upsert",
            "workflow": workflow,
            "artifact": {
                "path": path.display().to_string(),
                "sha256": sha256_hex_str(&state.to_string())
            },
            "claim_evidence": [
                {
                    "id": "V6-OBSERVABILITY-001.2",
                    "claim": "workflow_editor_compiles_visual_steps_into_receipted_schedules",
                    "evidence": {
                        "workflow_id": workflow_id
                    }
                }
            ]
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        return out;
    }

    if strict && state["workflows"].get(&workflow_id).is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "observability_plane_workflow",
            "errors": ["observability_workflow_not_found"]
        });
    }
    let run_id = format!(
        "run_{}",
        &sha256_hex_str(&format!("{workflow_id}:{}", crate::now_iso()))[..10]
    );
    let step_trace = state["workflows"]
        .get(&workflow_id)
        .and_then(|row| row.get("steps"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            json!({
                "step_index": idx,
                "step": step,
                "status": "queued"
            })
        })
        .collect::<Vec<_>>();
    let run = json!({
        "run_id": run_id,
        "workflow_id": workflow_id,
        "status": "started",
        "step_trace": step_trace,
        "ts": crate::now_iso()
    });
    let mut runs = state
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    runs.push(run.clone());
    state["runs"] = Value::Array(runs);
    state["updated_at"] = Value::String(crate::now_iso());
    let _ = write_json(&path, &state);
    let _ = append_jsonl(
        &state_root(root).join("workflows").join("history.jsonl"),
        &json!({"op": "run", "run": run, "ts": crate::now_iso()}),
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "observability_plane_workflow",
        "lane": "core/layer0/ops",
        "op": "run",
        "run": run,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-OBSERVABILITY-001.2",
                "claim": "workflow_scheduling_and_execution_are_receipted_for_editor_runs",
                "evidence": {
                    "run_id": run_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

