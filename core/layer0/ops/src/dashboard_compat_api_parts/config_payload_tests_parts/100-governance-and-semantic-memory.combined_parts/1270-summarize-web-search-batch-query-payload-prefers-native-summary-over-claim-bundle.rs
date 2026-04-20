fn summarize_web_search_batch_query_payload_prefers_native_summary_over_claim_bundle() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "type": "batch_query",
            "summary": "Key findings: langchain.com: LangGraph overview.; crewai.com: CrewAI overview.; openai.github.io: OpenAI Agents SDK overview.",
            "evidence_refs": [
                {"title":"Web result from langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from crewai.com","locator":"https://crewai.com/"},
                {"title":"Web result from openai.github.io","locator":"https://openai.github.io/openai-agents-python/"}
            ],
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "partial",
                            "text": "Key findings: langchain.com: LangGraph overview."
                        }
                    ]
                }
            }
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
}

#[test]
