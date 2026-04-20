
fn chat_workflow_tool_hints_for_message(message: &str) -> Vec<Value> {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::<Value>::new();
    let mut seen = HashSet::<String>::new();
    let mut push_hint = |tool: &str,
                         label: &str,
                         reason: &str,
                         proposed_input: Value,
                         selection_source: &str,
                         workflow_only: bool| {
        let normalized = normalize_tool_name(tool);
        if normalized.is_empty() {
            return;
        }
        if normalized != "tool_command_router" && seen.contains(&normalized) {
            return;
        }
        if normalized != "tool_command_router" {
            seen.insert(normalized.clone());
        }
        let receipt = crate::deterministic_receipt_hash(&json!({
            "tool": normalized,
            "label": label,
            "reason": reason,
            "message": cleaned.as_str(),
            "input": proposed_input.clone(),
            "selection_source": selection_source,
            "workflow_only": workflow_only
        }));
        out.push(json!({
            "tool": normalized,
            "label": clean_text(label, 80),
            "reason": clean_text(reason, 240),
            "requires_confirmation": true,
            "proposed_input": proposed_input,
            "selection_source": clean_text(selection_source, 80),
            "workflow_only": workflow_only,
            "discovery_receipt": receipt
        }));
    };

    if let Some(parsed_explicit) = parse_explicit_tool_command_from_message(&cleaned) {
        match parsed_explicit {
            Ok((tool, proposed_input)) => {
                push_hint(
                    &tool,
                    "honor explicit tool request",
                    "The user used explicit tool syntax in chat. Treat it as a strong workflow hint; the tool has not executed yet.",
                    proposed_input,
                    "explicit_tool_command",
                    false,
                );
            }
            Err(payload) => {
                push_hint(
                    "tool_command_router",
                    "explain explicit tool syntax error",
                    "The user used explicit tool syntax in chat, but it was malformed or unsupported. Explain the error and suggest the supported form instead of pretending a tool ran.",
                    payload,
                    "explicit_tool_command_error",
                    true,
                );
            }
        }
        return out;
    }

    if !cleaned.starts_with('/') {
        return out;
    }

    let mut split = cleaned.splitn(2, char::is_whitespace);
    let command = split
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let arg = split.next().map(str::trim).unwrap_or("");

    let slash_error = |error: &str, msg: &str| {
        json!({
            "ok": false,
            "error": clean_text(error, 80),
            "message": clean_text(msg, 320)
        })
    };

    match command.as_str() {
        "/file" => {
            if arg.is_empty() {
                push_hint(
                    "tool_command_router",
                    "explain slash file usage",
                    "The user asked for slash file access in chat, but no path was supplied.",
                    slash_error("slash_path_required", "`/file` needs a path, for example `/file notes/plan.txt`."),
                    "slash_file_error",
                    true,
                );
            } else {
                push_hint(
                    "file_read",
                    "open requested file",
                    "The user used slash file syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                    json!({"path": arg, "full": true}),
                    "slash_file_hint",
                    false,
                );
            }
        }
        "/folder" => {
            if arg.is_empty() {
                push_hint(
                    "tool_command_router",
                    "explain slash folder usage",
                    "The user asked for slash folder export in chat, but no path was supplied.",
                    slash_error("slash_path_required", "`/folder` needs a path, for example `/folder notes`."),
                    "slash_folder_error",
                    true,
                );
            } else {
                push_hint(
                    "folder_export",
                    "inspect requested folder",
                    "The user used slash folder syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                    json!({"path": arg, "full": true}),
                    "slash_folder_hint",
                    false,
                );
            }
        }
        "/terminal" | "/term" | "/shell" => {
            if arg.is_empty() {
                push_hint(
                    "tool_command_router",
                    "explain slash terminal usage",
                    "The user asked for slash terminal execution in chat, but no command was supplied.",
                    slash_error("slash_command_required", "`/terminal` needs a command, for example `/terminal rg TODO .`."),
                    "slash_terminal_error",
                    true,
                );
            } else {
                push_hint(
                    "terminal_exec",
                    "run requested terminal command",
                    "The user used slash terminal syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                    json!({
                        "command": arg,
                        "confirm": true,
                        "approval_note": "user slash terminal request via workflow"
                    }),
                    "slash_terminal_hint",
                    false,
                );
            }
        }
        "/browse" | "/web" => {
            if arg.is_empty() {
                push_hint(
                    "tool_command_router",
                    "explain slash web usage",
                    "The user asked for slash web fetch in chat, but no URL was supplied.",
                    slash_error("slash_url_required", "`/web` needs an absolute http(s) URL, for example `/web https://example.com`."),
                    "slash_web_error",
                    true,
                );
            } else {
                push_hint(
                    "web_fetch",
                    "fetch requested url",
                    "The user used slash web/browse syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                    json!({"url": arg, "summary_only": true}),
                    "slash_web_hint",
                    false,
                );
            }
        }
        "/search" | "/batch" => {
            if arg.is_empty() {
                push_hint(
                    "tool_command_router",
                    "explain slash search usage",
                    "The user asked for slash web search in chat, but no query was supplied.",
                    slash_error("slash_query_required", "`/search` needs a query, for example `/search top AI agentic frameworks`."),
                    "slash_search_error",
                    true,
                );
            } else {
                push_hint(
                    "batch_query",
                    "search live web",
                    "The user used slash search syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                    json!({"source": "web", "query": arg, "aperture": "medium"}),
                    "slash_search_hint",
                    false,
                );
            }
        }
        "/capabilities" | "/tools" => {
            push_hint(
                "tool_capabilities",
                "inspect tool catalog",
                "The user asked about tool capabilities in chat. Treat it as a workflow hint rather than a pre-executed command.",
                json!({"scope": "agent", "reason": "slash_capabilities"}),
                "slash_capabilities_hint",
                false,
            );
        }
        "/swarm" | "/spawn" | "/subagents" => {
            let mut count = 3usize;
            let mut objective = arg;
            let mut tokens = arg.splitn(2, char::is_whitespace);
            if let Some(first) = tokens.next() {
                if let Ok(value) = first.trim().parse::<usize>() {
                    count = value.clamp(1, 8);
                    objective = tokens.next().map(str::trim).unwrap_or("");
                }
            }
            if objective.is_empty() {
                objective = "Parallel descendant task requested by user directive.";
            }
            push_hint(
                "spawn_subagents",
                "parallel subagents",
                "The user used slash swarm syntax in chat. Treat it as a strong workflow hint rather than a pre-executed command.",
                json!({
                    "count": count,
                    "objective": clean_text(objective, 800),
                    "confirm": true,
                    "approval_note": "user slash spawn request via workflow"
                }),
                "slash_spawn_hint",
                false,
            );
        }
        "/memory" => {
            let mut memory_parts = arg.splitn(3, char::is_whitespace);
            let action = memory_parts
                .next()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let key = memory_parts.next().map(str::trim).unwrap_or("");
            let raw_value = memory_parts.next().map(str::trim).unwrap_or("");
            match action.as_str() {
                "list" | "ls" => push_hint(
                    "memory_kv_list",
                    "list memory keys",
                    "The user used slash memory syntax in chat. Treat it as a workflow hint rather than a pre-executed command.",
                    json!({}),
                    "slash_memory_list_hint",
                    false,
                ),
                "query" | "search" => {
                    let query_source = if key.is_empty() {
                        raw_value.to_string()
                    } else if raw_value.is_empty() {
                        key.to_string()
                    } else {
                        format!("{key} {raw_value}")
                    };
                    let query = clean_text(&query_source, 600);
                    if query.is_empty() {
                        push_hint(
                            "tool_command_router",
                            "explain slash memory query usage",
                            "The user asked for slash memory query in chat, but no query was supplied.",
                            slash_error("slash_query_required", "`/memory query` needs a query string."),
                            "slash_memory_query_error",
                            true,
                        );
                    } else {
                        push_hint(
                            "memory_semantic_query",
                            "query semantic memory",
                            "The user used slash memory query syntax in chat. Treat it as a workflow hint rather than a pre-executed command.",
                            json!({"query": query, "limit": 8}),
                            "slash_memory_query_hint",
                            false,
                        );
                    }
                }
                "get" => {
                    if key.is_empty() {
                        push_hint(
                            "tool_command_router",
                            "explain slash memory get usage",
                            "The user asked for slash memory get in chat, but no key was supplied.",
                            slash_error("slash_key_required", "`/memory get` needs a key."),
                            "slash_memory_get_error",
                            true,
                        );
                    } else {
                        push_hint(
                            "memory_kv_get",
                            "read memory key",
                            "The user used slash memory get syntax in chat. Treat it as a workflow hint rather than a pre-executed command.",
                            json!({"key": key}),
                            "slash_memory_get_hint",
                            false,
                        );
                    }
                }
                "set" => {
                    if key.is_empty() {
                        push_hint(
                            "tool_command_router",
                            "explain slash memory set usage",
                            "The user asked for slash memory set in chat, but no key was supplied.",
                            slash_error("slash_key_required", "`/memory set` needs a key and value."),
                            "slash_memory_set_error",
                            true,
                        );
                    } else {
                        let parsed_value = serde_json::from_str::<Value>(raw_value)
                            .ok()
                            .unwrap_or_else(|| json!(raw_value));
                        push_hint(
                            "memory_kv_set",
                            "store memory value",
                            "The user used slash memory set syntax in chat. Treat it as a workflow hint rather than a pre-executed command.",
                            json!({"key": key, "value": parsed_value, "confirm": true}),
                            "slash_memory_set_hint",
                            false,
                        );
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    out
}
