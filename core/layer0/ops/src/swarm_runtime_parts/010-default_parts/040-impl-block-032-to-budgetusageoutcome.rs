
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
