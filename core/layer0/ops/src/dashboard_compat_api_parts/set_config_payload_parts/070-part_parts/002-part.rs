fn latent_tool_candidates_for_message(message: &str, workspace_hints: &[Value]) -> Vec<Value> {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty()
        || cleaned.starts_with('/')
        || cleaned.contains("tool::")
        || message_explicitly_disallows_tool_calls(&cleaned)
        || message_is_affirmative_confirmation(&cleaned)
        || message_is_negative_confirmation(&cleaned)
        || message_is_tooling_status_check(&cleaned)
        || !workspace_hints.is_empty()
    {
        return Vec::new();
    }
    // Natural-language query classification is intentionally disabled here.
    // Recovery must come from the workflow-selected tool family/tool contract,
    // not from Rust guessing what a user meant based on prompt phrasing.
    Vec::new()
}
