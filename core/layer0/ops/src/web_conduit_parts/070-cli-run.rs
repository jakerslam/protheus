pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let nexus_connection = match command.as_str() {
        "fetch" | "browse" => {
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "web_conduit_fetch",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    println!(
                        "{}",
                        json!({
                            "ok": false,
                            "type": "web_conduit_nexus_gate",
                            "error": "nexus_route_denied",
                            "command": clean_text(command.as_str(), 40),
                            "reason": clean_text(&err, 240),
                            "fail_closed": true
                        })
                    );
                    return 1;
                }
            }
        }
        "search" => {
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "web_search",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    println!(
                        "{}",
                        json!({
                            "ok": false,
                            "type": "web_conduit_nexus_gate",
                            "error": "nexus_route_denied",
                            "command": clean_text(command.as_str(), 40),
                            "reason": clean_text(&err, 240),
                            "fail_closed": true
                        })
                    );
                    return 1;
                }
            }
        }
        _ => None,
    };
    let mut payload = match command.as_str() {
        "help" => {
            usage();
            json!({"ok": true, "type": "web_conduit_help"})
        }
        "status" => api_status(root),
        "providers" => api_providers(root),
        "receipts" => {
            let limit = parse_u64(parsed.flags.get("limit"), 20, 1, 200) as usize;
            api_receipts(root, limit)
        }
        "fetch" | "browse" => {
            let url = clean_text(
                parsed
                    .flags
                    .get("url")
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                2200,
            );
            api_fetch(
                root,
                &json!({
                    "url": url,
                    "human_approved": parse_bool(parsed.flags.get("human-approved")) || parse_bool(parsed.flags.get("human_approved")),
                    "approval_id": clean_text(
                        parsed
                            .flags
                            .get("approval-id")
                            .or_else(|| parsed.flags.get("approval_id"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        160
                    ),
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
                }),
            )
        }
        "search" => {
            let query = clean_text(
                parsed
                    .flags
                    .get("query")
                    .or_else(|| parsed.flags.get("q"))
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                600,
            );
            let allowed_domains = parsed
                .flags
                .get("allowed-domains")
                .or_else(|| parsed.flags.get("allowed_domains"))
                .cloned()
                .unwrap_or_default();
            let provider = clean_text(
                parsed
                    .flags
                    .get("provider")
                    .or_else(|| parsed.flags.get("source"))
                    .or_else(|| parsed.flags.get("search-provider"))
                    .or_else(|| parsed.flags.get("search_provider"))
                    .map(String::as_str)
                    .unwrap_or("auto"),
                40,
            );
            let top_k = parse_u64(
                parsed
                    .flags
                    .get("top-k")
                    .or_else(|| parsed.flags.get("top_k"))
                    .or_else(|| parsed.flags.get("max-results"))
                    .or_else(|| parsed.flags.get("max_results")),
                8,
                1,
                12,
            );
            api_search(
                root,
                &json!({
                    "query": query,
                    "allowed_domains": normalize_allowed_domains(&json!(allowed_domains)),
                    "provider": provider,
                    "top_k": top_k,
                    "exclude_subdomains": parse_bool(parsed.flags.get("exclude-subdomains")) || parse_bool(parsed.flags.get("exclude_subdomains")) || parse_bool(parsed.flags.get("exact-domain-only")) || parse_bool(parsed.flags.get("exact_domain_only")),
                    "human_approved": parse_bool(parsed.flags.get("human-approved")) || parse_bool(parsed.flags.get("human_approved")),
                    "approval_id": clean_text(
                        parsed
                            .flags
                            .get("approval-id")
                            .or_else(|| parsed.flags.get("approval_id"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        160
                    ),
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
                }),
            )
        }
        _ => json!({
            "ok": false,
            "error": "web_conduit_unknown_command",
            "command": command
        }),
    };
    if let Some(meta) = nexus_connection {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
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
