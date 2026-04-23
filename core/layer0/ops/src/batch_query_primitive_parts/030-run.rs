fn usage() {
    println!("batch-query commands:");
    println!(
        "  infring-ops batch-query query --source=web --query=\"...\" --aperture=small|medium"
    );
    println!("  infring-ops batch-query status");
    println!("  infring-ops batch-query policy");
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
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
            let flag_or_positional = |flags: &[&str], positional_idx: usize, fallback: &str| {
                flags
                    .iter()
                    .find_map(|flag| parsed.flags.get(*flag).map(String::as_str))
                    .or_else(|| parsed.positional.get(positional_idx).map(String::as_str))
                    .unwrap_or(fallback)
                    .to_string()
            };
            let query = clean_text(&flag_or_positional(&["query", "q"], 1, ""), 600);
            let source = flag_or_positional(&["source"], usize::MAX, "web");
            let aperture = flag_or_positional(&["aperture"], usize::MAX, "medium");
            api_batch_query(
                root,
                &json!({"source": source, "query": query, "aperture": aperture}),
            )
        }
        _ => {
            json!({"ok": false, "status": "blocked", "error": "batch_query_unknown_command", "command": command})
        }
    };
    print_payload(&payload);
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}
