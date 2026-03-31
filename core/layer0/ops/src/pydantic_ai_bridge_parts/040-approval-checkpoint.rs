fn approval_checkpoint(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let queue_path = approval_queue_path(root, argv, payload);
    let action_id = clean_token(payload.get("action_id").and_then(Value::as_str), "");
    let decision = clean_token(payload.get("decision").and_then(Value::as_str), "pending");
    let result = if action_id.is_empty() || decision == "pending" {
        let new_action_id = if action_id.is_empty() {
            stable_id(
                "pydaiapproval",
                &json!({
                    "tool_id": payload.get("tool_id"),
                    "summary": payload.get("summary"),
                    "risk": payload.get("risk")
                }),
            )
        } else {
            action_id.clone()
        };
        let action_envelope = json!({
            "action_id": new_action_id,
            "directive_id": "pydantic-ai-bridge",
            "type": "tool_invocation",
            "summary": clean_text(payload.get("summary").and_then(Value::as_str), 200),
            "payload_pointer": clean_text(payload.get("tool_id").and_then(Value::as_str), 160),
        });
        let queue_payload = json!({
            "action_envelope": action_envelope,
            "reason": clean_text(payload.get("reason").and_then(Value::as_str), 200),
        });
        let encoded = BASE64_STANDARD.encode(encode_json_arg(&queue_payload)?.as_bytes());
        let exit = crate::approval_gate_kernel::run(
            root,
            &[
                "queue".to_string(),
                format!("--payload-base64={encoded}"),
                format!("--queue-path={}", queue_path.display()),
            ],
        );
        if exit != 0 {
            return Err("pydantic_ai_approval_queue_failed".to_string());
        }
        json!({
            "action_id": new_action_id,
            "decision": "pending",
            "status": approval_status_from_queue(&queue_path, &new_action_id),
        })
    } else {
        let args = if decision == "approve" {
            vec![
                "approve".to_string(),
                format!("--action-id={action_id}"),
                format!("--queue-path={}", queue_path.display()),
            ]
        } else {
            vec![
                "deny".to_string(),
                format!("--action-id={action_id}"),
                format!(
                    "--reason={}",
                    clean_text(payload.get("reason").and_then(Value::as_str), 120)
                ),
                format!("--queue-path={}", queue_path.display()),
            ]
        };
        let exit = crate::approval_gate_kernel::run(root, &args);
        if exit != 0 {
            return Err(format!("pydantic_ai_approval_{}_failed", decision));
        }
        json!({
            "action_id": action_id,
            "decision": decision,
            "status": approval_status_from_queue(&queue_path, &action_id),
        })
    };
    let action_id = result
        .get("action_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "approval_records").insert(
        action_id.clone(),
        json!({
            "action_id": action_id,
            "tool_id": payload.get("tool_id").cloned().unwrap_or(Value::Null),
            "queue_path": rel(root, &queue_path),
            "status": result.get("status").cloned().unwrap_or(Value::Null),
            "updated_at": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "approval": result,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.6", pydantic_claim("V6-WORKFLOW-015.6")),
    }))
}

fn rewind_session(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(payload.get("session_id").and_then(Value::as_str), "");
    if session_id.is_empty() {
        return Err("pydantic_ai_rewind_session_id_required".to_string());
    }
    let snapshot = state
        .get("session_snapshots")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .ok_or_else(|| format!("pydantic_ai_snapshot_missing:{session_id}"))?;
    let context_payload = snapshot
        .get("context_payload")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let context_json = encode_json_arg(&context_payload)?;
    let exit = crate::swarm_runtime::run(
        root,
        &[
            "sessions".to_string(),
            "context-put".to_string(),
            format!("--session-id={session_id}"),
            format!("--context-json={context_json}"),
            "--merge=0".to_string(),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if exit != 0 {
        return Err("pydantic_ai_rewind_context_restore_failed".to_string());
    }
    emit_native_trace(
        root,
        &clean_token(
            snapshot.get("snapshot_id").and_then(Value::as_str),
            "pydantic-ai-rewind",
        ),
        "pydantic_ai_rewind",
        &format!("rewound session {session_id}"),
    )?;
    Ok(json!({
        "ok": true,
        "restored": {
            "session_id": session_id,
            "snapshot": snapshot,
            "swarm_state_path": rel(root, &swarm_state_path),
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.5", pydantic_claim("V6-WORKFLOW-015.5")),
    }))
}

fn record_evaluation(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(
        payload.get("session_id").and_then(Value::as_str),
        "pydantic-ai-session",
    );
    let metrics = payload.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let evaluation = json!({
        "evaluation_id": stable_id("pydaieval", &json!({"session_id": session_id, "metrics": metrics})),
        "session_id": session_id,
        "metrics": metrics,
        "score": parse_f64_value(payload.get("score"), 0.0, 0.0, 1.0),
        "profile": clean_token(payload.get("profile").and_then(Value::as_str), "rich"),
        "evaluated_at": now_iso(),
    });
    let evaluation_id = evaluation
        .get("evaluation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "evaluations").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "pydantic_ai_eval",
        &format!(
            "session_id={} score={:.2}",
            session_id,
            evaluation
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": evaluation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.10", pydantic_claim("V6-WORKFLOW-015.10")),
    }))
}

fn sandbox_execute(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("pydantic_ai_sandbox_language_invalid".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let cloud = clean_token(payload.get("cloud").and_then(Value::as_str), "gcp");
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/pydantic_ai_protocol_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("pydantic_ai_sandbox_bridge_must_be_adapter_owned".to_string());
    }
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && !cloud.is_empty();
    let reason_code = if degraded {
        "cloud_integration_requires_rich_profile"
    } else {
        let tier = clean_token(payload.get("tier").and_then(Value::as_str), "wasm");
        let escape_attempt = parse_bool_value(payload.get("escape_attempt"), false);
        let exit = crate::canyon_plane::run(
            root,
            &[
                "sandbox".to_string(),
                "--op=run".to_string(),
                format!("--tier={tier}"),
                format!("--language={language}"),
                format!(
                    "--fuel={}",
                    parse_u64_value(payload.get("fuel"), 2000, 200, 50000)
                ),
                format!(
                    "--epoch={}",
                    parse_u64_value(payload.get("epoch"), 4, 1, 64)
                ),
                format!("--escape-attempt={}", if escape_attempt { 1 } else { 0 }),
                "--strict=1".to_string(),
            ],
        );
        if exit != 0 {
            return Err("pydantic_ai_sandbox_execution_failed".to_string());
        }
        "sandbox_ok"
    };
    let record = json!({
        "sandbox_id": stable_id("pydaisbx", &json!({"language": language, "profile": profile, "cloud": cloud})),
        "language": language,
        "profile": profile,
        "cloud": cloud,
        "bridge_path": bridge_path,
        "degraded": degraded,
        "reason_code": reason_code,
        "executed_at": now_iso(),
    });
    let sandbox_id = record
        .get("sandbox_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "sandbox_runs").insert(sandbox_id, record.clone());
    Ok(json!({
        "ok": true,
        "sandbox": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.4", pydantic_claim("V6-WORKFLOW-015.4")),
    }))
}

fn deploy_shell(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_shell_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/pydantic_ai_bridge.ts"),
    )?;
    let target = clean_token(payload.get("target").and_then(Value::as_str), "local");
    let record = json!({
        "deployment_id": stable_id("pydaidep", &json!({"shell_path": shell_path, "target": target})),
        "shell_name": clean_token(payload.get("shell_name").and_then(Value::as_str), "pydantic-ai-shell"),
        "shell_path": shell_path,
        "target": target,
        "deletable": true,
        "authority_delegate": "core://pydantic-ai-bridge",
        "artifact_path": clean_text(payload.get("artifact_path").and_then(Value::as_str), 240),
        "deployed_at": now_iso(),
    });
    let deployment_id = record
        .get("deployment_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "deployments").insert(deployment_id, record.clone());
    Ok(json!({
        "ok": true,
        "deployment": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.4", pydantic_claim("V6-WORKFLOW-015.4")),
    }))
}

fn register_agent(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut normalized = payload.clone();
    normalized
        .entry("bridge_path".to_string())
        .or_insert_with(|| json!("adapters/protocol/pydantic_ai_protocol_bridge.ts"));
    normalized
        .entry("language".to_string())
        .or_insert_with(|| json!("python"));
    let response = register_a2a_agent(root, state, &normalized)?;
    let agent = response
        .get("agent")
        .cloned()
        .ok_or_else(|| "pydantic_ai_agent_receipt_missing".to_string())?;
    let agent_id = clean_token(agent.get("agent_id").and_then(Value::as_str), "");
    if agent_id.is_empty() {
        return Err("pydantic_ai_agent_id_missing".to_string());
    }
    let typed_record = json!({
        "agent_id": agent_id,
        "name": agent.get("name").cloned().unwrap_or(Value::Null),
        "language": agent.get("language").cloned().unwrap_or(Value::Null),
        "bridge_path": agent.get("bridge_path").cloned().unwrap_or(Value::Null),
        "input_required": parse_string_list(payload.get("input_required")),
        "output_required": parse_string_list(payload.get("output_required")),
        "dependencies": parse_string_list(payload.get("dependencies")),
        "dependency_schema": payload.get("dependency_schema").cloned().unwrap_or_else(|| json!({})),
        "output_schema": payload.get("output_schema").cloned().unwrap_or_else(|| json!({})),
        "supported_profiles": agent.get("supported_profiles").cloned().unwrap_or_else(|| json!(["rich"])),
        "registered_at": now_iso(),
    });
    as_object_mut(state, "typed_agents").insert(agent_id.clone(), typed_record.clone());
    Ok(json!({
        "ok": true,
        "agent": typed_record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.1", pydantic_claim("V6-WORKFLOW-015.1")),
    }))
}

fn validate_output(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let agent_id = clean_token(payload.get("agent_id").and_then(Value::as_str), "");
    let data = payload.get("data").cloned().unwrap_or_else(|| json!({}));
    let data_obj = data.as_object().cloned().unwrap_or_default();
    let mut required_fields = parse_string_list(payload.get("required_fields"));
    if required_fields.is_empty() && !agent_id.is_empty() {
        if let Some(agent) = state
            .get("typed_agents")
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(&agent_id))
        {
            required_fields = parse_string_list(agent.get("output_required"));
        }
    }
    let missing_fields = required_fields
        .iter()
        .filter(|field| !data_obj.contains_key(*field))
        .cloned()
        .collect::<Vec<_>>();
    let attempt = parse_u64_value(payload.get("attempt"), 1, 1, 16);
    let max_retries = parse_u64_value(payload.get("max_retries"), 1, 0, 8);
    let nested_depth = parse_u64_value(payload.get("nested_depth"), data_obj.len() as u64, 0, 32);
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && nested_depth > 3;
    let status = if missing_fields.is_empty() {
        "accepted"
    } else if attempt <= max_retries {
        "retry"
    } else {
        "reject"
    };
    let record = json!({
        "validation_id": stable_id("pydaival", &json!({"agent_id": agent_id, "required_fields": required_fields, "data": data})),
        "agent_id": if agent_id.is_empty() { Value::Null } else { json!(agent_id) },
        "required_fields": required_fields,
        "missing_fields": missing_fields,
        "attempt": attempt,
        "max_retries": max_retries,
        "profile": profile,
        "degraded": degraded,
        "reason_code": if degraded { "profile_nested_validation_limited" } else { "validation_profile_ok" },
        "status": status,
        "validated_at": now_iso(),
    });
    let validation_id = clean_token(record.get("validation_id").and_then(Value::as_str), "");
    as_object_mut(state, "structured_validations").insert(validation_id, record.clone());
    Ok(json!({
        "ok": missing_fields.is_empty(),
        "validation": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.2", pydantic_claim("V6-WORKFLOW-015.2")),
    }))
}

fn register_tool_context(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut normalized = payload.clone();
    normalized
        .entry("bridge_path".to_string())
        .or_insert_with(|| json!("adapters/protocol/pydantic_ai_protocol_bridge.ts"));
    let response = register_tool_manifest(root, state, &normalized)?;
    let tool_id = response
        .get("tool")
        .and_then(|row| row.get("tool_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| "pydantic_ai_tool_id_missing".to_string())?
        .to_string();
    let required_args = parse_string_list(payload.get("required_args"));
    let required_dependencies = parse_string_list(payload.get("required_dependencies"));
    let updated = {
        let tool = as_object_mut(state, "tool_manifests")
            .get_mut(&tool_id)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("unknown_pydantic_ai_tool:{tool_id}"))?;
        tool.insert(
            "dependency_context".to_string(),
            payload
                .get("dependency_context")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        tool.insert("required_args".to_string(), json!(required_args));
        tool.insert(
            "required_dependencies".to_string(),
            json!(required_dependencies),
        );
        tool.insert(
            "argument_schema".to_string(),
            payload
                .get("argument_schema")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        Value::Object(tool.clone())
    };
    Ok(json!({
        "ok": true,
        "tool_context": updated,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.3", pydantic_claim("V6-WORKFLOW-015.3")),
    }))
}

