
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
    role_card: Option<RoleCard>,
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

fn session_metadata_base(
    session_id: String,
    parent_id: Option<String>,
    depth: u8,
    task: String,
    status: String,
) -> SessionMetadata {
    SessionMetadata {
        session_id,
        parent_id,
        children: Vec::new(),
        depth,
        task,
        created_at: now_iso(),
        status,
        reachable: true,
        byzantine: false,
        corruption_type: None,
        report: None,
        metrics: None,
        budget_telemetry: None,
        scaled_task: None,
        budget_action_taken: None,
        role: None,
        role_card: None,
        agent_label: None,
        tool_access: default_session_tool_access(),
        context_vars: BTreeMap::new(),
        context_mode: None,
        handoff_ids: Vec::new(),
        registered_tool_ids: Vec::new(),
        stream_turn_ids: Vec::new(),
        turn_run_ids: Vec::new(),
        network_ids: Vec::new(),
        check_ins: Vec::new(),
        metrics_timeline: Vec::new(),
        anomalies: Vec::new(),
        persistent: None,
        background_worker: false,
        budget_parent_session_id: None,
        budget_reservation_tokens: 0,
        budget_reservation_settled: false,
        thorn_cell: false,
        thorn_target_session_id: None,
        thorn_expires_at_ms: None,
        quarantine_reason: None,
        quarantine_previous_status: None,
    }
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
