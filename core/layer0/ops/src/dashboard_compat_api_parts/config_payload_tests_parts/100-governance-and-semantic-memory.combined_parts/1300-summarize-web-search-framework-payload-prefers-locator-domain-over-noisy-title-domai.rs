fn summarize_web_search_framework_payload_prefers_locator_domain_over_noisy_title_domain() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: framework search returned official documentation hits.",
            "evidence_refs": [
                {"title":"Web result from academy.langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from watsonx.ai","locator":"https://crewai.com/"},
                {"title":"Web result from github.com","locator":"https://github.com/huggingface/smolagents"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph (langchain.com)"), "{summary}");
    assert!(lowered.contains("crewai (crewai.com)"), "{summary}");
    assert!(!lowered.contains("academy.langchain.com"), "{summary}");
    assert!(!lowered.contains("watsonx.ai"), "{summary}");
}

#[test]
