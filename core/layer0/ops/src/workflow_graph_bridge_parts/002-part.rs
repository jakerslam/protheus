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
