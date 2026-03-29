fn run_llm_agent(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "pydantic-ai-llm-agent",
    );
    let instruction = clean_text(payload.get("instruction").and_then(Value::as_str), 240);
    if instruction.is_empty() {
        return Err("pydantic_ai_llm_agent_instruction_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "sequential");
    if !allowed_workflow_mode(&mode) {
        return Err("pydantic_ai_llm_agent_mode_invalid".to_string());
    }
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("pydantic_ai_llm_agent_steps_required".to_string());
    }
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let route = route_model(
        state,
        &Map::from_iter([
            (
                "bridge_id".to_string(),
                payload
                    .get("runtime_bridge_id")
                    .cloned()
                    .unwrap_or(Value::String(String::new())),
            ),
            (
                "language".to_string(),
                payload
                    .get("language")
                    .cloned()
                    .unwrap_or_else(|| json!("python")),
            ),
            (
                "provider".to_string(),
                payload
                    .get("provider")
                    .cloned()
                    .unwrap_or_else(|| json!("openai-compatible")),
            ),
            (
                "model".to_string(),
                payload
                    .get("model")
                    .cloned()
                    .unwrap_or_else(|| json!("gemini-2.0-flash")),
            ),
            ("profile".to_string(), json!(profile.clone())),
        ]),
    )?;
    let primary_task = format!("pydantic-ai:llm:{}:{}", name, instruction);
    let primary_session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &primary_task,
        &name,
        Some("llm-agent"),
        None,
        parse_u64_value(payload.get("budget"), 640, 64, 8192),
    )?;

    let mut step_reports = Vec::new();
    let mut child_sessions = Vec::new();
    match mode.as_str() {
        "sequential" => {
            for (idx, step) in steps.iter().enumerate() {
                let step_id = clean_token(
                    step.get("id").and_then(Value::as_str),
                    &format!("step-{}", idx + 1),
                );
                step_reports.push(json!({
                    "step_id": step_id,
                    "mode": "sequential",
                    "budget": parse_u64_value(step.get("budget"), 96, 16, 2048),
                }));
            }
        }
        "parallel" => {
            for (idx, step) in steps.iter().enumerate() {
                let step_id = clean_token(
                    step.get("id").and_then(Value::as_str),
                    &format!("parallel-{}", idx + 1),
                );
                let task = format!("pydantic-ai:parallel:{name}:{step_id}");
                let child = ensure_session_for_task(
                    root,
                    &swarm_state_path,
                    &task,
                    &step_id,
                    Some("llm-worker"),
                    Some(&primary_session_id),
                    parse_u64_value(step.get("budget"), 128, 16, 2048),
                )?;
                child_sessions.push(child.clone());
                step_reports
                    .push(json!({"step_id": step_id, "mode": "parallel", "session_id": child}));
            }
        }
        "loop" => {
            let max_iterations = parse_u64_value(payload.get("max_iterations"), 2, 1, 6);
            for iter in 0..max_iterations {
                for (idx, step) in steps.iter().enumerate() {
                    let step_id = clean_token(
                        step.get("id").and_then(Value::as_str),
                        &format!("loop-{}", idx + 1),
                    );
                    step_reports.push(json!({
                        "step_id": step_id,
                        "mode": "loop",
                        "iteration": iter + 1,
                        "budget": parse_u64_value(step.get("budget"), 64, 16, 1024),
                    }));
                }
            }
        }
        _ => unreachable!(),
    }

    let agent = json!({
        "agent_id": stable_id("pydaiagent", &json!({"name": name, "instruction": instruction, "mode": mode})),
        "name": name,
        "instruction": instruction,
        "mode": mode,
        "profile": profile,
        "route": route.get("route").cloned().unwrap_or(Value::Null),
        "primary_session_id": primary_session_id,
        "child_sessions": child_sessions,
        "steps": step_reports,
        "executed_at": now_iso(),
    });
    let agent_id = agent
        .get("agent_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    snapshot_record(
        state,
        agent
            .get("primary_session_id")
            .and_then(Value::as_str)
            .unwrap_or("pydantic-ai-session"),
        json!({
            "snapshot_id": stable_id("pydaisnap", &json!({"agent_id": agent_id})),
            "agent_id": agent_id,
            "context_payload": {"instruction": instruction, "mode": mode, "profile": profile},
            "route": route.get("route").cloned().unwrap_or(Value::Null),
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "llm_agents").insert(agent_id, agent.clone());
    Ok(json!({
        "ok": true,
        "agent": agent,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.5", pydantic_claim("V6-WORKFLOW-015.5")),
    }))
}

fn register_tool_manifest(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "pydantic-ai-tool",
    );
    let kind = clean_token(payload.get("kind").and_then(Value::as_str), "custom");
    if !allowed_tool_kind(&kind) {
        return Err("pydantic_ai_tool_kind_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/pydantic_ai_protocol_bridge.ts"),
    )?;
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let openapi_url = clean_text(payload.get("openapi_url").and_then(Value::as_str), 200);
    if kind == "openapi"
        && !(openapi_url.starts_with("https://") || openapi_url.ends_with("openapi.json"))
    {
        return Err("pydantic_ai_tool_openapi_url_invalid".to_string());
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
            return Err("pydantic_ai_tool_mcp_capability_validation_failed".to_string());
        }
    }
    let record = json!({
        "tool_id": stable_id("pydaitool", &json!({"name": name, "kind": kind, "bridge_path": bridge_path})),
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
    as_object_mut(state, "tool_manifests").insert(tool_id, record.clone());
    Ok(json!({
        "ok": true,
        "tool": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.3", pydantic_claim("V6-WORKFLOW-015.3")),
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
        return Err("pydantic_ai_tool_id_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let queue_path = approval_queue_path(root, argv, payload);
    let tools = as_object_mut(state, "tool_manifests");
    let tool = tools
        .get_mut(&tool_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_pydantic_ai_tool:{tool_id}"))?;
    let supported_profiles = parse_string_list(tool.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("pydantic_ai_tool_profile_unsupported:{profile}"));
    }
    let requires_approval = parse_bool_value(tool.get("requires_approval"), false)
        || parse_bool_value(payload.get("requires_approval"), false);
    if requires_approval {
        let approval_action_id = clean_token(
            payload.get("approval_action_id").and_then(Value::as_str),
            "",
        );
        if approval_action_id.is_empty() {
            return Err("pydantic_ai_tool_requires_approval".to_string());
        }
        if !approval_is_approved(&queue_path, &approval_action_id) {
            return Err("pydantic_ai_tool_approval_not_granted".to_string());
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.3", pydantic_claim("V6-WORKFLOW-015.3")),
    }))
}

fn coordinate_hierarchy(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "pydantic-ai-hierarchy",
    );
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let coordinator_label = clean_token(
        payload.get("coordinator_label").and_then(Value::as_str),
        "pydantic-ai-coordinator",
    );
    let coordinator_task = format!("pydantic-ai:hierarchy:{name}:coordinator");
    let coordinator_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &coordinator_task,
        &coordinator_label,
        Some("coordinator"),
        None,
        parse_u64_value(payload.get("budget"), 960, 96, 12288),
    )?;
    let all_agents = payload
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if all_agents.is_empty() {
        return Err("pydantic_ai_hierarchy_agents_required".to_string());
    }
    let degraded = matches!(profile.as_str(), "pure" | "tiny-max") && all_agents.len() > 1;
    let selected_agents = if degraded {
        vec![all_agents[0].clone()]
    } else {
        all_agents
    };
    let mut rows = Vec::new();
    for agent in selected_agents {
        let obj = agent
            .as_object()
            .ok_or_else(|| "pydantic_ai_hierarchy_agent_object_required".to_string())?;
        let label = clean_token(obj.get("label").and_then(Value::as_str), "subagent");
        let role = clean_token(obj.get("role").and_then(Value::as_str), "specialist");
        let task = format!("pydantic-ai:hierarchy:{name}:{label}");
        let child_id = ensure_session_for_task(
            root,
            &swarm_state_path,
            &task,
            &label,
            Some(&role),
            Some(&coordinator_id),
            parse_u64_value(obj.get("budget"), 224, 32, 4096),
        )?;
        let handoff_exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "handoff".to_string(),
                format!("--session-id={coordinator_id}"),
                format!("--target-session-id={child_id}"),
                format!(
                    "--reason={}",
                    clean_text(obj.get("reason").and_then(Value::as_str), 120)
                ),
                format!(
                    "--importance={:.2}",
                    parse_f64_value(obj.get("importance"), 0.78, 0.0, 1.0)
                ),
                format!("--state-path={}", swarm_state_path.display()),
            ],
        );
        if handoff_exit != 0 {
            return Err(format!("pydantic_ai_hierarchy_handoff_failed:{label}"));
        }
        let context = obj.get("context").cloned().unwrap_or_else(|| json!({}));
        let context_json = encode_json_arg(&context)?;
        let context_exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "context-put".to_string(),
                format!("--session-id={child_id}"),
                format!("--context-json={context_json}"),
                "--merge=1".to_string(),
                format!("--state-path={}", swarm_state_path.display()),
            ],
        );
        if context_exit != 0 {
            return Err(format!("pydantic_ai_hierarchy_context_put_failed:{label}"));
        }
        rows.push(json!({
            "label": label,
            "role": role,
            "session_id": child_id,
            "context_budget": parse_u64_value(obj.get("context_budget"), 96, 16, 4096),
        }));
    }
    let record = json!({
        "hierarchy_id": stable_id("pydaih", &json!({"name": name, "profile": profile})),
        "name": name,
        "profile": profile,
        "coordinator_session_id": coordinator_id,
        "agents": rows,
        "degraded": degraded,
        "reason_code": if degraded { "hierarchy_profile_limited_to_single_subagent" } else { "hierarchy_ok" },
        "executed_at": now_iso(),
    });
    let hierarchy_id = record
        .get("hierarchy_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "hierarchies").insert(hierarchy_id, record.clone());
    Ok(json!({
        "ok": true,
        "hierarchy": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-015.8", pydantic_claim("V6-WORKFLOW-015.8")),
    }))
}

