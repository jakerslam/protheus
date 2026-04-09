fn direct_tool_intent_from_user_message(message: &str) -> Option<(String, Value)> {
    let trimmed = message.trim();
    if let Some(parsed_explicit) = parse_explicit_tool_command_from_message(trimmed) {
        return match parsed_explicit {
            Ok(route) => Some(route),
            Err(payload) => Some(("tool_command_router".to_string(), payload)),
        };
    }
    if !trimmed.starts_with('/') {
        if message_explicitly_disallows_tool_calls(trimmed) {
            return None;
        }
        let lowered = clean_text(trimmed, 2200).to_ascii_lowercase();
        let asks_file_read = lowered.contains("read file")
            || lowered.contains("open file")
            || lowered.contains("show file")
            || lowered.contains("view file")
            || lowered.contains("inspect file")
            || lowered.starts_with("cat ");
        if asks_file_read {
            for raw in trimmed.split_whitespace() {
                let candidate = clean_text(
                    raw.trim_matches(|ch| matches!(ch, '`' | '"' | '\'' | ',' | ')' | ']' | '>')),
                    4000,
                );
                if candidate.is_empty()
                    || candidate.starts_with("http://")
                    || candidate.starts_with("https://")
                {
                    continue;
                }
                let has_path_shape = candidate.contains('/')
                    || candidate.contains('\\')
                    || candidate.starts_with("./")
                    || candidate.starts_with("../");
                let ext = Path::new(&candidate)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if has_path_shape
                    || matches!(
                        ext.as_str(),
                        "rs" | "ts"
                            | "tsx"
                            | "js"
                            | "jsx"
                            | "json"
                            | "md"
                            | "toml"
                            | "yaml"
                            | "yml"
                            | "txt"
                            | "sh"
                            | "py"
                    )
                {
                    return Some((
                        "file_read".to_string(),
                        json!({"path": candidate, "full": true}),
                    ));
                }
            }
        }
        if let Some(route) = natural_web_intent_from_user_message(trimmed) {
            return Some(route);
        }
        if memory_recall_requested(trimmed) {
            return None;
        }
        let lowered = clean_text(trimmed, 120).to_ascii_lowercase();
        if lowered.contains("what did we decide") && lowered.contains("about") {
            return Some((
                "memory_semantic_query".to_string(),
                json!({"query": clean_text(trimmed, 600), "limit": 8}),
            ));
        }
        let undo_like = lowered == "undo"
            || lowered == "undo that"
            || lowered == "undo last"
            || lowered == "rewind";
        if undo_like {
            return Some(("session_rollback_last_turn".to_string(), json!({})));
        }
        return None;
    }
    let mut split = trimmed.splitn(2, char::is_whitespace);
    let command = split
        .next()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let arg = split.next().map(str::trim).unwrap_or("");
    match command.as_str() {
        "/file" => {
            if arg.is_empty() {
                None
            } else {
                Some(("file_read".to_string(), json!({"path": arg, "full": true})))
            }
        }
        "/folder" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "folder_export".to_string(),
                    json!({"path": arg, "full": true}),
                ))
            }
        }
        "/terminal" | "/term" | "/shell" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "terminal_exec".to_string(),
                    json!({
                        "command": arg,
                        "confirm": true,
                        "approval_note": "user slash terminal invocation"
                    }),
                ))
            }
        }
        "/browse" | "/web" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "web_fetch".to_string(),
                    json!({"url": arg, "summary_only": true}),
                ))
            }
        }
        "/search" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "batch_query".to_string(),
                    json!({"source": "web", "query": arg, "aperture": "medium"}),
                ))
            }
        }
        "/batch" => {
            if arg.is_empty() {
                None
            } else {
                Some((
                    "batch_query".to_string(),
                    json!({"source": "web", "query": arg, "aperture": "medium"}),
                ))
            }
        }
        "/swarm" | "/spawn" | "/subagents" => {
            let mut count = 3usize;
            let mut objective = arg;
            let mut tokens = arg.splitn(2, char::is_whitespace);
            if let Some(first) = tokens.next() {
                let parsed = first.trim().parse::<usize>().ok();
                if let Some(value) = parsed {
                    count = value.clamp(1, 8);
                    objective = tokens.next().map(str::trim).unwrap_or("");
                }
            }
            if objective.is_empty() {
                objective = "Parallel descendant task requested by user directive.";
            }
            Some((
                "spawn_subagents".to_string(),
                json!({
                    "count": count,
                    "objective": clean_text(objective, 800),
                    "confirm": true,
                    "approval_note": "user slash spawn request"
                }),
            ))
        }
        "/undo" | "/rewind" | "/rollback" => {
            Some(("session_rollback_last_turn".to_string(), json!({})))
        }
        "/memory" => {
            let mut memory_parts = arg.splitn(3, char::is_whitespace);
            let action = memory_parts
                .next()
                .map(|value| value.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let key = memory_parts.next().map(str::trim).unwrap_or("");
            let raw_value = memory_parts.next().map(str::trim).unwrap_or("");
            if action == "list" || action == "ls" {
                Some(("memory_kv_list".to_string(), json!({})))
            } else if action == "query" || action == "search" {
                let query_source = if key.is_empty() {
                    raw_value.to_string()
                } else if raw_value.is_empty() {
                    key.to_string()
                } else {
                    format!("{key} {raw_value}")
                };
                let query = clean_text(&query_source, 600);
                if query.is_empty() {
                    None
                } else {
                    Some((
                        "memory_semantic_query".to_string(),
                        json!({"query": query, "limit": 8}),
                    ))
                }
            } else if action == "get" {
                if key.is_empty() {
                    None
                } else {
                    Some(("memory_kv_get".to_string(), json!({"key": key})))
                }
            } else if action == "set" {
                if key.is_empty() {
                    None
                } else {
                    let parsed_value = serde_json::from_str::<Value>(raw_value)
                        .ok()
                        .unwrap_or_else(|| json!(raw_value));
                    Some((
                        "memory_kv_set".to_string(),
                        json!({"key": key, "value": parsed_value, "confirm": true}),
                    ))
                }
            } else {
                None
            }
        }
        "/cron" | "/schedule" => cron_tool_request_from_args(arg),
        _ => None,
    }
}

fn message_explicitly_disallows_tool_calls(message: &str) -> bool {
    let lowered = clean_text(message, 400).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    lowered.contains("dont use a tool")
        || lowered.contains("don't use a tool")
        || lowered.contains("do not use a tool")
        || lowered.contains("dont call a tool")
        || lowered.contains("don't call a tool")
        || lowered.contains("do not call a tool")
        || lowered.contains("without tool")
        || lowered.contains("no tool call")
        || lowered.contains("just talk to me")
        || lowered.contains("just answer")
}

fn inline_tool_calls_allowed_for_user_message(message: &str) -> bool {
    let cleaned = clean_text(message, 2_200);
    if cleaned.is_empty() {
        return false;
    }
    if message_explicitly_disallows_tool_calls(&cleaned) {
        return false;
    }
    if direct_tool_intent_from_user_message(&cleaned).is_some() {
        return true;
    }
    let lowered = cleaned.to_ascii_lowercase();
    swarm_intent_requested(&cleaned)
        || lowered.contains("multi-agent")
        || lowered.contains("multi agent")
        || lowered.contains("use tool")
        || lowered.contains("run tool")
        || lowered.contains("call tool")
        || lowered.contains("execute tool")
        || lowered.contains("do a tool call")
        || lowered.contains("run a tool call")
}

