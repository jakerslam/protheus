use crate::backend_registry::{live_backend_registry, ToolBackendHealth};
use crate::capability::{
    all_capabilities_for_callers, capability_probe_for, ToolCapability, ToolCapabilityProbe,
    ToolReasonCode,
};
use crate::request_validation::{clean_text, repair_and_validate_args};
use crate::schemas::{NormalizedToolMetrics, NormalizedToolResult, NormalizedToolStatus};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    pub attempt: ToolAttemptEnvelope,
    pub normalized_result: NormalizedToolResult,
    pub raw_payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolAttemptStatus {
    Ok,
    Unavailable,
    Blocked,
    InvalidArgs,
    ExecutionError,
    TransportError,
    Timeout,
    PolicyDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolAttemptEnvelope {
    pub attempt: ToolAttemptReceipt,
    pub normalized_result: Option<NormalizedToolResult>,
    pub raw_payload: Option<Value>,
    pub error: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolAttemptReceipt {
    pub attempt_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub caller: BrokerCaller,
    pub tool_name: String,
    pub status: ToolAttemptStatus,
    pub outcome: String,
    pub reason_code: ToolReasonCode,
    pub reason: String,
    pub latency_ms: u64,
    pub required_args: Vec<String>,
    pub backend: String,
    pub discoverable: bool,
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
    attempt_receipts: Vec<ToolAttemptReceipt>,
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
            "folder_export",
            "manage_agent",
            "spawn_subagents",
            "terminal_exec",
            "workspace_analyze",
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
            attempt_receipts: Vec::new(),
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

    pub fn capability_catalog(&self) -> Vec<ToolCapability> {
        all_capabilities_for_callers(&self.allowed_tools)
    }

    pub fn backend_registry(&self) -> Vec<ToolBackendHealth> {
        live_backend_registry()
    }

    pub fn capability_probe(&self, caller: BrokerCaller, tool_name: &str) -> ToolCapabilityProbe {
        capability_probe_for(
            &self.allowed_tools,
            caller,
            clean_text(tool_name, 120).as_str(),
        )
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

    pub fn attempt_receipts(&self) -> &[ToolAttemptReceipt] {
        self.attempt_receipts.as_slice()
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
        let event_ts = now_ms();
        let probe = self.capability_probe(request.caller, &tool_name);
        if !probe.available {
            let attempt_status = match probe.reason_code {
                ToolReasonCode::UnknownTool | ToolReasonCode::TransportUnavailable => {
                    ToolAttemptStatus::Unavailable
                }
                ToolReasonCode::DaemonUnavailable | ToolReasonCode::WebsocketUnavailable => {
                    ToolAttemptStatus::Unavailable
                }
                ToolReasonCode::CallerNotAuthorized | ToolReasonCode::PolicyDenied => {
                    ToolAttemptStatus::Blocked
                }
                ToolReasonCode::AuthRequired => ToolAttemptStatus::Blocked,
                ToolReasonCode::BackendDegraded => ToolAttemptStatus::TransportError,
                ToolReasonCode::InvalidArgs => ToolAttemptStatus::InvalidArgs,
                ToolReasonCode::Timeout => ToolAttemptStatus::Timeout,
                ToolReasonCode::ExecutionError => ToolAttemptStatus::ExecutionError,
                ToolReasonCode::Ok => ToolAttemptStatus::Ok,
            };
            self.record_attempt_receipt(
                request.trace_id.as_str(),
                request.task_id.as_str(),
                request.caller,
                tool_name.as_str(),
                attempt_status,
                probe.reason.as_str(),
                probe.reason_code,
                event_ts,
                0,
                &probe,
            );
            return Err(BrokerError::UnauthorizedToolRequest(tool_name));
        }
        let normalized_args = match repair_and_validate_args(&tool_name, &request.args) {
            Ok(v) => v,
            Err(err) => {
                self.record_attempt_receipt(
                    request.trace_id.as_str(),
                    request.task_id.as_str(),
                    request.caller,
                    tool_name.as_str(),
                    ToolAttemptStatus::InvalidArgs,
                    "invalid_args",
                    ToolReasonCode::InvalidArgs,
                    event_ts,
                    0,
                    &probe,
                );
                return Err(err);
            }
        };
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
        let (attempt_status, reason_code) = match status {
            NormalizedToolStatus::Ok => (ToolAttemptStatus::Ok, ToolReasonCode::Ok),
            NormalizedToolStatus::Blocked => {
                (ToolAttemptStatus::Blocked, ToolReasonCode::PolicyDenied)
            }
            NormalizedToolStatus::Error => (
                ToolAttemptStatus::ExecutionError,
                ToolReasonCode::ExecutionError,
            ),
        };
        let status_tag = match status {
            NormalizedToolStatus::Ok => "ok",
            NormalizedToolStatus::Error => "error",
            NormalizedToolStatus::Blocked => "blocked",
        };
        let attempt_receipt = self.record_attempt_receipt(
            request.trace_id.as_str(),
            request.task_id.as_str(),
            request.caller,
            tool_name.as_str(),
            attempt_status,
            errors
                .first()
                .map(String::as_str)
                .unwrap_or(if status_tag == "ok" {
                    "ok"
                } else {
                    "execution_error"
                }),
            reason_code,
            event_ts,
            duration_ms,
            &probe,
        );
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
        let attempt = ToolAttemptEnvelope {
            attempt: attempt_receipt,
            normalized_result: Some(normalized_result.clone()),
            raw_payload: Some(raw_payload.clone()),
            error: None,
        };
        Ok(ToolBrokerExecution {
            attempt,
            normalized_result,
            raw_payload,
        })
    }

    pub fn execute_and_envelope<F>(
        &mut self,
        request: ToolCallRequest,
        executor: F,
    ) -> ToolAttemptEnvelope
    where
        F: FnOnce(&Value) -> Result<Value, String>,
    {
        let before = self.attempt_receipts.len();
        match self.execute_and_normalize(request.clone(), executor) {
            Ok(out) => out.attempt,
            Err(err) => {
                let attempt = self
                    .attempt_receipts
                    .get(before)
                    .cloned()
                    .or_else(|| self.attempt_receipts.last().cloned())
                    .unwrap_or_else(|| ToolAttemptReceipt {
                        attempt_id: deterministic_hash(&json!({
                            "kind": "tool_attempt_receipt_fallback",
                            "trace_id": request.trace_id,
                            "task_id": request.task_id,
                            "tool_name": request.tool_name,
                            "timestamp": now_ms()
                        })),
                        trace_id: clean_text(&request.trace_id, 160),
                        task_id: clean_text(&request.task_id, 160),
                        caller: request.caller,
                        tool_name: clean_text(&request.tool_name, 120),
                        status: ToolAttemptStatus::ExecutionError,
                        outcome: "error".to_string(),
                        reason_code: ToolReasonCode::ExecutionError,
                        reason: clean_text(&err.as_message(), 300),
                        latency_ms: 0,
                        required_args: Vec::new(),
                        backend: "unknown".to_string(),
                        discoverable: false,
                        timestamp: now_ms(),
                    });
                ToolAttemptEnvelope {
                    attempt,
                    normalized_result: None,
                    raw_payload: None,
                    error: Some(err.as_message()),
                }
            }
        }
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

    fn record_attempt_receipt(
        &mut self,
        trace_id: &str,
        task_id: &str,
        caller: BrokerCaller,
        tool_name: &str,
        status: ToolAttemptStatus,
        reason: &str,
        reason_code: ToolReasonCode,
        timestamp: u64,
        latency_ms: u64,
        probe: &ToolCapabilityProbe,
    ) -> ToolAttemptReceipt {
        let outcome = match status {
            ToolAttemptStatus::Ok => "ok",
            ToolAttemptStatus::Unavailable => "unavailable",
            ToolAttemptStatus::Blocked | ToolAttemptStatus::PolicyDenied => "blocked",
            ToolAttemptStatus::InvalidArgs
            | ToolAttemptStatus::ExecutionError
            | ToolAttemptStatus::TransportError
            | ToolAttemptStatus::Timeout => "error",
        };
        let receipt = ToolAttemptReceipt {
            attempt_id: deterministic_hash(&json!({
                "kind": "tool_attempt_receipt",
                "trace_id": trace_id,
                "task_id": task_id,
                "caller": format!("{caller:?}").to_ascii_lowercase(),
                "tool_name": tool_name,
                "outcome": outcome,
                "timestamp": timestamp,
                "sequence": self.attempt_receipts.len() + 1
            })),
            trace_id: clean_text(trace_id, 160),
            task_id: clean_text(task_id, 160),
            caller,
            tool_name: clean_text(tool_name, 120),
            status,
            outcome: clean_text(outcome, 40),
            reason_code,
            reason: clean_text(reason, 300),
            latency_ms,
            required_args: probe.required_args.clone(),
            backend: clean_text(&probe.backend, 120),
            discoverable: probe.discoverable,
            timestamp,
        };
        self.attempt_receipts.push(receipt.clone());
        receipt
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_rejects_unauthorized_tool_request() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
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

    #[test]
    fn capability_catalog_and_probe_are_deterministic() {
        let broker = ToolBroker::default();
        let catalog = broker.capability_catalog();
        assert!(catalog.iter().any(|row| row.tool_name == "web_search"));
        assert!(catalog.iter().any(|row| row.tool_name == "terminal_exec"));
        let allowed = broker.capability_probe(BrokerCaller::Client, "web_search");
        assert!(allowed.available);
        assert!(matches!(
            allowed.reason_code,
            ToolReasonCode::Ok | ToolReasonCode::BackendDegraded
        ));
        assert_eq!(allowed.backend, "retrieval_plane");
        assert_eq!(allowed.required_args, vec!["query".to_string()]);
        let unknown = broker.capability_probe(BrokerCaller::Client, "tool_that_does_not_exist");
        assert!(!unknown.available);
        assert_eq!(unknown.reason, "unknown_tool");
        assert_eq!(unknown.reason_code, ToolReasonCode::UnknownTool);
    }

    #[test]
    fn unauthorized_attempts_are_receipted() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-attempt".to_string(),
                task_id: "task-attempt".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"ls"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert!(out.is_err());
        let attempt = broker.attempt_receipts().last().expect("attempt");
        assert_eq!(attempt.outcome, "blocked");
        assert_eq!(attempt.status, ToolAttemptStatus::Blocked);
        assert_eq!(attempt.reason, "caller_not_authorized");
        assert_eq!(attempt.reason_code, ToolReasonCode::CallerNotAuthorized);
        assert_eq!(attempt.backend, "governed_terminal");
        assert_eq!(attempt.required_args, vec!["command".to_string()]);
    }

    #[test]
    fn successful_attempts_are_receipted() {
        let mut broker = ToolBroker::default();
        let out = broker.execute_and_normalize(
            ToolCallRequest {
                trace_id: "trace-ok".to_string(),
                task_id: "task-ok".to_string(),
                tool_name: "web_search".to_string(),
                args: json!({"query":"latency"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"results":[{"summary":"ok"}]})),
        );
        assert!(out.is_ok());
        let attempt = broker.attempt_receipts().last().expect("attempt");
        assert_eq!(attempt.outcome, "ok");
        assert_eq!(attempt.status, ToolAttemptStatus::Ok);
        assert_eq!(attempt.reason_code, ToolReasonCode::Ok);
    }

    #[test]
    fn execute_and_envelope_returns_structured_failure_attempt() {
        let mut broker = ToolBroker::default();
        broker
            .allowed_tools
            .insert(BrokerCaller::Client, HashSet::new());
        let attempt = broker.execute_and_envelope(
            ToolCallRequest {
                trace_id: "trace-envelope".to_string(),
                task_id: "task-envelope".to_string(),
                tool_name: "terminal_exec".to_string(),
                args: json!({"command":"ls"}),
                lineage: vec![],
                caller: BrokerCaller::Client,
                policy_revision: None,
                tool_version: None,
                freshness_window_ms: None,
                force_no_dedupe: false,
            },
            |_| Ok(json!({"ok": true})),
        );
        assert_eq!(attempt.attempt.status, ToolAttemptStatus::Blocked);
        assert_eq!(
            attempt.attempt.reason_code,
            ToolReasonCode::CallerNotAuthorized
        );
        assert!(attempt.normalized_result.is_none());
        assert!(attempt.error.is_some());
    }
}
