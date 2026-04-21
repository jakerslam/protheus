
fn ingest_config(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/metagpt_config_bridge.ts"),
    )?;
    let yaml = payload
        .get("config_yaml")
        .and_then(Value::as_str)
        .ok_or_else(|| "metagpt_config_yaml_required".to_string())?;
    let parsed_yaml: Value = serde_yaml::from_str::<Value>(yaml)
        .map_err(|err| format!("metagpt_config_yaml_parse_failed:{err}"))?;
    let extensions = parsed_yaml
        .get("extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let unsupported: Vec<String> = extensions
        .iter()
        .filter_map(|row| row.as_str())
        .filter(|row| row.contains("shell:") || row.contains("rm "))
        .map(ToString::to_string)
        .collect();
    if !unsupported.is_empty() {
        return Err(format!(
            "metagpt_config_extension_unsupported:{}",
            unsupported.join(",")
        ));
    }
    let record = json!({
        "config_id": stable_id("mgcfg", &json!({"yaml": yaml})),
        "bridge_path": adapter_path,
        "roles": parsed_yaml.get("roles").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "sops": parsed_yaml.get("sops").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "extensions": extensions,
        "parsed": parsed_yaml,
        "ingested_at": now_iso(),
    });
    let id = record
        .get("config_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "configs").insert(id, record.clone());
    Ok(
        json!({"ok": true, "config": record, "claim_evidence": claim("V6-WORKFLOW-006.8", "metagpt_yaml_and_extension_assets_are_ingested_through_governed_adapter_owned_manifests")}),
    )
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
            print_json_line(&cli_error("metagpt_bridge_error", &error));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let approval_queue_path = approval_queue_path(root, argv, payload);
    let trace_path = trace_path(root, argv, payload);

    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let mut state = load_state(&state_path);
    let payload_out = match command {
        "status" => Ok(json!({
            "ok": true,
            "schema_version": state.get("schema_version").cloned().unwrap_or_else(|| json!(null)),
            "companies": state.get("companies").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "sop_runs": state.get("sop_runs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "pr_simulations": state.get("pr_simulations").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "debates": state.get("debates").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "requirements": state.get("requirements").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "oversight": state.get("oversight").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "traces": state.get("traces").and_then(Value::as_array).map(|row| row.len()).unwrap_or(0),
            "configs": state.get("configs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
            "state_path": rel(root, &state_path),
            "history_path": rel(root, &history_path),
        })),
        "register-company" => register_company(&mut state, payload),
        "run-sop" => run_sop(&mut state, payload),
        "simulate-pr" => simulate_pr(root, &mut state, payload),
        "run-debate" => run_debate(&mut state, payload),
        "plan-requirements" => plan_requirements(&mut state, payload),
        "record-oversight" => record_oversight(&mut state, &approval_queue_path, payload),
        "record-pipeline-trace" => record_pipeline_trace(root, &mut state, &trace_path, payload),
        "ingest-config" => ingest_config(root, &mut state, payload),
        _ => Err(format!("unknown_metagpt_bridge_command:{command}")),
    };

    let payload_out = match payload_out {
        Ok(value) => value,
        Err(error) => {
            print_json_line(&cli_error("metagpt_bridge_error", &error));
            return 1;
        }
    };

    let receipt = cli_receipt("metagpt_bridge_receipt", payload_out);
    state["last_receipt"] = receipt.clone();
    if let Err(error) = save_state(&state_path, &state) {
        print_json_line(&cli_error("metagpt_bridge_error", &error));
        return 1;
    }
    if let Err(error) = append_history(&history_path, &receipt) {
        print_json_line(&cli_error("metagpt_bridge_error", &error));
        return 1;
    }
    print_json_line(&receipt);
    0
}
