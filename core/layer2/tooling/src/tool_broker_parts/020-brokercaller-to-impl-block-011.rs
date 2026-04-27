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
    pub execution_receipt: ToolExecutionReceipt,
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
    pub execution_receipt: ToolExecutionReceipt,
    pub normalized_result: Option<NormalizedToolResult>,
    pub raw_payload: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionReceiptStatus {
    Success,
    Error,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolExecutionReceipt {
    pub attempt_id: String,
    pub trace_id: String,
    pub task_id: String,
    pub status: ToolExecutionReceiptStatus,
    pub tool_id: String,
    pub input_hash: String,
    pub started_at: u64,
    pub ended_at: u64,
    pub latency_ms: u64,
    pub error_code: Option<String>,
    pub data_ref: Option<String>,
    pub evidence_count: usize,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSubstrateHealthReport {
    pub generated_at: u64,
    pub bounded_workspace_root: String,
    pub backends: Vec<ToolBackendHealth>,
    pub available_tool_count: usize,
    pub degraded_tool_count: usize,
    pub blocked_tool_count: usize,
    pub unavailable_tool_count: usize,
    pub receipt_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolExecutionLedgerEvent {
    pub event_id: String,
    pub event_sequence: u64,
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub attempt_sequence: u64,
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
    pub attempt_sequence: u64,
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
    execution_receipts: Vec<ToolExecutionReceipt>,
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
            execution_receipts: Vec::new(),
            ledger_path: default_ledger_path(),
        };
        let _ = out.recover_from_ledger();
        out
    }
}
