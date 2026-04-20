fn workflow_retry_sanitizer_drops_follow_up_tool_markup_tail() {
    let response = "My search for \"top AI agentic frameworks\" didn't return specific framework listings. Let me try a more targeted approach with some well-known framework names.\n\n<function=web_search>{\"query\":\"LangChain AutoGPT BabyAGI AI agent frameworks comparison\"}</function>";
    assert!(workflow_response_requests_more_tooling(response));
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "My search for \"top AI agentic frameworks\" didn't return specific framework listings."
    );
}

#[test]
