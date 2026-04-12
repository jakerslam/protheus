fn parse_json_flag(raw: Option<&String>) -> Result<Value, String> {
    let Some(raw_value) = raw else {
        return Ok(Value::Null);
    };
    serde_json::from_str::<Value>(raw_value)
        .map_err(|err| format!("invalid_json_flag:{}", clean_text(&err.to_string(), 240)))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let nexus_connection = match command.as_str() {
        "fetch" | "browse" | "media" => {
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
        "setup" => api_setup(
            root,
            &json!({
                "provider": clean_text(
                    parsed
                        .flags
                        .get("provider")
                        .or_else(|| parsed.flags.get("search-provider"))
                        .or_else(|| parsed.flags.get("search_provider"))
                        .map(String::as_str)
                        .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                    60
                ),
                "api_key": clean_text(
                    parsed
                        .flags
                        .get("api-key")
                        .or_else(|| parsed.flags.get("api_key"))
                        .map(String::as_str)
                        .unwrap_or(""),
                    600
                ),
                "api_key_env": clean_text(
                    parsed
                        .flags
                        .get("api-key-env")
                        .or_else(|| parsed.flags.get("api_key_env"))
                        .map(String::as_str)
                        .unwrap_or(""),
                    160
                ),
                "apply": parse_bool(parsed.flags.get("apply")),
                "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
            }),
        ),
        "native-codex" => {
            let payload = match parse_json_flag(
                parsed
                    .flags
                    .get("payload-json")
                    .or_else(|| parsed.flags.get("payload_json")),
            ) {
                Ok(value) => value,
                Err(err) => {
                    println!(
                        "{}",
                        json!({
                            "ok": false,
                            "type": "web_conduit_native_codex",
                            "error": "invalid_payload_json",
                            "reason": err
                        })
                    );
                    return 1;
                }
            };
            api_native_codex(
                root,
                &json!({
                    "model_provider": clean_text(
                        parsed
                            .flags
                            .get("model-provider")
                            .or_else(|| parsed.flags.get("model_provider"))
                            .or_else(|| parsed.flags.get("provider"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        80
                    ),
                    "model_api": clean_text(
                        parsed
                            .flags
                            .get("model-api")
                            .or_else(|| parsed.flags.get("model_api"))
                            .or_else(|| parsed.flags.get("api"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        80
                    ),
                    "payload": payload,
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
                }),
            )
        }
        "migrate-legacy-config" => api_migrate_legacy_config(
            root,
            &json!({
                "source_path": clean_text(
                    parsed
                        .flags
                        .get("source-path")
                        .or_else(|| parsed.flags.get("source_path"))
                        .or_else(|| parsed.flags.get("from-path"))
                        .or_else(|| parsed.flags.get("from_path"))
                        .map(String::as_str)
                        .unwrap_or(""),
                    2200
                ),
                "apply": parse_bool(parsed.flags.get("apply")),
                "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only"))
            }),
        ),
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
            let provider = clean_text(
                parsed
                    .flags
                    .get("provider")
                    .or_else(|| parsed.flags.get("source"))
                    .or_else(|| parsed.flags.get("fetch-provider"))
                    .or_else(|| parsed.flags.get("fetch_provider"))
                    .map(String::as_str)
                    .unwrap_or("auto"),
                40,
            );
            let resolve_citation_redirect = parsed
                .flags
                .get("resolve-citation-redirect")
                .or_else(|| parsed.flags.get("resolve_citation_redirect"))
                .map(|raw| {
                    !matches!(
                        raw.trim().to_ascii_lowercase().as_str(),
                        "0" | "false" | "no" | "off"
                    )
                })
                .unwrap_or(true);
            api_fetch(
                root,
                &json!({
                    "url": url,
                    "provider": provider,
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
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only")),
                    "extract_mode": clean_text(
                        parsed
                            .flags
                            .get("extract-mode")
                            .or_else(|| parsed.flags.get("extract_mode"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        24
                    ),
                    "max_chars": parse_u64(
                        parsed.flags.get("max-chars").or_else(|| parsed.flags.get("max_chars")),
                        50000,
                        100,
                        200000
                    ),
                    "max_response_bytes": parse_u64(
                        parsed
                            .flags
                            .get("max-response-bytes")
                            .or_else(|| parsed.flags.get("max_response_bytes")),
                        350000,
                        4096,
                        4000000
                    ),
                    "timeout_ms": parse_u64(parsed.flags.get("timeout-ms").or_else(|| parsed.flags.get("timeout_ms")), 9000, 1000, 120000),
                    "cache_ttl_minutes": parse_u64(
                        parsed
                            .flags
                            .get("cache-ttl-minutes")
                            .or_else(|| parsed.flags.get("cache_ttl_minutes")),
                        15,
                        0,
                        240
                    ),
                    "resolve_citation_redirect": resolve_citation_redirect
                }),
            )
        }
        "media" => {
            let source = clean_text(
                parsed
                    .flags
                    .get("url")
                    .or_else(|| parsed.flags.get("path"))
                    .map(String::as_str)
                    .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or("")),
                4000,
            );
            let resolve_citation_redirect = parsed
                .flags
                .get("resolve-citation-redirect")
                .or_else(|| parsed.flags.get("resolve_citation_redirect"))
                .map(|raw| {
                    !matches!(
                        raw.trim().to_ascii_lowercase().as_str(),
                        "0" | "false" | "no" | "off"
                    )
                })
                .unwrap_or(true);
            api_media(
                root,
                &json!({
                    "url": source,
                    "path": clean_text(parsed.flags.get("path").map(String::as_str).unwrap_or(""), 4000),
                    "workspace_dir": clean_text(
                        parsed
                            .flags
                            .get("workspace-dir")
                            .or_else(|| parsed.flags.get("workspace_dir"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        2200
                    ),
                    "local_roots": clean_text(
                        parsed
                            .flags
                            .get("local-roots")
                            .or_else(|| parsed.flags.get("local_roots"))
                            .or_else(|| parsed.flags.get("local-root"))
                            .or_else(|| parsed.flags.get("local_root"))
                            .map(String::as_str)
                            .unwrap_or(""),
                        4000
                    ),
                    "host_read_capability": parse_bool(parsed.flags.get("host-read-capability"))
                        || parse_bool(parsed.flags.get("host_read_capability"))
                        || parse_bool(parsed.flags.get("allow-host-read"))
                        || parse_bool(parsed.flags.get("allow_host_read")),
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
                    "summary_only": parse_bool(parsed.flags.get("summary-only")) || parse_bool(parsed.flags.get("summary_only")),
                    "provider": clean_text(
                        parsed
                            .flags
                            .get("provider")
                            .or_else(|| parsed.flags.get("fetch-provider"))
                            .or_else(|| parsed.flags.get("fetch_provider"))
                            .map(String::as_str)
                            .unwrap_or("auto"),
                        40
                    ),
                    "resolve_citation_redirect": resolve_citation_redirect,
                    "timeout_ms": parse_u64(parsed.flags.get("timeout-ms").or_else(|| parsed.flags.get("timeout_ms")), 9000, 1000, 120000),
                    "max_bytes": parse_u64(parsed.flags.get("max-bytes").or_else(|| parsed.flags.get("max_bytes")), 8 * 1024 * 1024, 4096, 32 * 1024 * 1024)
                }),
            )
        }
        "parse-media" => {
            let text = parsed
                .flags
                .get("text")
                .map(String::as_str)
                .unwrap_or_else(|| parsed.positional.get(1).map(String::as_str).unwrap_or(""));
            api_parse_media(&json!({ "text": text }))
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
                    .or_else(|| parsed.flags.get("count"))
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
                    "count": top_k,
                    "exclude_subdomains": parse_bool(parsed.flags.get("exclude-subdomains")) || parse_bool(parsed.flags.get("exclude_subdomains")) || parse_bool(parsed.flags.get("exact-domain-only")) || parse_bool(parsed.flags.get("exact_domain_only")),
                    "timeout_ms": parse_u64(
                        parsed.flags.get("timeout-ms").or_else(|| parsed.flags.get("timeout_ms")),
                        9000,
                        1000,
                        120000
                    ),
                    "cache_ttl_minutes": parse_u64(
                        parsed.flags.get("cache-ttl-minutes").or_else(|| parsed.flags.get("cache_ttl_minutes")),
                        8,
                        0,
                        240
                    ),
                    "country": clean_text(parsed.flags.get("country").map(String::as_str).unwrap_or(""), 32),
                    "language": clean_text(parsed.flags.get("language").map(String::as_str).unwrap_or(""), 32),
                    "freshness": clean_text(parsed.flags.get("freshness").map(String::as_str).unwrap_or(""), 32),
                    "date_after": clean_text(
                        parsed.flags.get("date-after").or_else(|| parsed.flags.get("date_after")).map(String::as_str).unwrap_or(""),
                        32
                    ),
                    "date_before": clean_text(
                        parsed.flags.get("date-before").or_else(|| parsed.flags.get("date_before")).map(String::as_str).unwrap_or(""),
                        32
                    ),
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
