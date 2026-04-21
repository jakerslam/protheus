
fn route_provider(
    root: &Path,
    state: &mut Value,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let profile = profile(payload.get("profile"));
    let modality = clean_token(payload.get("modality").and_then(Value::as_str), "text");
    let adapter_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("adapters/protocol/dify_connector_bridge.ts"),
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
    let supports_modality = match profile.as_str() {
        "tiny-max" => modality == "text",
        "pure" => matches!(modality.as_str(), "text" | "image"),
        _ => true,
    };
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
        return Err("dify_provider_route_target_required".to_string());
    };
    let route = json!({
        "route_id": stable_id("difyroute", &json!({"profile": profile, "modality": modality, "selected_route": selected_route})),
        "profile": profile,
        "modality": modality,
        "bridge_path": adapter_path,
        "selected_route": selected_route,
        "degraded": !supports_modality,
        "routed_at": now_iso(),
    });
    let id = route
        .get("route_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "provider_routes").insert(id, route.clone());
    Ok(json!({
        "ok": true,
        "provider_route": route,
        "claim_evidence": claim("V6-WORKFLOW-005.5", "dify_provider_compatibility_is_absorbed_into_governed_route_and_invocation_receipts"),
    }))
}

fn matches_condition(context: &Map<String, Value>, condition: Option<&Map<String, Value>>) -> bool {
    let Some(condition) = condition else {
        return false;
    };
    let field = condition.get("field").and_then(Value::as_str).unwrap_or("");
    if field.is_empty() {
        return false;
    }
    match (context.get(field), condition.get("equals")) {
        (Some(Value::String(left)), Some(Value::String(right))) => left == right,
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn run_conditional_flow(
    root: &Path,
    state: &mut Value,
    swarm_state_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let flow_name = clean_text(payload.get("flow_name").and_then(Value::as_str), 120);
    if flow_name.is_empty() {
        return Err("dify_flow_name_required".to_string());
    }
    let profile = profile(payload.get("profile"));
    let context = payload
        .get("context")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let branches = payload
        .get("branches")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected_branch = branches
        .iter()
        .find(|row| {
            row.get("condition")
                .and_then(Value::as_object)
                .map(|cond| matches_condition(&context, Some(cond)))
                .unwrap_or_else(|| row.get("default").and_then(Value::as_bool).unwrap_or(false))
        })
        .cloned()
        .unwrap_or_else(|| json!({"id": "default", "target": "complete", "default": true}));
    let loop_cfg = payload
        .get("loop")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut iterations = loop_cfg
        .get("max_iterations")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if let Some(condition) = loop_cfg.get("continue_while").and_then(Value::as_object) {
        if !matches_condition(&context, Some(condition)) {
            iterations = 1;
        }
    }
    let mut degraded = false;
    if profile == "tiny-max" && iterations > 2 {
        iterations = 2;
        degraded = true;
    }
    let handoff_target = payload
        .get("handoffs")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find_map(|row| {
                let cond = row.get("when").and_then(Value::as_object);
                if matches_condition(&context, cond) {
                    row.get("target")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                } else {
                    None
                }
            })
        });
    let swarm_record = json!({
        "flow_name": flow_name,
        "selected_target": selected_branch.get("target").cloned().unwrap_or_else(|| json!("complete")),
        "handoff_target": handoff_target,
        "iterations": iterations,
        "profile": profile,
        "recorded_at": now_iso(),
    });
    lane_utils::write_json(swarm_state_path, &swarm_record)?;
    let record = json!({
        "flow_run_id": stable_id("difyflow", &json!({"flow_name": flow_name, "context": context, "selected_branch": selected_branch, "iterations": iterations})),
        "flow_name": flow_name,
        "profile": profile,
        "context": context,
        "selected_branch": selected_branch,
        "iterations": iterations,
        "handoff_target": handoff_target,
        "swarm_state_path": rel(root, swarm_state_path),
        "degraded": degraded,
        "executed_at": now_iso(),
    });
    let id = record
        .get("flow_run_id")
        .and_then(Value::as_str)
        .unwrap()
        .to_string();
    as_object_mut(state, "flow_runs").insert(id, record.clone());
    Ok(json!({
        "ok": true,
        "flow_run": record,
        "claim_evidence": claim("V6-WORKFLOW-005.6", "dify_conditional_branches_loops_and_agent_handoffs_route_through_authoritative_workflow_primitives"),
    }))
}

fn record_audit_trace(
    root: &Path,
    state: &mut Value,
    trace_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let bridge_path = normalize_bridge_path(
        root,
        payload
            .get("bridge_path")
            .and_then(Value::as_str)
            .unwrap_or("client/runtime/lib/dify_bridge.ts"),
    )?;
    let trace = json!({
        "trace_id": stable_id("difytrace", &json!({"stage": payload.get("stage"), "message": payload.get("message")})),
        "stage": clean_token(payload.get("stage").and_then(Value::as_str), "run"),
        "message": clean_text(payload.get("message").and_then(Value::as_str), 180),
        "metrics": payload.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "logs": payload.get("logs").cloned().unwrap_or_else(|| json!([])),
        "bridge_path": bridge_path,
        "trace_path": rel(root, trace_path),
        "recorded_at": now_iso(),
    });
    lane_utils::append_jsonl(trace_path, &trace)?;
    as_array_mut(state, "audit_traces").push(trace.clone());
    Ok(json!({
        "ok": true,
        "audit_trace": trace,
        "claim_evidence": claim("V6-WORKFLOW-005.7", "dify_logs_metrics_and_debugging_traces_stream_through_native_observability_and_receipt_lanes"),
    }))
}
