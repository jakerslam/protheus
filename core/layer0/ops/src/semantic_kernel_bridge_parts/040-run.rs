fn invoke_dotnet_bridge(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let bridge_id = clean_token(payload.get("bridge_id").and_then(Value::as_str), "");
    if bridge_id.is_empty() {
        return Err("semantic_kernel_dotnet_bridge_required".to_string());
    }
    let bridges = as_object_mut(state, "dotnet_bridges");
    let bridge = bridges
        .get_mut(&bridge_id)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "semantic_kernel_dotnet_bridge_not_found".to_string())?;
    let dry_run = payload
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let invocation = if dry_run
        || bridge
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
    {
        json!({
            "mode": "dry_run",
            "operation": clean_token(payload.get("operation").and_then(Value::as_str), "invoke"),
            "arguments": payload.get("args").cloned().unwrap_or_else(|| json!({})),
            "simulated": true,
        })
    } else {
        let command = bridge.get("command").and_then(Value::as_str).unwrap_or("");
        let command_args = bridge
            .get("command_args")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        let operation = clean_token(payload.get("operation").and_then(Value::as_str), "invoke");
        let args_json = payload
            .get("args")
            .cloned()
            .unwrap_or_else(|| json!({}))
            .to_string();
        let run = Command::new(command)
            .args(command_args)
            .arg(operation)
            .env("PROTHEUS_SK_DOTNET_ARGS", args_json)
            .output()
            .map_err(|err| format!("semantic_kernel_dotnet_exec_failed:{err}"))?;
        if !run.status.success() {
            return Err(format!(
                "semantic_kernel_dotnet_exec_nonzero:{}",
                String::from_utf8_lossy(&run.stderr)
            ));
        }
        json!({
            "mode": "process_exec",
            "stdout": String::from_utf8_lossy(&run.stdout).trim().to_string(),
            "stderr": String::from_utf8_lossy(&run.stderr).trim().to_string(),
            "exit_code": run.status.code().unwrap_or(0),
        })
    };
    bridge.insert("last_invoked_at".to_string(), json!(now_iso()));
    Ok(json!({
        "ok": true,
        "bridge_id": bridge_id,
        "invocation": invocation,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-008.9", semantic_claim("V6-WORKFLOW-008.9")),
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match payload_json(&argv[1..]) {
        Ok(payload) => payload,
        Err(err) => {
            print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
            return 1;
        }
    };
    let input = payload_obj(&payload);
    let state_path = state_path(root, argv, input);
    let history_path = history_path(root, argv, input);
    let mut state = load_state(&state_path);

    let result = match command.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
            "services": as_object_mut(&mut state, "services").len(),
            "plugins": as_object_mut(&mut state, "plugins").len(),
            "collaborations": as_object_mut(&mut state, "collaborations").len(),
            "plans": as_object_mut(&mut state, "plans").len(),
            "vector_connectors": as_object_mut(&mut state, "vector_connectors").len(),
            "llm_connectors": as_object_mut(&mut state, "llm_connectors").len(),
            "structured_processes": as_object_mut(&mut state, "structured_processes").len(),
            "enterprise_events": as_array_mut(&mut state, "enterprise_events").len(),
            "dotnet_bridges": as_object_mut(&mut state, "dotnet_bridges").len(),
            "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
        })),
        "register-service" => register_service(&mut state, input),
        "register-plugin" => register_plugin(root, &mut state, input),
        "invoke-plugin" => invoke_plugin(&mut state, input),
        "collaborate" => collaborate(root, argv, &mut state, input),
        "plan" => plan(&mut state, input),
        "register-vector-connector" => register_vector_connector(&mut state, input),
        "retrieve" => retrieve(&mut state, input),
        "register-llm-connector" => register_llm_connector(&mut state, input),
        "route-llm" => route_llm(&mut state, input),
        "validate-structured-output" => validate_structured_output(&mut state, input),
        "emit-enterprise-event" => emit_enterprise_event(&mut state, input),
        "register-dotnet-bridge" => register_dotnet_bridge(root, &mut state, input),
        "invoke-dotnet-bridge" => invoke_dotnet_bridge(&mut state, input),
        _ => Err(format!("unknown_command:{command}")),
    };

    match result {
        Ok(payload_out) => {
            let receipt = cli_receipt(
                &format!("semantic_kernel_bridge_{}", command.replace('-', "_")),
                payload_out,
            );
            state["last_receipt"] = receipt.clone();
            if let Err(err) = save_state(&state_path, &state) {
                print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
                return 1;
            }
            if let Err(err) = append_history(&history_path, &receipt) {
                print_json_line(&cli_error("semantic_kernel_bridge_error", &err));
                return 1;
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            let receipt = cli_error("semantic_kernel_bridge_error", &err);
            print_json_line(&receipt);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_output_validator_catches_missing_required() {
        let schema = json!({
            "type": "object",
            "required": ["answer"],
            "properties": {
                "answer": { "type": "string" }
            }
        });
        let output = json!({"other": true});
        let mut violations = Vec::new();
        validate_json_schema(&schema, &output, "$", &mut violations);
        assert!(violations
            .iter()
            .any(|row| row.contains("missing_required")));
    }

    #[test]
    fn planner_prefers_matching_functions() {
        let mut state = default_state();
        let service = register_service(
            &mut state,
            &json!({"name":"planner-service","execution_surface":"workflow-executor"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .expect("service");
        let service_id = service["service"]["service_id"]
            .as_str()
            .unwrap()
            .to_string();
        let result = plan(
            &mut state,
            &json!({
                "service_id": service_id,
                "objective": "summarize then route the case",
                "functions": [
                    {"name":"route","score":0.6},
                    {"name":"summarize","score":0.4}
                ]
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .expect("plan");
        let steps = result["plan"]["steps"].as_array().expect("steps");
        assert_eq!(
            steps
                .first()
                .and_then(|row| row.get("function_name"))
                .and_then(Value::as_str),
            Some("route")
        );
    }
}

