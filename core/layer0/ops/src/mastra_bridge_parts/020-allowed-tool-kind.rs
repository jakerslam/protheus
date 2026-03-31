fn allowed_tool_kind(kind: &str) -> bool {
    matches!(kind, "native" | "mcp" | "openapi" | "custom")
}

fn allowed_workflow_mode(mode: &str) -> bool {
    matches!(mode, "sequential" | "parallel" | "loop")
}

fn emit_native_trace(
    root: &Path,
    trace_id: &str,
    intent: &str,
    message: &str,
) -> Result<(), String> {
    let enable_exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=enable".to_string(),
            "--enabled=1".to_string(),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if enable_exit != 0 {
        return Err("mastra_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=mastra-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("mastra_observability_trace_failed".to_string());
    }
    Ok(())
}

fn ensure_session_for_task(
    root: &Path,
    swarm_state_path: &Path,
    task: &str,
    label: &str,
    role: Option<&str>,
    parent_session_id: Option<&str>,
    max_tokens: u64,
) -> Result<String, String> {
    let mut args = vec![
        "spawn".to_string(),
        format!("--task={task}"),
        format!("--agent-label={label}"),
        format!("--max-tokens={max_tokens}"),
        format!("--state-path={}", swarm_state_path.display()),
    ];
    if let Some(role) = role {
        args.push(format!("--role={role}"));
    }
    if let Some(parent) = parent_session_id {
        args.push(format!("--session-id={parent}"));
    }
    let exit = crate::swarm_runtime::run(root, &args);
    if exit != 0 {
        return Err(format!("mastra_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("mastra_swarm_session_missing:{label}"))
}

fn register_runtime_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-runtime",
    );
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("mastra_runtime_language_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/mastra_runtime_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("mastra_runtime_bridge_must_be_adapter_owned".to_string());
    }
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let record = json!({
        "bridge_id": stable_id("mastrart", &json!({"name": name, "language": language, "bridge_path": bridge_path})),
        "name": name,
        "language": language,
        "provider": clean_token(payload.get("provider").and_then(Value::as_str), "openai-compatible"),
        "model_family": clean_token(payload.get("model_family").and_then(Value::as_str), "gemini"),
        "models": payload.get("models").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": supported_profiles,
        "bridge_path": bridge_path,
        "registered_at": now_iso(),
    });
    let bridge_id = record
        .get("bridge_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "runtime_bridges").insert(bridge_id, record.clone());
    Ok(json!({
        "ok": true,
        "runtime_bridge": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.7", mastra_claim("V6-WORKFLOW-011.7")),
    }))
}

fn select_runtime_bridge<'a>(
    bridges: &'a Map<String, Value>,
    bridge_id: &str,
    language: &str,
    provider: &str,
) -> Result<&'a Value, String> {
    if !bridge_id.is_empty() {
        return bridges
            .get(bridge_id)
            .ok_or_else(|| format!("unknown_mastra_runtime_bridge:{bridge_id}"));
    }
    bridges
        .values()
        .find(|row| {
            let language_match = row.get("language").and_then(Value::as_str) == Some(language);
            let provider_match = provider.is_empty()
                || row.get("provider").and_then(Value::as_str) == Some(provider);
            language_match && provider_match
        })
        .ok_or_else(|| format!("mastra_runtime_bridge_not_found:{language}:{provider}"))
}

fn route_model(state: &Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let bridge_id = clean_token(payload.get("bridge_id").and_then(Value::as_str), "");
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    let provider = clean_token(
        payload.get("provider").and_then(Value::as_str),
        "openai-compatible",
    );
    let model = clean_token(
        payload.get("model").and_then(Value::as_str),
        "gemini-2.0-flash",
    );
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let bridges = state
        .get("runtime_bridges")
        .and_then(Value::as_object)
        .ok_or_else(|| "mastra_runtime_bridges_missing".to_string())?;
    let bridge = select_runtime_bridge(bridges, &bridge_id, &language, &provider)?;
    let supported_profiles = parse_string_list(bridge.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("mastra_runtime_profile_unsupported:{profile}"));
    }
    let polyglot_requires_rich = matches!(language.as_str(), "python" | "go" | "java")
        && matches!(profile.as_str(), "pure" | "tiny-max");
    Ok(json!({
        "ok": true,
        "route": {
            "bridge_id": bridge.get("bridge_id").cloned().unwrap_or(Value::Null),
            "bridge_path": bridge.get("bridge_path").cloned().unwrap_or(Value::Null),
            "language": language,
            "provider": provider,
            "model": model,
            "profile": profile,
            "degraded": polyglot_requires_rich,
            "reason_code": if polyglot_requires_rich { "polyglot_runtime_requires_rich_profile" } else { "route_ok" },
            "invocation_mode": "adapter_owned_runtime_bridge"
        },
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.7", mastra_claim("V6-WORKFLOW-011.7")),
    }))
}

fn snapshot_record(state: &mut Value, session_id: &str, payload: Value) {
    as_object_mut(state, "run_snapshots").insert(session_id.to_string(), payload);
}

fn run_llm_agent(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-llm-agent",
    );
    let instruction = clean_text(payload.get("instruction").and_then(Value::as_str), 240);
    if instruction.is_empty() {
        return Err("mastra_llm_agent_instruction_required".to_string());
    }
    let mode = clean_token(payload.get("mode").and_then(Value::as_str), "sequential");
    if !allowed_workflow_mode(&mode) {
        return Err("mastra_llm_agent_mode_invalid".to_string());
    }
    let steps = payload
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        return Err("mastra_llm_agent_steps_required".to_string());
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
    let primary_task = format!("mastra:llm:{}:{}", name, instruction);
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
                let task = format!("mastra:parallel:{name}:{step_id}");
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
        "agent_id": stable_id("mastraagent", &json!({"name": name, "instruction": instruction, "mode": mode})),
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
            .unwrap_or("mastra-session"),
        json!({
            "snapshot_id": stable_id("mastrasnap", &json!({"agent_id": agent_id})),
            "agent_id": agent_id,
            "context_payload": {"instruction": instruction, "mode": mode, "profile": profile},
            "route": route.get("route").cloned().unwrap_or(Value::Null),
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "agent_loops").insert(agent_id, agent.clone());
    Ok(json!({
        "ok": true,
        "agent": agent,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.2", mastra_claim("V6-WORKFLOW-011.2")),
    }))
}

fn register_graph(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(payload.get("name").and_then(Value::as_str), "mastra-graph");
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("mastra_graph_nodes_required".to_string());
    }
    let edges = payload.get("edges").cloned().unwrap_or_else(|| json!([]));
    let record = json!({
        "graph_id": stable_id("mastragraph", &json!({"name": name, "nodes": nodes})),
        "name": name,
        "nodes": nodes,
        "edges": edges,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "start"),
        "parallel_branches": payload
            .get("nodes")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().filter(|row| parse_bool_value(row.get("parallel"), false)).count())
            .unwrap_or(0),
        "registered_at": now_iso(),
    });
    let graph_id = record
        .get("graph_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "graphs").insert(graph_id, record.clone());
    Ok(json!({
        "ok": true,
        "graph": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.1", mastra_claim("V6-WORKFLOW-011.1")),
    }))
}

