        "inspection_id": stable_id("lginspect", &json!({"graph_id": graph_id, "checkpoint_id": checkpoint_id, "state": state_view})),
        "graph_id": graph_id,
        "checkpoint_id": if checkpoint_id.is_empty() { json!(null) } else { json!(checkpoint_id) },
        "operator_id": clean_token(payload.get("operator_id").and_then(Value::as_str), "operator"),
        "inspection_mode": if intervention.as_object().map(|row| !row.is_empty()).unwrap_or(false) { json!("intervened") } else { json!("inspect_only") },
        "view_fields": payload.get("view_fields").cloned().unwrap_or_else(|| json!([])),
        "state_view": state_view,
        "intervention_patch": intervention,
        "change_applied": payload.get("intervention_patch").and_then(Value::as_object).map(|row| !row.is_empty()).unwrap_or(false),
        "inspected_at": now_iso(),
    });
    let inspection_id = inspection
        .get("inspection_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "inspections").insert(inspection_id, inspection.clone());
    Ok(json!({
        "ok": true,
        "inspection": inspection,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.3", semantic_claim("V6-WORKFLOW-002.3")),
    }))
}

fn interrupt_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let checkpoint_id = clean_token(payload.get("checkpoint_id").and_then(Value::as_str), "");
    if checkpoint_id.is_empty() {
        return Err("workflow_graph_interrupt_checkpoint_id_required".to_string());
    }
    let checkpoint = state
        .get("checkpoints")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&checkpoint_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_checkpoint:{checkpoint_id}"))?;
    let graph_id = clean_token(
        checkpoint.get("graph_id").and_then(Value::as_str),
        "workflow_graph-graph",
    );
    let reason = clean_text(payload.get("reason").and_then(Value::as_str), 160);
    let interrupt = json!({
        "interrupt_id": stable_id("lginterrupt", &json!({"checkpoint_id": checkpoint_id, "reason": reason})),
        "graph_id": graph_id,
        "checkpoint_id": checkpoint_id,
        "thread_id": checkpoint.get("thread_id").cloned().unwrap_or_else(|| json!(null)),
        "resume_token": stable_id("lgresume", &json!({"checkpoint_id": checkpoint_id, "reason": reason})),
        "requested_by": clean_token(payload.get("requested_by").and_then(Value::as_str), "operator"),
        "reason": reason,
        "snapshot": checkpoint.get("snapshot").cloned().unwrap_or_else(|| json!({})),
        "status": "paused",
        "created_at": now_iso(),
    });
    let interrupt_id = interrupt
        .get("interrupt_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "interrupts").insert(interrupt_id, interrupt.clone());
    Ok(json!({
        "ok": true,
        "interrupt": interrupt,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.7", semantic_claim("V6-WORKFLOW-002.7")),
    }))
}

fn find_interrupt_key(
    interrupts: &Map<String, Value>,
    interrupt_id: &str,
    resume_token: &str,
) -> Option<String> {
    if !interrupt_id.is_empty() && interrupts.contains_key(interrupt_id) {
        return Some(interrupt_id.to_string());
    }
    if resume_token.is_empty() {
        return None;
    }
    interrupts.iter().find_map(|(id, row)| {
        (row.get("resume_token").and_then(Value::as_str) == Some(resume_token))
            .then(|| id.to_string())
    })
}

fn resume_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let interrupt_id = clean_token(payload.get("interrupt_id").and_then(Value::as_str), "");
    let resume_token = clean_token(payload.get("resume_token").and_then(Value::as_str), "");
    if interrupt_id.is_empty() && resume_token.is_empty() {
        return Err("workflow_graph_resume_interrupt_or_token_required".to_string());
    }
    let key = {
        let interrupts = state
            .get("interrupts")
            .and_then(Value::as_object)
            .ok_or_else(|| "workflow_graph_interrupt_store_missing".to_string())?;
        find_interrupt_key(interrupts, &interrupt_id, &resume_token)
            .ok_or_else(|| "workflow_graph_interrupt_not_found".to_string())?
    };
    let updated = {
        let interrupts = as_object_mut(state, "interrupts");
        let row = interrupts
            .get_mut(&key)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "workflow_graph_interrupt_record_invalid".to_string())?;
        if row
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "paused")
        {
            return Err("workflow_graph_interrupt_not_paused".to_string());
        }
        row.insert("status".to_string(), json!("resumed"));
        row.insert("resumed_at".to_string(), json!(now_iso()));
        row.insert(
            "resume_mode".to_string(),
            json!(clean_token(
                payload.get("resume_mode").and_then(Value::as_str),
                "continue",
            )),
        );
        row.insert(
            "resume_context".to_string(),
            payload
                .get("resume_context")
                .cloned()
                .unwrap_or_else(|| json!({})),
        );
        Value::Object(row.clone())
    };
    Ok(json!({
        "ok": true,
        "interrupt": updated,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.7", semantic_claim("V6-WORKFLOW-002.7")),
    }))
}

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

fn stream_graph(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let graph_id = clean_token(payload.get("graph_id").and_then(Value::as_str), "");
    if graph_id.is_empty() {
        return Err("workflow_graph_stream_graph_id_required".to_string());
    }
    let graph = state
        .get("graphs")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&graph_id))
        .cloned()
        .ok_or_else(|| format!("unknown_workflow_graph_graph:{graph_id}"))?;
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let max_steps = match profile.as_str() {
        "tiny-max" => 2usize,
        "pure" => 3usize,
        _ => 8usize,
    };
    let mut current = clean_token(
        payload
            .get("entry_node")
            .and_then(Value::as_str)
            .or_else(|| graph.get("entry_node").and_then(Value::as_str)),
        "",
    );
    if current.is_empty() || !node_exists(&graph, &current) {
        return Err("workflow_graph_stream_entry_node_unknown".to_string());
    }
    let mut visited = Vec::new();
    let mut events = Vec::new();
    let mut degraded = false;
    for step in 0..max_steps {
        visited.push(Value::String(current.clone()));
        events.push(json!({
            "event": "node_enter",
            "step": step,
            "node_id": current,
        }));
        let edges = outgoing_edges(&graph, &current);
        if edges.is_empty() {
            break;
        }
        let Some(edge) = select_edge(&edges, &context) else {
            return Err("workflow_graph_stream_no_matching_edge_fail_closed".to_string());
        };
        let next = clean_token(edge.get("to").and_then(Value::as_str), "");
        if next.is_empty() || !node_exists(&graph, &next) {
            return Err("workflow_graph_stream_edge_target_unknown".to_string());
        }
        events.push(json!({
            "event": "edge_selected",
            "step": step,
            "from": current,
            "to": next,
            "label": edge.get("label").cloned().unwrap_or_else(|| json!(null)),
            "conditional": edge.get("condition").map(|row| !row.is_null()).unwrap_or(false),
        }));
        current = next;
    }
    if profile != "rich" {
        degraded = true;
        events.push(json!({
            "event": "degraded_profile",
            "profile": profile,
            "reason": "bounded_stream_step_cap",
        }));
    }
    let record = json!({
        "stream_id": stable_id("lgstream", &json!({"graph_id": graph_id, "context": context})),
        "graph_id": graph_id,
        "profile": profile,
        "visited": visited,
        "events": events,
        "degraded": degraded,
        "stream_mode": clean_token(payload.get("stream_mode").and_then(Value::as_str), "updates"),
        "streamed_at": now_iso(),
    });
    as_array_mut(state, "streams").push(record.clone());
    Ok(json!({
        "ok": true,
        "stream": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.6", semantic_claim("V6-WORKFLOW-002.6")),
    }))
}

fn run_governed_workflow(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
