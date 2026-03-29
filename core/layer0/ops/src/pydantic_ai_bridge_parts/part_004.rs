fn invoke_tool_context(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let tool_id = clean_token(payload.get("tool_id").and_then(Value::as_str), "");
    if tool_id.is_empty() {
        return Err("pydantic_ai_tool_id_required".to_string());
    }
    let tool = state
        .get("tool_manifests")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&tool_id))
        .cloned()
        .ok_or_else(|| format!("unknown_pydantic_ai_tool:{tool_id}"))?;
    let args_obj = payload
        .get("args")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let dependency_keys = parse_string_list(payload.get("dependency_keys"));
    let missing_args = parse_string_list(tool.get("required_args"))
        .into_iter()
        .filter(|key| !args_obj.contains_key(key))
        .collect::<Vec<_>>();
    let missing_dependencies = parse_string_list(tool.get("required_dependencies"))
        .into_iter()
        .filter(|key| !dependency_keys.contains(key))
        .collect::<Vec<_>>();
    if !missing_args.is_empty() || !missing_dependencies.is_empty() {
        return Ok(json!({
            "ok": false,
            "tool_id": tool_id,
            "reason_code": "tool_context_validation_failed",
            "missing_args": missing_args,
            "missing_dependencies": missing_dependencies,
            "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.3", pydantic_claim("V6-WORKFLOW-015.3")),
        }));
    }
    let invocation = invoke_tool_manifest(root, argv, state, payload)?;
    Ok(json!({
        "ok": true,
        "tool_invocation": invocation.get("invocation").cloned().unwrap_or(Value::Null),
        "tool_id": tool_id,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.3", pydantic_claim("V6-WORKFLOW-015.3")),
    }))
}

fn bridge_protocol(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let protocol_kind = clean_token(payload.get("protocol_kind").and_then(Value::as_str), "a2a");
    let event = if protocol_kind == "a2a" {
        let receipt = send_a2a_message(root, argv, state, payload)?;
        json!({
            "event_id": stable_id("pydaiproto", &json!({"protocol_kind": protocol_kind, "agent_id": payload.get("agent_id")})),
            "protocol_kind": protocol_kind,
            "delivery": receipt.get("a2a_message").cloned().unwrap_or(Value::Null),
            "degraded": false,
            "recorded_at": now_iso(),
        })
    } else {
        let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
        json!({
            "event_id": stable_id("pydaiproto", &json!({"protocol_kind": protocol_kind, "name": payload.get("name")})),
            "protocol_kind": protocol_kind,
            "bridge_path": payload.get("bridge_path").cloned().unwrap_or_else(|| json!("adapters/protocol/pydantic_ai_protocol_bridge.ts")),
            "event_name": clean_token(payload.get("event_name").and_then(Value::as_str), "protocol-event"),
            "endpoint": clean_text(payload.get("endpoint").and_then(Value::as_str), 240),
            "payload_size": payload.get("event").and_then(|row| serde_json::to_string(row).ok()).map(|row| row.len()).unwrap_or(0),
            "degraded": matches!(profile.as_str(), "pure" | "tiny-max") && protocol_kind == "ui",
            "reason_code": if matches!(profile.as_str(), "pure" | "tiny-max") && protocol_kind == "ui" { "profile_ui_protocol_limited" } else { "protocol_ok" },
            "recorded_at": now_iso(),
        })
    };
    let event_id = clean_token(event.get("event_id").and_then(Value::as_str), "");
    as_object_mut(state, "protocol_events").insert(event_id, event.clone());
    Ok(json!({
        "ok": true,
        "protocol_event": event,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.4", pydantic_claim("V6-WORKFLOW-015.4")),
    }))
}

fn durable_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut normalized = payload.clone();
    normalized
        .entry("name".to_string())
        .or_insert_with(|| json!("pydantic-ai-durable-agent"));
    normalized
        .entry("instruction".to_string())
        .or_insert_with(|| json!("produce a validated typed response"));
    normalized
        .entry("steps".to_string())
        .or_insert_with(|| json!([{"id": "plan"}, {"id": "execute"}]));
    normalized
        .entry("mode".to_string())
        .or_insert_with(|| json!("loop"));

    let resumed = if let Some(session_id) = payload.get("resume_session_id").and_then(Value::as_str)
    {
        let rewind_payload = Map::from_iter([("session_id".to_string(), json!(session_id))]);
        let _ = rewind_session(root, argv, state, &rewind_payload)?;
        true
    } else {
        false
    };
    let agent = run_llm_agent(root, argv, state, &normalized)?;
    let session_id = agent
        .get("agent")
        .and_then(|row| row.get("primary_session_id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let record = json!({
        "run_id": stable_id("pydairun", &json!({"session_id": session_id, "instruction": normalized.get("instruction")})),
        "session_id": session_id,
        "resume_token": stable_id("pydairesume", &json!({"session_id": session_id, "retry": payload.get("retry_count")})),
        "retry_count": parse_u64_value(payload.get("retry_count"), 0, 0, 12),
        "resumed": resumed,
        "agent": agent.get("agent").cloned().unwrap_or(Value::Null),
        "recorded_at": now_iso(),
    });
    let run_id = clean_token(record.get("run_id").and_then(Value::as_str), "");
    as_object_mut(state, "durable_runs").insert(run_id, record.clone());
    Ok(json!({
        "ok": true,
        "durable_run": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.5", pydantic_claim("V6-WORKFLOW-015.5")),
    }))
}

fn record_logfire(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let trace_id = clean_token(
        payload.get("trace_id").and_then(Value::as_str),
        &stable_id("pydailog", &json!(payload)),
    );
    let event_name = clean_token(
        payload.get("event_name").and_then(Value::as_str),
        "pydantic-ai-logfire",
    );
    let message = clean_text(payload.get("message").and_then(Value::as_str), 160);
    emit_native_trace(root, &trace_id, &event_name, &message)?;
    let record = json!({
        "trace_id": trace_id,
        "event_name": event_name,
        "message": message,
        "cost_usd": parse_f64_value(payload.get("cost_usd"), 0.0, 0.0, 1000.0),
        "tokens": parse_u64_value(payload.get("tokens"), 0, 0, 1_000_000),
        "recorded_at": now_iso(),
    });
    let trace_id = clean_token(record.get("trace_id").and_then(Value::as_str), "");
    as_object_mut(state, "logfire_events").insert(trace_id, record.clone());
    Ok(json!({
        "ok": true,
        "logfire_event": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.7", pydantic_claim("V6-WORKFLOW-015.7")),
    }))
}

fn execute_graph(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("pydantic_ai_graph_nodes_required".to_string());
    }
    let agents = nodes
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let label = clean_token(row.get("id").and_then(Value::as_str), &format!("node-{}", idx + 1));
            json!({
                "label": label.clone(),
                "role": clean_token(row.get("kind").and_then(Value::as_str), "worker"),
                "reason": clean_token(row.get("reason").and_then(Value::as_str), &format!("graph_step_{label}")),
                "budget": parse_u64_value(row.get("budget"), 256, 192, 2048),
            })
        })
        .collect::<Vec<_>>();
    let normalized = Map::from_iter([
        (
            "name".to_string(),
            payload
                .get("name")
                .cloned()
                .unwrap_or_else(|| json!("pydantic-ai-graph")),
        ),
        ("agents".to_string(), json!(agents)),
        (
            "importance".to_string(),
            payload
                .get("importance")
                .cloned()
                .unwrap_or_else(|| json!(0.8)),
        ),
        (
            "profile".to_string(),
            payload
                .get("profile")
                .cloned()
                .unwrap_or_else(|| json!("rich")),
        ),
    ]);
    let hierarchy = coordinate_hierarchy(root, argv, state, &normalized)?;
    let graph_record = json!({
        "graph_id": stable_id("pydaigraph", &json!({"name": payload.get("name"), "nodes": nodes})),
        "name": payload.get("name").cloned().unwrap_or_else(|| json!("pydantic-ai-graph")),
        "node_count": nodes.len(),
        "edge_count": payload.get("edges").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "hierarchy": hierarchy.get("hierarchy").cloned().unwrap_or(Value::Null),
        "executed_at": now_iso(),
    });
    let graph_id = clean_token(graph_record.get("graph_id").and_then(Value::as_str), "");
    as_object_mut(state, "graph_runs").insert(graph_id, graph_record.clone());
    Ok(json!({
        "ok": true,
        "graph_run": graph_record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.8", pydantic_claim("V6-WORKFLOW-015.8")),
    }))
}

fn stream_model(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let route = route_model(state, payload)?;
    let structured_fields = parse_string_list(payload.get("structured_fields"));
    let chunks = payload
        .get("chunks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            structured_fields
                .iter()
                .map(|field| json!({"field": field, "content": format!("{field}:ok")}))
                .collect()
        });
    let record = json!({
        "stream_id": stable_id("pydaistream", &json!({"route": route.get("route"), "fields": structured_fields})),
        "route": route.get("route").cloned().unwrap_or(Value::Null),
        "structured_fields": structured_fields,
        "chunk_count": chunks.len(),
        "chunks": chunks,
        "recorded_at": now_iso(),
    });
    let stream_id = clean_token(record.get("stream_id").and_then(Value::as_str), "");
    as_object_mut(state, "model_streams").insert(stream_id, record.clone());
    Ok(json!({
        "ok": true,
        "stream": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.9", pydantic_claim("V6-WORKFLOW-015.9")),
    }))
}

fn record_eval(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let response = record_evaluation(root, state, payload)?;
    Ok(json!({
        "ok": true,
        "evaluation": response.get("evaluation").cloned().unwrap_or(Value::Null),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.10", pydantic_claim("V6-WORKFLOW-015.10")),
    }))
}

fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut normalized = payload.clone();
    normalized
        .entry("shell_path".to_string())
        .or_insert_with(|| json!("client/runtime/systems/workflow/pydantic_ai_bridge.ts"));
    deploy_shell(root, state, &normalized)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("pydantic_ai_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "typed_agents": as_object_mut(&mut state, "typed_agents").len(),
            "structured_validations": as_object_mut(&mut state, "structured_validations").len(),
            "tool_contexts": as_object_mut(&mut state, "tool_manifests").len(),
            "protocol_events": as_object_mut(&mut state, "protocol_events").len(),
            "durable_runs": as_object_mut(&mut state, "durable_runs").len(),
            "approval_records": as_object_mut(&mut state, "approval_records").len(),
            "logfire_events": as_object_mut(&mut state, "logfire_events").len(),
            "graph_runs": as_object_mut(&mut state, "graph_runs").len(),
            "model_streams": as_object_mut(&mut state, "model_streams").len(),
            "evaluations": as_object_mut(&mut state, "evaluations").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-agent" => register_agent(root, &mut state, input),
        "validate-output" => validate_output(&mut state, input),
        "register-tool-context" => register_tool_context(root, &mut state, input),
        "invoke-tool-context" => invoke_tool_context(root, argv, &mut state, input),
        "bridge-protocol" => bridge_protocol(root, argv, &mut state, input),
        "durable-run" => durable_run(root, argv, &mut state, input),
        "register-a2a-agent" => register_a2a_agent(root, &mut state, input),
        "send-a2a-message" => send_a2a_message(root, argv, &mut state, input),
        "register-runtime-bridge" => register_runtime_bridge(root, &mut state, input),
        "route-model" => route_model(&state, input),
        "run-llm-agent" => run_llm_agent(root, argv, &mut state, input),
        "register-tool-manifest" => register_tool_manifest(root, &mut state, input),
        "invoke-tool-manifest" => invoke_tool_manifest(root, argv, &mut state, input),
        "coordinate-hierarchy" => coordinate_hierarchy(root, argv, &mut state, input),
        "approval-checkpoint" => approval_checkpoint(root, argv, &mut state, input),
        "record-logfire" => record_logfire(root, &mut state, input),
        "execute-graph" => execute_graph(root, argv, &mut state, input),
        "stream-model" => stream_model(&mut state, input),
        "record-eval" => record_eval(root, &mut state, input),
        "assimilate-intake" => assimilate_intake(root, &mut state, input),
        "rewind-session" => rewind_session(root, argv, &mut state, input),
        "record-evaluation" => record_evaluation(root, &mut state, input),
        "sandbox-execute" => sandbox_execute(root, &mut state, input),
        "deploy-shell" => deploy_shell(root, &mut state, input),
        _ => Err(format!("unknown_pydantic_ai_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("pydantic_ai_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("pydantic_ai_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("pydantic_ai_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bridge_route_degrades_polyglot_in_pure_mode() {
        let mut state = default_state();
        let payload = json!({
            "name": "python-gateway",
            "language": "python",
            "provider": "google",
            "bridge_path": "adapters/polyglot/pydantic_ai_protocol_bridge.ts",
            "supported_profiles": ["rich", "pure"]
        });
        let _ = register_runtime_bridge(Path::new("."), &mut state, payload.as_object().unwrap())
            .expect("register");
        let out = route_model(
            &state,
            &Map::from_iter([
                ("language".to_string(), json!("python")),
                ("provider".to_string(), json!("google")),
                ("model".to_string(), json!("gemini-2.0-flash")),
                ("profile".to_string(), json!("pure")),
            ]),
        )
        .expect("route");
        assert_eq!(out["route"]["degraded"].as_bool(), Some(true));
    }
}

