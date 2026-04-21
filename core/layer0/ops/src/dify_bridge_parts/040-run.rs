
pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(error) => {
            print_json_line(&cli_error("dify_bridge_error", &error));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let trace_path = trace_path(root, argv, payload);
    let dashboard_dir = dashboard_dir(root, argv, payload);

    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let mut state = load_state(&state_path);
    let payload_out = match command {
        "status" => Ok(json!({
            "ok": true,
            "schema_version": state.get("schema_version").cloned().unwrap_or_else(|| json!(null)),
            "canvases": state.get("canvases").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "knowledge_bases": state.get("knowledge_bases").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "agent_apps": state.get("agent_apps").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "dashboards": state.get("dashboards").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "provider_routes": state.get("provider_routes").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "flow_runs": state.get("flow_runs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "audit_traces": state.get("audit_traces").and_then(Value::as_array).map(|row| row.len()).unwrap_or(0),
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
        })),
        "register-canvas" => register_canvas(&mut state, payload),
        "sync-knowledge-base" => sync_knowledge_base(root, &mut state, payload),
        "register-agent-app" => register_agent_app(root, &mut state, payload),
        "publish-dashboard" => publish_dashboard(root, &mut state, &dashboard_dir, payload),
        "route-provider" => route_provider(root, &mut state, payload),
        "run-conditional-flow" => {
            run_conditional_flow(root, &mut state, &swarm_state_path, payload)
        }
        "record-audit-trace" => record_audit_trace(root, &mut state, &trace_path, payload),
        _ => Err(format!("unknown_dify_bridge_command:{command}")),
    };

    let payload_out = match payload_out {
        Ok(value) => value,
        Err(error) => {
            print_json_line(&cli_error("dify_bridge_error", &error));
            return 1;
        }
    };

    let receipt = cli_receipt("dify_bridge_receipt", payload_out);
    state["last_receipt"] = receipt.clone();
    if let Err(error) = save_state(&state_path, &state) {
        print_json_line(&cli_error("dify_bridge_error", &error));
        return 1;
    }
    if let Err(error) = append_history(&history_path, &receipt) {
        print_json_line(&cli_error("dify_bridge_error", &error));
        return 1;
    }
    print_json_line(&receipt);
    0
}
