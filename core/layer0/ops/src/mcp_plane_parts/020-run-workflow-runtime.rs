fn run_workflow_runtime(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DURABLE_WORKFLOW_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "durable_mcp_workflow_contract",
            "max_retries": 5,
            "checkpoint_relpath": "workflow_runtime/state.json"
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("durable_workflow_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "durable_mcp_workflow_contract"
    {
        errors.push("durable_workflow_contract_kind_invalid".to_string());
    }
    let op = clean(
        parsed
            .flags
            .get("op")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_else(|| "status".to_string()),
        32,
    )
    .to_ascii_lowercase();
    if !matches!(
        op.as_str(),
        "start" | "pause" | "resume" | "retry" | "status"
    ) {
        errors.push("workflow_op_invalid".to_string());
    }
    let workflow_id = clean(
        parsed
            .flags
            .get("workflow-id")
            .cloned()
            .unwrap_or_else(|| "wf-default".to_string()),
        120,
    );
    if workflow_id.is_empty() {
        errors.push("workflow_id_required".to_string());
    }
    let max_retries = contract
        .get("max_retries")
        .and_then(Value::as_u64)
        .unwrap_or(5);
    let checkpoint_relpath = clean(
        contract
            .get("checkpoint_relpath")
            .and_then(Value::as_str)
            .unwrap_or("workflow_runtime/state.json"),
        200,
    );
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_workflow_runtime",
            "errors": errors
        });
    }

    let state_path = state_root(root).join(checkpoint_relpath);
    let mut state = read_json(&state_path).unwrap_or_else(|| {
        json!({
            "version": "v1",
            "kind": "mcp_workflow_runtime_state",
            "workflows": {}
        })
    });
    if !state
        .get("workflows")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        state["workflows"] = Value::Object(Map::new());
    }
    let workflows = state
        .get("workflows")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut current = workflows.get(&workflow_id).cloned().unwrap_or_else(|| {
        json!({
            "workflow_id": workflow_id,
            "status": "idle",
            "step": 0_u64,
            "retry_count": 0_u64,
            "resume_count": 0_u64,
            "last_event_hash": "genesis"
        })
    });

    let status_before = current
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("idle")
        .to_string();
    let mut apply_state = false;
    match op.as_str() {
        "status" => {}
        "start" => {
            current["status"] = Value::String("running".to_string());
            current["step"] = json!(current.get("step").and_then(Value::as_u64).unwrap_or(0) + 1);
            current["updated_at"] = Value::String(now_iso());
            if let Some(checkpoint) = load_checkpoint(root, parsed) {
                current["checkpoint"] = checkpoint;
            }
            apply_state = true;
        }
        "pause" => {
            if strict && status_before != "running" {
                errors.push("workflow_pause_requires_running_status".to_string());
            } else {
                current["status"] = Value::String("paused".to_string());
                current["pause_count"] = json!(
                    current
                        .get("pause_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        + 1
                );
                current["updated_at"] = Value::String(now_iso());
                apply_state = true;
            }
        }
        "resume" => {
            if strict && status_before != "paused" {
                errors.push("workflow_resume_requires_paused_status".to_string());
            } else {
                current["status"] = Value::String("running".to_string());
                current["resume_count"] = json!(
                    current
                        .get("resume_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        + 1
                );
                current["updated_at"] = Value::String(now_iso());
                apply_state = true;
            }
        }
        "retry" => {
            let retries = current
                .get("retry_count")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if strict && retries >= max_retries {
                errors.push("workflow_retry_limit_exceeded".to_string());
            } else {
                current["status"] = Value::String("running".to_string());
                current["retry_count"] = json!(retries + 1);
                current["last_retry_reason"] = Value::String(clean(
                    parsed
                        .flags
                        .get("reason")
                        .cloned()
                        .unwrap_or_else(|| "operator_retry".to_string()),
                    240,
                ));
                current["updated_at"] = Value::String(now_iso());
                apply_state = true;
            }
        }
        _ => {}
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_workflow_runtime",
            "errors": errors
        });
    }

    let mut event = Value::Null;
    if apply_state {
        let prev_hash = current
            .get("last_event_hash")
            .and_then(Value::as_str)
            .unwrap_or("genesis")
            .to_string();
        let event_payload = json!({
            "workflow_id": workflow_id,
            "op": op,
            "status": current.get("status").and_then(Value::as_str).unwrap_or("idle"),
            "step": current.get("step").and_then(Value::as_u64).unwrap_or(0),
            "retry_count": current.get("retry_count").and_then(Value::as_u64).unwrap_or(0),
            "resume_count": current.get("resume_count").and_then(Value::as_u64).unwrap_or(0),
            "ts": now_iso()
        });
        let chain_hash = sha256_hex_str(&format!(
            "{}:{}",
            prev_hash,
            canonical_json_string(&event_payload)
        ));
        current["last_event_hash"] = Value::String(chain_hash.clone());
        event = json!({
            "event": event_payload,
            "prev_hash": prev_hash,
            "chain_hash": chain_hash
        });
        let _ = append_jsonl(
            &state_root(root)
                .join("workflow_runtime")
                .join("history.jsonl"),
            &event,
        );
    }

    let mut workflows_next = state
        .get("workflows")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    workflows_next.insert(workflow_id.clone(), current.clone());
    state["workflows"] = Value::Object(workflows_next);
    let _ = write_json(&state_path, &state);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "mcp_plane_workflow_runtime",
        "lane": "core/layer0/ops",
        "workflow_id": workflow_id,
        "op": op,
        "state_path": state_path.display().to_string(),
        "workflow": current,
        "event": event,
        "claim_evidence": [
            {
                "id": "V6-MCP-001.2",
                "claim": "durable_mcp_workflow_execution_supports_pause_resume_retry_with_policy_scoped_checkpointing_and_deterministic_recovery_receipts",
                "evidence": {
                    "op": op,
                    "state_path": state_path.display().to_string()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_expose(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        EXPOSURE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mcp_exposure_contract",
            "default_max_rps": 10,
            "max_tools": 16
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mcp_exposure_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mcp_exposure_contract"
    {
        errors.push("mcp_exposure_contract_kind_invalid".to_string());
    }
    let agent = clean(
        parsed
            .flags
            .get("agent")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if agent.is_empty() {
        errors.push("agent_required".to_string());
    }
    let tools = parse_csv_flag(&parsed.flags, "tools", 120);
    let max_tools = contract
        .get("max_tools")
        .and_then(Value::as_u64)
        .unwrap_or(16) as usize;
    if strict && tools.len() > max_tools {
        errors.push("tool_limit_exceeded".to_string());
    }
    let max_rps = parse_u64(
        parsed.flags.get("max-rps"),
        contract
            .get("default_max_rps")
            .and_then(Value::as_u64)
            .unwrap_or(10),
    )
    .clamp(1, 50_000);
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_expose",
            "errors": errors
        });
    }

    let exposure_id = format!(
        "mcp_{}",
        &sha256_hex_str(&format!("{}:{}:{}", agent, tools.join(","), max_rps))[..16]
    );
    let mut registry =
        read_json(&state_root(root).join("exposure_registry.json")).unwrap_or_else(|| {
            json!({
                "version": "v1",
                "kind": "mcp_exposure_registry",
                "entries": {}
            })
        });
    if !registry
        .get("entries")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        registry["entries"] = Value::Object(Map::new());
    }
    let mut entries = registry
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let entry = json!({
        "id": exposure_id,
        "agent": agent,
        "tools": tools,
        "max_rps": max_rps,
        "created_at": now_iso(),
        "endpoint": format!("mcp://{}", exposure_id)
    });
    entries.insert(exposure_id.clone(), entry.clone());
    registry["entries"] = Value::Object(entries);
    let registry_path = state_root(root).join("exposure_registry.json");
    let _ = write_json(&registry_path, &registry);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "mcp_plane_expose",
        "lane": "core/layer0/ops",
        "registry_path": registry_path.display().to_string(),
        "exposed": entry,
        "claim_evidence": [
            {
                "id": "V6-MCP-001.3",
                "claim": "agent_or_workflow_can_be_exposed_as_an_mcp_server_with_bounded_exposure_controls",
                "evidence": {
                    "exposure_id": exposure_id,
                    "max_rps": max_rps
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

