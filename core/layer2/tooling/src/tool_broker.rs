use crate::schemas::{NormalizedToolMetrics, NormalizedToolResult, NormalizedToolStatus};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerCaller {
    Client,
    Worker,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallRequest {
    pub trace_id: String,
    pub task_id: String,
    pub tool_name: String,
    pub args: Value,
    pub lineage: Vec<String>,
    pub caller: BrokerCaller,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolBrokerExecution {
    pub normalized_result: NormalizedToolResult,
    pub raw_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrokerError {
    UnauthorizedToolRequest(String),
    InvalidArgs(String),
    ExecutionError(String),
    DirectToolBypassDenied(String),
}

impl BrokerError {
    pub fn as_message(&self) -> String {
        match self {
            Self::UnauthorizedToolRequest(v) => format!("unauthorized_tool_request:{v}"),
            Self::InvalidArgs(v) => format!("invalid_args:{v}"),
            Self::ExecutionError(v) => format!("execution_error:{v}"),
            Self::DirectToolBypassDenied(v) => format!("direct_tool_bypass_denied:{v}"),
        }
    }
}

pub struct ToolBroker {
    allowed_tools: HashMap<BrokerCaller, HashSet<String>>,
    dedupe_lookup: HashMap<String, String>,
    raw_payloads: HashMap<String, Value>,
}

impl Default for ToolBroker {
    fn default() -> Self {
        let mut allowed_tools = HashMap::<BrokerCaller, HashSet<String>>::new();
        let default_tools = [
            "web_search",
            "web_fetch",
            "batch_query",
            "file_read",
            "file_read_many",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<HashSet<_>>();
        allowed_tools.insert(BrokerCaller::Client, default_tools.clone());
        allowed_tools.insert(BrokerCaller::Worker, default_tools.clone());
        allowed_tools.insert(BrokerCaller::System, default_tools);
        Self {
            allowed_tools,
            dedupe_lookup: HashMap::new(),
            raw_payloads: HashMap::new(),
        }
    }
}

impl ToolBroker {
    pub fn allow_tool_for(&mut self, caller: BrokerCaller, tool_name: &str) {
        self.allowed_tools
            .entry(caller)
            .or_default()
            .insert(clean_text(tool_name, 120).to_ascii_lowercase());
    }

    pub fn direct_tool_bypass_attempt(&self, caller: BrokerCaller) -> Result<(), BrokerError> {
        let caller_label = match caller {
            BrokerCaller::Client => "client",
            BrokerCaller::Worker => "worker",
            BrokerCaller::System => "system",
        };
        Err(BrokerError::DirectToolBypassDenied(format!(
            "tool_broker_required_for_external_calls:{caller_label}"
        )))
    }

    pub fn execute_and_normalize<F>(
        &mut self,
        request: ToolCallRequest,
        executor: F,
    ) -> Result<ToolBrokerExecution, BrokerError>
    where
        F: FnOnce(&Value) -> Result<Value, String>,
    {
        let tool_name = clean_text(&request.tool_name, 120).to_ascii_lowercase();
        let allowed = self
            .allowed_tools
            .get(&request.caller)
            .map(|set| set.contains(&tool_name))
            .unwrap_or(false);
        if !allowed {
            return Err(BrokerError::UnauthorizedToolRequest(tool_name));
        }
        let normalized_args = repair_and_validate_args(&tool_name, &request.args)?;
        let dedupe_hash = deterministic_hash(&json!({
            "tool_name": tool_name,
            "normalized_args": normalized_args
        }));
        let started = now_ms();
        let execution = executor(&normalized_args);
        let duration_ms = now_ms().saturating_sub(started);
        let (status, raw_payload, errors) = match execution {
            Ok(raw_payload) => (NormalizedToolStatus::Ok, raw_payload, Vec::new()),
            Err(err) => (
                NormalizedToolStatus::Error,
                Value::Null,
                vec![clean_text(&err, 500)],
            ),
        };
        let status_tag = match status {
            NormalizedToolStatus::Ok => "ok",
            NormalizedToolStatus::Error => "error",
            NormalizedToolStatus::Blocked => "blocked",
        };
        let existing_result = self.dedupe_lookup.get(&dedupe_hash).cloned();
        let result_id = existing_result.unwrap_or_else(|| {
            deterministic_hash(&json!({
                "trace_id": request.trace_id,
                "task_id": request.task_id,
                "tool_name": tool_name,
                "status": status_tag,
                "ts": now_ms()
            }))
        });
        self.dedupe_lookup
            .entry(dedupe_hash.clone())
            .or_insert_with(|| result_id.clone());
        let raw_ref = format!("raw://{result_id}");
        self.raw_payloads
            .insert(raw_ref.clone(), raw_payload.clone());
        let metrics = NormalizedToolMetrics {
            duration_ms,
            output_bytes: serde_json::to_vec(&raw_payload)
                .map(|v| v.len())
                .unwrap_or(0),
        };
        let normalized_result = NormalizedToolResult {
            result_id,
            trace_id: clean_text(&request.trace_id, 160),
            task_id: clean_text(&request.task_id, 160),
            tool_name,
            status,
            normalized_args,
            dedupe_hash,
            lineage: sanitize_lineage(&request.lineage),
            timestamp: now_ms(),
            metrics,
            raw_ref,
            errors,
        };
        Ok(ToolBrokerExecution {
            normalized_result,
            raw_payload,
        })
    }

    pub fn raw_payload(&self, raw_ref: &str) -> Option<&Value> {
        self.raw_payloads.get(raw_ref)
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect::<String>()
}

fn sanitize_lineage(lineage: &[String]) -> Vec<String> {
    let mut rows = lineage
        .iter()
        .map(|v| clean_text(v, 200))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.push("tool_broker_v1".to_string());
    }
    rows
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

fn repair_and_validate_args(tool_name: &str, args: &Value) -> Result<Value, BrokerError> {
    let mut map = args
        .as_object()
        .cloned()
        .ok_or_else(|| BrokerError::InvalidArgs("args_must_be_object".to_string()))?;
    set_if_missing(&mut map, "query", "q");
    set_if_missing(&mut map, "path", "file_path");
    set_if_missing(&mut map, "paths", "sources");
    set_if_missing(&mut map, "url", "uri");
    map.remove("q");
    map.remove("file_path");
    map.remove("sources");
    map.remove("uri");
    let repaired = canonicalize_value(&Value::Object(map.clone()));
    let repaired_map = repaired.as_object().cloned().unwrap_or_default();
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
        "file_read" => {
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
            let has_single_path = repaired_map
                .get("path")
                .and_then(Value::as_str)
                .map(|v| !clean_text(v, 2000).is_empty())
                .unwrap_or(false);
            if !has_paths && !has_single_path {
                return Err(BrokerError::InvalidArgs("paths_required".to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_rejects_unauthorized_tool_request() {
        let mut broker = ToolBroker::default();
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace".to_string(),
                task_id: "task".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"echo hi"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert!(matches!(out, Err(BrokerError::UnauthorizedToolRequest(_))));
    }

    #[test]
    fn broker_argument_validation_normalizes_and_dedupes() {
        let mut broker = ToolBroker::default();
        let first = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"q":"  latency benchmarks  "}),
                    lineage: vec!["worker-1".to_string()],
                    caller: BrokerCaller::Worker,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let second = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-2".to_string(),
                    task_id: "task-2".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_eq!(
            first
                .normalized_result
                .normalized_args
                .get("query")
                .and_then(Value::as_str),
            Some("latency benchmarks")
        );
        assert_eq!(
            first.normalized_result.dedupe_hash,
            second.normalized_result.dedupe_hash
        );
    }

    #[test]
    fn direct_broker_bypass_is_impossible_for_all_callers() {
        let broker = ToolBroker::default();
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::Client),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::Worker),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
        assert!(matches!(
            broker.direct_tool_bypass_attempt(BrokerCaller::System),
            Err(BrokerError::DirectToolBypassDenied(_))
        ));
    }
}
