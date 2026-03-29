// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::{deterministic_receipt_hash, now_iso};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_STATE_PATH: &str = "local/state/ops/swarm_runtime/latest.json";
const MAX_EVENT_ROWS: usize = 256;
const MAX_DEAD_LETTER_ROWS: usize = 256;
const DEFAULT_MESSAGE_TTL_MS: u64 = 300_000;
const MAX_MAILBOX_UNREAD: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwarmState {
    version: String,
    updated_at: String,
    byzantine_test_mode: bool,
    #[serde(default)]
    sessions: BTreeMap<String, SessionMetadata>,
    #[serde(default)]
    mailboxes: BTreeMap<String, SessionMailbox>,
    #[serde(default)]
    channels: BTreeMap<String, MessageChannel>,
    #[serde(default)]
    service_registry: BTreeMap<String, Vec<ServiceInstance>>,
    #[serde(default)]
    result_registry: BTreeMap<String, AgentResult>,
    #[serde(default)]
    handoff_registry: BTreeMap<String, Value>,
    #[serde(default)]
    tool_registry: BTreeMap<String, Value>,
    #[serde(default)]
    stream_registry: BTreeMap<String, Vec<Value>>,
    #[serde(default)]
    turn_registry: BTreeMap<String, Value>,
    #[serde(default)]
    network_registry: BTreeMap<String, Value>,
    #[serde(default)]
    results_by_session: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    results_by_label: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    results_by_role: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    exactly_once_dedupe: BTreeMap<String, String>,
    #[serde(default)]
    dead_letters: Vec<DeadLetterMessage>,
    #[serde(default)]
    scheduled_tasks: BTreeMap<String, ScheduledTask>,
    #[serde(default)]
    events: Vec<Value>,
}

impl Default for SwarmState {
    fn default() -> Self {
        Self {
            version: "swarm-runtime/v1".to_string(),
            updated_at: now_iso(),
            byzantine_test_mode: false,
            sessions: BTreeMap::new(),
            mailboxes: BTreeMap::new(),
            channels: BTreeMap::new(),
            service_registry: BTreeMap::new(),
            result_registry: BTreeMap::new(),
            handoff_registry: BTreeMap::new(),
            tool_registry: BTreeMap::new(),
            stream_registry: BTreeMap::new(),
            turn_registry: BTreeMap::new(),
            network_registry: BTreeMap::new(),
            results_by_session: BTreeMap::new(),
            results_by_label: BTreeMap::new(),
            results_by_role: BTreeMap::new(),
            exactly_once_dedupe: BTreeMap::new(),
            dead_letters: Vec::new(),
            scheduled_tasks: BTreeMap::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadata {
    session_id: String,
    parent_id: Option<String>,
    #[serde(default)]
    children: Vec<String>,
    depth: u8,
    task: String,
    created_at: String,
    status: String,
    reachable: bool,
    byzantine: bool,
    #[serde(default)]
    corruption_type: Option<String>,
    #[serde(default)]
    report: Option<Value>,
    #[serde(default)]
    metrics: Option<SpawnMetrics>,
    #[serde(default)]
    budget_telemetry: Option<BudgetTelemetry>,
    #[serde(default)]
    scaled_task: Option<String>,
    #[serde(default)]
    budget_action_taken: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    agent_label: Option<String>,
    #[serde(default = "default_session_tool_access")]
    tool_access: Vec<String>,
    #[serde(default)]
    context_vars: BTreeMap<String, Value>,
    #[serde(default)]
    context_mode: Option<String>,
    #[serde(default)]
    handoff_ids: Vec<String>,
    #[serde(default)]
    registered_tool_ids: Vec<String>,
    #[serde(default)]
    stream_turn_ids: Vec<String>,
    #[serde(default)]
    turn_run_ids: Vec<String>,
    #[serde(default)]
    network_ids: Vec<String>,
    #[serde(default)]
    check_ins: Vec<Value>,
    #[serde(default)]
    metrics_timeline: Vec<MetricsSnapshot>,
    #[serde(default)]
    anomalies: Vec<String>,
    #[serde(default)]
    persistent: Option<PersistentRuntime>,
    #[serde(default)]
    background_worker: bool,
    #[serde(default)]
    budget_parent_session_id: Option<String>,
    #[serde(default)]
    budget_reservation_tokens: u32,
    #[serde(default)]
    budget_reservation_settled: bool,
    #[serde(default)]
    thorn_cell: bool,
    #[serde(default)]
    thorn_target_session_id: Option<String>,
    #[serde(default)]
    thorn_expires_at_ms: Option<u64>,
    #[serde(default)]
    quarantine_reason: Option<String>,
    #[serde(default)]
    quarantine_previous_status: Option<String>,
}

fn default_session_tool_access() -> Vec<String> {
    vec![
        "sessions_spawn".to_string(),
        "sessions_send".to_string(),
        "sessions_receive".to_string(),
        "sessions_ack".to_string(),
        "sessions_handoff".to_string(),
        "sessions_context_put".to_string(),
        "sessions_context_get".to_string(),
        "sessions_query".to_string(),
        "sessions_state".to_string(),
        "sessions_tick".to_string(),
        "tools_register_json_schema".to_string(),
        "tools_invoke".to_string(),
        "stream_emit".to_string(),
        "stream_render".to_string(),
        "turns_run".to_string(),
        "turns_show".to_string(),
        "networks_create".to_string(),
        "networks_status".to_string(),
    ]
}

fn thorn_session_tool_access() -> Vec<String> {
    vec![
        "sessions_state".to_string(),
        "sessions_receive".to_string(),
        "sessions_ack".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnMetrics {
    request_received_ms: u64,
    queue_wait_ms: u64,
    spawn_initiated_ms: u64,
    spawn_completed_ms: u64,
    execution_start_ms: u64,
    execution_end_ms: u64,
    report_back_latency_ms: u64,
}

impl SpawnMetrics {
    fn total_latency_ms(&self) -> u64 {
        self.execution_end_ms
            .saturating_sub(self.request_received_ms)
            .saturating_add(self.report_back_latency_ms)
    }

    fn execution_time_ms(&self) -> u64 {
        self.execution_end_ms
            .saturating_sub(self.execution_start_ms)
    }

    fn queue_overhead_pct(&self) -> f64 {
        let total = self.total_latency_ms();
        if total == 0 {
            0.0
        } else {
            (self.queue_wait_ms as f64 / total as f64) * 100.0
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "request_received_ms": self.request_received_ms,
            "queue_wait_ms": self.queue_wait_ms,
            "spawn_initiated_ms": self.spawn_initiated_ms,
            "spawn_completed_ms": self.spawn_completed_ms,
            "execution_start_ms": self.execution_start_ms,
            "execution_end_ms": self.execution_end_ms,
            "execution_time_ms": self.execution_time_ms(),
            "report_back_latency_ms": self.report_back_latency_ms,
            "total_latency_ms": self.total_latency_ms(),
            "queue_overhead_pct": self.queue_overhead_pct(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BudgetAction {
    FailHard,
    AllowWithWarning,
    TriggerCompaction,
}

impl BudgetAction {
    fn from_flag(raw: Option<String>) -> Self {
        match raw
            .unwrap_or_else(|| "fail".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "warn" | "allow" | "allow_with_warning" => Self::AllowWithWarning,
            "compact" | "trigger_compaction" | "trigger-compaction" => Self::TriggerCompaction,
            _ => Self::FailHard,
        }
    }

    fn as_label(&self) -> &'static str {
        match self {
            Self::FailHard => "fail",
            Self::AllowWithWarning => "warn",
            Self::TriggerCompaction => "compact",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReportMode {
    Always,
    AnomaliesOnly,
    FinalOnly,
}

impl ReportMode {
    fn from_flag(raw: Option<String>) -> Self {
        match raw
            .unwrap_or_else(|| "always".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "anomalies" | "anomalies_only" => Self::AnomaliesOnly,
            "final" | "final_only" => Self::FinalOnly,
            _ => Self::Always,
        }
    }

    fn as_label(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::AnomaliesOnly => "anomalies_only",
            Self::FinalOnly => "final_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenBudgetConfig {
    max_tokens: u32,
    warning_threshold: f32,
    exhaustion_action: BudgetAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentAgentConfig {
    lifespan_sec: u64,
    check_in_interval_sec: u64,
    report_mode: ReportMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentRuntime {
    mode: String,
    config: PersistentAgentConfig,
    started_at_ms: u64,
    deadline_ms: u64,
    next_check_in_ms: u64,
    check_in_count: u64,
    #[serde(default)]
    last_check_in_ms: Option<u64>,
    #[serde(default)]
    terminated_at_ms: Option<u64>,
    #[serde(default)]
    terminated_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsSnapshot {
    timestamp_ms: u64,
    cumulative_tokens: u32,
    context_percentage: f64,
    response_latency_ms: u64,
    memory_usage_mb: u64,
    active_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduledTask {
    task_id: String,
    task: String,
    interval_sec: u64,
    max_runtime_sec: u64,
    next_run_ms: u64,
    remaining_runs: u64,
    #[serde(default)]
    last_run_ms: Option<u64>,
    #[serde(default)]
    last_session_id: Option<String>,
    active: bool,
}

#[derive(Debug, Clone)]
enum ExecutionMode {
    TaskOriented,
    Persistent(PersistentAgentConfig),
    Background(PersistentAgentConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageSnapshot {
    timestamp_ms: u64,
    cumulative_usage: u32,
    tool: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BudgetTelemetry {
    session_id: String,
    budget_config: TokenBudgetConfig,
    #[serde(default)]
    usage_over_time: Vec<UsageSnapshot>,
    #[serde(default)]
    tool_breakdown: BTreeMap<String, u32>,
    final_usage: u32,
    budget_exhausted: bool,
    warning_emitted: bool,
    warning_at_tokens: u32,
    compaction_triggered: bool,
    #[serde(default)]
    reserved_for_children: u32,
    #[serde(default)]
    child_reservations: BTreeMap<String, u32>,
    #[serde(default)]
    settled_child_tokens: u32,
}

enum BudgetUsageOutcome {
    Ok,
    Warning(Value),
    ExhaustedAllowed { event: Value, action: String },
    ExceededDenied(String),
}
