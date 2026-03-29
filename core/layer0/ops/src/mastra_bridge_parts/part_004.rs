fn deploy_shell(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_shell_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/mastra_bridge.ts"),
    )?;
    let target = clean_token(payload.get("target").and_then(Value::as_str), "local");
    let record = json!({
        "deployment_id": stable_id("mastradep", &json!({"shell_path": shell_path, "target": target})),
        "shell_name": clean_token(payload.get("shell_name").and_then(Value::as_str), "mastra-shell"),
        "shell_path": shell_path,
        "target": target,
        "deletable": true,
        "authority_delegate": "core://mastra-bridge",
        "artifact_path": clean_text(payload.get("artifact_path").and_then(Value::as_str), 240),
        "deployed_at": now_iso(),
    });
    let deployment_id = record
        .get("deployment_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "deployments").insert(deployment_id, record.clone());
    Ok(json!({
        "ok": true,
        "deployment": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-011.8", mastra_claim("V6-WORKFLOW-011.8")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = argv[0].as_str();
    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("mastra_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "graphs": as_object_mut(&mut state, "graphs").len(),
            "graph_runs": as_object_mut(&mut state, "graph_runs").len(),
            "agent_loops": as_object_mut(&mut state, "agent_loops").len(),
            "memory_recalls": as_object_mut(&mut state, "memory_recalls").len(),
            "suspended_runs": as_object_mut(&mut state, "suspended_runs").len(),
            "mcp_bridges": as_object_mut(&mut state, "mcp_bridges").len(),
            "run_snapshots": as_object_mut(&mut state, "run_snapshots").len(),
            "eval_traces": as_object_mut(&mut state, "eval_traces").len(),
            "deployments": as_object_mut(&mut state, "deployments").len(),
            "runtime_bridges": as_object_mut(&mut state, "runtime_bridges").len(),
            "intakes": as_object_mut(&mut state, "intakes").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-graph" => register_graph(&mut state, input),
        "execute-graph" => execute_graph(root, argv, &mut state, input),
        "run-agent-loop" => run_agent_loop(root, argv, &mut state, input),
        "memory-recall" => memory_recall(root, &mut state, input),
        "suspend-run" => suspend_run(root, argv, &mut state, input),
        "resume-run" => resume_run(root, argv, &mut state, input),
        "register-mcp-bridge" => register_mcp_bridge(root, &mut state, input),
        "invoke-mcp-bridge" => invoke_mcp_bridge(root, argv, &mut state, input),
        "record-eval-trace" => record_eval_trace(root, &mut state, input),
        "register-runtime-bridge" => register_runtime_bridge(root, &mut state, input),
        "route-model" => route_model(&state, input),
        "deploy-shell" => deploy_shell(root, &mut state, input),
        "scaffold-intake" => scaffold_intake(root, &mut state, input),
        "run-llm-agent" => run_llm_agent(root, argv, &mut state, input),
        "register-tool-manifest" => register_tool_manifest(root, &mut state, input),
        "invoke-tool-manifest" => invoke_tool_manifest(root, argv, &mut state, input),
        "approval-checkpoint" => approval_checkpoint(root, argv, &mut state, input),
        "rewind-session" => rewind_session(root, argv, &mut state, input),
        "record-evaluation" => record_evaluation(root, &mut state, input),
        _ => Err(format!("unknown_mastra_bridge_command:{command}")),
    };

    match result {
        Ok(payload) => {
            let receipt = cli_receipt(
                &format!("mastra_bridge_{}", command.replace('-', "_")),
                payload,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state)
                .and_then(|_| append_history(&history_path, &receipt))
            {
                print_json_line(&cli_error("mastra_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("mastra_bridge_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_bridge_route_degrades_polyglot_in_pure_mode() {
        let mut state = default_state();
        let payload = json!({
            "name": "python-gateway",
            "language": "python",
            "provider": "google",
            "bridge_path": "adapters/polyglot/mastra_runtime_bridge.ts",
            "supported_profiles": ["rich", "pure"]
        });
        let _ = register_runtime_bridge(Path::new("."), &mut state, payload.as_object().unwrap())
            .expect("register");
        let out = route_model(
            &state,
            &Map::from_iter([
                ("language".to_string(), json!("python")),
                ("provider".to_string(), json!("google")),
                ("model".to_string(), json!("gemini-2.0-flash")),
                ("profile".to_string(), json!("pure")),
            ]),
        )
        .expect("route");
        assert_eq!(out["route"]["degraded"].as_bool(), Some(true));
    }
}

