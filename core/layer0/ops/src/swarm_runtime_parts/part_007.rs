fn show_turn_run(state: &SwarmState, session_id: &str, run_id: &str) -> Result<Value, String> {
    if !session_exists(state, session_id) {
        return Err(format!("unknown_session:{session_id}"));
    }
    let run = state
        .turn_registry
        .get(run_id)
        .cloned()
        .ok_or_else(|| format!("unknown_turn_run:{run_id}"))?;
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_turn_show",
        "session_id": session_id,
        "run": run,
    }))
}

fn create_agent_network(
    state: &mut SwarmState,
    owner_session_id: Option<&str>,
    spec: Value,
) -> Result<Value, String> {
    let spec_obj = match spec {
        Value::Object(map) => map,
        _ => return Err("network_spec_object_required".to_string()),
    };
    let name = spec_obj
        .get("name")
        .and_then(Value::as_str)
        .map(|value| clean_text(value, 120))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "swarm-network".to_string());
    let nodes = spec_obj
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("network_nodes_required".to_string());
    }
    let network_id = format!(
        "net-{}",
        &deterministic_receipt_hash(&json!({
            "name": name,
            "node_count": nodes.len(),
            "ts": now_epoch_ms(),
        }))[..12]
    );
    let mut node_rows = Vec::new();
    let mut label_to_session = BTreeMap::new();
    let mut participant_ids = Vec::new();
    for node in nodes {
        let node_obj = match node {
            Value::Object(map) => map,
            _ => return Err("network_node_object_required".to_string()),
        };
        let task = node_obj
            .get("task")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 160))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "network-node-task".to_string());
        let mut options = default_spawn_options();
        options.role = node_obj
            .get("role")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 80));
        options.agent_label = node_obj
            .get("label")
            .or_else(|| node_obj.get("agent_label"))
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 80));
        options.capabilities = node_obj
            .get("capabilities")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|value| clean_text(value, 80))
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        options.token_budget = node_obj
            .get("token_budget")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        let spawned = spawn_single(state, owner_session_id, &task, 8, &options)?;
        let session_id = spawned
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "network_spawn_missing_session_id".to_string())?
            .to_string();
        if let Some(context) = node_obj.get("context").cloned() {
            let session = state
                .sessions
                .get_mut(&session_id)
                .ok_or_else(|| format!("unknown_session:{session_id}"))?;
            let _ = apply_context_update(session, context, true, "network_create")?;
        }
        if let Some(label) = options.agent_label.clone() {
            label_to_session.insert(label.clone(), session_id.clone());
        }
        participant_ids.push(session_id.clone());
        node_rows.push(json!({
            "session_id": session_id,
            "role": options.role,
            "label": options.agent_label,
            "attention_weight": node_obj.get("attention_weight").cloned().unwrap_or_else(|| json!(1.0)),
            "importance": node_obj.get("importance").cloned().unwrap_or_else(|| json!(0.5)),
            "task": task,
        }));
    }
    let channel = create_channel(state, &format!("{name}-channel"), participant_ids.clone())?;
    let channel_id = channel
        .get("channel")
        .and_then(|row| row.get("channel_id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut edge_rows = Vec::new();
    if let Some(edges) = spec_obj.get("edges").and_then(Value::as_array) {
        for edge in edges {
            let edge_obj = match edge {
                Value::Object(map) => map,
                _ => return Err("network_edge_object_required".to_string()),
            };
            let from_key = edge_obj
                .get("from")
                .and_then(Value::as_str)
                .ok_or_else(|| "network_edge_from_required".to_string())?;
            let to_key = edge_obj
                .get("to")
                .and_then(Value::as_str)
                .ok_or_else(|| "network_edge_to_required".to_string())?;
            let from_session = label_to_session
                .get(from_key)
                .cloned()
                .unwrap_or_else(|| from_key.to_string());
            let to_session = label_to_session
                .get(to_key)
                .cloned()
                .unwrap_or_else(|| to_key.to_string());
            if edge_obj
                .get("auto_handoff")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                let _ = register_handoff(
                    state,
                    &from_session,
                    &to_session,
                    edge_obj
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("network_edge"),
                    edge_obj
                        .get("importance")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.5),
                    edge_obj.get("context").cloned(),
                    Some(network_id.clone()),
                )?;
            }
            edge_rows.push(json!({
                "from": from_session,
                "to": to_session,
                "relation": edge_obj.get("relation").cloned().unwrap_or_else(|| json!("handoff")),
                "importance": edge_obj.get("importance").cloned().unwrap_or_else(|| json!(0.5)),
            }));
        }
    }
    let receipt = json!({
        "network_id": network_id,
        "name": name,
        "owner_session_id": owner_session_id,
        "channel_id": channel_id,
        "nodes": node_rows,
        "edges": edge_rows,
        "status": "active",
        "created_at": now_iso(),
    });
    state
        .network_registry
        .insert(network_id.clone(), receipt.clone());
    for participant in participant_ids {
        if let Some(session) = state.sessions.get_mut(&participant) {
            if !session.network_ids.iter().any(|row| row == &network_id) {
                session.network_ids.push(network_id.clone());
            }
        }
    }
    append_event(
        state,
        json!({
            "type": "swarm_network_created",
            "network_id": network_id,
            "name": name,
            "timestamp": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_network_create",
        "network": receipt,
    }))
}

fn network_status(
    state: &SwarmState,
    session_id: Option<&str>,
    network_id: &str,
) -> Result<Value, String> {
    let network = state
        .network_registry
        .get(network_id)
        .cloned()
        .ok_or_else(|| format!("unknown_network:{network_id}"))?;
    if let Some(session_id) = session_id {
        if !session_exists(state, session_id) {
            return Err(format!("unknown_session:{session_id}"));
        }
    }
    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_network_status",
        "network_id": network_id,
        "network": network,
    }))
}

fn wildcard_matches(pattern: &str, candidate: &str) -> bool {
    if pattern.is_empty() || pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == candidate;
    }
    let mut remainder = candidate;
    let mut first = true;
    for part in pattern.split('*') {
        if part.is_empty() {
            continue;
        }
        if first && !pattern.starts_with('*') {
            if !remainder.starts_with(part) {
                return false;
            }
            remainder = &remainder[part.len()..];
            first = false;
            continue;
        }
        if let Some(pos) = remainder.find(part) {
            remainder = &remainder[pos + part.len()..];
        } else {
            return false;
        }
        first = false;
    }
    if !pattern.ends_with('*')
        && pattern
            .rsplit('*')
            .find(|part| !part.is_empty())
            .map(|tail| !candidate.ends_with(tail))
            .unwrap_or(false)
    {
        return false;
    }
    true
}

fn next_result_id(state: &SwarmState, session_id: &str, task_id: &str) -> String {
    let mut salt = 0u64;
    loop {
        let digest = deterministic_receipt_hash(&json!({
            "session_id": session_id,
            "task_id": task_id,
            "salt": salt,
            "ts": now_epoch_ms(),
        }));
        let result_id = format!("result-{}", &digest[..12]);
        if !state.result_registry.contains_key(&result_id) {
            return result_id;
        }
        salt = salt.saturating_add(1);
    }
}

fn index_result_id(index: &mut BTreeMap<String, Vec<String>>, key: String, result_id: &str) {
    let ids = index.entry(key).or_default();
    if !ids.iter().any(|row| row == result_id) {
        ids.push(result_id.to_string());
    }
}

fn publish_result(
    state: &mut SwarmState,
    session_id: &str,
    agent_label: Option<String>,
    task_id: Option<String>,
    payload: ResultPayload,
    data: Value,
    confidence: f64,
    verification_status: String,
) -> Result<Value, String> {
    let Some(session) = state.sessions.get(session_id) else {
        return Err(format!("unknown_session:{session_id}"));
    };
    if !(0.0..=1.0).contains(&confidence) {
        return Err(format!("invalid_confidence:{confidence}"));
    }
    let role = session
        .role
        .clone()
        .unwrap_or_else(|| "unassigned".to_string());
    let task = task_id.unwrap_or_else(|| session.task.clone());
    let label = agent_label.unwrap_or_else(|| session_id.to_string());
    let result_id = next_result_id(state, session_id, &task);
    let result = AgentResult {
        result_id: result_id.clone(),
        session_id: session_id.to_string(),
        agent_label: label.clone(),
        agent_role: role.clone(),
        task_id: task.clone(),
        payload,
        data,
        confidence,
        verification_status,
        timestamp_ms: now_epoch_ms(),
        created_at: now_iso(),
    };
    state
        .result_registry
        .insert(result_id.clone(), result.clone());
    index_result_id(
        &mut state.results_by_session,
        session_id.to_string(),
        &result_id,
    );
    index_result_id(&mut state.results_by_label, label.clone(), &result_id);
    index_result_id(&mut state.results_by_role, role.clone(), &result_id);

    append_event(
        state,
        json!({
            "type": "swarm_result_published",
            "result_id": result_id,
            "session_id": session_id,
            "agent_label": label,
            "agent_role": role,
            "task_id": task,
            "timestamp": now_iso(),
        }),
    );

    Ok(json!({
        "ok": true,
        "type": "swarm_runtime_results_publish",
        "result": result,
    }))
}

fn parse_result_filters(argv: &[String]) -> ResultFilters {
    ResultFilters {
        label_pattern: parse_flag(argv, "label-pattern")
            .or_else(|| parse_flag(argv, "label"))
            .filter(|value| !value.trim().is_empty()),
        role: parse_flag(argv, "role").filter(|value| !value.trim().is_empty()),
        task_id: parse_flag(argv, "task-id").filter(|value| !value.trim().is_empty()),
        session_id: parse_flag(argv, "session-id").filter(|value| !value.trim().is_empty()),
    }
}

fn query_results(state: &SwarmState, filters: &ResultFilters) -> Vec<AgentResult> {
    let mut results = state
        .result_registry
        .values()
        .filter(|result| {
            filters
                .label_pattern
                .as_deref()
                .map(|pattern| wildcard_matches(pattern, &result.agent_label))
                .unwrap_or(true)
        })
        .filter(|result| {
            filters
                .role
                .as_deref()
                .map(|role| role == result.agent_role)
                .unwrap_or(true)
        })
        .filter(|result| {
            filters
                .task_id
                .as_deref()
                .map(|task_id| task_id == result.task_id)
                .unwrap_or(true)
        })
        .filter(|result| {
            filters
                .session_id
                .as_deref()
                .map(|session_id| session_id == result.session_id)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    results.sort_by_key(|result| result.timestamp_ms);
    results
}

fn wait_for_results(
    state_file: &Path,
    state: &SwarmState,
    filters: &ResultFilters,
    min_count: usize,
    timeout_ms: u64,
) -> Result<Vec<AgentResult>, String> {
    let min_count = min_count.max(1);
    let initial = query_results(state, filters);
    if initial.len() >= min_count {
        return Ok(initial);
    }
    let deadline = now_epoch_ms().saturating_add(timeout_ms.max(1));
    loop {
        let snapshot = load_state(state_file).unwrap_or_else(|_| state.clone());
        let results = query_results(&snapshot, filters);
        if results.len() >= min_count {
            return Ok(results);
        }
        if now_epoch_ms() >= deadline {
            return Err(format!(
                "result_wait_timeout:min_count={min_count}:found={}",
                results.len()
            ));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn consensus_value_from_result(result: &AgentResult, field: &str) -> Option<Value> {
    if field.trim().is_empty() || field == "value" {
        return result
            .payload
            .numeric_value()
            .map(|value| json!(value))
            .or_else(|| result.payload.field_value("value"))
            .or_else(|| result.data.get("value").cloned());
    }
    result
        .payload
        .field_value(field)
        .or_else(|| result.data.get(field).cloned())
}
