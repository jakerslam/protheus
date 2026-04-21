fn chat_ui_turn_requires_local_lookup(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    chat_ui_contains_any(
        &lowered,
        &[
            "repo",
            "repository",
            "workspace",
            "codebase",
            "project files",
            "memory file",
            "local memory",
            "logs",
            "read file",
            "check file",
            "inspect file",
            "file tooling",
            "file tool",
            "local file",
            "use local tools",
            "use file tools",
            "can you access files",
            "can you use file",
            "in this repo",
            "in our system",
        ],
    )
}
