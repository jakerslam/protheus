use crate::tool_broker::BrokerError;
use serde_json::{json, Map, Value};

pub(crate) fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect::<String>()
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = std::collections::BTreeMap::<String, Value>::new();
            for (k, v) in map {
                sorted.insert(clean_text(k, 200), canonicalize_value(v));
            }
            let mut out = Map::<String, Value>::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalize_value).collect::<Vec<_>>()),
        Value::String(text) => Value::String(clean_text(text, 4000)),
        _ => value.clone(),
    }
}

fn set_if_missing(map: &mut Map<String, Value>, to: &str, from: &str) {
    if map.contains_key(to) {
        return;
    }
    if let Some(v) = map.get(from).cloned() {
        map.insert(to.to_string(), v);
    }
}

fn normalized_string_array(value: Option<&Value>, max_items: usize, max_len: usize) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let Some(value) = value else {
        return out;
    };
    match value {
        Value::Array(rows) => {
            for row in rows {
                let text = row
                    .as_str()
                    .map(|v| clean_text(v, max_len))
                    .unwrap_or_default();
                if !text.is_empty() {
                    out.push(text);
                }
                if out.len() >= max_items {
                    break;
                }
            }
        }
        Value::String(row) => {
            for token in split_path_candidates(row, max_items, max_len) {
                if !token.is_empty() {
                    out.push(token);
                }
                if out.len() >= max_items {
                    break;
                }
            }
        }
        _ => {}
    }
    out
}

fn split_path_candidates(raw: &str, max_items: usize, max_len: usize) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let separators = [',', '\n', ';', '\t'];
    if separators.iter().all(|sep| !trimmed.contains(*sep)) {
        return vec![clean_text(trimmed, max_len)];
    }
    trimmed
        .split(|ch| separators.contains(&ch))
        .map(|row| row.trim().trim_matches('`').trim_matches('"'))
        .filter(|row| !row.is_empty())
        .map(|row| clean_text(row, max_len))
        .take(max_items)
        .collect::<Vec<_>>()
}

fn route_hint_from_workspace_query(query: &str) -> &'static str {
    let lowered = query.to_ascii_lowercase();
    if lowered.contains("git") || lowered.contains("worktree") || lowered.contains("branch") {
        return "workspace_git";
    }
    if lowered.contains("powershell") || lowered.contains("terminal") || lowered.contains("shell") {
        return "terminal_exec";
    }
    if lowered.contains("cost") || lowered.contains("price") || lowered.contains("token") {
        return "synthesis_cost";
    }
    if lowered.contains("env") || lowered.contains("environment") || lowered.contains("variable") {
        return "workspace_env";
    }
    "workspace_general"
}

fn route_hint_from_workspace_shape(query: &str, command: &str) -> &'static str {
    if !command.trim().is_empty() {
        return "terminal_exec";
    }
    route_hint_from_workspace_query(query)
}

pub fn repair_and_validate_args(tool_name: &str, args: &Value) -> Result<Value, BrokerError> {
    let mut map = args
        .as_object()
        .cloned()
        .ok_or_else(|| BrokerError::InvalidArgs("args_must_be_object".to_string()))?;
    if matches!(tool_name, "web_search" | "batch_query") {
        if let Some(nested_payload) = map.get("payload").and_then(Value::as_object).cloned() {
            for key in [
                "query",
                "queries",
                "aperture",
                "source",
                "context",
                "keywords",
                "required_coverage",
                "aliases",
                "negative_terms",
                "query_metadata_policy",
            ] {
                if !map.contains_key(key) {
                    if let Some(value) = nested_payload.get(key) {
                        map.insert(key.to_string(), value.clone());
                    }
                }
            }
        }
    }
    set_if_missing(&mut map, "query", "q");
    set_if_missing(&mut map, "query", "task");
    if tool_name != "workspace_analyze" {
        set_if_missing(&mut map, "query", "path");
        set_if_missing(&mut map, "query", "pattern");
        set_if_missing(&mut map, "query", "needle");
        set_if_missing(&mut map, "query", "search");
        set_if_missing(&mut map, "query", "text");
    }
    set_if_missing(&mut map, "query", "workspace_query");
    set_if_missing(&mut map, "query", "workspace_context");
    set_if_missing(&mut map, "query", "context");
    set_if_missing(&mut map, "query", "goal");
    set_if_missing(&mut map, "path", "file_path");
    set_if_missing(&mut map, "path", "workspace_path");
    set_if_missing(&mut map, "path", "repo_path");
    set_if_missing(&mut map, "path", "repository_path");
    set_if_missing(&mut map, "path", "file");
    set_if_missing(&mut map, "path", "cwd");
    set_if_missing(&mut map, "path", "working_directory");
    set_if_missing(&mut map, "paths", "sources");
    set_if_missing(&mut map, "paths", "files");
    set_if_missing(&mut map, "url", "uri");
    set_if_missing(&mut map, "url", "original_url");
    set_if_missing(&mut map, "url", "repository_url");
    set_if_missing(&mut map, "url", "repo_url");
    set_if_missing(&mut map, "command", "cmd");
    set_if_missing(&mut map, "command", "shell_command");
    set_if_missing(&mut map, "command", "powershell");
    set_if_missing(&mut map, "command", "command_line");
    set_if_missing(&mut map, "objective", "task");
    set_if_missing(&mut map, "objective", "message");
    set_if_missing(&mut map, "objective", "prompt");
    set_if_missing(&mut map, "agent_id", "target_agent_id");
    set_if_missing(&mut map, "agent_id", "session_id");
    for alias in [
        "q",
        "file_path",
        "workspace_path",
        "repo_path",
        "repository_path",
        "sources",
        "files",
        "file",
        "cwd",
        "working_directory",
        "uri",
        "original_url",
        "repository_url",
        "repo_url",
        "cmd",
        "shell_command",
        "powershell",
        "command_line",
        "prompt",
        "target_agent_id",
    ] {
        map.remove(alias);
    }
    let repaired = canonicalize_value(&Value::Object(map.clone()));
    let mut repaired_map = repaired.as_object().cloned().unwrap_or_default();
    match tool_name {
        "web_search" | "batch_query" => {
            if repaired_map
                .get("query")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                if let Some(query) = normalized_string_array(repaired_map.get("queries"), 6, 1200)
                    .into_iter()
                    .find(|query| !query.is_empty())
                {
                    repaired_map.insert("query".to_string(), Value::String(query));
                }
            }
            if repaired_map
                .get("aperture")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                repaired_map.insert("aperture".to_string(), json!("medium"));
            }
            let query = repaired_map
                .get("query")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 1200))
                .unwrap_or_default();
            if query.is_empty() {
                return Err(BrokerError::InvalidArgs("query_required".to_string()));
            }
        }
        "web_fetch" => {
            let url = repaired_map
                .get("url")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 2000))
                .unwrap_or_default();
            if url.is_empty() {
                return Err(BrokerError::InvalidArgs("url_required".to_string()));
            }
        }
        "file_read" | "folder_export" => {
            let path = repaired_map
                .get("path")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 2000))
                .unwrap_or_default();
            if path.is_empty() {
                return Err(BrokerError::InvalidArgs("path_required".to_string()));
            }
        }
        "file_read_many" => {
            let mut has_paths = repaired_map
                .get("paths")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false);
            if !has_paths {
                let path_from_string = repaired_map
                    .get("paths")
                    .and_then(Value::as_str)
                    .map(|v| split_path_candidates(v, 12, 2000))
                    .unwrap_or_default();
                if !path_from_string.is_empty() {
                    repaired_map.insert("paths".to_string(), json!(path_from_string));
                    has_paths = true;
                }
            }
            let single_path = repaired_map
                .get("path")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 2000))
                .unwrap_or_default();
            if !has_paths && single_path.is_empty() {
                return Err(BrokerError::InvalidArgs("paths_required".to_string()));
            }
            if !has_paths && !single_path.is_empty() {
                repaired_map.insert("paths".to_string(), json!([single_path]));
                repaired_map.remove("path");
            }
        }
        "terminal_exec" => {
            let command = repaired_map
                .get("command")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 4000))
                .unwrap_or_default();
            if command.is_empty() {
                return Err(BrokerError::InvalidArgs("command_required".to_string()));
            }
        }
        "workspace_analyze" => {
            let mut query = repaired_map
                .get("query")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 1200))
                .unwrap_or_default();
            if query.is_empty() {
                query = repaired_map
                    .get("query")
                    .filter(|row| row.is_array() || row.is_object())
                    .map(|row| clean_text(&row.to_string(), 1200))
                    .unwrap_or_default();
                if !query.is_empty() {
                    repaired_map.insert("query".to_string(), Value::String(query.clone()));
                }
            }
            let context_mentions = normalized_string_array(
                repaired_map
                    .get("context_mentions")
                    .or_else(|| repaired_map.get("mentions"))
                    .or_else(|| repaired_map.get("references")),
                4,
                300,
            );
            if !context_mentions.is_empty() {
                repaired_map.insert(
                    "context_mentions".to_string(),
                    json!(context_mentions.clone()),
                );
            }
            let path_rows = normalized_string_array(repaired_map.get("paths"), 6, 1200);
            if !path_rows.is_empty() {
                repaired_map.insert("paths".to_string(), json!(path_rows.clone()));
            }
            let pattern_hint = repaired_map
                .get("pattern")
                .or_else(|| repaired_map.get("needle"))
                .or_else(|| repaired_map.get("text"))
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 400))
                .unwrap_or_default();
            let command_hint = repaired_map
                .get("command")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 600))
                .unwrap_or_default();
            if query.is_empty() {
                let path = repaired_map
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|v| clean_text(v, 1200))
                    .unwrap_or_default();
                let path_scope = if !path_rows.is_empty() {
                    path_rows.join(", ")
                } else {
                    path.clone()
                };
                let url = repaired_map
                    .get("url")
                    .and_then(Value::as_str)
                    .map(|v| clean_text(v, 1200))
                    .unwrap_or_default();
                query = match (
                    pattern_hint.is_empty(),
                    path_scope.is_empty(),
                    context_mentions.is_empty(),
                ) {
                    (false, false, _) => format!("search `{pattern_hint}` in `{path_scope}`"),
                    (false, true, _) => pattern_hint.clone(),
                    (true, false, _) => format!("inspect `{path_scope}`"),
                    (true, true, false) => {
                        format!("synthesize context from {}", context_mentions.join(", "))
                    }
                    (true, true, true) => String::new(),
                };
                if query.is_empty() && !command_hint.is_empty() {
                    query = format!("analyze command behavior `{command_hint}`");
                }
                if query.is_empty() && !url.is_empty() {
                    query = format!("inspect repository source `{url}`");
                }
                if !query.is_empty() {
                    repaired_map.insert("query".to_string(), Value::String(query.clone()));
                }
            }
            if query.is_empty() {
                return Err(BrokerError::InvalidArgs("query_required".to_string()));
            }
            repaired_map.insert(
                "route_hint".to_string(),
                Value::String(route_hint_from_workspace_shape(&query, &command_hint).to_string()),
            );
        }
        "spawn_subagents" => {
            let objective = repaired_map
                .get("objective")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 1200))
                .unwrap_or_default();
            if objective.is_empty() {
                return Err(BrokerError::InvalidArgs("objective_required".to_string()));
            }
        }
        "manage_agent" => {
            let action = repaired_map
                .get("action")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let agent_id = repaired_map
                .get("agent_id")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 200))
                .unwrap_or_default();
            if action.is_empty() {
                return Err(BrokerError::InvalidArgs("action_required".to_string()));
            }
            if agent_id.is_empty() {
                return Err(BrokerError::InvalidArgs("agent_id_required".to_string()));
            }
        }
        _ => {
            return Err(BrokerError::InvalidArgs(
                "unsupported_tool_name".to_string(),
            ))
        }
    }
    Ok(Value::Object(repaired_map))
}
