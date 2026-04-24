
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
    _message: &str,
    candidate_response: &str,
    initial_draft_response: &str,
    _latest_assistant_text: &str,
    _response_tools: &[Value],
    _inline_tools_allowed: bool,
    _memory_fallback: Option<&str>,
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

    // Policy: never inject non-LLM fallback text into chat output.
    (
        String::new(),
        "withheld_non_llm_fallback_response".to_string(),
        false,
        false,
    )
}

fn initial_model_invoke_failure_response(_message: &str, _err: &str) -> String {
    // No system-authored fallback text; preserve LLM-only chat output contract.
    String::new()
}

#[cfg(test)]
mod repair_visible_response_tests {
    use super::*;

    #[test]
    fn repair_visible_response_does_not_reuse_latest_assistant_text() {
        let (response, outcome, _, _) = repair_visible_response_after_workflow(
            "try again",
            "I don't have usable tool findings from this turn yet.",
            "I don't have usable tool findings from this turn yet.",
            "Hello! I'm here to help with whatever you need.",
            &[],
            false,
            None,
        );
        assert!(response.is_empty(), "{response}");
        assert_eq!(outcome, "withheld_non_llm_fallback_response");
    }
}
