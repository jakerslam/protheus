
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let (payload, exit_code) = match cmd.as_str() {
        "status" => status_command(root, argv),
        "validate" => validate_command(root, argv),
        "compress" => compress_command(root, argv),
        "decompress" => decompress_command(root, argv),
        "send" => send_command(root, argv),
        "log" => log_command(root, argv),
        "agent-prompt" | "prompt" => agent_prompt_command(root, argv),
        "resolve-modules" | "resolve" => resolve_modules_command(root, argv),
        "export-lexicon" => export_lexicon_command(root, argv),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => (
            error_payload(
                "nexus_internal_comms_error",
                cmd.as_str(),
                "unknown_command",
            ),
            1,
        ),
    };
    print_json(&payload);
    exit_code
}

