fn workflow_retry_sanitizer_drops_polite_more_search_tail() {
    let response = "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents. Would you like me to search for deeper benchmark comparisons too?";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "I searched official framework sources and found LangGraph, OpenAI Agents SDK, CrewAI, and smolagents."
    );
}

#[test]
