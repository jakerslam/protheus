fn run_process(
    state: &mut Value,
    swarm_state_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_process_crew_id_required".to_string());
    }
    let crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let tasks = payload
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if tasks.is_empty() {
        return Err("crewai_process_tasks_required".to_string());
    }
    let normalized_tasks: Vec<Value> = tasks
        .iter()
        .enumerate()
        .map(|(idx, row)| normalize_task(row, idx))
        .collect();
    let agents = crew
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let process_type = clean_token(
        payload
            .get("process_type")
            .and_then(Value::as_str)
            .or_else(|| crew.get("process_type").and_then(Value::as_str)),
        "sequential",
    );
    let max_children = match profile.as_str() {
        "tiny-max" => 1usize,
        "pure" => 2usize,
        _ => normalized_tasks.len().max(1),
    };
    let degraded = normalized_tasks.len() > max_children;
    let selected_tasks: Vec<Value> = normalized_tasks.into_iter().take(max_children).collect();
    let run_id = stable_id(
        "crrun",
        &json!({"crew_id": crew_id, "process_type": process_type, "tasks": selected_tasks}),
    );
    let manager_role = crew
        .get("manager_role")
        .and_then(Value::as_str)
        .unwrap_or("manager");
    let manager = agents
        .iter()
        .find(|agent| agent.get("role").and_then(Value::as_str) == Some(manager_role))
        .cloned()
        .or_else(|| agents.first().cloned())
        .unwrap_or_else(|| json!({"agent_id": "manager", "role": "manager"}));
    let child_sessions: Vec<Value> = selected_tasks
        .iter()
        .enumerate()
        .filter_map(|(idx, task)| {
            select_agent_for_task(&agents, task).map(|agent| {
                json!({
                    "session_id": stable_id("crewsess", &json!({"run_id": run_id, "idx": idx, "task": task})),
                    "agent_id": agent.get("agent_id").cloned().unwrap_or_else(|| json!(null)),
                    "role": agent.get("role").cloned().unwrap_or_else(|| json!(null)),
                    "task": task,
                })
            })
        })
        .collect();

    let mut swarm = read_swarm_state(swarm_state_path);
    let sessions = as_object_mut(&mut swarm, "sessions");
    let manager_session_id = stable_id("crewsess", &json!({"run_id": run_id, "role": "manager"}));
    sessions.insert(
        manager_session_id.clone(),
        json!({
            "session_id": manager_session_id,
            "crew_id": crew_id,
            "run_id": run_id,
            "role": manager.get("role").cloned().unwrap_or_else(|| json!("manager")),
            "agent_id": manager.get("agent_id").cloned().unwrap_or_else(|| json!("manager")),
            "task": if process_type == "hierarchical" { json!("manager_review") } else { json!("sequential_dispatch") },
            "created_at": now_iso(),
        }),
    );
    for child in &child_sessions {
        let session_id = child
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        sessions.insert(
            session_id.clone(),
            json!({
                "session_id": session_id,
                "crew_id": crew_id,
                "run_id": run_id,
                "parent_session_id": manager_session_id,
                "role": child.get("role").cloned().unwrap_or_else(|| json!(null)),
                "agent_id": child.get("agent_id").cloned().unwrap_or_else(|| json!(null)),
                "task": child.get("task").cloned().unwrap_or_else(|| json!(null)),
                "created_at": now_iso(),
            }),
        );
    }
    save_swarm_state(swarm_state_path, &swarm)?;

    let record = json!({
        "run_id": run_id,
        "crew_id": crew_id,
        "process_type": process_type,
        "profile": profile,
        "degraded": degraded,
        "manager_session_id": manager_session_id,
        "child_sessions": child_sessions,
        "task_count": selected_tasks.len(),
        "executed_at": now_iso(),
    });
    let record_id = record
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "process_runs").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "process_run": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.2", semantic_claim("V6-WORKFLOW-004.2")),
    }))
}

fn run_flow(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_flow_crew_id_required".to_string());
    }
    let _crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let flow_name = clean_token(payload.get("flow_name").and_then(Value::as_str), "flow");
    let trigger_event = clean_token(
        payload.get("trigger_event").and_then(Value::as_str),
        "start",
    );
    let routes = payload
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if routes.is_empty() {
        return Err("crewai_flow_routes_required".to_string());
    }
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let selected = routes
        .iter()
        .find(|route| allowed_route(route, &trigger_event, &context))
        .cloned();
    let Some(route) = selected else {
        return Err("crewai_flow_no_matching_route_fail_closed".to_string());
    };
    let decorators = payload
        .get("decorators")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let listeners = payload
        .get("listeners")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let record = json!({
        "flow_run_id": stable_id("crflow", &json!({"crew_id": crew_id, "flow_name": flow_name, "trigger_event": trigger_event})),
        "crew_id": crew_id,
        "flow_name": flow_name,
        "trigger_event": trigger_event,
        "selected_route": route,
        "decorators": decorators,
        "listeners": listeners,
        "context": context,
        "executed_at": now_iso(),
    });
    let record_id = record
        .get("flow_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "flow_runs").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "flow": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.3", semantic_claim("V6-WORKFLOW-004.3")),
    }))
}

fn memory_bridge(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_memory_crew_id_required".to_string());
    }
    let _crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let memories = payload
        .get("memories")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized: Vec<Value> = memories
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let obj = row.as_object().cloned().unwrap_or_default();
            json!({
                "memory_id": clean_token(obj.get("memory_id").and_then(Value::as_str), &format!("mem{}", idx + 1)),
                "scope": clean_token(obj.get("scope").and_then(Value::as_str), "crew"),
                "text": clean_text(obj.get("text").and_then(Value::as_str), 240),
                "agent_id": clean_token(obj.get("agent_id").and_then(Value::as_str), ""),
            })
        })
        .collect();
    let query = clean_text(payload.get("recall_query").and_then(Value::as_str), 120);
    let recall_hits: Vec<Value> = normalized
        .iter()
        .filter(|row| {
            query.is_empty()
                || row
                    .get("text")
                    .and_then(Value::as_str)
                    .map(|text| {
                        text.to_ascii_lowercase()
                            .contains(&query.to_ascii_lowercase())
                    })
                    .unwrap_or(false)
        })
        .take(5)
        .cloned()
        .collect();
    let record = json!({
        "memory_run_id": stable_id("crmem", &json!({"crew_id": crew_id, "query": query})),
        "crew_id": crew_id,
        "thread_id": clean_token(payload.get("thread_id").and_then(Value::as_str), "thread"),
        "summary": clean_text(payload.get("summary").and_then(Value::as_str), 180),
        "memories": normalized,
        "recall_query": query,
        "recall_hits": recall_hits,
        "stored_at": now_iso(),
    });
    let record_id = record
        .get("memory_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_records").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "memory": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.4", semantic_claim("V6-WORKFLOW-004.4")),
    }))
}

fn parse_config_payload(payload: &Map<String, Value>) -> Result<Value, String> {
    if let Some(raw_yaml) = payload
        .get("config_yaml")
        .and_then(Value::as_str)
        .or_else(|| payload.get("yaml").and_then(Value::as_str))
    {
        return serde_yaml::from_str::<Value>(raw_yaml)
            .map_err(|err| format!("crewai_config_yaml_parse_failed:{err}"));
    }
    Ok(payload
        .get("config_json")
        .cloned()
        .unwrap_or_else(|| json!({})))
}

fn ingest_config(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let config = parse_config_payload(payload)?;
    let config_obj = config
        .as_object()
        .cloned()
        .ok_or_else(|| "crewai_config_object_required".to_string())?;
    let unsupported_keys = top_level_unsupported_keys(&config_obj);
    if !unsupported_keys.is_empty() {
        return Err(format!(
            "crewai_config_unsupported_keys_fail_closed:{}",
            unsupported_keys.join(",")
        ));
    }
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/crewai_tool_bridge.ts"),
    )?;
    let manifest = json!({
        "config_id": stable_id("crcfg", &json!({"config": config, "adapter": adapter_path})),
        "adapter_path": adapter_path,
        "crew_name": clean_token(
            config_obj
                .get("crew")
                .and_then(Value::as_object)
                .and_then(|crew| crew.get("name"))
                .and_then(Value::as_str),
            "crew",
        ),
        "agent_count": config_obj.get("agents").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "task_count": config_obj.get("tasks").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "flow_count": config_obj.get("flows").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
        "config": config,
        "ingested_at": now_iso(),
    });
    let config_id = manifest
        .get("config_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "configs").insert(config_id, manifest.clone());
    Ok(json!({
        "ok": true,
        "config": manifest,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.5", semantic_claim("V6-WORKFLOW-004.5")),
    }))
}

fn route_delegation(
    root: &Path,
    state: &mut Value,
    swarm_state_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_delegation_crew_id_required".to_string());
    }
    let crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let task = normalize_task(
        &json!({
            "task_id": payload.get("task_id").cloned().unwrap_or_else(|| json!(null)),
            "name": payload.get("task_name").cloned().unwrap_or_else(|| json!("delegate")),
            "description": payload.get("task").cloned().unwrap_or_else(|| json!(null)),
            "role_hint": payload.get("role_hint").cloned().unwrap_or_else(|| json!(null)),
            "required_tool": payload.get("required_tool").cloned().unwrap_or_else(|| json!(null)),
        }),
        0,
    );
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/crewai_tool_bridge.ts"),
    )?;
    let agents = crew
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected_agent = select_agent_for_task(&agents, &task)
        .ok_or_else(|| "crewai_no_agent_available".to_string())?;
    let mut selected_tools = selected_agent
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut degraded = false;
    if profile == "pure" && selected_tools.len() > 2 {
        selected_tools.truncate(2);
        degraded = true;
    }
    if profile == "tiny-max" {
        if selected_tools.len() > 1 {
            selected_tools.truncate(1);
        }
        if selected_agent
            .get("multimodal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            degraded = true;
        }
    }
    let session_id = stable_id("crewsess", &json!({"crew_id": crew_id, "task": task}));
    let mut swarm = read_swarm_state(swarm_state_path);
    as_object_mut(&mut swarm, "sessions").insert(
        session_id.clone(),
        json!({
            "session_id": session_id,
            "crew_id": crew_id,
            "agent_id": selected_agent.get("agent_id").cloned().unwrap_or_else(|| json!(null)),
            "role": selected_agent.get("role").cloned().unwrap_or_else(|| json!(null)),
            "task": task,
            "created_at": now_iso(),
        }),
    );
    save_swarm_state(swarm_state_path, &swarm)?;
    let record = json!({
        "delegation_id": stable_id("crdel", &json!({"crew_id": crew_id, "task": task, "agent": selected_agent})),
        "crew_id": crew_id,
        "profile": profile,
        "bridge_path": adapter_path,
        "selected_agent": selected_agent,
        "selected_tools": selected_tools,
        "task": task,
        "session_id": session_id,
        "degraded": degraded,
        "delegated_at": now_iso(),
    });
    let record_id = record
        .get("delegation_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "delegations").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "delegation": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.6", semantic_claim("V6-WORKFLOW-004.6")),
    }))
}

