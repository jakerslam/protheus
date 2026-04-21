
fn run_mention(root: &Path, parsed: &crate::ParsedArgs, default_query: &str) -> Value {
    let search_payload = run_search(root, parsed, default_query);
    if !search_payload
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "ok": false,
            "status": "blocked",
            "type": "workspace_file_search_mention",
            "error": search_payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("workspace_file_search_failed"),
            "source": "cline:file-search-mention",
        });
    }

    let mention_prefix = parsed
        .flags
        .get("mention-prefix")
        .map(|row| crate::clean(row, 8))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| "@".to_string());
    let query = search_payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let warnings = search_payload
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let results = search_payload
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if let Some(first) = results.first() {
        let path = first
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mention = format!("{mention_prefix}{path}");
        let receipt = json!({
            "type": "workspace_file_search_mention_receipt",
            "ts": crate::now_iso(),
            "source": "cline/src/utils/file-search.ts",
            "query": query,
            "mention": mention,
            "path": path,
            "workspace_name": first
                .get("workspace_name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "result_item_type": first
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "warnings": warnings,
        });
        append_receipt(root, &receipt);
        return json!({
            "ok": true,
            "status": "ok",
            "type": "workspace_file_search_mention",
            "source": "cline:file-search-mention",
            "query": query,
            "mention": mention,
            "path": path,
            "workspace_name": first
                .get("workspace_name")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "item_type": first
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "warnings": warnings,
        });
    }

    json!({
        "ok": true,
        "status": "no_results",
        "type": "workspace_file_search_mention",
        "source": "cline:file-search-mention",
        "query": query,
        "mention": Value::Null,
        "path": Value::Null,
        "warnings": warnings,
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "search".to_string());
    let payload = match command.as_str() {
        "help" | "--help" | "-h" => {
            usage();
            json!({"ok": true, "type": "workspace_file_search_help"})
        }
        "status" => json!({
            "ok": true,
            "type": "workspace_file_search_status",
            "source": "cline:file-search",
            "ripgrep_binary": std::env::var("PROTHEUS_RG_BINARY").unwrap_or_else(|_| "rg".to_string()),
            "ripgrep_install_hint": ripgrep_install_hint()
        }),
        "list" => run_search(root, &parsed, ""),
        "search" => run_search(root, &parsed, &positional_query(&parsed)),
        "mention" => run_mention(root, &parsed, &positional_query(&parsed)),
        _ => {
            json!({"ok": false, "status": "blocked", "error": "workspace_file_search_unknown_command", "command": command})
        }
    };
    print_payload_and_exit(&payload)
}
