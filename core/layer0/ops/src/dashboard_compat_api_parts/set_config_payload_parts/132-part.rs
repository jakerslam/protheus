fn message_requests_live_web_comparison(message: &str) -> bool {
    let lowered = clean_text(message, 500).to_ascii_lowercase();
    if lowered.is_empty() || !message_requests_comparative_answer(message) {
        return false;
    }
    let mentions_external_peer = lowered.contains("openclaw")
        || lowered.contains("chatgpt")
        || lowered.contains("claude")
        || lowered.contains("codex")
        || lowered.contains("cursor")
        || lowered.contains("copilot")
        || lowered.contains("windsurf")
        || lowered.contains("perplexity");
    let asks_live_web = lowered.contains("web")
        || lowered.contains("web search")
        || lowered.contains("search the web")
        || lowered.contains("search online")
        || lowered.contains("browse")
        || lowered.contains("online")
        || lowered.contains("latest")
        || lowered.contains("current")
        || lowered.contains("today")
        || lowered.contains("source-backed")
        || lowered.contains("with sources");
    asks_live_web && mentions_external_peer
}

fn comparative_web_query_from_message(message: &str) -> Option<String> {
    if !message_requests_live_web_comparison(message) {
        return None;
    }
    let query = clean_text(message, 600);
    if query.is_empty() { None } else { Some(query) }
}

fn comparative_no_findings_fallback(message: &str) -> String {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    let asks_rank = lowered.contains("rank") || lowered.contains("ranking");
    let asks_structured_compare = lowered.contains("compare")
        || lowered.contains("comparison")
        || lowered.contains("vs")
        || lowered.contains("versus");
    if asks_rank || asks_structured_compare {
        return "Live web retrieval was low-signal in this turn (search-engine chrome without extractable findings). Provisional comparison: Infring is strongest in identity persistence, memory continuity, and integrated tool orchestration; top peers are currently stronger on tool/search failure recovery and handoff consistency. Ask me to rerun `batch_query` with named competitors and I will return a source-backed ranked table.".to_string();
    }
    "Live web retrieval was low-signal in this turn, so here is the stable comparison: Infring is strongest in identity persistence, memory continuity, and integrated tool orchestration, while mature peers are still stronger on failure recovery and handoff consistency. If you want live sourcing, I can rerun with `batch_query` and a narrower competitor set.".to_string()
}

fn comparative_natural_web_intent_from_message(message: &str) -> Option<(String, Value)> {
    comparative_web_query_from_message(message).map(|query| {
        (
            "batch_query".to_string(),
            json!({"source": "web", "query": query, "aperture": "medium"}),
        )
    })
}

fn message_requests_workspace_plus_web_comparison(message: &str) -> bool {
    let lowered = clean_text(message, 600).to_ascii_lowercase();
    if lowered.is_empty() || !message_requests_comparative_answer(message) {
        return false;
    }
    let mentions_local_subject = lowered.contains("infring")
        || lowered.contains("this system")
        || lowered.contains("this workspace")
        || lowered.contains("this repo")
        || lowered.contains("this repository")
        || lowered.contains("this codebase")
        || lowered.contains("this project");
    let mentions_external_peer = lowered.contains("openclaw")
        || lowered.contains("chatgpt")
        || lowered.contains("claude")
        || lowered.contains("codex")
        || lowered.contains("cursor")
        || lowered.contains("copilot")
        || lowered.contains("windsurf")
        || lowered.contains("perplexity");
    mentions_local_subject && mentions_external_peer
}

fn workspace_plus_web_comparison_queries_from_message(message: &str) -> Option<(String, String)> {
    if !message_requests_workspace_plus_web_comparison(message) {
        return None;
    }
    let cleaned = clean_text(message, 600);
    if cleaned.is_empty() {
        return None;
    }
    let workspace_query = clean_text(
        &format!(
            "Analyze this local workspace/system for architecture, workflow, tooling, and implementation evidence relevant to: {cleaned}"
        ),
        600,
    );
    if workspace_query.is_empty() {
        return None;
    }
    Some((workspace_query, cleaned))
}

fn workspace_plus_web_comparison_web_payload_from_message(message: &str) -> Option<Value> {
    let (_, cleaned) = workspace_plus_web_comparison_queries_from_message(message)?;
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("openclaw") {
        return Some(json!({
            "source": "web",
            "query": "OpenClaw AI assistant architecture features docs",
            "queries": [
                "OpenClaw AI assistant architecture",
                "OpenClaw AI assistant features",
                "site:openclaw.ai OpenClaw docs"
            ],
            "aperture": "medium"
        }));
    }
    Some(json!({
        "source": "web",
        "query": cleaned,
        "aperture": "medium"
    }))
}

fn framework_catalog_web_payload_from_query(query: &str) -> Option<Value> {
    let cleaned = clean_text(query, 600);
    let lowered = cleaned.to_ascii_lowercase();
    if cleaned.is_empty() {
        return None;
    }
    let asks_ranking = lowered.contains("top")
        || lowered.contains("best")
        || lowered.contains("leading")
        || lowered.contains("popular")
        || lowered.contains("comparison")
        || lowered.contains("compare")
        || lowered.contains("rank");
    let mentions_agent_frameworks = (lowered.contains("agentic") || lowered.contains("agent"))
        && (lowered.contains("framework")
            || lowered.contains("sdk")
            || lowered.contains("library")
            || lowered.contains("stack"));
    if !(asks_ranking && mentions_agent_frameworks) {
        return None;
    }
    Some(json!({
        "source": "web",
        "query": "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
        "queries": [
            "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
            "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
            "site:langchain.com LangGraph agent framework overview",
            "site:openai.github.io/openai-agents-python OpenAI Agents SDK overview",
            "site:crewai.com CrewAI agent framework overview",
            "site:microsoft.github.io AutoGen framework overview",
            "site:github.com huggingface/smolagents smolagents framework overview",
            "OpenAI Agents SDK official docs overview"
        ],
        "aperture": "medium"
    }))
}
