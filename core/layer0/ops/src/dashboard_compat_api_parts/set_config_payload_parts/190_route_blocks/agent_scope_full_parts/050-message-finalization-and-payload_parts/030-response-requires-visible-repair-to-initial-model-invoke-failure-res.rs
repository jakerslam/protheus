
fn response_requires_visible_repair(text: &str) -> bool {
    let cleaned = clean_chat_text(text, 32_000);
    cleaned.trim().is_empty()
        || response_is_no_findings_placeholder(&cleaned)
        || response_looks_like_tool_ack_without_findings(&cleaned)
        || response_is_deferred_execution_preamble(&cleaned)
        || response_is_deferred_retry_prompt(&cleaned)
        || workflow_response_requests_more_tooling(&cleaned)
        || response_contains_speculative_web_blocker_language(&cleaned)
        || response_looks_like_unsynthesized_web_snippet_dump(&cleaned)
        || response_looks_like_raw_web_artifact_dump(&cleaned)
}

fn repair_visible_response_after_workflow(
    message: &str,
    candidate_response: &str,
    initial_draft_response: &str,
    latest_assistant_text: &str,
    response_tools: &[Value],
    inline_tools_allowed: bool,
    memory_fallback: Option<&str>,
) -> (String, String, bool, bool) {
    let cleaned = clean_chat_text(candidate_response, 32_000);
    if !response_requires_visible_repair(&cleaned) {
        return (cleaned, "unchanged".to_string(), false, false);
    }

    let cleaned_initial_draft = clean_chat_text(initial_draft_response, 32_000);
    if !response_requires_visible_repair(&cleaned_initial_draft)
        && !response_contains_speculative_web_blocker_language(&cleaned_initial_draft)
    {
        return (
            cleaned_initial_draft,
            "repaired_with_initial_draft".to_string(),
            false,
            false,
        );
    }

    let cleaned_latest_assistant = clean_chat_text(latest_assistant_text, 32_000);
    if !response_requires_visible_repair(&cleaned_latest_assistant)
        && !response_contains_speculative_web_blocker_language(&cleaned_latest_assistant)
    {
        return (
            cleaned_latest_assistant,
            "repaired_with_latest_assistant".to_string(),
            false,
            false,
        );
    }

    let findings_summary = clean_text(&response_tools_summary_for_user(response_tools, 4), 4_000);
    if !findings_summary.is_empty() {
        return (
            findings_summary,
            "repaired_with_tool_findings_summary".to_string(),
            false,
            false,
        );
    }

    let failure_reason = clean_text(
        &response_tools_failure_reason_for_user(response_tools, 4),
        4_000,
    );
    if !failure_reason.is_empty() {
        return (
            failure_reason,
            "repaired_with_tool_failure_reason".to_string(),
            false,
            false,
        );
    }

    if message_requests_comparative_answer(message) {
        return (
            comparative_no_findings_fallback(message),
            "repaired_with_comparative_guidance".to_string(),
            false,
            true,
        );
    }

    if let Some(tooling_guidance) =
        maybe_tooling_failure_fallback(message, initial_draft_response, latest_assistant_text)
    {
        return (
            tooling_guidance,
            "repaired_with_tooling_guidance".to_string(),
            true,
            false,
        );
    }

    if let Some(memory_response) = memory_fallback {
        let cleaned_memory = clean_chat_text(memory_response, 32_000);
        if !cleaned_memory.is_empty() {
            return (
                cleaned_memory,
                "repaired_with_memory_fallback".to_string(),
                false,
                false,
            );
        }
    }

    if !response_tools.is_empty() {
        let readability_guidance =
            clean_text(&ensure_tool_turn_response_text(initial_draft_response, response_tools), 4_000);
        if !readability_guidance.is_empty() {
            return (
                readability_guidance,
                "repaired_with_tool_readability_guidance".to_string(),
                false,
                false,
            );
        }
    }

    if response_tools.is_empty() && !inline_tools_allowed {
        return (
            "I can answer this directly without tool calls. Ask your question naturally and I’ll respond conversationally unless you explicitly request a tool run.".to_string(),
            "repaired_with_direct_answer_guard".to_string(),
            false,
            false,
        );
    }

    (
        "I completed the workflow gate, but the visible response stayed empty or low-signal. Please retry and I’ll rerun the chain and explain what worked or failed directly.".to_string(),
        "repaired_with_generic_workflow_failure".to_string(),
        false,
        false,
    )
}

fn initial_model_invoke_failure_response(message: &str, err: &str) -> String {
    let cleaned_error = clean_text(err, 220);
    let base = if message_requests_comparative_answer(message) {
        "I couldn’t start the first model step for this comparison turn, so I did not finish gathering workspace and web evidence yet. Retry and I’ll rerun the full chain."
            .to_string()
    } else {
        "I couldn’t start the first model step for this turn, so the workflow could not continue normally. Retry and I’ll rerun the chain."
            .to_string()
    };
    if cleaned_error.is_empty() {
        return base;
    }
    format!("{base} Backend error: {cleaned_error}.")
}
