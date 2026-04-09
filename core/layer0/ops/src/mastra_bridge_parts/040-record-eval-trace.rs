fn record_eval_trace(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let base = record_evaluation(root, state, payload)?;
    let mut evaluation = base.get("evaluation").cloned().unwrap_or_else(|| json!({}));
    let evaluation_id = clean_token(
        evaluation.get("evaluation_id").and_then(Value::as_str),
        "mastra-eval",
    );
    if let Some(obj) = evaluation.as_object_mut() {
        obj.insert(
            "trace".to_string(),
            payload.get("trace").cloned().unwrap_or_else(|| json!([])),
        );
        obj.insert(
            "token_telemetry".to_string(),
            payload
                .get("token_telemetry")
                .cloned()
                .unwrap_or_else(|| json!({"prompt_tokens": 0, "completion_tokens": 0})),
        );
        obj.insert(
            "log_summary".to_string(),
            json!(clean_text(
                payload.get("log_summary").and_then(Value::as_str),
                240
            )),
        );
    }
    as_object_mut(state, "eval_traces").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "mastra_eval_trace",
        &format!(
            "session_id={} score={:.2}",
            clean_token(
                payload.get("session_id").and_then(Value::as_str),
                "mastra-session"
            ),
            evaluation
                .get("score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
    )?;
    Ok(json!({
        "ok": true,
        "evaluation": evaluation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.6", mastra_claim("V6-WORKFLOW-011.6")),
    }))
}

fn scaffold_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let output_dir = normalize_shell_path(
        root,
        payload
            .get("output_dir")
            .and_then(Value::as_str)
            .unwrap_or("apps/mastra-studio"),
    )?;
    let abs_dir = repo_path(root, &output_dir);
    let src_dir = abs_dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|err| format!("mastra_scaffold_create_dir_failed:{err}"))?;
    let package_name = clean_token(
        payload.get("package_name").and_then(Value::as_str),
        "mastra-assimilation-shell",
    );
    std::fs::write(
        abs_dir.join("package.json"),
        format!(
            "{{\n  \"name\": \"{package_name}\",\n  \"private\": true,\n  \"version\": \"0.1.0\",\n  \"scripts\": {{\n    \"bridge\": \"node ../../client/runtime/systems/workflow/mastra_bridge.ts\"\n  }}\n}}\n"
        ),
    )
    .map_err(|err| format!("mastra_scaffold_package_write_failed:{err}"))?;
    std::fs::write(
        src_dir.join("mastra.graph.ts"),
        "export const mastraGraph = {\n  name: 'mastra-assimilated-graph',\n  bridge: 'client/runtime/systems/workflow/mastra_bridge.ts',\n  steps: [{ id: 'reason', budget: 96 }],\n};\n",
    )
    .map_err(|err| format!("mastra_scaffold_graph_write_failed:{err}"))?;
    std::fs::write(
        abs_dir.join("README.md"),
        "# Mastra Assimilated Shell\n\nThis shell is non-authoritative. All execution delegates to `core://mastra-bridge`.\n",
    )
    .map_err(|err| format!("mastra_scaffold_readme_write_failed:{err}"))?;
    let record = json!({
        "intake_id": stable_id("mastraintake", &json!({"output_dir": output_dir, "package_name": package_name})),
        "output_dir": output_dir,
        "files": [
            format!("{}/package.json", rel(root, &abs_dir)),
            format!("{}/src/mastra.graph.ts", rel(root, &abs_dir)),
            format!("{}/README.md", rel(root, &abs_dir)),
        ],
        "created_at": now_iso(),
    });
    let intake_id = record
        .get("intake_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "intakes").insert(intake_id, record.clone());
    Ok(json!({
        "ok": true,
        "intake": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.9", mastra_claim("V6-WORKFLOW-011.9")),
    }))
}

fn register_tool_manifest(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "mastra-tool");
    let kind = clean_token(payload.get("kind").and_then(Value::as_str), "custom");
    if !allowed_tool_kind(&kind) {
        return Err("mastra_tool_kind_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/mastra_mcp_bridge.ts"),
    )?;
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let openapi_url = clean_text(payload.get("openapi_url").and_then(Value::as_str), 200);
    if kind == "openapi"
        && !(openapi_url.starts_with("https://") || openapi_url.ends_with("openapi.json"))
    {
        return Err("mastra_tool_openapi_url_invalid".to_string());
    }
    if kind == "mcp" {
        let exit = crate::mcp_plane::run(
            root,
            &[
                "capability-matrix".to_string(),
                "--server-capabilities=tools,resources,prompts".to_string(),
                "--strict=1".to_string(),
            ],
        );
        if exit != 0 {
            return Err("mastra_tool_mcp_capability_validation_failed".to_string());
        }
    }
    let record = json!({
        "tool_id": stable_id("mastratool", &json!({"name": name, "kind": kind, "bridge_path": bridge_path})),
        "name": name,
        "kind": kind,
        "bridge_path": bridge_path,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke"),
        "openapi_url": openapi_url,
        "requires_approval": parse_bool_value(payload.get("requires_approval"), false),
        "supported_profiles": supported_profiles,
        "schema": payload.get("schema").cloned().unwrap_or(Value::Null),
        "capabilities": payload.get("capabilities").cloned().unwrap_or_else(|| json!([])),
        "registered_at": now_iso(),
        "invocation_count": 0,
        "fail_closed": true,
    });
    let tool_id = record
        .get("tool_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "mcp_bridges").insert(tool_id, record.clone());
    Ok(json!({
        "ok": true,
        "tool": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn invoke_tool_manifest(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let tool_id = clean_token(payload.get("tool_id").and_then(Value::as_str), "");
    if tool_id.is_empty() {
        return Err("mastra_tool_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let queue_path = approval_queue_path(root, argv, payload);
    let tools = as_object_mut(state, "mcp_bridges");
    let tool = tools
        .get_mut(&tool_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_mastra_tool:{tool_id}"))?;
    let supported_profiles = parse_string_list(tool.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("mastra_tool_profile_unsupported:{profile}"));
    }
    let requires_approval = parse_bool_value(tool.get("requires_approval"), false)
        || parse_bool_value(payload.get("requires_approval"), false);
    if requires_approval {
        let approval_action_id = clean_token(
            payload.get("approval_action_id").and_then(Value::as_str),
            "",
        );
        if approval_action_id.is_empty() {
            return Err("mastra_tool_requires_approval".to_string());
        }
        if !approval_is_approved(&queue_path, &approval_action_id) {
            return Err("mastra_tool_approval_not_granted".to_string());
        }
    }
    let kind = clean_token(tool.get("kind").and_then(Value::as_str), "custom");
    let args = payload.get("args").cloned().unwrap_or_else(|| json!({}));
    let invocation = match kind.as_str() {
        "openapi" => json!({
            "mode": "openapi_request",
            "target": tool.get("openapi_url").cloned().unwrap_or(Value::Null),
            "method": payload.get("method").cloned().unwrap_or_else(|| json!("POST")),
            "path": payload.get("path").cloned().unwrap_or_else(|| json!("/invoke")),
            "body": args,
        }),
        "mcp" => json!({
            "mode": "mcp_tool_call",
            "tool": tool.get("name").cloned().unwrap_or_else(|| json!("tool")),
            "arguments": args,
        }),
        "native" => json!({
            "mode": "native_call",
            "entrypoint": tool.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": args,
        }),
        _ => json!({
            "mode": "custom_function",
            "entrypoint": tool.get("entrypoint").cloned().unwrap_or_else(|| json!("invoke")),
            "arguments": args,
        }),
    };
    let invocation_count = tool
        .get("invocation_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    tool.insert("invocation_count".to_string(), json!(invocation_count));
    tool.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "tool_id": tool_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

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
                "mastraapproval",
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
            "directive_id": "mastra-bridge",
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
            return Err("mastra_approval_queue_failed".to_string());
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
            return Err(format!("mastra_approval_{}_failed", decision));
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
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
        return Err("mastra_rewind_session_id_required".to_string());
    }
    let snapshot = state
        .get("run_snapshots")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&session_id))
        .cloned()
        .ok_or_else(|| format!("mastra_snapshot_missing:{session_id}"))?;
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
        return Err("mastra_rewind_context_restore_failed".to_string());
    }
    emit_native_trace(
        root,
        &clean_token(
            snapshot.get("snapshot_id").and_then(Value::as_str),
            "mastra-rewind",
        ),
        "mastra_rewind",
        &format!("rewound session {session_id}"),
    )?;
    Ok(json!({
        "ok": true,
        "restored": {
            "session_id": session_id,
            "snapshot": snapshot,
            "swarm_state_path": rel(root, &swarm_state_path),
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn record_evaluation(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let session_id = clean_token(
        payload.get("session_id").and_then(Value::as_str),
        "mastra-session",
    );
    let metrics = payload.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let evaluation = json!({
        "evaluation_id": stable_id("mastraeval", &json!({"session_id": session_id, "metrics": metrics})),
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
    as_object_mut(state, "eval_traces").insert(evaluation_id.clone(), evaluation.clone());
    emit_native_trace(
        root,
        &evaluation_id,
        "mastra_eval",
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.6", mastra_claim("V6-WORKFLOW-011.6")),
    }))
}

