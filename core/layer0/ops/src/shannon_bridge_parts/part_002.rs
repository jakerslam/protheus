fn schedule_run(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let job_name = clean_text(payload.get("job_name").and_then(Value::as_str), 120);
    if job_name.is_empty() {
        return Err("shannon_schedule_job_name_required".to_string());
    }
    let cron = clean_text(payload.get("cron").and_then(Value::as_str), 80);
    if !looks_like_cron(&cron) {
        return Err("shannon_schedule_invalid_cron".to_string());
    }
    let record = json!({
        "schedule_id": stable_id("shsched", &json!({"job_name": job_name, "cron": cron})),
        "job_name": job_name,
        "cron": cron,
        "pattern_id": clean_token(payload.get("pattern_id").and_then(Value::as_str), ""),
        "priority": parse_u64(payload.get("priority"), 5, 1, 10),
        "budget": payload.get("budget").cloned().unwrap_or_else(|| json!({"tokens": 1024})),
        "recorded_at": now_iso(),
    });
    let id = record
        .get("schedule_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "schedules").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "schedule": record,
        "claim_evidence": claim("V6-WORKFLOW-001.10", "shannon_cron_and_scheduled_runs_emit_receipts_under_existing_budget_and_priority_controls")
    }))
}

fn desktop_shell(
    root: &Path,
    state: &mut Value,
    desktop_history_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let surface = clean_token(payload.get("surface").and_then(Value::as_str), "tray");
    if !matches!(surface.as_str(), "tray" | "notify" | "history") {
        return Err("shannon_desktop_surface_unsupported".to_string());
    }
    let record = json!({
        "desktop_event_id": stable_id("shdesktop", &json!({"surface": surface, "action": payload.get("action")})),
        "surface": surface,
        "action": clean_token(payload.get("action").and_then(Value::as_str), "open"),
        "title": clean_text(payload.get("title").and_then(Value::as_str), 120),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "history_path": rel(root, desktop_history_path),
        "deletable_shell": true,
        "authority_delegate": "core://shannon-bridge",
        "recorded_at": now_iso(),
    });
    if let Some(parent) = desktop_history_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("shannon_desktop_dir_create_failed:{err}"))?;
    }
    lane_utils::write_json(desktop_history_path, &record)?;
    let id = record
        .get("desktop_event_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "desktop_events").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "desktop_event": record,
        "claim_evidence": claim("V6-WORKFLOW-001.11", "shannon_desktop_surfaces_remain_thin_deletable_shells_over_governed_authority")
    }))
}

fn p2p_reliability(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let peer_id = clean_token(payload.get("peer_id").and_then(Value::as_str), "");
    if peer_id.is_empty() {
        return Err("shannon_p2p_peer_id_required".to_string());
    }
    let version = clean_token(payload.get("version").and_then(Value::as_str), "v1");
    let supported_versions = payload
        .get("supported_versions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("v1")]);
    let allowed = supported_versions
        .iter()
        .filter_map(Value::as_str)
        .any(|row| row == version);
    if !allowed {
        return Err("shannon_p2p_version_gate_denied".to_string());
    }
    let message_ids = payload
        .get("message_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut dedup = BTreeSet::new();
    for message_id in &message_ids {
        if let Some(raw) = message_id.as_str() {
            dedup.insert(raw.to_string());
        }
    }
    let record = json!({
        "reliability_id": stable_id("shp2p", &json!({"peer_id": peer_id, "version": version})),
        "peer_id": peer_id,
        "version": version,
        "supported_versions": supported_versions,
        "deduplicated_messages": dedup.len(),
        "version_gate": true,
        "recorded_at": now_iso(),
    });
    let id = record
        .get("reliability_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "p2p_reliability").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "p2p_reliability": record,
        "claim_evidence": claim("V6-WORKFLOW-001.12", "shannon_p2p_reliability_and_deduplication_remain_inside_authoritative_swarm_controls")
    }))
}

fn assimilate_intake(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let shell_path = normalize_surface_path(
        root,
        payload
            .get("shell_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/systems/workflow/shannon_desktop_shell.ts"),
        &["client/runtime/", "apps/"],
    )?;
    let adapter_path = normalize_surface_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/shannon_gateway_bridge.ts"),
        &["adapters/", "client/runtime/"],
    )?;
    let record = json!({
        "intake_id": stable_id("shintake", &json!({"shell_path": shell_path, "adapter_path": adapter_path})),
        "shell_path": shell_path,
        "adapter_path": adapter_path,
        "deletable": true,
        "authority_delegate": "core://shannon-bridge",
        "recorded_at": now_iso(),
    });
    let id = record
        .get("intake_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "intakes").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "intake": record,
        "claim_evidence": claim("V6-WORKFLOW-001.9", "assimilate_shannon_routes_through_a_governed_skill_and_adapter_intake_path")
    }))
}

fn status(root: &Path, state: &Value, state_path: &Path, history_path: &Path) -> Value {
    json!({
        "ok": true,
        "state_path": rel(root, state_path),
        "history_path": rel(root, history_path),
        "patterns": state.get("patterns").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "budget_guards": state.get("budget_guards").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "memory_workspaces": state.get("memory_workspaces").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "replays": state.get("replays").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "approvals": state.get("approvals").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "sandbox_runs": state.get("sandbox_runs").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "observability": state.get("observability").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "gateway_routes": state.get("gateway_routes").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "tool_registrations": state.get("tool_registrations").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "schedules": state.get("schedules").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "desktop_events": state.get("desktop_events").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "p2p_reliability": state.get("p2p_reliability").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "intakes": state.get("intakes").and_then(Value::as_object).map(|rows| rows.len()).unwrap_or(0),
        "last_receipt": state.get("last_receipt").cloned().unwrap_or(Value::Null),
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.trim().to_ascii_lowercase()) else {
        usage();
        return 0;
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("shannon_bridge_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let state_path = state_path(root, argv, payload);
    let history_path = history_path(root, argv, payload);
    let approval_queue_path = approval_queue_path(root, argv, payload);
    let replay_dir = replay_dir(root, argv, payload);
    let observability_trace_path = observability_trace_path(root, argv, payload);
    let observability_metrics_path = observability_metrics_path(root, argv, payload);
    let desktop_history_path = desktop_history_path(root, argv, payload);
    let mut state = load_state(&state_path);

    let result = match command.as_str() {
        "status" => Ok(status(root, &state, &state_path, &history_path)),
        "register-pattern" => record_pattern(&mut state, payload),
        "guard-budget" => guard_budget(&mut state, payload),
        "memory-bridge" => memory_bridge(&mut state, payload),
        "replay-run" => replay_run(root, &mut state, &replay_dir, payload),
        "approval-checkpoint" => approval_checkpoint(&mut state, &approval_queue_path, payload),
        "sandbox-execute" => sandbox_execute(&mut state, payload),
        "record-observability" => record_observability(
            root,
            &mut state,
            &observability_trace_path,
            &observability_metrics_path,
            payload,
        ),
        "gateway-route" => gateway_route(root, &mut state, payload),
        "register-tooling" => register_tooling(root, &mut state, payload),
        "schedule-run" => schedule_run(&mut state, payload),
        "desktop-shell" => desktop_shell(root, &mut state, &desktop_history_path, payload),
        "p2p-reliability" => p2p_reliability(&mut state, payload),
        "assimilate-intake" => assimilate_intake(root, &mut state, payload),
        other => Err(format!("shannon_bridge_unknown_command:{other}")),
    };

    match result {
        Ok(payload_out) => {
            let receipt = cli_receipt(
                match command.as_str() {
                    "status" => "shannon_bridge_status",
                    "register-pattern" => "shannon_bridge_register_pattern",
                    "guard-budget" => "shannon_bridge_guard_budget",
                    "memory-bridge" => "shannon_bridge_memory_bridge",
                    "replay-run" => "shannon_bridge_replay_run",
                    "approval-checkpoint" => "shannon_bridge_approval_checkpoint",
                    "sandbox-execute" => "shannon_bridge_sandbox_execute",
                    "record-observability" => "shannon_bridge_record_observability",
                    "gateway-route" => "shannon_bridge_gateway_route",
                    "register-tooling" => "shannon_bridge_register_tooling",
                    "schedule-run" => "shannon_bridge_schedule_run",
                    "desktop-shell" => "shannon_bridge_desktop_shell",
                    "p2p-reliability" => "shannon_bridge_p2p_reliability",
                    "assimilate-intake" => "shannon_bridge_assimilate_intake",
                    _ => "shannon_bridge_command",
                },
                payload_out,
            );
            state["last_receipt"] = receipt.clone();
            if command != "status" {
                if let Err(err) = save_state(&state_path, &state) {
                    print_json_line(&cli_error("shannon_bridge_error", &err));
                    return 1;
                }
                if let Err(err) = append_history(&history_path, &receipt) {
                    print_json_line(&cli_error("shannon_bridge_error", &err));
                    return 1;
                }
            }
            print_json_line(&receipt);
            0
        }
        Err(err) => {
            print_json_line(&cli_error("shannon_bridge_error", &err));
            1
        }
    }
}

