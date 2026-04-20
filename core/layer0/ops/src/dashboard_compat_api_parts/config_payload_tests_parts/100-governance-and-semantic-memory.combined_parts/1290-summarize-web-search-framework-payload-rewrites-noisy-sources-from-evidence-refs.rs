fn summarize_web_search_framework_payload_rewrites_noisy_sources_from_evidence_refs() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: langgraph.com.cn: LangGraph overview; zhihu.com: AutoGen discussion; crewai.org.cn: CrewAI overview.",
            "evidence_refs": [
                {"title":"LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain","locator":"https://www.langchain.com/langgraph"},
                {"title":"CrewAI official site","locator":"https://crewai.com/"},
                {"title":"OpenAI Agents SDK docs","locator":"https://openai.github.io/openai-agents-python/"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
    assert!(!lowered.contains("langgraph.com.cn"), "{summary}");
    assert!(!lowered.contains("crewai.org.cn"), "{summary}");
    assert!(!lowered.contains("zhihu.com"), "{summary}");
}

#[test]
