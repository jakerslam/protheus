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
    let response_guard =
        final_response_guard_report(message, &response_text, &response_tools, false);
    if response_guard_bool(&response_guard, "final_contract_violation") {
        // Chat output stays LLM-authored only; the runtime may retry synthesis, but it must not
        // inject deterministic fallback text into the visible response.
        response_text.clear();
        final_fallback_used = true;
        if response_guard_bool(&response_guard, "final_contamination_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "contamination_reject");
        }
        if response_guard_bool(&response_guard, "current_turn_dominance_violation") {
            bump_workflow_quality_counter(&mut response_workflow, "current_turn_dominance_reject");
        }
        if response_guard_bool(&response_guard, "unsupported_tool_success_claim") {
            bump_workflow_quality_counter(
                &mut response_workflow,
                "unsupported_tool_success_claim_reject",
            );
        }
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            final_response_guard_outcome(&response_guard),
            200,
        );
        let mut guard_recovery_events = workflow_system_events.clone();
        guard_recovery_events.push(turn_workflow_event(
            "final_response_guard_recovery",
            json!({
                "selection_authority": "llm_only",
                "automatic_execution_allowed": false,
                "guard_outcome": final_response_guard_outcome(&response_guard),
                "visible_gate_choice_leakage": response_guard_bool(&response_guard, "visible_gate_choice_leakage"),
                "unsupported_tool_success_claim": response_guard_bool(&response_guard, "unsupported_tool_success_claim")
            }),
        ));
        let (recovery_provider, recovery_model) =
            visible_response_recovery_model(&provider, &model);
        let mut recovered_workflow = run_turn_workflow_final_response(
            root,
            &recovery_provider,
            &recovery_model,
            &active_messages,
            message,
            &workflow_mode,
            &response_tools,
            &guard_recovery_events,
            "",
            &latest_assistant_text,
        );
        recovered_workflow["visible_response_recovery_model"] = json!({
            "provider": recovery_provider,
            "model": recovery_model
        });
        let recovered_text = clean_chat_text(
            recovered_workflow
                .get("response")
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        if workflow_final_response_used(&recovered_workflow) && !recovered_text.trim().is_empty() {
            let recovered_guard =
                final_response_guard_report(message, &recovered_text, &response_tools, false);
            if !response_guard_bool(&recovered_guard, "final_contract_violation") {
                let (contract_finalized, contract_report, contract_outcome) =
                    enforce_user_facing_finalization_contract(message, recovered_text, &response_tools);
                if !contract_finalized.trim().is_empty() {
                    response_workflow = recovered_workflow;
                    response_text = contract_finalized;
                    tool_completion = enrich_tool_completion_receipt(contract_report, &response_tools);
                    workflow_used = true;
                    finalization_outcome = merge_response_outcomes(
                        &finalization_outcome,
                        "final_response_guard_recovered_by_llm",
                        220,
                    );
                    finalization_outcome =
                        merge_response_outcomes(&finalization_outcome, &contract_outcome, 220);
                }
            }
        }
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
