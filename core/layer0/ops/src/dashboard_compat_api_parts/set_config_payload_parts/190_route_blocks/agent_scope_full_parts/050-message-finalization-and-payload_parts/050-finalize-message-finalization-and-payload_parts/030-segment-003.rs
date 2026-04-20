            &tooling_failure_code,
            "tool_status",
        );
        response_text = next_response;
        if repaired {
            tooling_invariant_repair_used = true;
            final_fallback_used = true;
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                "tooling_failure_code_appended",
                200,
            );
        }
    }
    let final_contract_violation = response_fails_base_final_answer_contract(&response_text)
        || workflow_response_requests_more_tooling(&response_text);
    if final_contract_violation {
        response_text = build_deterministic_final_fallback_response(
            &response_tools,
            web_intent_detected,
            &web_turn_classification,
            &web_failure_code,
            tooling_attempted,
            &tooling_turn_classification,
            &tooling_failure_code,
            inline_tools_allowed,
        );
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "deterministic_final_fallback_enforced",
            200,
        );
    }
    response_text = append_next_actions_line_if_actionable(message, &response_text, &response_tools);
    let tool_gate_should_call_tools = response_workflow
        .pointer("/tool_gate/should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let direct_answer_rate =
        response_workflow_quality_rate(&response_workflow, "direct_answer_rate");
    let retry_rate = response_workflow_quality_rate(&response_workflow, "retry_rate");
    let off_topic_reject_rate =
        response_workflow_quality_rate(&response_workflow, "off_topic_reject_rate");
    let tool_overcall_rate = if !tool_gate_should_call_tools && tooling_attempted {
        1.0
    } else {
        0.0
    };
    response_workflow["quality_telemetry"]["final_fallback_used"] = Value::Bool(final_fallback_used);
    let final_ack_only = response_looks_like_tool_ack_without_findings(&response_text);
    let response_quality_telemetry = build_response_quality_telemetry_payload(
        &response_workflow,
        final_fallback_used,
        tooling_invariant_repair_used,
        &tooling_failure_code,
        direct_answer_rate,
        retry_rate,
        tool_overcall_rate,
        off_topic_reject_rate,
    );
    let tooling_invariant = json!({
        "tool_attempted": tooling_attempted,
        "tool_blocked": tooling_blocked,
        "low_signal": tooling_low_signal,
        "classification": tooling_turn_classification,
        "failure_code": tooling_failure_code,
        "invariant_repair_used": tooling_invariant_repair_used
    });
    let web_invariant = json!({
        "requires_live_web": web_intent_detected,
        "intent_source": web_intent_source,
        "intent_confidence": web_intent_confidence,
        "selected_route": web_intent_route.clone(),
        "tool_attempted": web_tool_attempted,
        "tool_blocked": web_tool_blocked,
        "low_signal": web_tool_low_signal,
        "classification": web_turn_classification,
        "failure_code": web_failure_code,
        "forced_fallback_attempted": web_forced_fallback_attempted,
        "invariant_repair_used": web_invariant_repair_used
    });
    let response_finalization = build_response_finalization_payload(
        &finalization_outcome,
        initial_ack_only,
        final_ack_only,
        &tool_completion,
        tooling_fallback_used,
        comparative_fallback_used,
        workflow_system_fallback_used,
        visible_response_repaired,
        &response_quality_telemetry,
        &tooling_invariant,
        &web_invariant,
    );
    let process_summary =
        build_turn_process_summary(message, &response_tools, &response_workflow, &response_finalization);
    let turn_transaction = crate::dashboard_tool_turn_loop::turn_transaction_payload(
        "complete",
        if response_tools.is_empty() {
            "none"
        } else {
            "complete"
        },
        "complete",
        "complete",
    );
    let terminal_transcript = tool_terminal_transcript(&response_tools);
    let turn_receipt = append_turn_receipt_with_metadata(
        root,
        agent_id,
        message,
        &response_text,
        Value::Array(response_tools.clone()),
        &response_workflow,
        &response_finalization,
        &process_summary,
        &turn_transaction,
        &terminal_transcript,
    );
    let runtime_model = clean_text(
        result
            .get("runtime_model")
            .and_then(Value::as_str)
            .unwrap_or(&model),
        240,
    );
    let mut runtime_patch = json!({
        "runtime_model": runtime_model,
        "context_window": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "context_window_tokens": result.get("context_window").cloned().unwrap_or_else(|| json!(0)),
        "updated_at": crate::now_iso()
    });
    if auto_route.is_some() {
        runtime_patch["runtime_provider"] = json!(provider.clone());
        if !requested_provider.eq_ignore_ascii_case("auto")
            && !requested_model.is_empty()
            && !requested_model.eq_ignore_ascii_case("auto")
        {
            runtime_patch["model_provider"] = json!(provider.clone());
            runtime_patch["model_name"] = json!(model.clone());
            runtime_patch["model_override"] = json!(format!("{provider}/{model}"));
        }
    }
    let _ = update_profile_patch(root, agent_id, &runtime_patch);
    let mut payload = result.clone();
    payload["ok"] = json!(true);
    payload["agent_id"] = json!(agent_id);
    payload["provider"] = json!(provider);
    payload["model"] = json!(model);
    payload["iterations"] = json!(1);
    payload["response"] = json!(response_text);
    payload["runtime_sync"] = runtime_summary;
    payload["tools"] = Value::Array(response_tools);
    payload["response_workflow"] = response_workflow;
    payload["terminal_transcript"] = Value::Array(terminal_transcript);
    payload["response_finalization"] = response_finalization;
    payload["process_summary"] = process_summary;
    payload["response_quality_telemetry"] = response_quality_telemetry;
    payload["web_intent"] = json!({
        "detected": web_intent_detected,
        "source": web_intent_source,
        "confidence": web_intent_confidence,
        "selected_route": web_intent_route
    });
    payload["turn_transaction"] = turn_transaction;
    payload["context_window"] = json!(fallback_window.max(0));
    payload["context_tokens"] = json!(context_active_tokens.max(0));
    payload["context_used_tokens"] = json!(context_active_tokens.max(0));
    payload["context_ratio"] = json!(context_ratio);
    payload["context_pressure"] = json!(context_pressure.clone());
    payload["attention_queue"] = turn_receipt
        .get("attention_queue")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["memory_capture"] = turn_receipt
        .get("memory_capture")
        .cloned()
        .unwrap_or_else(|| json!({}));
    payload["context_pool"] = json!({
