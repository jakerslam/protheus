    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"Try to web search \"top AI agentic frameworks\" and return the results"}"#,
        &snapshot,
    )
    .expect("message response");
    assert_eq!(response.status, 200);
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/name")
            .and_then(Value::as_str),
        Some("batch_query")
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/status")
            .and_then(Value::as_str),
        Some("timeout")
    );
    assert_eq!(
        response
            .payload
            .pointer("/tools/0/is_error")
            .and_then(Value::as_bool),
        Some(true)
    );
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lowered = response_text.to_ascii_lowercase();
    assert!(
        !response_text.trim().is_empty(),
        "expected a synthesized failure response"
    );
    assert!(
        lowered.contains("timeout"),
        "expected timeout detail in final response: {response_text}"
    );
    assert!(
        lowered.contains("batch_query") || lowered.contains("search"),
        "expected tool context in final response: {response_text}"
    );
    assert!(!response_is_no_findings_placeholder(response_text));
    assert_eq!(
        response
            .payload
            .pointer("/response_workflow/final_llm_response/status")
            .and_then(Value::as_str),
        Some("synthesized")
    );

    let tool_calls = read_json(&governance_test_tool_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert_eq!(tool_calls.len(), 4);
    assert_eq!(
        tool_calls[0].get("tool").and_then(Value::as_str),
        Some("batch_query")
    );

    let model_calls = read_json(&governance_test_chat_script_path(root.path()))
        .and_then(|value| value.get("calls").cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    assert!(
        model_calls.len() >= 2,
        "expected at least two model passes, got {}",
        model_calls.len()
    );
    let final_user_message = model_calls
        .last()
        .and_then(|row| row.get("user_message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(final_user_message.contains("Recorded tool outcomes"));
    assert!(final_user_message.contains("provider timeout after 30s"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_web_file_better_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "does the web or file tooling seem any better?",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("routing/finalization miss"));
    assert!(lowered.contains("web_search"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_route_mapping_suggestion_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Run Improve command-to-route mapping for higher supported tool hit rate",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("implementation task"));
    assert!(lowered.contains("command-to-route mapping"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_spawn_route_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "Implement a supported Rust route for `tool::spawn_subagents`",
        &no_findings_user_facing_response(),
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("runtime-route implementation task"));
    assert!(lowered.contains("spawn_subagents"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

