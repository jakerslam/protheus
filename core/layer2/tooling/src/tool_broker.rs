use crate::schemas::{NormalizedToolMetrics, NormalizedToolResult, NormalizedToolStatus};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

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
    pub policy_revision: Option<String>,
    pub tool_version: Option<String>,
    pub freshness_window_ms: Option<u64>,
    pub force_no_dedupe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolBrokerExecution {
    pub normalized_result: NormalizedToolResult,
    pub raw_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolExecutionLedgerEvent {
    pub event_id: String,
    pub event_sequence: u64,
    pub result_id: String,
    pub result_content_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub caller: BrokerCaller,
    pub tool_name: String,
    pub status: NormalizedToolStatus,
    pub dedupe_hash: String,
    pub policy_revision: String,
    pub tool_version: String,
    pub freshness_window_ms: u64,
    pub freshness_bucket: u64,
    pub raw_ref: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrokerError {
    UnauthorizedToolRequest(String),
    InvalidArgs(String),
    ExecutionError(String),
    DirectToolBypassDenied(String),
    LedgerWriteFailed(String),
}

impl BrokerError {
    pub fn as_message(&self) -> String {
        match self {
            Self::UnauthorizedToolRequest(v) => format!("unauthorized_tool_request:{v}"),
            Self::InvalidArgs(v) => format!("invalid_args:{v}"),
            Self::ExecutionError(v) => format!("execution_error:{v}"),
            Self::DirectToolBypassDenied(v) => format!("direct_tool_bypass_denied:{v}"),
            Self::LedgerWriteFailed(v) => format!("ledger_write_failed:{v}"),
        }
    }
}

pub struct ToolBroker {
    allowed_tools: HashMap<BrokerCaller, HashSet<String>>,
    dedupe_lookup: HashMap<String, String>,
    raw_payloads: HashMap<String, Value>,
    event_sequence: u64,
    ledger_events: Vec<ToolExecutionLedgerEvent>,
    ledger_path: PathBuf,
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
        let mut out = Self {
            allowed_tools,
            dedupe_lookup: HashMap::new(),
            raw_payloads: HashMap::new(),
            event_sequence: 0,
            ledger_events: Vec::new(),
            ledger_path: default_ledger_path(),
        };
        let _ = out.recover_from_ledger();
        out
    }
}

impl ToolBroker {
    fn ledger_error<E: std::fmt::Display>(&self, context: &str, err: E) -> BrokerError {
        BrokerError::LedgerWriteFailed(format!("{context}:{}:{err}", self.ledger_path.display()))
    }

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
        let event_ts = now_ms();
        let normalized_args = repair_and_validate_args(&tool_name, &request.args)?;
        let policy_revision = clean_text(
            request.policy_revision.as_deref().unwrap_or("policy_v1"),
            120,
        );
        let tool_version = clean_text(request.tool_version.as_deref().unwrap_or("tool_v1"), 120);
        let freshness_window_ms =
            dedupe_freshness_window_ms(&tool_name, request.freshness_window_ms);
        let freshness_bucket = if freshness_window_ms == 0 {
            0
        } else {
            event_ts / freshness_window_ms
        };
        let dedupe_hash = deterministic_hash(&json!({
            "tool_name": tool_name,
            "normalized_args": normalized_args,
            "policy_revision": policy_revision,
            "tool_version": tool_version,
            "freshness_window_ms": freshness_window_ms,
            "freshness_bucket": freshness_bucket,
        }));
        let started = event_ts;
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
        let content_fingerprint = deterministic_hash(&json!({
            "kind": "normalized_tool_result_content",
            "tool_name": tool_name,
            "normalized_args": normalized_args,
            "status": status_tag,
            "raw_payload": raw_payload,
            "errors": errors,
            "policy_revision": policy_revision,
            "tool_version": tool_version
        }));
        let result_content_id = content_fingerprint.clone();
        let dedupe_allowed = matches!(status, NormalizedToolStatus::Ok) && !request.force_no_dedupe;
        let existing_result = if dedupe_allowed {
            self.dedupe_lookup.get(&dedupe_hash).cloned()
        } else {
            None
        };
        let result_id = existing_result.unwrap_or_else(|| result_content_id.clone());
        if dedupe_allowed {
            self.dedupe_lookup
                .entry(dedupe_hash.clone())
                .or_insert_with(|| result_id.clone());
        }
        self.event_sequence = self.event_sequence.saturating_add(1);
        let event_sequence = self.event_sequence;
        let event_id = deterministic_hash(&json!({
            "kind": "tool_execution_event",
            "trace_id": request.trace_id,
            "task_id": request.task_id,
            "caller": format!("{:?}", request.caller),
            "event_ts": event_ts,
            "event_sequence": event_sequence
        }));
        let raw_ref = format!("raw://{result_id}/{event_id}");
        self.raw_payloads
            .insert(raw_ref.clone(), raw_payload.clone());
        let metrics = NormalizedToolMetrics {
            duration_ms,
            output_bytes: serde_json::to_vec(&raw_payload)
                .map(|v| v.len())
                .unwrap_or(0),
        };
        let mut lineage = sanitize_lineage(&request.lineage);
        lineage.push(format!("policy_revision:{policy_revision}"));
        lineage.push(format!("tool_version:{tool_version}"));
        if freshness_window_ms > 0 {
            lineage.push(format!("freshness_window_ms:{freshness_window_ms}"));
            lineage.push(format!("freshness_bucket:{freshness_bucket}"));
        }
        lineage.push(format!("broker_event:{event_id}"));
        let normalized_result = NormalizedToolResult {
            result_id,
            result_content_id,
            result_event_id: event_id.clone(),
            trace_id: clean_text(&request.trace_id, 160),
            task_id: clean_text(&request.task_id, 160),
            tool_name,
            status,
            normalized_args,
            dedupe_hash,
            lineage,
            timestamp: event_ts,
            metrics,
            raw_ref,
            errors,
        };
        let ledger_event = ToolExecutionLedgerEvent {
            event_id,
            event_sequence,
            result_id: normalized_result.result_id.clone(),
            result_content_id: normalized_result.result_content_id.clone(),
            trace_id: normalized_result.trace_id.clone(),
            task_id: normalized_result.task_id.clone(),
            caller: request.caller,
            tool_name: normalized_result.tool_name.clone(),
            status: normalized_result.status.clone(),
            dedupe_hash: normalized_result.dedupe_hash.clone(),
            policy_revision,
            tool_version,
            freshness_window_ms,
            freshness_bucket,
            raw_ref: normalized_result.raw_ref.clone(),
            timestamp: normalized_result.timestamp,
        };
        self.persist_ledger_event(&ledger_event)?;
        self.ledger_events.push(ledger_event);
        Ok(ToolBrokerExecution {
            normalized_result,
            raw_payload,
        })
    }

    pub fn raw_payload(&self, raw_ref: &str) -> Option<&Value> {
        self.raw_payloads.get(raw_ref)
    }

    pub fn ledger_events(&self) -> &[ToolExecutionLedgerEvent] {
        self.ledger_events.as_slice()
    }

    pub fn ledger_path(&self) -> &PathBuf {
        &self.ledger_path
    }

    pub fn recover_from_ledger(&mut self) -> Result<usize, BrokerError> {
        if !self.ledger_path.exists() {
            return Ok(0);
        }
        let file = File::open(&self.ledger_path)
            .map_err(|err| self.ledger_error("open_for_recovery", err))?;
        self.dedupe_lookup.clear();
        self.ledger_events.clear();
        self.event_sequence = 0;
        let mut recovered = 0usize;
        for line in BufReader::new(file).lines() {
            let row = line.map_err(|err| self.ledger_error("read_recovery_line", err))?;
            let trimmed = row.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event = serde_json::from_str::<ToolExecutionLedgerEvent>(trimmed)
                .map_err(|err| self.ledger_error("decode_recovery_line", err))?;
            self.event_sequence = self.event_sequence.max(event.event_sequence);
            self.dedupe_lookup
                .entry(event.dedupe_hash.clone())
                .or_insert_with(|| event.result_id.clone());
            self.ledger_events.push(event);
            recovered = recovered.saturating_add(1);
        }
        Ok(recovered)
    }

    fn persist_ledger_event(&self, event: &ToolExecutionLedgerEvent) -> Result<(), BrokerError> {
        if let Some(parent) = self.ledger_path.parent() {
            create_dir_all(parent).map_err(|err| self.ledger_error("create_dir", err))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|err| self.ledger_error("open", err))?;
        let row = serde_json::to_string(event)
            .map_err(|err| BrokerError::LedgerWriteFailed(format!("encode_event:{err}")))?;
        file.write_all(format!("{row}\n").as_bytes())
            .map_err(|err| self.ledger_error("append", err))?;
        Ok(())
    }
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect::<String>()
}

fn dedupe_freshness_window_ms(tool_name: &str, requested: Option<u64>) -> u64 {
    let default = if matches!(tool_name, "web_search" | "web_fetch" | "batch_query") {
        30_000
    } else {
        0
    };
    requested.unwrap_or(default).min(86_400_000)
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

#[cfg(test)]
fn default_ledger_path() -> PathBuf {
    if let Some(path) = std::env::var("INFRING_TOOL_BROKER_LEDGER_PATH")
        .ok()
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return path;
    }
    std::env::temp_dir().join(format!(
        "infring_tool_broker_test_{}_{}.jsonl",
        std::process::id(),
        now_ms()
    ))
}

#[cfg(not(test))]
fn default_ledger_path() -> PathBuf {
    if let Some(path) = std::env::var("INFRING_TOOL_BROKER_LEDGER_PATH")
        .ok()
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return path;
    }
    if let Some(root) = std::env::var("INFRING_ROOT")
        .ok()
        .or_else(|| std::env::var("PROTHEUS_ROOT").ok())
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return root
            .join("core")
            .join("local")
            .join("state")
            .join("tooling")
            .join("tool_broker_events.jsonl");
    }
    std::env::temp_dir()
        .join("infring")
        .join("tool_broker_events.jsonl")
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
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
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
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
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
                    policy_revision: None,
                    tool_version: None,
                    freshness_window_ms: None,
                    force_no_dedupe: false,
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
        assert_eq!(
            first.normalized_result.result_id,
            second.normalized_result.result_id
        );
    }

    #[test]
    fn broker_dedupe_hash_changes_when_policy_revision_changes() {
        let mut broker = ToolBroker::default();
        let first = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let second = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace".to_string(),
                    task_id: "task".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"latency benchmarks"}),
                    lineage: vec![],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.v2".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_ne!(
            first.normalized_result.dedupe_hash,
            second.normalized_result.dedupe_hash
        );
        assert_ne!(
            first.normalized_result.result_id,
            second.normalized_result.result_id
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

    #[test]
    fn broker_writes_append_only_ledger_events() {
        let mut broker = ToolBroker::default();
        let execution = broker
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-ledger".to_string(),
                    task_id: "task-ledger".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"ledger event"}),
                    lineage: vec!["test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.ledger.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("execute");
        assert!(!broker.ledger_events().is_empty());
        let last = broker.ledger_events().last().expect("last event");
        assert_eq!(last.trace_id, "trace-ledger");
        assert_eq!(last.task_id, "task-ledger");
        assert_eq!(last.result_id, execution.normalized_result.result_id);
        assert_eq!(
            last.result_content_id,
            execution.normalized_result.result_content_id
        );
        assert_eq!(last.event_id, execution.normalized_result.result_event_id);
        assert!(broker.ledger_path().exists());
    }

    #[test]
    fn broker_can_recover_dedupe_state_from_ledger() {
        let ledger_path =
            std::env::temp_dir().join(format!("infring_tool_broker_recover_{}.jsonl", now_ms()));
        let mut writer = ToolBroker::default();
        writer.ledger_path = ledger_path.clone();
        let first = writer
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-recover-1".to_string(),
                    task_id: "task-recover-1".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"recoverable dedupe"}),
                    lineage: vec!["recover-test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.recover.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("first");
        let first_result_id = first.normalized_result.result_id;
        let mut recovered = ToolBroker::default();
        recovered.ledger_path = ledger_path.clone();
        let recovered_count = recovered.recover_from_ledger().expect("recover");
        assert!(recovered_count >= 1);
        let second = recovered
            .execute_and_normalize(
                ToolCallRequest {
                    trace_id: "trace-recover-2".to_string(),
                    task_id: "task-recover-2".to_string(),
                    tool_name: "web_search".to_string(),
                    args: json!({"query":"recoverable dedupe"}),
                    lineage: vec!["recover-test".to_string()],
                    caller: BrokerCaller::Worker,
                    policy_revision: Some("policy.recover.v1".to_string()),
                    tool_version: Some("web_search.v1".to_string()),
                    freshness_window_ms: Some(60_000),
                    force_no_dedupe: false,
                },
                |_| Ok(json!({"results":[{"summary":"ok"}]})),
            )
            .expect("second");
        assert_eq!(second.normalized_result.result_id, first_result_id);
        let _ = std::fs::remove_file(&ledger_path);
    }
}
