
fn finalize_message_invoke_failure_and_payload(
    root: &Path,
    agent_id: &str,
    message: &str,
    provider: &str,
    model: &str,
    error_text: &str,
    active_messages: &[Value],
    workspace_hints: Value,
    latent_tool_candidates: Value,
) -> CompatApiResponse {
    let workflow_events = vec![turn_workflow_event(
        "initial_model_invoke_failed",
        json!({
            "error": clean_text(error_text, 240),
            "provider": clean_text(provider, 80),
            "model": clean_text(model, 240)
        }),
    )];
    let latest_assistant_text = latest_assistant_message_text(active_messages);
    let response_workflow = run_turn_workflow_final_response(
        root,
        provider,
        model,
        active_messages,
        message,
        "model_initial_invoke_failed",
        &[],
        &workflow_events,
        &initial_model_invoke_failure_response(message, error_text),
        &latest_assistant_text,
    );
    let workflow_status = workflow_final_response_status(&response_workflow);
    let workflow_used = workflow_final_response_used(&response_workflow);
    let workflow_fallback_allowed =
        workflow_final_response_allows_system_fallback(&response_workflow);
    let mut response_text = response_workflow
        .get("response")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut finalization_outcome = if workflow_used {
        "workflow_authored".to_string()
    } else {
        "workflow_llm_unavailable".to_string()
    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            220,
        );
    }
    let mut workflow_system_fallback_used = false;
    if !workflow_used && workflow_fallback_allowed {
        response_text = initial_model_invoke_failure_response(message, error_text);
        workflow_system_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_system_fallback",
            220,
        );
    }
    let (repaired_response, repair_outcome, _, comparative_repair_used) =
        repair_visible_response_after_workflow(
            message,
            &response_text,
            &response_text,
            &latest_assistant_text,
            &[],
            true,
            None,
        );
    let (finalized_response, tool_completion, contract_outcome) =
        enforce_user_facing_finalization_contract(message, repaired_response, &[]);
    finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 220);
    finalization_outcome = merge_response_outcomes(&finalization_outcome, &contract_outcome, 220);
    let response_quality_telemetry = json!({});
    let tooling_invariant = json!({
        "tool_attempted": false,
        "tool_blocked": false,
        "low_signal": false,
        "classification": "no_tooling",
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
    let mut response_finalization = build_response_finalization_payload(
        &finalization_outcome,
        false,
        response_looks_like_tool_ack_without_findings(&finalized_response),
        &tool_completion,
        false,
        comparative_repair_used,
        workflow_system_fallback_used,
        repair_outcome != "unchanged",
        &response_quality_telemetry,
        &tooling_invariant,
        &web_invariant,
    );
    response_finalization["workflow_control"] = json!({
        "conversation_bypass": workflow_conversation_bypass_control_from_workflow(&response_workflow)
    });
    response_finalization["initial_model_invoke_failed"] = Value::Bool(true);
    let process_summary =
        build_turn_process_summary(message, &[], &response_workflow, &response_finalization);
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        "none",
        "invoke_failed",
        "complete",
    );
    let terminal_transcript = Vec::<Value>::new();
    let turn_receipt = append_turn_receipt_with_metadata(
        root,
        agent_id,
        message,
        &finalized_response,
        json!([]),
        &response_workflow,
        &response_finalization,
        &process_summary,
        &turn_transaction,
        &terminal_transcript,
    );
    CompatApiResponse {
        status: 200,
        payload: json!({
            "ok": true,
            "agent_id": agent_id,
            "provider": provider,
            "model": model,
            "runtime_model": model,
            "iterations": 1,
            "response": finalized_response,
            "tools": [],
            "response_workflow": response_workflow,
            "response_finalization": response_finalization,
            "process_summary": process_summary,
            "turn_transaction": turn_transaction,
            "terminal_transcript": terminal_transcript,
            "workspace_hints": workspace_hints,
            "latent_tool_candidates": latent_tool_candidates,
            "attention_queue": turn_receipt
                .get("attention_queue")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "memory_capture": turn_receipt
                .get("memory_capture")
                .cloned()
                .unwrap_or_else(|| json!({})),
            "error": clean_text(error_text, 280),
            "degraded": true,
            "initial_invoke_error": true
        }),
    }
}
