    };
    if !workflow_status.is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            &format!("workflow:{workflow_status}"),
            200,
        );
    }
    let mut tooling_fallback_used = false;
    let mut comparative_fallback_used = false;
    let mut workflow_system_fallback_used = false;
    let mut visible_response_repaired = false;
    let mut final_fallback_used = false;
    if workflow_used {
        tool_completion = tool_completion_report_for_response(
            &finalized_response,
            &response_tools,
            "workflow_authored",
        );
    } else if workflow_fallback_allowed {
        let mut fallback_response = maybe_tooling_failure_fallback(
            message,
            &initial_draft_response,
            &latest_assistant_text,
        )
        .unwrap_or_default();
        tooling_fallback_used = !fallback_response.is_empty();
        if fallback_response.is_empty()
            && !response_requires_visible_repair(&initial_draft_response)
        {
            fallback_response = initial_draft_response.clone();
        }
        if fallback_response.is_empty()
            && message_requests_comparative_answer(message)
            && (response_is_no_findings_placeholder(&initial_draft_response)
                || response_tools_failure_reason_for_user(&response_tools, 4).is_empty())
        {
            comparative_fallback_used = true;
            fallback_response = comparative_no_findings_fallback(message);
        }
        if fallback_response.is_empty() && memory_recall_requested(message) {
            fallback_response = build_memory_recall_response(&state, &messages, message);
        }
        if fallback_response.is_empty() && !response_tools.is_empty() {
            fallback_response = ensure_tool_turn_response_text(&initial_draft_response, &response_tools);
        }
        if fallback_response.is_empty() && response_tools.is_empty() && !inline_tools_allowed {
            fallback_response =
                "I can answer directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string();
        }
        if fallback_response.is_empty() {
            fallback_response =
                "I hit a response-synthesis failure after collecting this turn. Please retry and I’ll explain what worked or failed directly.".to_string();
        }
        workflow_system_fallback_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_system_fallback",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, fallback_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    } else {
        workflow_system_fallback_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "workflow_unexpected_state",
            200,
        );
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(
                message,
                "I completed the workflow gate, but the final workflow state was unexpected. Please retry so I can rerun the chain cleanly."
                    .to_string(),
                &response_tools,
            );
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    let (repaired_response, repair_outcome, repair_tooling_used, repair_comparative_used) =
        repair_visible_response_after_workflow(
            message,
            &finalized_response,
            &initial_draft_response,
            &latest_assistant_text,
            &response_tools,
            inline_tools_allowed,
            memory_fallback.as_deref(),
        );
    if repair_outcome != "unchanged" {
        visible_response_repaired = true;
        final_fallback_used = true;
        tooling_fallback_used |= repair_tooling_used;
        comparative_fallback_used |= repair_comparative_used;
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(message, repaired_response, &response_tools);
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome = merge_response_outcomes(&finalization_outcome, &repair_outcome, 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = finalized_response;
    let web_tool_attempted = response_tools_include_web_attempt(&response_tools);
    let web_tool_blocked = response_tools_web_blocked(&response_tools);
    let web_tool_low_signal = response_tools_web_low_signal(&response_tools);
    let web_turn_classification = classify_web_turn_state(
        web_intent_detected,
        web_tool_attempted,
        web_tool_blocked,
        web_tool_low_signal,
    );
    let mut web_failure_code = web_failure_code_from_response_tools(&response_tools);
    if web_intent_detected && !web_tool_attempted {
        web_failure_code = "web_route_parse_failed".to_string();
    } else if web_failure_code.is_empty() && web_tool_low_signal {
        web_failure_code = "web_tool_low_signal".to_string();
    }
    let tooling_attempted = !response_tools.is_empty();
    let tooling_blocked = response_tools_any_blocked(&response_tools);
    let tooling_low_signal = response_tools_any_low_signal(&response_tools);
    let tooling_failure_code = tool_failure_code_from_response_tools(&response_tools);
    let tooling_turn_classification = if !tooling_attempted {
        "not_attempted".to_string()
    } else if tooling_blocked {
        "policy_blocked".to_string()
    } else if tooling_low_signal {
        "low_signal".to_string()
    } else if !tooling_failure_code.is_empty() {
        "failed".to_string()
    } else {
        "healthy".to_string()
    };
    let mut tooling_invariant_repair_used = false;
    let mut web_invariant_repair_used = false;
    if web_intent_detected && !web_tool_attempted {
        response_text = format!(
            "I detected a live web request, but no web tool lane executed in this turn. web_status: parse_failed. error_code: {}. Retry with `tool::web_search:::<query>` or `tool::web_tooling_health_probe`.",
            web_failure_code
        );
        web_invariant_repair_used = true;
        final_fallback_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_invariant_missing_tool_attempt",
            200,
        );
    } else if web_tool_attempted
        && (web_tool_blocked || web_tool_low_signal || !web_failure_code.is_empty())
    {
        let (next_response, repaired) = append_failure_status_line_if_missing(
            response_text,
            &web_turn_classification,
            &web_failure_code,
            "web_status",
        );
        response_text = next_response;
        if repaired {
            web_invariant_repair_used = true;
            final_fallback_used = true;
            finalization_outcome = merge_response_outcomes(
                &finalization_outcome,
                "web_failure_code_appended",
                200,
            );
        }
    }
    if tooling_attempted {
        let (next_response, repaired) = append_failure_status_line_if_missing(
            response_text,
            &tooling_turn_classification,
