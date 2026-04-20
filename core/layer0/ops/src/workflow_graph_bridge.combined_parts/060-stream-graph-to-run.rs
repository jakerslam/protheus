
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

    let governed =
        crate::framework_adapter_contract::execute_governed_workflow("langgraph", payload)?;
    let workflow_id = governed.workflow_id.clone();
    as_object_mut(state, "governed_workflows")
        .insert(workflow_id.clone(), governed.payload.clone());
    Ok(json!({
        "ok": true,
        "workflow_id": workflow_id,
        "governed_workflow": governed.payload,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-002.8", semantic_claim("V6-WORKFLOW-002.8")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(error) => {
            print_json_line(&cli_error("workflow_graph_bridge_error", &error));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let swarm_path = swarm_state_path(root, argv, payload);
    let native_trace_path = trace_path(root, argv, payload);

    if command == "status" {
        let state = load_state(&state_path);
        let receipt = cli_receipt(
            "workflow_graph_bridge_status",
            json!({
                "ok": true,
                "schema_version": state.get("schema_version").cloned().unwrap_or_else(|| json!(null)),
                "graphs": state.get("graphs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "checkpoints": state.get("checkpoints").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "inspections": state.get("inspections").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "interrupts": state.get("interrupts").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "subgraphs": state.get("subgraphs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "governed_workflows": state.get("governed_workflows").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "traces": state.get("traces").and_then(Value::as_array).map(|row| row.len()).unwrap_or(0),
                "streams": state.get("streams").and_then(Value::as_array).map(|row| row.len()).unwrap_or(0),
                "state_path": rel(root, &state_path),
                "history_path": rel(root, &history_path),
            }),
        );
        print_json_line(&receipt);
        return 0;
    }

    let mut state = load_state(&state_path);
    let payload_result = match command {
        "register-graph" => register_graph(&mut state, payload),
        "checkpoint-run" => checkpoint_run(&mut state, payload),
        "inspect-state" => inspect_state(&mut state, payload),
        "interrupt-run" => interrupt_run(&mut state, payload),
        "resume-run" => resume_run(&mut state, payload),
        "coordinate-subgraph" => coordinate_subgraph(&mut state, &swarm_path, payload),
        "record-trace" => record_trace(root, &mut state, &native_trace_path, payload),
        "stream-graph" => stream_graph(&mut state, payload),
        "run-governed-workflow" => run_governed_workflow(&mut state, payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => {
            print_json_line(&cli_error(
                "workflow_graph_bridge_error",
                &format!("unknown_workflow_graph_bridge_command:{command}"),
            ));
            return 1;
        }
    };

    let payload_out = match payload_result {
        Ok(value) => value,
        Err(error) => {
            let receipt = cli_error("workflow_graph_bridge_error", &error);
            print_json_line(&receipt);
            return 1;
        }
    };
    let receipt = cli_receipt("workflow_graph_bridge_receipt", payload_out);
    state["last_receipt"] = receipt.clone();
    if let Err(error) = save_state(&state_path, &state) {
        let err = cli_error("workflow_graph_bridge_error", &error);
        print_json_line(&err);
        return 1;
    }
    if let Err(error) = append_history(&history_path, &receipt) {
        let err = cli_error("workflow_graph_bridge_error", &error);
        print_json_line(&err);
        return 1;
    }
    print_json_line(&receipt);
    0
}

