fn message_route_failure_response(
    root: &Path,
    agent_id: &str,
    message: &str,
    status: u16,
    error_code: &str,
    error_detail: &str,
    provider: &str,
    model: &str,
    final_stage_status: &str,
    workspace_hints: Value,
    latent_tool_candidates: Value,
) -> CompatApiResponse {
    let clean_error = clean_text(error_code, 120);
    let clean_detail = clean_text(error_detail, 600);
    let application_diagnostic = clean_error == "final_response_empty";
    let transport_retryable = !application_diagnostic && status >= 500;
    let diagnostic_class = if application_diagnostic {
        "application_finalization_failure"
    } else if status >= 500 {
        "infrastructure_route_failure"
    } else {
        "application_route_failure"
    };
    let events = vec![turn_workflow_event(
        "message_route_failure",
        json!({
            "error_code": clean_error,
            "detail": clean_detail,
            "diagnostic_class": diagnostic_class,
            "retryable": transport_retryable,
            "provider": clean_text(provider, 80),
            "model": clean_text(model, 240)
        }),
    )];
    let mut response_workflow =
        turn_workflow_metadata("message_route_failure", &[], &events, "", message);
    response_workflow["final_llm_response"]["attempted"] = Value::Bool(false);
    response_workflow["final_llm_response"]["used"] = Value::Bool(false);
    response_workflow["final_llm_response"]["status"] =
        Value::String(clean_text(final_stage_status, 80));
    response_workflow["final_llm_response"]["error"] = Value::String(clean_error.clone());
    set_turn_workflow_final_stage_status(&mut response_workflow, final_stage_status);
    let route_direct_response_path =
        if response_tools_prompt_only_gate_required(message, &latent_tool_candidates) {
            "gate_1_pending_llm_tool_choice"
        } else {
            "gate_1_unresolved"
        };
    response_workflow["workflow_control"]["direct_response_path"] =
        Value::String(route_direct_response_path.to_string());

    let response_quality_telemetry = json!({
        "route_failure": true,
        "route_failure_code": clean_error,
        "route_failure_diagnostic_class": diagnostic_class,
        "application_diagnostic": application_diagnostic,
        "transport_retryable": transport_retryable,
        "final_fallback_used": false
    });
    let tooling_invariant = json!({
        "tool_attempted": false,
        "tool_blocked": false,
        "low_signal": false,
        "classification": "not_attempted",
        "failure_code": "",
        "invariant_repair_used": false
    });
    let web_invariant = json!({
        "requires_live_web": false,
        "intent_source": "none",
        "intent_confidence": 0.0,
        "selected_route": "none",
        "tool_attempted": false,
        "tool_blocked": false,
        "low_signal": false,
        "classification": "none",
        "failure_code": "",
        "forced_fallback_attempted": false,
        "invariant_repair_used": false
    });
    let tool_completion = json!({
        "completion_state": "not_applicable",
        "findings_available": false,
        "outcome": "route_failure"
    });
    let mut response_finalization = build_response_finalization_payload(
        &format!("route_failure:{clean_error}"),
        false,
        false,
        &tool_completion,
        false,
        false,
        false,
        false,
        &response_quality_telemetry,
        &tooling_invariant,
        &web_invariant,
    );
    response_finalization["workflow_control"] = json!({
        "mode": "tool_menu_interface_v1",
        "direct_response_path": route_direct_response_path
    });
    response_finalization["route_failure"] = json!({
        "error_code": clean_error,
        "detail": clean_detail,
        "diagnostic_class": diagnostic_class,
        "application_diagnostic": application_diagnostic,
        "retryable": transport_retryable,
        "transport_retryable": transport_retryable,
        "infrastructure_failure": !application_diagnostic && status >= 500,
        "transport_status": status,
        "chat_text_authored": false
    });
    apply_visible_response_provenance(&mut response_workflow, &mut response_finalization, "none");

    let response_tools = Vec::<Value>::new();
    let process_summary =
        build_turn_process_summary(message, &response_tools, &response_workflow, &response_finalization);
    let workflow_visibility = workflow_visibility_payload(&response_workflow, &response_finalization);
    let previous_assistant =
        latest_assistant_message_text(&session_messages(&load_session_state(root, agent_id)));
    let live_eval_monitor = live_eval_monitor_turn(
        root,
        agent_id,
        message,
        "",
        &previous_assistant,
        &response_finalization,
    );
    let turn_receipt = json!({"live_eval_monitor": live_eval_monitor.clone()});
    let agent_health_snapshot = persist_agent_control_plane_health_snapshot_for_turn(
        root,
        agent_id,
        message,
        "",
        &response_workflow,
        &response_finalization,
        &process_summary,
        &turn_receipt,
    );

    let mut payload = json!({});
    payload["ok"] = Value::Bool(false);
    payload["error"] = Value::String(clean_error.clone());
    payload["error_code"] = Value::String(clean_error);
    payload["error_detail"] = Value::String(clean_detail);
    payload["diagnostic_class"] = Value::String(diagnostic_class.to_string());
    payload["application_diagnostic"] = Value::Bool(application_diagnostic);
    payload["retryable"] = Value::Bool(transport_retryable);
    payload["transport_retryable"] = Value::Bool(transport_retryable);
    payload["infrastructure_failure"] = Value::Bool(!application_diagnostic && status >= 500);
    payload["agent_id"] = Value::String(clean_agent_id(agent_id));
    payload["provider"] = Value::String(clean_text(provider, 80));
    payload["model"] = Value::String(clean_text(model, 240));
    payload["runtime_model"] = Value::String(clean_text(model, 240));
    payload["response"] = Value::String(String::new());
    payload["tools"] = Value::Array(response_tools);
    payload["response_workflow"] = response_workflow;
    payload["response_finalization"] = response_finalization;
    payload["process_summary"] = process_summary;
    payload["workflow_visibility"] = workflow_visibility;
    payload["response_quality_telemetry"] = response_quality_telemetry;
    payload["visible_response_source"] = Value::String("none".to_string());
    payload["system_chat_injection_used"] = Value::Bool(false);
    payload["live_eval_monitor"] = live_eval_monitor;
    payload["dashboard_health_indicator"] = agent_health_snapshot
        .get("dashboard_health_indicator")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["agent_health_snapshot"] = agent_health_snapshot;
    payload["workspace_hints"] = workspace_hints;
    payload["latent_tool_candidates"] = latent_tool_candidates;
    CompatApiResponse { status, payload }
}

fn no_models_available_message_response(
    root: &Path,
    agent_id: &str,
    message: &str,
    workspace_hints: Value,
    latent_tool_candidates: Value,
) -> CompatApiResponse {
    let mut response = message_route_failure_response(
        root,
        agent_id,
        message,
        503,
        "no_models_available",
        "No usable LLM provider is available for this turn.",
        "none",
        "none",
        "skipped_missing_model",
        workspace_hints,
        latent_tool_candidates,
    );
    response.payload["hint"] = json!("No usable LLMs are available yet. Install Ollama or add an API key.");
    response.payload["setup"] = json!({
        "steps": [
            "Install Ollama: https://ollama.com/download",
            "Start Ollama: ollama serve",
            "Pull at least one model: ollama pull qwen2.5:3b-instruct",
            "Or add API keys in Settings or via /apikey <key>"
        ]
    });
    response
}

fn final_response_empty_message_response(
    root: &Path,
    agent_id: &str,
    message: &str,
    provider: &str,
    model: &str,
    workspace_hints: Value,
    latent_tool_candidates: Value,
) -> CompatApiResponse {
    let mut response = message_route_failure_response(
        root,
        agent_id,
        message,
        200,
        "final_response_empty",
        "The final LLM-authored response was empty after safety/finalization guards.",
        provider,
        model,
        "empty_final_response",
        workspace_hints,
        latent_tool_candidates,
    );
    let persistence_receipt = append_turn_message(root, agent_id, message, "");
    response.payload["turn_persistence"] = json!({
        "user_message_persisted": persistence_receipt.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "assistant_message_persisted": false,
        "diagnostics_in_chat": false,
        "receipt": persistence_receipt
    });
    response
}
