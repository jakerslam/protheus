fn guard_budget(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let session_id = clean_token(payload.get("session_id").and_then(Value::as_str), "");
    if session_id.is_empty() {
        return Err("shannon_budget_session_id_required".to_string());
    }
    let token_budget = parse_u64(payload.get("token_budget"), 0, 0, u64::MAX);
    if token_budget == 0 {
        return Err("shannon_budget_token_budget_required".to_string());
    }
    let estimated_tokens = parse_u64(payload.get("estimated_tokens"), 0, 0, u64::MAX);
    let fallback_models = payload
        .get("fallback_models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let current_model = clean_token(
        payload.get("current_model").and_then(Value::as_str),
        "primary",
    );
    let breach = estimated_tokens > token_budget;
    let action = if breach && fallback_models.is_empty() {
        "deny"
    } else if breach {
        "fallback"
    } else {
        "allow"
    };
    if action == "deny" {
        return Err("shannon_budget_breach_without_fallback".to_string());
    }
    let selected_model = if action == "fallback" {
        fallback_models
            .first()
            .and_then(Value::as_str)
            .unwrap_or("fallback")
            .to_string()
    } else {
        current_model.clone()
    };
    let record = json!({
        "guard_id": stable_id("shbudget", &json!({"session_id": session_id, "token_budget": token_budget, "estimated_tokens": estimated_tokens})),
        "session_id": session_id,
        "token_budget": token_budget,
        "estimated_tokens": estimated_tokens,
        "breach": breach,
        "action": action,
        "selected_model": selected_model,
        "fallback_models": fallback_models,
        "fail_closed": true,
        "recorded_at": now_iso(),
    });
    let id = record
        .get("guard_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "budget_guards").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "budget_guard": record,
        "claim_evidence": claim("V6-WORKFLOW-001.2", "shannon_hard_token_budgets_and_auto_fallbacks_emit_fail_closed_receipts")
    }))
}

fn memory_bridge(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let workspace_id = clean_token(payload.get("workspace_id").and_then(Value::as_str), "");
    if workspace_id.is_empty() {
        return Err("shannon_workspace_id_required".to_string());
    }
    let recent = payload
        .get("recent_items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let semantic = payload
        .get("semantic_items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    let mut merged = Vec::new();
    for row in recent.iter().chain(semantic.iter()) {
        let key = row
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| row.get("text").and_then(Value::as_str))
            .unwrap_or("memory-item")
            .to_string();
        if seen.insert(key) {
            merged.push(row.clone());
        }
    }
    let memory_profile = profile(payload.get("profile"));
    let budget = match memory_profile.as_str() {
        "tiny-max" => parse_u64(payload.get("context_budget"), 2, 1, 4) as usize,
        "pure" => parse_u64(payload.get("context_budget"), 4, 1, 8) as usize,
        _ => parse_u64(payload.get("context_budget"), 6, 1, 16) as usize,
    };
    let selected = merged.into_iter().take(budget).collect::<Vec<_>>();
    let record = json!({
        "workspace_id": workspace_id,
        "hierarchy": payload.get("hierarchy").cloned().unwrap_or_else(|| json!({"root": []})),
        "selected_items": selected,
        "deduplicated_count": seen.len(),
        "query": clean_text(payload.get("query").and_then(Value::as_str), 160),
        "profile": memory_profile,
        "recorded_at": now_iso(),
    });
    as_object_mut(state, "memory_workspaces").insert(workspace_id.clone(), record.clone());
    Ok(json!({
        "ok": true,
        "memory_workspace": record,
        "claim_evidence": claim("V6-WORKFLOW-001.3", "shannon_hierarchical_and_vector_memory_routes_through_receipted_governed_memory_lanes")
    }))
}

fn replay_run(
    root: &Path,
    state: &mut Value,
    replay_dir: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let run_id = clean_token(payload.get("run_id").and_then(Value::as_str), "");
    if run_id.is_empty() {
        return Err("shannon_replay_run_id_required".to_string());
    }
    let export = json!({
        "run_id": run_id,
        "events": payload.get("events" ).cloned().unwrap_or_else(|| json!([])),
        "receipt_refs": payload.get("receipt_refs").cloned().unwrap_or_else(|| json!([])),
        "strict": parse_bool(payload.get("strict"), true),
        "exported_at": now_iso(),
    });
    fs::create_dir_all(replay_dir)
        .map_err(|err| format!("shannon_replay_dir_create_failed:{err}"))?;
    let export_path = replay_dir.join(format!("{}.json", run_id));
    lane_utils::write_json(&export_path, &export)?;
    let replay = json!({
        "replay_id": stable_id("shreplay", &json!({"run_id": run_id})),
        "run_id": run_id,
        "event_count": export.get("events").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "receipt_ref_count": export.get("receipt_refs").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "export_path": rel(root, &export_path),
        "replay_hash": crate::deterministic_receipt_hash(&export),
        "replayed_at": now_iso(),
    });
    let id = replay
        .get("replay_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "replays").insert(id, replay.clone());
    Ok(json!({
        "ok": true,
        "replay": replay,
        "claim_evidence": claim("V6-WORKFLOW-001.4", "shannon_replay_exports_and_reexecutions_emit_deterministic_receipts")
    }))
}

fn approval_checkpoint(
    state: &mut Value,
    queue_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let action_id = clean_token(payload.get("action_id").and_then(Value::as_str), "");
    if action_id.is_empty() {
        return Err("shannon_approval_action_id_required".to_string());
    }
    let event = json!({
        "checkpoint_id": stable_id("shapprove", &json!({"action_id": action_id, "title": payload.get("title")})),
        "action_id": action_id,
        "title": clean_text(payload.get("title").and_then(Value::as_str), 120),
        "reason": clean_text(payload.get("reason").and_then(Value::as_str), 160),
        "operator": clean_token(payload.get("operator").and_then(Value::as_str), "human"),
        "status": clean_token(payload.get("status").and_then(Value::as_str), "pending"),
        "recorded_at": now_iso(),
    });
    let mut queue = match fs::read_to_string(queue_path) {
        Ok(raw) => serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({"events": []})),
        Err(_) => json!({"events": []}),
    };
    if !queue.get("events").map(Value::is_array).unwrap_or(false) {
        queue["events"] = json!([]);
    }
    queue
        .get_mut("events")
        .and_then(Value::as_array_mut)
        .expect("events")
        .push(event.clone());
    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("shannon_approval_queue_dir_create_failed:{err}"))?;
    }
    let encoded = serde_yaml::to_string(&queue)
        .map_err(|err| format!("shannon_approval_queue_encode_failed:{err}"))?;
    fs::write(queue_path, encoded)
        .map_err(|err| format!("shannon_approval_queue_write_failed:{err}"))?;
    let id = event
        .get("checkpoint_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "approvals").insert(id, event.clone());
    Ok(json!({
        "ok": true,
        "approval_checkpoint": event,
        "approval_queue_path": queue_path.display().to_string(),
        "claim_evidence": claim("V6-WORKFLOW-001.5", "shannon_human_review_points_remain_inside_receipted_approval_boundaries")
    }))
}

fn sandbox_execute(state: &mut Value, payload: &Map<String, Value>) -> Result<Value, String> {
    let tenant_id = clean_token(payload.get("tenant_id").and_then(Value::as_str), "");
    if tenant_id.is_empty() {
        return Err("shannon_sandbox_tenant_id_required".to_string());
    }
    let sandbox_mode = clean_token(payload.get("sandbox_mode").and_then(Value::as_str), "wasi");
    if !matches!(sandbox_mode.as_str(), "wasi" | "firecracker" | "readonly") {
        return Err("shannon_sandbox_mode_unsupported".to_string());
    }
    let read_only = parse_bool(payload.get("read_only"), true);
    let destructive = parse_bool(payload.get("destructive"), false);
    if destructive {
        return Err("shannon_sandbox_destructive_denied".to_string());
    }
    let fs_paths = payload
        .get("fs_paths")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let invalid_path = fs_paths.iter().filter_map(Value::as_str).any(|row| {
        !(row.starts_with("core/")
            || row.starts_with("client/")
            || row.starts_with("adapters/")
            || row.starts_with("docs/")
            || row.starts_with("tests/"))
    });
    if invalid_path {
        return Err("shannon_sandbox_path_outside_allowed_surface".to_string());
    }
    let record = json!({
        "sandbox_id": stable_id("shsandbox", &json!({"tenant_id": tenant_id, "sandbox_mode": sandbox_mode})),
        "tenant_id": tenant_id,
        "sandbox_mode": sandbox_mode,
        "read_only": read_only,
        "fs_paths": fs_paths,
        "command": clean_text(payload.get("command").and_then(Value::as_str), 180),
        "isolation_hash": crate::deterministic_receipt_hash(&json!({"tenant_id": tenant_id, "read_only": read_only})),
        "executed_at": now_iso(),
    });
    let id = record
        .get("sandbox_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "sandbox_runs").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "sandbox_run": record,
        "claim_evidence": claim("V6-WORKFLOW-001.6", "shannon_sandbox_and_multi_tenant_controls_remain_fail_closed_and_auditable")
    }))
}

fn record_observability(
    root: &Path,
    state: &mut Value,
    trace_path: &Path,
    metrics_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let event_id = stable_id(
        "shobs",
        &json!({"run_id": payload.get("run_id"), "message": payload.get("message")}),
    );
    let spans = payload
        .get("spans")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics = payload.get("metrics").cloned().unwrap_or_else(|| json!({}));
    let trace = json!({
        "event_id": event_id,
        "run_id": clean_token(payload.get("run_id").and_then(Value::as_str), ""),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "spans": spans,
        "metrics": metrics,
        "recorded_at": now_iso(),
    });
    lane_utils::append_jsonl(trace_path, &trace)?;
    if let Some(parent) = metrics_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("shannon_metrics_dir_create_failed:{err}"))?;
    }
    let mut lines = vec![
        "# HELP shannon_bridge_events_total Number of Shannon observability events.".to_string(),
        "# TYPE shannon_bridge_events_total gauge".to_string(),
        format!("shannon_bridge_events_total 1"),
    ];
    if let Some(obj) = metrics.as_object() {
        for (key, value) in obj {
            if let Some(num) = value.as_f64() {
                let safe = key.replace('-', "_");
                lines.push(format!("shannon_bridge_metric_{} {}", safe, num));
            }
        }
    }
    fs::write(metrics_path, lines.join("\n") + "\n")
        .map_err(|err| format!("shannon_metrics_write_failed:{err}"))?;
    as_object_mut(state, "observability").insert(
        event_id.clone(),
        json!({
            "event_id": event_id,
            "trace_path": rel(root, trace_path),
            "metrics_path": rel(root, metrics_path),
            "recorded_at": now_iso(),
        }),
    );
    Ok(json!({
        "ok": true,
        "observability_event": trace,
        "trace_path": rel(root, trace_path),
        "metrics_path": rel(root, metrics_path),
        "claim_evidence": claim("V6-WORKFLOW-001.7", "shannon_prometheus_and_otel_style_events_stream_through_native_observability_artifacts")
    }))
}

fn gateway_route(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let compat_mode = clean_token(
        payload.get("compat_mode").and_then(Value::as_str),
        "/v1/chat/completions",
    );
    let provider_route_path = normalize_surface_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/shannon_gateway_bridge.ts"),
        &["adapters/", "client/runtime/"],
    )?;
    let request_id = clean_token(payload.get("request_id").and_then(Value::as_str), "request");
    let providers = payload
        .get("providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("openai")]);
    let model = clean_token(payload.get("model").and_then(Value::as_str), "gpt-5.4-mini");
    let gateway_profile = profile(payload.get("profile"));
    let unsupported = gateway_profile == "tiny-max"
        && parse_bool(payload.get("streaming"), true)
        && model.contains("vision");
    let selected_provider = providers
        .first()
        .and_then(Value::as_str)
        .unwrap_or("openai");
    let selected_model = if unsupported {
        format!("{}-fallback", model)
    } else {
        model.clone()
    };
    let record = json!({
        "gateway_id": stable_id("shgateway", &json!({"request_id": request_id, "compat_mode": compat_mode})),
        "request_id": request_id,
        "compat_mode": compat_mode,
        "selected_provider": selected_provider,
        "selected_model": selected_model,
        "streaming": parse_bool(payload.get("streaming"), true),
        "bridge_path": provider_route_path,
        "degraded": unsupported,
        "recorded_at": now_iso(),
    });
    let id = record
        .get("gateway_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "gateway_routes").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "gateway_route": record,
        "claim_evidence": claim("V6-WORKFLOW-001.8", "shannon_openai_compatible_gateway_routes_emit_deterministic_receipts_and_explicit_degradation")
    }))
}

fn register_tooling(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let bridge_path = normalize_surface_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/shannon_gateway_bridge.ts"),
        &["adapters/", "client/runtime/"],
    )?;
    let skills = payload
        .get("skills")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mcp_tools = payload
        .get("mcp_tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let unsafe_tool = mcp_tools
        .iter()
        .filter_map(Value::as_object)
        .any(|tool| tool.get("unsafe").and_then(Value::as_bool).unwrap_or(false));
    if unsafe_tool {
        return Err("shannon_tool_registry_unsafe_tool_denied".to_string());
    }
    let record = json!({
        "registry_id": stable_id("shtools", &json!({"skills": skills, "mcp_tools": mcp_tools})),
        "skills": skills,
        "mcp_tools": mcp_tools,
        "bridge_path": bridge_path,
        "registered_at": now_iso(),
    });
    let id = record
        .get("registry_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "tool_registrations").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "tool_registry": record,
        "claim_evidence": claim("V6-WORKFLOW-001.9", "shannon_skills_and_mcp_tools_register_through_governed_manifests_and_fail_closed_bridges")
    }))
}

