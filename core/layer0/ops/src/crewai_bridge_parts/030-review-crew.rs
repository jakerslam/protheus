fn review_crew(
    state: &mut Value,
    approval_queue_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_review_crew_id_required".to_string());
    }
    let _crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let review = json!({
        "review_id": stable_id("crreview", &json!({"crew_id": crew_id, "run_id": payload.get("run_id"), "operator": payload.get("operator_id")})),
        "crew_id": crew_id,
        "run_id": clean_token(payload.get("run_id").and_then(Value::as_str), ""),
        "operator_id": clean_token(payload.get("operator_id").and_then(Value::as_str), "operator"),
        "action": clean_token(payload.get("action").and_then(Value::as_str), "approve"),
        "notes": clean_text(payload.get("notes").and_then(Value::as_str), 180),
        "reviewed_at": now_iso(),
    });
    let mut queue = load_review_queue(approval_queue_path);
    as_array_mut(&mut queue, "entries").push(review.clone());
    save_review_queue(approval_queue_path, &queue)?;
    let review_id = review
        .get("review_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "reviews").insert(review_id, review.clone());
    Ok(json!({
        "ok": true,
        "review": review,
        "approval_queue_path": approval_queue_path.display().to_string(),
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.7", semantic_claim("V6-WORKFLOW-004.7")),
    }))
}

fn record_amp_trace(
    root: &Path,
    state: &mut Value,
    trace_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let crew_id = clean_token(payload.get("crew_id").and_then(Value::as_str), "");
    if crew_id.is_empty() {
        return Err("crewai_trace_crew_id_required".to_string());
    }
    let _crew = state
        .get("crews")
        .and_then(Value::as_object)
        .and_then(|rows| rows.get(&crew_id))
        .cloned()
        .ok_or_else(|| format!("unknown_crewai_crew:{crew_id}"))?;
    let trace = json!({
        "trace_id": stable_id("crtrace", &json!({"crew_id": crew_id, "stage": payload.get("stage"), "message": payload.get("message")})),
        "crew_id": crew_id,
        "run_id": clean_token(payload.get("run_id").and_then(Value::as_str), ""),
        "stage": clean_token(payload.get("stage").and_then(Value::as_str), "execution"),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "controls": payload.get("controls").cloned().unwrap_or_else(|| json!({})),
        "trace_path": rel(root, trace_path),
        "recorded_at": now_iso(),
    });
    emit_amp_trace(trace_path, &trace)?;
    as_array_mut(state, "traces").push(trace.clone());
    Ok(json!({
        "ok": true,
        "trace": trace,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.8", semantic_claim("V6-WORKFLOW-004.8")),
    }))
}

fn benchmark_parity(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let metrics = payload
        .get("metrics")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| "crewai_benchmark_metrics_required".to_string())?;
    let cold_start_ms = metrics
        .get("cold_start_ms")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let throughput_ops_sec = metrics
        .get("throughput_ops_sec")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let memory_mb = metrics
        .get("memory_mb")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let targets = payload.get("targets").and_then(Value::as_object).cloned().unwrap_or_else(|| {
        match profile.as_str() {
            "tiny-max" => json!({"max_cold_start_ms": 8.0, "min_throughput_ops_sec": 3000.0, "max_memory_mb": 8.0}).as_object().cloned().unwrap(),
            "pure" => json!({"max_cold_start_ms": 10.0, "min_throughput_ops_sec": 2500.0, "max_memory_mb": 12.0}).as_object().cloned().unwrap(),
            _ => json!({"max_cold_start_ms": 20.0, "min_throughput_ops_sec": 2000.0, "max_memory_mb": 64.0}).as_object().cloned().unwrap(),
        }
    });
    let parity_ok = cold_start_ms
        <= targets
            .get("max_cold_start_ms")
            .and_then(Value::as_f64)
            .unwrap_or(f64::MAX)
        && throughput_ops_sec
            >= targets
                .get("min_throughput_ops_sec")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        && memory_mb
            <= targets
                .get("max_memory_mb")
                .and_then(Value::as_f64)
                .unwrap_or(f64::MAX);
    let record = json!({
        "benchmark_id": stable_id("crbench", &json!({"profile": profile, "metrics": metrics})),
        "profile": profile,
        "metrics": metrics,
        "targets": targets,
        "parity_ok": parity_ok,
        "recorded_at": now_iso(),
    });
    let record_id = record
        .get("benchmark_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "benchmarks").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "benchmark": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.9", semantic_claim("V6-WORKFLOW-004.9")),
    }))
}

fn route_model(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let profile = clean_token(payload.get("profile").and_then(Value::as_str), "rich");
    let modality = clean_token(payload.get("modality").and_then(Value::as_str), "text");
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/crewai_tool_bridge.ts"),
    )?;
    let local_models = payload
        .get("local_models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let providers = payload
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supported = match profile.as_str() {
        "tiny-max" => matches!(modality.as_str(), "text"),
        "pure" => matches!(modality.as_str(), "text" | "image"),
        _ => true,
    };
    let degraded = !supported;
    let selected_route = if payload
        .get("prefer_local")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && !local_models.is_empty()
    {
        json!({"route_kind": "local_model", "target": local_models.first().cloned().unwrap_or_else(|| json!(null))})
    } else if !providers.is_empty() {
        json!({"route_kind": "provider", "target": providers.first().cloned().unwrap_or_else(|| json!(null))})
    } else if !local_models.is_empty() {
        json!({"route_kind": "local_model", "target": local_models.first().cloned().unwrap_or_else(|| json!(null))})
    } else {
        return Err("crewai_model_route_target_required".to_string());
    };
    let record = json!({
        "route_id": stable_id("crroute", &json!({"profile": profile, "modality": modality, "route": selected_route})),
        "profile": profile,
        "modality": modality,
        "bridge_path": adapter_path,
        "selected_route": selected_route,
        "degraded": degraded,
        "routed_at": now_iso(),
    });
    let record_id = record
        .get("route_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "model_routes").insert(record_id, record.clone());
    Ok(json!({
        "ok": true,
        "model_route": record,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.10", semantic_claim("V6-WORKFLOW-004.10")),
    }))
}

fn run_governed_workflow(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let governed = crate::framework_adapter_contract::execute_governed_workflow("crewai", payload)?;
    let workflow_id = governed.workflow_id.clone();
    as_object_mut(state, "governed_workflows")
        .insert(workflow_id.clone(), governed.payload.clone());
    Ok(json!({
        "ok": true,
        "workflow_id": workflow_id,
        "governed_workflow": governed.payload,
        "claim_evidence": default_claim_evidence("V6-WORKFLOW-004.11", semantic_claim("V6-WORKFLOW-004.11")),
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
            print_json_line(&cli_error("crewai_bridge_error", &error));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let swarm_state_path = swarm_state_path(root, argv, payload);
    let approval_queue_path = approval_queue_path(root, argv, payload);
    let trace_path = trace_path(root, argv, payload);

    if command == "status" {
        let state = load_state(&state_path);
        let receipt = cli_receipt(
            "crewai_bridge_status",
            json!({
                "ok": true,
                "schema_version": state.get("schema_version").cloned().unwrap_or_else(|| json!(null)),
                "crews": state.get("crews").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "process_runs": state.get("process_runs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "flow_runs": state.get("flow_runs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "memory_records": state.get("memory_records").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "configs": state.get("configs").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "delegations": state.get("delegations").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "reviews": state.get("reviews").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "traces": state.get("traces").and_then(Value::as_array).map(|row| row.len()).unwrap_or(0),
                "benchmarks": state.get("benchmarks").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "model_routes": state.get("model_routes").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "governed_workflows": state.get("governed_workflows").and_then(Value::as_object).map(|row| row.len()).unwrap_or(0),
                "state_path": rel(root, &state_path),
                "history_path": rel(root, &history_path),
            }),
        );
        print_json_line(&receipt);
        return 0;
    }

    let mut state = load_state(&state_path);
    let payload_out = match command {
        "register-crew" => register_crew(&mut state, payload),
        "run-process" => run_process(&mut state, &swarm_state_path, payload),
        "run-flow" => run_flow(&mut state, payload),
        "memory-bridge" => memory_bridge(&mut state, payload),
        "ingest-config" => ingest_config(root, &mut state, payload),
        "route-delegation" => route_delegation(root, &mut state, &swarm_state_path, payload),
        "review-crew" => review_crew(&mut state, &approval_queue_path, payload),
        "record-amp-trace" => record_amp_trace(root, &mut state, &trace_path, payload),
        "benchmark-parity" => benchmark_parity(&mut state, payload),
        "route-model" => route_model(root, &mut state, payload),
        "run-governed-workflow" => run_governed_workflow(&mut state, payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => {
            print_json_line(&cli_error(
                "crewai_bridge_error",
                &format!("unknown_crewai_bridge_command:{command}"),
            ));
            return 1;
        }
    };

    let payload_out = match payload_out {
        Ok(value) => value,
        Err(error) => {
            print_json_line(&cli_error("crewai_bridge_error", &error));
            return 1;
        }
    };

    let receipt = cli_receipt("crewai_bridge_receipt", payload_out);
    state["last_receipt"] = receipt.clone();
    if let Err(error) = save_state(&state_path, &state) {
        print_json_line(&cli_error("crewai_bridge_error", &error));
        return 1;
    }
    if let Err(error) = append_history(&history_path, &receipt) {
        print_json_line(&cli_error("crewai_bridge_error", &error));
        return 1;
    }
    print_json_line(&receipt);
    0
}
