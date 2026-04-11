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

pub(crate) fn repair_and_validate_args(
    tool_name: &str,
    args: &Value,
) -> Result<Value, BrokerError> {
    let mut map = args
        .as_object()
        .cloned()
        .ok_or_else(|| BrokerError::InvalidArgs("args_must_be_object".to_string()))?;
    set_if_missing(&mut map, "query", "q");
    set_if_missing(&mut map, "query", "task");
    set_if_missing(&mut map, "query", "path");
    set_if_missing(&mut map, "path", "file_path");
    set_if_missing(&mut map, "paths", "sources");
    set_if_missing(&mut map, "url", "uri");
    set_if_missing(&mut map, "command", "cmd");
    set_if_missing(&mut map, "objective", "task");
    set_if_missing(&mut map, "objective", "message");
    set_if_missing(&mut map, "agent_id", "target_agent_id");
    set_if_missing(&mut map, "agent_id", "session_id");
    map.remove("q");
    map.remove("file_path");
    map.remove("sources");
    map.remove("uri");
    map.remove("cmd");
    map.remove("target_agent_id");
    let repaired = canonicalize_value(&Value::Object(map.clone()));
    let mut repaired_map = repaired.as_object().cloned().unwrap_or_default();
    match tool_name {
        "web_search" | "batch_query" => {
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
            let has_paths = repaired_map
                .get("paths")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false);
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
            let query = repaired_map
                .get("query")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 1200))
                .unwrap_or_default();
            if query.is_empty() {
                return Err(BrokerError::InvalidArgs("query_required".to_string()));
            }
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
            ));
        }
    }
    Ok(Value::Object(repaired_map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_accepts_governed_tooling_surface_contracts() {
        let folder = repair_and_validate_args("folder_export", &json!({"path":"notes"}))
            .expect("folder_export");
        let terminal = repair_and_validate_args("terminal_exec", &json!({"cmd":"ls -la"}))
            .expect("terminal_exec");
        let workspace = repair_and_validate_args(
            "workspace_analyze",
            &json!({"query":"inspect the workspace tree"}),
        )
        .expect("workspace_analyze");
        let spawn =
            repair_and_validate_args("spawn_subagents", &json!({"task":"parallelize tool audit"}))
                .expect("spawn_subagents");
        let manage = repair_and_validate_args(
            "manage_agent",
            &json!({"action":"message","session_id":"agent-7"}),
        )
        .expect("manage_agent");

        assert_eq!(folder.get("path").and_then(Value::as_str), Some("notes"));
        assert_eq!(
            terminal.get("command").and_then(Value::as_str),
            Some("ls -la")
        );
        assert_eq!(
            workspace.get("query").and_then(Value::as_str),
            Some("inspect the workspace tree")
        );
        assert_eq!(
            spawn.get("objective").and_then(Value::as_str),
            Some("parallelize tool audit")
        );
        assert_eq!(
            manage.get("agent_id").and_then(Value::as_str),
            Some("agent-7")
        );
    }

    #[test]
    fn file_read_many_normalizes_single_path_into_paths_array() {
        let normalized =
            repair_and_validate_args("file_read_many", &json!({"path":"docs/plan.md"}))
                .expect("file_read_many");
        assert_eq!(normalized.get("path"), None);
        assert_eq!(
            normalized
                .get("paths")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(Value::as_str),
            Some("docs/plan.md")
        );
    }
}
