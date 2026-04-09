fn execute_graph(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("mastra_execute_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_mastra_graph:{graph_id}"))?;
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let name = clean_token(graph.get("name").and_then(Value::as_str), "mastra-graph");
    let swarm_state = swarm_state_path(root, argv, payload);
    let root_session_id = ensure_session_for_task(
        root,
        &swarm_state,
        &format!("mastra:graph:{name}"),
        &name,
        Some("coordinator"),
        None,
        parse_u64_value(payload.get("budget"), 768, 64, 8192),
    )?;
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let low_profile = matches!(profile.as_str(), "pure" | "tiny-max");
    let mut child_sessions = Vec::new();
    let mut node_reports = Vec::new();
    let mut consumed_parallel = false;
    for (idx, node) in nodes.iter().enumerate() {
        let obj = node
            .as_object()
            .ok_or_else(|| "mastra_graph_node_object_required".to_string())?;
        let node_id = clean_token(
            obj.get("id").and_then(Value::as_str),
            &format!("node-{}", idx + 1),
        );
        let is_parallel = parse_bool_value(obj.get("parallel"), false);
        let selected = !(low_profile && is_parallel && consumed_parallel);
        if is_parallel && selected {
            consumed_parallel = true;
        }
        let session_id = if selected && (is_parallel || parse_bool_value(obj.get("spawn"), false)) {
            child_sessions.push(root_session_id.clone());
            Some(root_session_id.clone())
        } else {
            None
        };
        node_reports.push(json!({
            "node_id": node_id,
            "parallel": is_parallel,
            "selected": selected,
            "branch": clean_token(obj.get("branch").and_then(Value::as_str), "default"),
            "session_id": session_id,
        }));
    }
    let run = json!({
        "run_id": stable_id("mastragraphrun", &json!({"graph_id": graph_id, "profile": profile})),
        "graph_id": graph_id,
        "profile": profile,
        "root_session_id": root_session_id,
        "child_sessions": child_sessions,
        "nodes": node_reports,
        "degraded": low_profile && graph.get("parallel_branches").and_then(Value::as_u64).unwrap_or(0) > 1,
        "reason_code": if low_profile && graph.get("parallel_branches").and_then(Value::as_u64).unwrap_or(0) > 1 {
            "graph_parallelism_profile_limited"
        } else {
            "graph_execution_ok"
        },
        "executed_at": now_iso(),
    });
    let run_id = run
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    snapshot_record(
        state,
        &root_session_id,
        json!({
            "snapshot_id": stable_id("mastragraphsnap", &json!({"run_id": run_id})),
            "run_id": run_id,
            "context_payload": {"graph_id": graph_id, "profile": profile},
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "graph_runs").insert(run_id, run.clone());
    Ok(json!({
        "ok": true,
        "run": run,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.1", mastra_claim("V6-WORKFLOW-011.1")),
    }))
}

fn run_agent_loop(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut llm_payload = payload.clone();
    if llm_payload
        .get("instruction")
        .and_then(Value::as_str)
        .is_none()
    {
        return Err("mastra_agent_loop_instruction_required".to_string());
    }
    if llm_payload.get("steps").is_none() {
        let tool_rows = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let synthesized = if tool_rows.is_empty() {
            vec![json!({"id": "reason", "budget": 96})]
        } else {
            tool_rows
                .iter()
                .enumerate()
                .map(|(idx, row)| {
                    json!({
                        "id": clean_token(row.get("tool_id").and_then(Value::as_str), &format!("tool-{}", idx + 1)),
                        "budget": parse_u64_value(row.get("budget"), 96, 16, 2048),
                    })
                })
                .collect::<Vec<_>>()
        };
        llm_payload.insert("steps".to_string(), json!(synthesized));
    }
    llm_payload
        .entry("mode".to_string())
        .or_insert_with(|| json!("loop"));
    llm_payload
        .entry("max_iterations".to_string())
        .or_insert_with(|| json!(parse_u64_value(payload.get("max_iterations"), 2, 1, 6)));
    let tool_ids = payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            row.get("tool_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    let mut result = run_llm_agent(root, argv, state, &llm_payload)?;
    if let Some(agent) = result.get_mut("agent").and_then(Value::as_object_mut) {
        agent.insert("selected_tools".to_string(), json!(tool_ids));
        agent.insert(
            "reasoning_mode".to_string(),
            json!(clean_token(
                payload.get("reasoning_mode").and_then(Value::as_str),
                "bounded"
            )),
        );
    }
    Ok(result)
}

fn memory_recall(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let query = clean_text(payload.get("query").and_then(Value::as_str), 240);
    if query.is_empty() {
        return Err("mastra_memory_query_required".to_string());
    }
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let top = parse_u64_value(payload.get("top"), 5, 1, 25);
    let ambient_exit = crate::memory_ambient::run(
        root,
        &[
            "run".to_string(),
            "--memory-command=recall".to_string(),
            format!("--memory-arg=--query={query}"),
            format!("--memory-arg=--limit={top}"),
            "--run-context=mastra".to_string(),
        ],
    );
    let ambient_path = root.join("local/state/client/memory/ambient/latest.json");
    let ambient_receipt = lane_utils::read_json(&ambient_path).unwrap_or_else(|| json!({}));
    let degraded = ambient_exit != 0 || (matches!(profile.as_str(), "tiny-max") && top > 3);
    let record = json!({
        "recall_id": stable_id("mastrarecall", &json!({"query": query, "top": top, "profile": profile})),
        "query": query,
        "top": top,
        "profile": profile,
        "degraded": degraded,
        "reason_code": if ambient_exit != 0 {
            "memory_runtime_unavailable_degraded"
        } else if matches!(profile.as_str(), "tiny-max") && top > 3 {
            "memory_recall_top_trimmed_for_profile"
        } else {
            "memory_recall_ok"
        },
        "ambient_exit_code": ambient_exit,
        "ambient_receipt_path": rel(root, &ambient_path),
        "ambient_receipt": ambient_receipt,
        "recalled_at": now_iso(),
    });
    let recall_id = record
        .get("recall_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "memory_recalls").insert(recall_id, record.clone());
    Ok(json!({
        "ok": true,
        "memory_recall": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.3", mastra_claim("V6-WORKFLOW-011.3")),
    }))
}

fn register_mcp_bridge(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "mastra-mcp-bridge",
    );
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/mastra_mcp_bridge.ts"),
    )?;
    let record = json!({
        "tool_id": stable_id("mastramcp", &json!({"name": name, "bridge_path": bridge_path})),
        "name": name,
        "kind": "mcp",
        "bridge_path": bridge_path,
        "entrypoint": clean_token(payload.get("entrypoint").and_then(Value::as_str), "invoke"),
        "requires_approval": parse_bool_value(payload.get("requires_approval"), false),
        "supported_profiles": parse_string_list(payload.get("supported_profiles")),
        "capabilities": payload.get("capabilities").cloned().unwrap_or_else(|| json!(["tools", "resources"])),
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
        "mcp_bridge": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn invoke_mcp_bridge(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let bridge_id = clean_token(
        payload
            .get("bridge_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("tool_id").and_then(Value::as_str)),
        "",
    );
    if bridge_id.is_empty() {
        return Err("mastra_mcp_bridge_id_required".to_string());
    }
    let mut invoke_payload = payload.clone();
    invoke_payload.insert("tool_id".to_string(), json!(bridge_id));
    let out = invoke_tool_manifest(root, argv, state, &invoke_payload)?;
    Ok(json!({
        "ok": true,
        "mcp_invocation": out.get("invocation").cloned().unwrap_or(Value::Null),
        "tool_id": out.get("tool_id").cloned().unwrap_or(Value::Null),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.5", mastra_claim("V6-WORKFLOW-011.5")),
    }))
}

fn find_run_record(state: &Value, run_id: &str) -> Option<Value> {
    for key in ["graph_runs", "agent_loops"] {
        if let Some(row) = state
            .get(key)
            .and_then(Value::as_object)
            .and_then(|rows| rows.get(run_id))
        {
            return Some(row.clone());
        }
    }
    None
}

fn run_session_id(run: &Value) -> String {
    clean_token(
        run.get("root_session_id")
            .and_then(Value::as_str)
            .or_else(|| run.get("primary_session_id").and_then(Value::as_str)),
        "",
    )
}

fn suspend_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let run_id = clean_token(payload.get("run_id").and_then(Value::as_str), "");
    if run_id.is_empty() {
        return Err("mastra_suspend_run_id_required".to_string());
    }
    let run =
        find_run_record(state, &run_id).ok_or_else(|| format!("unknown_mastra_run:{run_id}"))?;
    let session_id = run_session_id(&run);
    let mut approval: Option<Value> = None;
    if parse_bool_value(payload.get("require_approval"), true) {
        let approval_out = approval_checkpoint(
            root,
            argv,
            state,
            &Map::from_iter([
                (
                    "summary".to_string(),
                    json!(clean_text(
                        payload.get("summary").and_then(Value::as_str),
                        200
                    )),
                ),
                (
                    "reason".to_string(),
                    json!(clean_text(
                        payload.get("reason").and_then(Value::as_str),
                        200
                    )),
                ),
                (
                    "action_id".to_string(),
                    payload.get("action_id").cloned().unwrap_or(Value::Null),
                ),
                (
                    "decision".to_string(),
                    payload.get("decision").cloned().unwrap_or(Value::Null),
                ),
            ]),
        )?;
        approval = approval_out.get("approval").cloned();
    }
    let record = json!({
        "run_id": run_id,
        "session_id": session_id,
        "resume_token": stable_id("mastraresume", &run),
        "approval": approval,
        "status": "suspended",
        "suspended_at": now_iso(),
    });
    snapshot_record(
        state,
        &session_id,
        json!({
            "snapshot_id": stable_id("mastrasuspend", &record),
            "run_id": run_id,
            "context_payload": {"status": "suspended"},
            "recorded_at": now_iso(),
        }),
    );
    as_object_mut(state, "suspended_runs")
        .insert(clean_token(Some(&run_id), &run_id), record.clone());
    Ok(json!({
        "ok": true,
        "suspension": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

fn resume_run(
    root: &Path,
    argv: &[String],
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let run_id = clean_token(payload.get("run_id").and_then(Value::as_str), "");
    if run_id.is_empty() {
        return Err("mastra_resume_run_id_required".to_string());
    }
    let record = as_object_mut(state, "suspended_runs")
        .get_mut(&run_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("unknown_mastra_suspended_run:{run_id}"))?;
    if let Some(action_id) = record
        .get("approval")
        .and_then(|value| value.get("action_id"))
        .and_then(Value::as_str)
    {
        let queue_path = approval_queue_path(root, argv, payload);
        if !approval_is_approved(&queue_path, action_id) {
            return Err("mastra_resume_requires_approved_checkpoint".to_string());
        }
    }
    let session_id = clean_token(record.get("session_id").and_then(Value::as_str), "");
    if !session_id.is_empty() {
        let swarm_state = swarm_state_path(root, argv, payload);
        let context_json = encode_json_arg(&json!({"status": "resumed", "run_id": run_id}))?;
        let exit = crate::swarm_runtime::run(
            root,
            &[
                "sessions".to_string(),
                "context-put".to_string(),
                format!("--session-id={session_id}"),
                format!("--context-json={context_json}"),
                "--merge=1".to_string(),
                format!("--state-path={}", swarm_state.display()),
            ],
        );
        if exit != 0 {
            return Err("mastra_resume_context_restore_failed".to_string());
        }
    }
    record.insert("status".to_string(), json!("resumed"));
    record.insert("resumed_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "resume": Value::Object(record.clone()),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.4", mastra_claim("V6-WORKFLOW-011.4")),
    }))
}

