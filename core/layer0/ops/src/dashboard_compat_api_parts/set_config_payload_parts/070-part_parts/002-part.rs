fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty()
        || cleaned.starts_with('/')
        || cleaned.contains("tool::")
        || message_explicitly_disallows_tool_calls(&cleaned)
        || message_is_affirmative_confirmation(&cleaned)
        || message_is_negative_confirmation(&cleaned)
        || message_is_tooling_status_check(&cleaned)
    {
        return Vec::new();
    }

    let Some(query) = implicit_live_web_research_query(&cleaned, workspace_hints) else {
        return Vec::new();
    };
    let input = json!({
        "source": "web",
        "query": query,
        "aperture": "medium"
    });
    let receipt = crate::deterministic_receipt_hash(&json!({
        "tool": "batch_query",
        "message": cleaned,
        "input": input
    }));
    vec![json!({
        "tool": "batch_query",
        "tool_name": "batch_query",
        "tool_key": "batch_query",
        "selected_tool_key": "batch_query",
        "selected_tool_family": "web_research",
        "selected_tool_label": "Research query pack",
        "label": "Research query pack",
        "reason": "Current external information request detected; preserve a web research candidate if the workflow LLM does not emit one.",
        "selection_source": "latent_live_web_research",
        "workflow_only": true,
        "requires_confirmation": true,
        "input": input.clone(),
        "request_payload": input,
        "discovery_receipt": receipt
    })]
}

fn implicit_live_web_research_query(message: &str, workspace_hints: &[Value]) -> Option<String> {
    let cleaned = clean_text(message, 800);
    if cleaned.is_empty() {
        return None;
    }
    let lowered = cleaned.to_ascii_lowercase();
    let local_anchor = [
        "this workspace",
        "this repo",
        "this repository",
        "this codebase",
        "this project",
        "this system",
        "this platform",
        "local workspace",
        "local repo",
        "local repository",
        "working tree",
        "current directory",
        "present working directory",
    ]
    .iter()
    .any(|token| lowered.contains(token));
    if local_anchor {
        return None;
    }

    let currentness_signal = [
        "right now",
        "current",
        "currently",
        "recent",
        "latest",
        "today",
        "as of",
        "benchmark",
        "benchmarks",
    ]
    .iter()
    .any(|token| lowered.contains(token));
    let research_signal = lowered.starts_with("research ")
        || lowered.starts_with("find ")
        || lowered.starts_with("look up ")
        || lowered.starts_with("lookup ")
        || lowered.starts_with("search ")
        || lowered.starts_with("which ")
        || lowered.starts_with("what ")
        || lowered.starts_with("who ")
        || lowered.starts_with("compare ")
        || lowered.starts_with("explain ")
        || lowered.contains('?')
        || lowered.contains(" strongest ")
        || lowered.contains(" strongest")
        || lowered.contains(" useful ")
        || lowered.contains(" useful")
        || lowered.contains(" open-source ")
        || lowered.contains("best ")
        || lowered.contains("top ");
    let workspace_only_turn = !workspace_hints.is_empty()
        && !currentness_signal
        && !lowered.contains("docs")
        && !lowered.contains("framework")
        && !lowered.contains("agent");
    if !research_signal || workspace_only_turn {
        return None;
    }

    Some(cleaned)
}
