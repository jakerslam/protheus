
fn enforce_tool_completion_contract(
    response_text: String,
    response_tools: &[Value],
) -> (String, Value) {
    let raw_actionable_reason = has_actionable_tool_reason(&response_text);
    let mut tools_present = 0usize;
    let mut successful_tools = 0usize;
    let mut error_tools = 0usize;
    for tool in response_tools {
        let name = clean_text(tool.get("name").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        tools_present += 1;
        if tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            error_tools += 1;
        } else {
            successful_tools += 1;
        }
    }
    let findings = {
        let candidate = response_tools_summary_for_user(response_tools, 4);
        let cleaned = clean_text(&candidate, 24_000);
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    };
    let findings_available = findings.is_some();
    let (mut finalized, mut outcome, initial_ack_only) =
        finalize_user_facing_response_with_outcome(response_text, findings.clone());
    let mut applied = outcome != "unchanged";

    if tools_present == 0 {
        let finalized_cleaned = clean_text(&finalized, 32_000);
        if response_is_no_findings_placeholder(&finalized_cleaned)
            && !finalized_cleaned.is_empty()
        {
            finalized.clear();
            outcome =
                append_tool_completion_outcome(&outcome, "no_tools_withheld_no_findings_copy");
            applied = true;
        } else if response_looks_like_tool_ack_without_findings(&finalized_cleaned) {
            finalized.clear();
            outcome = append_tool_completion_outcome(
                &outcome,
                "no_tools_withheld_unverified_tool_execution_claim",
            );
            applied = true;
        } else if response_is_deferred_execution_preamble(&finalized_cleaned)
            || response_is_deferred_retry_prompt(&finalized_cleaned)
        {
            finalized.clear();
            outcome = append_tool_completion_outcome(
                &outcome,
                "no_tools_withheld_deferred_execution_claim",
            );
            applied = true;
        }
    }

    if tools_present > 0 {
        let finalized_cleaned = clean_text(&finalized, 32_000);
        let actionable_reason =
            raw_actionable_reason || has_actionable_tool_reason(&finalized_cleaned);
        if actionable_reason && !findings_available {
            finalized = clean_text(&finalized_cleaned, 32_000);
            if response_is_no_findings_placeholder(&finalized) {
                finalized.clear();
            }
            outcome = append_tool_completion_outcome(&outcome, "tool_completion_preserved_reason");
            applied = true;
        }
        if findings_available
            && (finalized_cleaned.is_empty()
                || response_looks_like_tool_ack_without_findings(&finalized_cleaned)
                || response_is_no_findings_placeholder(&finalized_cleaned))
        {
            finalized.clear();
            outcome =
                append_tool_completion_outcome(&outcome, "tool_completion_withheld_missing_llm_text");
            applied = true;
        } else if !findings_available
            && !actionable_reason
            && (finalized_cleaned.is_empty()
                || response_looks_like_tool_ack_without_findings(&finalized_cleaned)
                || response_is_no_findings_placeholder(&finalized_cleaned))
        {
            finalized.clear();
            outcome = append_tool_completion_outcome(
                &outcome,
                "tool_completion_withheld_no_findings",
            );
            applied = true;
        }
        if response_looks_like_tool_ack_without_findings(&finalized)
            && !has_actionable_tool_reason(&finalized)
        {
            finalized.clear();
            outcome =
                append_tool_completion_outcome(&outcome, "tool_completion_withheld_ack_only");
            applied = true;
        }
        let deferred_execution = response_is_deferred_execution_preamble(&finalized)
            || response_is_deferred_retry_prompt(&finalized)
            || workflow_response_requests_more_tooling(&finalized);
        if deferred_execution && !has_actionable_tool_reason(&finalized) {
            finalized.clear();
            outcome = append_tool_completion_outcome(
                &outcome,
                "tool_completion_withheld_deferred_execution",
            );
            applied = true;
        }
    }

    let final_ack_only = response_looks_like_tool_ack_without_findings(&finalized);
    let final_no_findings = response_is_no_findings_placeholder(&finalized);
    let final_deferred_execution = response_is_deferred_execution_preamble(&finalized)
        || response_is_deferred_retry_prompt(&finalized)
        || workflow_response_requests_more_tooling(&finalized);
    let final_actionable_reason = has_actionable_tool_reason(&finalized);
    let final_reasoning = first_sentence(&finalized, 220);
    let task_complete = tools_present > 0
        && findings_available
        && !final_ack_only
        && !final_no_findings
        && !final_actionable_reason;
    let completion_state = if tools_present == 0 {
        "not_applicable"
    } else if findings_available {
        "reported_findings"
    } else if final_no_findings {
        "reported_no_findings"
    } else {
        "reported_reason"
    };

    (
        finalized,
        json!({
            "applied": applied,
            "outcome": clean_text(&outcome, 200),
            "tools_present": tools_present > 0,
            "tool_count": tools_present,
            "successful_tools": successful_tools,
            "error_tools": error_tools,
            "findings_available": findings_available,
            "initial_ack_only": initial_ack_only,
            "final_ack_only": final_ack_only,
            "final_no_findings": final_no_findings,
            "final_deferred_execution": final_deferred_execution,
            "completion_state": completion_state,
            "task_complete": task_complete,
            "reasoning": clean_text(&final_reasoning, 220)
        }),
    )
}
