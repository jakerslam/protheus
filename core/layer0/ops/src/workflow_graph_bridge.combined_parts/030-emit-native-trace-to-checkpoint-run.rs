
fn emit_native_trace(
    root: &Path,
    trace_path: &Path,
    trace_id: &str,
    stage: &str,
    message: &str,
) -> Result<(), String> {
    lane_utils::append_jsonl(
        trace_path,
        &json!({
            "trace_id": clean_token(Some(trace_id), "workflow_graph-trace"),
            "stage": clean_token(Some(stage), "graph"),
            "message": clean_text(Some(message), 200),
            "recorded_at": now_iso(),
            "root": rel(root, trace_path),
        }),
    )
}

fn read_swarm_state(path: &Path) -> Value {
    lane_utils::read_json(path).unwrap_or_else(|| json!({"sessions": {}, "handoff_registry": {}}))
}

fn save_swarm_state(path: &Path, state: &Value) -> Result<(), String> {
    lane_utils::write_json(path, state)
}

fn normalize_node(node: &Value) -> Value {
    let obj = node.as_object().cloned().unwrap_or_default();
    let node_id = clean_token(obj.get("id").and_then(Value::as_str), "node");
    json!({
        "id": node_id,
        "kind": clean_token(obj.get("kind").and_then(Value::as_str), "step"),
        "tool": clean_token(obj.get("tool").and_then(Value::as_str), ""),
        "checkpoint_key": clean_token(obj.get("checkpoint_key").and_then(Value::as_str), ""),
        "prompt": clean_text(obj.get("prompt").and_then(Value::as_str), 240),
    })
}

fn normalize_edge(edge: &Value) -> Value {
    let obj = edge.as_object().cloned().unwrap_or_default();
    json!({
        "from": clean_token(obj.get("from").and_then(Value::as_str), ""),
        "to": clean_token(obj.get("to").and_then(Value::as_str), ""),
        "label": clean_token(obj.get("label").and_then(Value::as_str), "edge"),
        "default": obj.get("default").and_then(Value::as_bool).unwrap_or(false),
        "condition": obj.get("condition").cloned().unwrap_or_else(|| json!(null)),
    })
}

fn condition_matches(condition: &Value, context: &Map<String, Value>) -> bool {
    let Some(obj) = condition.as_object() else {
        return false;
    };
    let field = obj.get("field").and_then(Value::as_str).unwrap_or_default();
    if field.is_empty() {
        return false;
    }
    let equals = obj.get("equals");
    let contains = obj.get("contains").and_then(Value::as_str);
    match (context.get(field), equals, contains) {
        (Some(actual), Some(expected), _) => actual == expected,
        (Some(actual), _, Some(needle)) => actual
            .as_str()
            .map(|row| row.contains(needle))
            .unwrap_or(false),
        _ => false,
    }
}

fn register_graph(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let name = clean_token(
        payload.get("name").and_then(Value::as_str),
        "workflow_graph-graph",
    );
    let nodes = payload
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if nodes.is_empty() {
        return Err("workflow_graph_nodes_required".to_string());
    }
    let edges = payload
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized_nodes: Vec<Value> = nodes.iter().map(normalize_node).collect();
    let normalized_edges: Vec<Value> = edges.iter().map(normalize_edge).collect();
    let entry_node = clean_token(
        payload
            .get("entry_node")
            .and_then(Value::as_str)
            .or_else(|| {
                normalized_nodes
                    .first()
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str)
            }),
        "start",
    );
    let graph = json!({
        "graph_id": stable_id("lggraph", &json!({"name": name, "entry": entry_node})),
        "name": name,
        "entry_node": entry_node,
        "checkpoint_mode": clean_token(payload.get("checkpoint_mode").and_then(Value::as_str), "per_node"),
        "nodes": normalized_nodes,
        "edges": normalized_edges,
        "node_count": nodes.len(),
        "edge_count": edges.len(),
        "conditional_edge_count": normalized_edges.iter().filter(|row| row.get("condition").map(|v| !v.is_null()).unwrap_or(false)).count(),
        "registered_at": now_iso(),
    });
    let graph_id = graph
        .get("graph_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "graphs").insert(graph_id, graph.clone());
    Ok(json!({
        "ok": true,
        "graph": graph,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.1", semantic_claim("V6-WORKFLOW-002.1")),
    }))
}

fn checkpoint_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("workflow_graph_checkpoint_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_graph:{graph_id}"))?;
    let snapshot = payload
        .get("state_snapshot")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let replay_enabled = payload
        .get("replay_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let checkpoint = json!({
        "checkpoint_id": stable_id("lgcp", &json!({"graph_id": graph_id, "snapshot": snapshot})),
        "graph_id": graph_id,
        "graph_name": graph.get("name").cloned().unwrap_or_else(|| json!(null)),
        "thread_id": clean_token(payload.get("thread_id").and_then(Value::as_str), "thread"),
        "checkpoint_label": clean_token(payload.get("checkpoint_label").and_then(Value::as_str), "graph_step"),
        "snapshot": snapshot,
        "snapshot_hash": deterministic_receipt_hash(&json!({"snapshot": payload.get("state_snapshot").cloned().unwrap_or_else(|| json!({}))})),
        "replay_enabled": replay_enabled,
        "replay_token": if replay_enabled { json!(stable_id("lgreplay", &json!({"graph_id": graph_id}))) } else { json!(null) },
        "rewind_from": clean_token(payload.get("rewind_from").and_then(Value::as_str), ""),
        "captured_at": now_iso(),
    });
    let checkpoint_id = checkpoint
        .get("checkpoint_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "checkpoints").insert(checkpoint_id, checkpoint.clone());
    Ok(json!({
        "ok": true,
        "checkpoint": checkpoint,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.2", semantic_claim("V6-WORKFLOW-002.2")),
    }))
}
