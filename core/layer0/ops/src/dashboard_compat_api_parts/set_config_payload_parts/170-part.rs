fn explicit_tool_command_error(
    command: &str,
    error: &str,
    message: &str,
    suggestion: Option<&str>,
) -> Value {
    json!({
        "ok": false,
        "error": clean_text(error, 80),
        "command": clean_text(command, 120),
        "message": clean_text(message, 320),
        "suggestion": suggestion.unwrap_or(""),
        "supported_commands": EXPLICIT_SUPPORTED_TOOL_COMMANDS
    })
}

fn parse_explicit_tool_command_from_message(message: &str) -> Option<Result<(String, Value), Value>> {
    let mut trimmed = message.trim().to_string();
    if trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() > 2 {
        trimmed = trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    let lowered = trimmed.to_ascii_lowercase();
    if !lowered.starts_with("tool::") {
        return None;
    }
    let malformed = || Some(Err(explicit_tool_command_error("", "tool_command_name_invalid", "Malformed command. Use `tool::<command>` or `tool::<command>:::<params>`.", None)));
    let command_payload = &trimmed["tool::".len()..];
    let (raw_command, raw_params) = if let Some((name, params)) = command_payload.split_once(":::")
    {
        let name = name.trim();
        if name.is_empty() || name.contains(':') {
            return malformed();
        }
        (name, params.trim())
    } else {
        if command_payload.contains("::") {
            return malformed();
        }
        (command_payload.trim(), "")
    };
    let command = clean_text(raw_command, 80)
        .to_ascii_lowercase()
        .replace('-', "_");
    if command.is_empty() || !command.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_') {
        return Some(Err(explicit_tool_command_error(
            &command,
            "tool_command_name_invalid",
            "Malformed command. Use `tool::<command>` or `tool::<command>:::<params>`.",
            None,
        )));
    }
    if !EXPLICIT_SUPPORTED_TOOL_COMMANDS.iter().any(|value| *value == command.as_str()) {
        let suggestion = closest_supported_tool_command(&command);
        let hint = if let Some(value) = suggestion {
            format!("Unsupported `tool::{command}`. Try `tool::{value}`.")
        } else {
            format!("Unsupported `tool::{command}` command.")
        };
        return Some(Err(explicit_tool_command_error(
            &command,
            "unsupported_tool_command",
            &hint,
            suggestion,
        )));
    }
    let mapped = command.as_str();
    let parsed_params = if raw_params.is_empty() {
        None
    } else {
        serde_json::from_str::<Value>(raw_params).ok()
    };
    let parsed_object = parsed_params.as_ref().and_then(Value::as_object);
    let mut out_tool = mapped.to_string();
    let mut out_input = json!({});

    match mapped {
        "capabilities" => {
            out_tool = "tool_capabilities".to_string();
            out_input = if let Some(obj) = parsed_object {
                Value::Object(obj.clone())
            } else {
                json!({"scope": "agent"})
            };
            if out_input
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                out_input["scope"] = json!("agent");
            }
        }
        "web_search" | "batch_query" => {
            let query = clean_text(
                parsed_object
                    .and_then(|obj| obj.get("query").or_else(|| obj.get("q")))
                    .and_then(Value::as_str)
                    .unwrap_or(if parsed_params.is_none() { raw_params } else { "" }),
                600,
            );
            if query.is_empty() {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_query_required",
                    "`web_search` and `batch_query` require a query string.",
                    None,
                )));
            }
            out_tool = if mapped == "web_search" {
                "web_search".to_string()
            } else {
                "batch_query".to_string()
            };
            out_input = if let Some(obj) = parsed_object {
                Value::Object(obj.clone())
            } else {
                json!({"query": query})
            };
            out_input["query"] = json!(query);
            if out_input
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                out_input["source"] = json!("web");
            }
            if out_input
                .get("aperture")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                out_input["aperture"] = json!("medium");
            }
        }
        "web_fetch" => {
            let url = clean_text(
                parsed_object
                    .and_then(|obj| obj.get("url").or_else(|| obj.get("link")))
                    .and_then(Value::as_str)
                    .unwrap_or(if parsed_params.is_none() { raw_params } else { "" }),
                2200,
            );
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_url_required",
                    "`web_fetch` requires an absolute http(s) URL.",
                    None,
                )));
            }
            out_tool = "web_fetch".to_string();
            out_input = if let Some(obj) = parsed_object {
                Value::Object(obj.clone())
            } else {
                json!({"url": url})
            };
            out_input["url"] = json!(url);
            if out_input.get("summary_only").is_none() {
                out_input["summary_only"] = json!(true);
            }
        }
        "spawn_subagents" => {
            let mut count = parsed_object
                .and_then(|obj| obj.get("count"))
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or(3)
                .clamp(1, 8);
            let mut objective = clean_text(
                parsed_object
                    .and_then(|obj| {
                        obj.get("objective")
                            .or_else(|| obj.get("task"))
                            .or_else(|| obj.get("message"))
                    })
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                800,
            );
            if objective.is_empty() {
                let mut tokens = raw_params.splitn(2, char::is_whitespace);
                if let Some(first) = tokens.next() {
                    if let Ok(parsed_count) = first.trim().parse::<usize>() {
                        count = parsed_count.clamp(1, 8);
                        objective = clean_text(tokens.next().unwrap_or(""), 800);
                    } else {
                        objective = clean_text(raw_params, 800);
                    }
                }
            }
            if objective.is_empty() {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_objective_required",
                    "`spawn_subagents` requires an objective.",
                    None,
                )));
            }
            out_tool = "spawn_subagents".to_string();
            out_input = json!({
                "count": count,
                "objective": objective,
                "confirm": true,
                "approval_note": "explicit tool command"
            });
        }
        "manage_agent" => {
            let Some(obj) = parsed_object else {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_params_required",
                    "`manage_agent` requires JSON params like {\"action\":\"message\",\"agent_id\":\"...\",\"message\":\"...\"}.",
                    None,
                )));
            };
            let action = clean_text(
                obj.get("action").and_then(Value::as_str).unwrap_or(""),
                80,
            )
            .to_ascii_lowercase();
            if action.is_empty() {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_action_required",
                    "`manage_agent` requires an `action` field.",
                    None,
                )));
            }
            out_tool = "manage_agent".to_string();
            out_input = Value::Object(obj.clone());
            out_input["action"] = json!(action);
        }
        "memory_store" => {
            let (key, value) = if let Some(obj) = parsed_object {
                let key = clean_text(obj.get("key").and_then(Value::as_str).unwrap_or(""), 180);
                let value = obj.get("value").cloned().unwrap_or(Value::Null);
                (key, value)
            } else if let Some((left, right)) = raw_params.split_once('=') {
                (clean_text(left, 180), json!(clean_text(right, 4_000)))
            } else {
                (String::new(), Value::Null)
            };
            if key.is_empty() {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_key_required",
                    "`memory_store` requires a key and value (e.g. tool::memory_store:::my.key=value).",
                    None,
                )));
            }
            out_tool = "memory_kv_set".to_string();
            out_input = json!({"key": key, "value": value, "confirm": true});
        }
        "memory_retrieve" => {
            if let Some(obj) = parsed_object {
                let key = clean_text(obj.get("key").and_then(Value::as_str).unwrap_or(""), 180);
                if !key.is_empty() {
                    out_tool = "memory_kv_get".to_string();
                    out_input = json!({"key": key});
                    return Some(Ok((out_tool, out_input)));
                }
            }
            let query = clean_text(
                parsed_object
                    .and_then(|obj| obj.get("query").or_else(|| obj.get("q")))
                    .and_then(Value::as_str)
                    .unwrap_or(if parsed_params.is_none() { raw_params } else { "" }),
                600,
            );
            if query.is_empty() {
                return Some(Err(explicit_tool_command_error(
                    mapped,
                    "tool_command_query_required",
                    "`memory_retrieve` requires a query or key.",
                    None,
                )));
            }
            out_tool = "memory_semantic_query".to_string();
            out_input = json!({"query": query, "limit": 8});
        }
        "workspace_analyze" => {
            let query = clean_text(
                parsed_object
                    .and_then(|obj| obj.get("query").or_else(|| obj.get("task")))
                    .and_then(Value::as_str)
                    .unwrap_or(if parsed_params.is_none() { raw_params } else { "" }),
                600,
            );
            out_tool = "workspace_analyze".to_string();
            out_input = if let Some(obj) = parsed_object {
                Value::Object(obj.clone())
            } else {
                json!({"query": if query.is_empty() { "workspace status" } else { query.as_str() }})
            };
            if out_input
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
            {
                out_input["query"] = json!(if query.is_empty() {
                    "workspace status"
                } else {
                    query.as_str()
                });
            }
        }
        _ => {}
    }
    Some(Ok((out_tool, out_input)))
}
