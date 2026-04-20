fn chat_ui_turn_is_meta_control_message(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    chat_ui_contains_any(
        &lowered,
        &[
            "that was just a test",
            "just a test",
            "just testing",
            "test only",
            "ignore that",
            "never mind",
            "nm",
            "thanks",
            "thank you",
            "cool",
            "sounds good",
            "did you try it",
            "did you do it",
            "what happened",
        ],
    ) && !chat_ui_contains_any(
        &lowered,
        &[
            "search",
            "web",
            "online",
            "internet",
            "file",
            "patch",
            "edit",
            "update",
            "create",
            "read",
            "memory",
            "repo",
            "codebase",
        ],
    )
}
