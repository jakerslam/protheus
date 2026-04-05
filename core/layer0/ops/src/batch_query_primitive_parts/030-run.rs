fn usage() {
    println!("batch-query commands:");
    println!("  protheus-ops batch-query query --source=web --query=\"...\" --aperture=small|medium");
    println!("  protheus-ops batch-query status");
    println!("  protheus-ops batch-query policy");
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "query".to_string());
    let payload = match command.as_str() {
        "help" => {
            usage();
            json!({"ok": true, "type": "batch_query_help"})
        }
        "status" => json!({
            "ok": true,
            "type": "batch_query_status",
            "policy_path": root.join(POLICY_REL).to_string_lossy().to_string(),
            "receipts_path": root.join(RECEIPTS_REL).to_string_lossy().to_string()
        }),
        "policy" => json!({"ok": true, "type": "batch_query_policy", "policy": load_policy(root)}),
        "query" => {
            let query = clean_text(
                parsed
                    .flags
                    .get("query")
                    .or_else(|| parsed.flags.get("q"))
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                600,
            );
            let source = parsed
                .flags
                .get("source")
                .map(String::as_str)
                .unwrap_or("web");
            let aperture = parsed
                .flags
                .get("aperture")
                .map(String::as_str)
                .unwrap_or("medium");
            api_batch_query(root, &json!({"source": source, "query": query, "aperture": aperture}))
        }
        _ => json!({"ok": false, "status": "blocked", "error": "batch_query_unknown_command", "command": command}),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}
