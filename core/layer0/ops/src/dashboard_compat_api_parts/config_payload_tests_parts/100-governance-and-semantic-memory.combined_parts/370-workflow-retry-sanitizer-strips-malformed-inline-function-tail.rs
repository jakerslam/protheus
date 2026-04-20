fn workflow_retry_sanitizer_strips_malformed_inline_function_tail() {
    let response = "No, the web tools still aren't executing.\n<function=web_search>{\"query\":\"test search functionality\"} <function=web_fetch>{\"url\":\"https://example.";
    assert_eq!(
        sanitize_workflow_final_response_candidate(response),
        "No, the web tools still aren't executing."
    );
}

#[test]
