fn helpfulness_detector_flags_prompt_echo_and_accepts_direct_answer() {
    assert!(response_prompt_echo_detected(
        "try searching for top agentic frameworks",
        "try searching for top agentic frameworks"
    ));
    assert!(!response_prompt_echo_detected(
        "try searching for top agentic frameworks",
        "Top agentic frameworks today include LangGraph, OpenAI Agents SDK, and AutoGen."
    ));
    assert!(response_answers_user_early(
        "what happened with the web tooling?",
        "The web tooling call failed because the provider returned low-signal results. We should retry with one narrower query."
    ));
}

#[test]
