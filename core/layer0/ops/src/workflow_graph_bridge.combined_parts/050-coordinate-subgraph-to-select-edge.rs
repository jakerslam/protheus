
fn coordinate_subgraph(
    state: &mut Value,
    swarm_state_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("workflow_graph_subgraph_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_graph:{graph_id}"))?;
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let requested = payload
        .get("subgraphs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if requested.is_empty() {
        return Err("workflow_graph_subgraphs_required".to_string());
    }
    let max_children = match profile.as_str() {
        "tiny-max" => 1usize,
        "pure" => 2usize,
        _ => requested.len().max(1),
    };
    let degraded = requested.len() > max_children;
    let subgraphs: Vec<Value> = requested.into_iter().take(max_children).collect();
    let coordinator_id = stable_id(
        "lgsession",
        &json!({"graph_id": graph_id, "role": "coordinator"}),
    );
    let child_rows: Vec<Value> = subgraphs
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let label = clean_token(
                row.get("name")
                    .and_then(Value::as_str)
                    .or_else(|| row.get("role").and_then(Value::as_str)),
                &format!("subgraph{}", idx + 1),
            );
            json!({
                "session_id": stable_id("lgsession", &json!({"graph_id": graph_id, "label": label, "index": idx})),
                "name": label,
                "role": clean_token(row.get("role").and_then(Value::as_str), "worker"),
                "task": clean_text(row.get("task").and_then(Value::as_str), 160),
            })
        })
        .collect();

    let mut swarm = read_swarm_state(swarm_state_path);
    let sessions = as_object_mut(&mut swarm, "sessions");
    sessions.insert(
        coordinator_id.clone(),
        json!({
            "session_id": coordinator_id,
            "task": format!("workflow_graph:{}", graph.get("name").and_then(Value::as_str).unwrap_or("graph")),
            "role": "coordinator",
            "graph_id": graph_id,
            "created_at": now_iso(),
        }),
    );
    for child in &child_rows {
        let session_id = child
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        sessions.insert(
            session_id.clone(),
            json!({
                "session_id": session_id,
                "task": child.get("task").cloned().unwrap_or_else(|| json!(null)),
                "role": child.get("role").cloned().unwrap_or_else(|| json!("worker")),
                "graph_id": graph_id,
                "parent_session_id": coordinator_id,
                "created_at": now_iso(),
            }),
        );
    }
    save_swarm_state(swarm_state_path, &swarm)?;

    let record = json!({
        "coordination_id": stable_id("lgsub", &json!({"graph_id": graph_id, "coordinator": coordinator_id})),
        "graph_id": graph_id,
        "graph_name": graph.get("name").cloned().unwrap_or_else(|| json!(null)),
        "profile": profile,
        "degraded": degraded,
        "coordinator_session_id": coordinator_id,
        "child_sessions": child_rows,
        "coordinated_at": now_iso(),
    });
    let record_id = record
        .get("coordination_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "subgraphs").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "coordination": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.4", semantic_claim("V6-WORKFLOW-002.4")),
    }))
}

fn record_trace(
    root: &Path,
    state: &mut Value,
    trace_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("workflow_graph_trace_graph_id_required".to_string());
    }
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/workflow_graph_trace_bridge.ts"),
    )?;
    let trace = json!({
        "trace_id": stable_id("lgtrace", &json!({"graph_id": graph_id, "message": payload.get("message")})),
        "graph_id": graph_id,
        "stage": clean_token(payload.get("stage").and_then(Value::as_str), "transition"),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "transitions": payload.get("transitions").cloned().unwrap_or_else(|| json!([])),
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "bridge_path": adapter_path,
        "recorded_at": now_iso(),
    });
    emit_native_trace(
        root,
        trace_path,
        trace
            .get("trace_id")
            .and_then(Value::as_str)
            .unwrap_or("workflow_graph-trace"),
        trace
            .get("stage")
            .and_then(Value::as_str)
            .unwrap_or("transition"),
        trace
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("trace"),
    )?;
    as_array_mut(state, "traces").push(trace.clone());
    Ok(json!({
        "ok": true,
        "trace": trace,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.5", semantic_claim("V6-WORKFLOW-002.5")),
    }))
}

fn outgoing_edges(graph: &Value, from: &str) -> Vec<Value> {
    graph
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.get("from").and_then(Value::as_str) == Some(from))
        .collect()
}

fn node_exists(graph: &Value, node_id: &str) -> bool {
    graph
        .get("nodes")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("id").and_then(Value::as_str) == Some(node_id))
        })
        .unwrap_or(false)
}

fn select_edge(edges: &[Value], context: &Map<String, Value>) -> Option<Value> {
    if let Some(row) = edges.iter().find(|row| {
        row.get("condition")
            .map(|condition| condition_matches(condition, context))
            .unwrap_or(false)
    }) {
        return Some(row.clone());
    }
    edges
        .iter()
        .find(|row| row.get("default").and_then(Value::as_bool).unwrap_or(false))
        .cloned()
}
