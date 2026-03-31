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
        return Err("google_adk_observability_enable_failed".to_string());
    }
    let exit = crate::observability_plane::run(
        root,
        &[
            "acp-provenance".to_string(),
            "--op=trace".to_string(),
            "--source-agent=google-adk-bridge".to_string(),
            format!("--target-agent={}", clean_token(Some(intent), "workflow")),
            format!("--intent={}", clean_text(Some(intent), 80)),
            format!("--message={}", clean_text(Some(message), 160)),
            format!("--trace-id={trace_id}"),
            "--visibility-mode=meta".to_string(),
            "--strict=1".to_string(),
        ],
    );
    if exit != 0 {
        return Err("google_adk_observability_trace_failed".to_string());
    }
    Ok(())
}

fn register_a2a_agent(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "google-adk-agent",
    );
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("google_adk_a2a_language_invalid".to_string());
    }
    let transport = clean_token(payload.get("transport").and_then(Value::as_str), "a2a");
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/google_adk_runtime_bridge.ts"),
    )?;
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let record = json!({
        "agent_id": stable_id("gadka2a", &json!({"name": name, "language": language, "bridge_path": bridge_path})),
        "name": name,
        "language": language,
        "transport": transport,
        "bridge_path": bridge_path,
        "endpoint": clean_text(payload.get("endpoint").and_then(Value::as_str), 240),
        "capabilities": payload.get("capabilities").cloned().unwrap_or_else(|| json!([])),
        "supported_profiles": supported_profiles,
        "registered_at": now_iso(),
        "session_id": Value::Null,
    });
    let agent_id = record
        .get("agent_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "a2a_agents").insert(agent_id, record.clone());
    Ok(json!({
        "ok": true,
        "agent": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.1", adk_claim("V6-WORKFLOW-010.1")),
    }))
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
        return Err(format!("google_adk_swarm_spawn_failed:{label}"));
    }
    let swarm_state = read_swarm_state(swarm_state_path);
    find_swarm_session_id_by_task(&swarm_state, task)
        .ok_or_else(|| format!("google_adk_swarm_session_missing:{label}"))
}

fn send_a2a_message(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let agent_id = clean_token(payload.get("agent_id").and_then(Value::as_str), "");
    if agent_id.is_empty() {
        return Err("google_adk_a2a_agent_id_required".to_string());
    }
    let message = clean_text(payload.get("message").and_then(Value::as_str), 400);
    if message.is_empty() {
        return Err("google_adk_a2a_message_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let agent = as_object_mut(state, "a2a_agents")
        .get_mut(&agent_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_google_adk_a2a_agent:{agent_id}"))?;
    let supported_profiles = parse_string_list(agent.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("google_adk_a2a_profile_unsupported:{profile}"));
    }
    let sender_label = clean_token(
        payload.get("sender_label").and_then(Value::as_str),
        "google-adk-sender",
    );
    let sender_task = format!(
        "google-adk:a2a:{}:{}",
        sender_label,
        clean_text(payload.get("sender_task").and_then(Value::as_str), 120)
    );
    let sender_session_id = ensure_session_for_task(
        root,
        &swarm_state_path,
        &sender_task,
        &sender_label,
        Some("coordinator"),
        None,
        parse_u64_value(payload.get("sender_budget"), 320, 64, 4096),
    )?;
    let agent_name = clean_token(agent.get("name").and_then(Value::as_str), "remote-agent");
    let remote_task = format!("google-adk:a2a:remote:{agent_name}");
    let existing_session = agent
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let remote_session_id = match existing_session {
        Some(id) if !id.is_empty() => id,
        _ => {
            let id = ensure_session_for_task(
                root,
                &swarm_state_path,
                &remote_task,
                &agent_name,
                Some("remote-agent"),
                None,
                parse_u64_value(payload.get("receiver_budget"), 320, 64, 4096),
            )?;
            agent.insert("session_id".to_string(), json!(id.clone()));
            id
        }
    };
    let send_exit = crate::swarm_runtime::run(
        root,
        &[
            "sessions".to_string(),
            "send".to_string(),
            format!("--sender-id={sender_session_id}"),
            format!("--session-id={remote_session_id}"),
            format!("--message={message}"),
            "--delivery=at_least_once".to_string(),
            format!(
                "--ttl-ms={}",
                parse_u64_value(payload.get("ttl_ms"), 60000, 1000, 300000)
            ),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if send_exit != 0 {
        return Err("google_adk_a2a_send_failed".to_string());
    }
    let handoff_reason = clean_text(payload.get("handoff_reason").and_then(Value::as_str), 120);
    let handoff_exit = crate::swarm_runtime::run(
        root,
        &[
            "sessions".to_string(),
            "handoff".to_string(),
            format!("--session-id={sender_session_id}"),
            format!("--target-session-id={remote_session_id}"),
            format!(
                "--reason={}",
                if handoff_reason.is_empty() {
                    "google_adk_a2a_handoff"
                } else {
                    handoff_reason.as_str()
                }
            ),
            format!(
                "--importance={:.2}",
                parse_f64_value(payload.get("importance"), 0.82, 0.0, 1.0)
            ),
            format!("--state-path={}", swarm_state_path.display()),
        ],
    );
    if handoff_exit != 0 {
        return Err("google_adk_a2a_handoff_failed".to_string());
    }
    let receipt = json!({
        "message_id": stable_id("gadka2amsg", &json!({"agent_id": agent_id, "message": message})),
        "agent_id": agent_id,
        "profile": profile,
        "sender_session_id": sender_session_id,
        "remote_session_id": remote_session_id,
        "message": message,
        "bridge_path": agent.get("bridge_path").cloned().unwrap_or(Value::Null),
        "sent_at": now_iso(),
    });
    Ok(json!({
        "ok": true,
        "a2a_message": receipt,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.1", adk_claim("V6-WORKFLOW-010.1")),
    }))
}

fn register_runtime_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "google-adk-runtime",
    );
    let language = clean_token(payload.get("language").and_then(Value::as_str), "python");
    if !allowed_language(&language) {
        return Err("google_adk_runtime_language_invalid".to_string());
    }
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/polyglot/google_adk_runtime_bridge.ts"),
    )?;
    if !bridge_path.starts_with("adapters/") {
        return Err("google_adk_runtime_bridge_must_be_adapter_owned".to_string());
    }
    let supported_profiles = parse_string_list(payload.get("supported_profiles"));
    let record = json!({
        "bridge_id": stable_id("gadkrt", &json!({"name": name, "language": language, "bridge_path": bridge_path})),
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.9", adk_claim("V6-WORKFLOW-010.9")),
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
            .ok_or_else(|| format!("unknown_google_adk_runtime_bridge:{bridge_id}"));
    }
    bridges
        .values()
        .find(|row| {
            let language_match = row.get("language").and_then(Value::as_str) == Some(language);
            let provider_match = provider.is_empty()
                || row.get("provider").and_then(Value::as_str) == Some(provider);
            language_match && provider_match
        })
        .ok_or_else(|| format!("google_adk_runtime_bridge_not_found:{language}:{provider}"))
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
        .ok_or_else(|| "google_adk_runtime_bridges_missing".to_string())?;
    let bridge = select_runtime_bridge(bridges, &bridge_id, &language, &provider)?;
    let supported_profiles = parse_string_list(bridge.get("supported_profiles"));
    if !profile_supported(&supported_profiles, &profile) {
        return Err(format!("google_adk_runtime_profile_unsupported:{profile}"));
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
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-010.9", adk_claim("V6-WORKFLOW-010.9")),
    }))
}

fn snapshot_record(state: &mut Value, session_id: &str, payload: Value) {
    as_object_mut(state, "session_snapshots").insert(session_id.to_string(), payload);
}

