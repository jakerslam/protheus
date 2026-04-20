fn chat_ui_turn_requires_file_mutation(raw_input: &str) -> bool {
    let lowered = clean(raw_input, 1_200).to_ascii_lowercase();
    chat_ui_contains_any(
        &lowered,
        &[
            "edit file",
            "modify file",
            "update file",
            "patch",
            "write ",
            "rewrite ",
            "create file",
            "add file",
            "delete file",
            "remove file",
            "rename file",
            "refactor",
            "implement",
        ],
    )
}
