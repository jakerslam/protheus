
fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let _ = (message, workspace_hints);
    // LLM-authoritative workflow mode: tool options must come from the JSON
    // workflow contract, not semantic guesses over the user message.
    Vec::new()
}
