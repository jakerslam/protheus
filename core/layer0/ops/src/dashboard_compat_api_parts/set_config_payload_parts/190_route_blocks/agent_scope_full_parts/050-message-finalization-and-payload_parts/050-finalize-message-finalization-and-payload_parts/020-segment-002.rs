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
    } else {
        // Keep chat output LLM-authored only; runtime fallback substitution is disabled.
        let (contract_finalized, contract_report, contract_outcome) =
            enforce_user_facing_finalization_contract(
                message,
                initial_draft_response.clone(),
                &response_tools,
            );
        finalized_response = contract_finalized;
        tool_completion = contract_report;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "workflow_no_runtime_fallback", 200);
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, &contract_outcome, 200);
    }
    tool_completion = enrich_tool_completion_receipt(tool_completion, &response_tools);
    response_text = finalized_response;
    if response_text.trim().is_empty() {
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "empty_final_response_no_system_retry",
            220,
        );
    }
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
        web_invariant_repair_used = true;
        finalization_outcome = merge_response_outcomes(
            &finalization_outcome,
            "web_invariant_missing_tool_attempt",
            200,
        );
    } else if web_tool_attempted
        && (web_tool_blocked || web_tool_low_signal || !web_failure_code.is_empty())
    {
        web_invariant_repair_used = true;
        finalization_outcome =
            merge_response_outcomes(&finalization_outcome, "web_failure_code_appended", 200);
    }
    if tooling_attempted
        && (tooling_blocked || tooling_low_signal || !tooling_failure_code.is_empty())
    {
