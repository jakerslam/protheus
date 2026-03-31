impl BudgetTelemetry {
    fn new(session_id: String, config: TokenBudgetConfig) -> Self {
        let threshold = ((config.max_tokens as f32) * config.warning_threshold)
            .round()
            .clamp(0.0, config.max_tokens as f32) as u32;
        Self {
            session_id,
            budget_config: config,
            usage_over_time: Vec::new(),
            tool_breakdown: BTreeMap::new(),
            final_usage: 0,
            budget_exhausted: false,
            warning_emitted: false,
            warning_at_tokens: threshold,
            compaction_triggered: false,
            reserved_for_children: 0,
            child_reservations: BTreeMap::new(),
            settled_child_tokens: 0,
        }
    }

    fn remaining_tokens(&self) -> u32 {
        self.budget_config
            .max_tokens
            .saturating_sub(self.final_usage.saturating_add(self.reserved_for_children))
    }

    fn utilization(&self) -> f64 {
        if self.budget_config.max_tokens == 0 {
            0.0
        } else {
            self.final_usage as f64 / self.budget_config.max_tokens as f64
        }
    }

    fn push_usage(&mut self, tool_name: &str, tokens_used: u32) {
        if tokens_used == 0 {
            return;
        }
        self.final_usage = self.final_usage.saturating_add(tokens_used);
        *self
            .tool_breakdown
            .entry(tool_name.to_string())
            .or_insert(0) += tokens_used;
        self.usage_over_time.push(UsageSnapshot {
            timestamp_ms: now_epoch_ms(),
            cumulative_usage: self.final_usage,
            tool: tool_name.to_string(),
        });
    }

    fn record_tool_usage(&mut self, tool_name: &str, requested_tokens: u32) -> BudgetUsageOutcome {
        let current = self.final_usage;
        let projected = current.saturating_add(requested_tokens);
        let max_tokens = self.budget_config.max_tokens;

        if projected > max_tokens {
            let remaining = max_tokens.saturating_sub(current);
            self.budget_exhausted = true;
            return match self.budget_config.exhaustion_action {
                BudgetAction::FailHard => BudgetUsageOutcome::ExceededDenied(format!(
                    "token_budget_exceeded:current={current}:requested={requested_tokens}:limit={max_tokens}:tool={tool_name}"
                )),
                BudgetAction::AllowWithWarning => {
                    self.push_usage(tool_name, remaining);
                    BudgetUsageOutcome::ExhaustedAllowed {
                        event: json!({
                            "type": "budget_exhausted",
                            "action": "allow_with_warning",
                            "tool": tool_name,
                            "current": current,
                            "requested": requested_tokens,
                            "applied": remaining,
                            "limit": max_tokens,
                            "remaining": self.remaining_tokens(),
                        }),
                        action: "warn".to_string(),
                    }
                }
                BudgetAction::TriggerCompaction => {
                    self.compaction_triggered = true;
                    let compacted_request = ((requested_tokens as f32) * 0.4).ceil() as u32;
                    let applied = compacted_request.min(remaining);
                    self.push_usage(tool_name, applied);
                    BudgetUsageOutcome::ExhaustedAllowed {
                        event: json!({
                            "type": "budget_exhausted",
                            "action": "trigger_compaction",
                            "tool": tool_name,
                            "current": current,
                            "requested": requested_tokens,
                            "compacted_request": compacted_request,
                            "applied": applied,
                            "limit": max_tokens,
                            "remaining": self.remaining_tokens(),
                        }),
                        action: "compact".to_string(),
                    }
                }
            };
        }

        self.push_usage(tool_name, requested_tokens);
        if !self.warning_emitted && self.final_usage >= self.warning_at_tokens {
            self.warning_emitted = true;
            return BudgetUsageOutcome::Warning(json!({
                "type": "budget_warning",
                "session_id": self.session_id,
                "current": self.final_usage,
                "threshold": self.warning_at_tokens,
                "remaining": self.remaining_tokens(),
                "utilization": self.utilization(),
            }));
        }
        BudgetUsageOutcome::Ok
    }

    fn generate_report(&self) -> Value {
        let most_expensive_tool = self
            .tool_breakdown
            .iter()
            .max_by_key(|(_, tokens)| **tokens)
            .map(|(name, _)| name.clone());
        json!({
            "budget": self.budget_config.max_tokens,
            "warning_threshold": self.budget_config.warning_threshold,
            "warning_at_tokens": self.warning_at_tokens,
            "on_budget_exhausted": self.budget_config.exhaustion_action.as_label(),
            "used": self.final_usage,
            "remaining": self.remaining_tokens(),
            "utilization": self.utilization(),
            "budget_exhausted": self.budget_exhausted,
            "warning_emitted": self.warning_emitted,
            "compaction_triggered": self.compaction_triggered,
            "reserved_for_children": self.reserved_for_children,
            "settled_child_tokens": self.settled_child_tokens,
            "child_reservations": self.child_reservations,
            "tool_breakdown": self.tool_breakdown,
            "most_expensive_tool": most_expensive_tool,
            "timeline": self.usage_over_time,
        })
    }
}

#[derive(Debug, Clone)]
struct SpawnOptions {
    verify: bool,
    timeout_ms: u64,
    metrics_detailed: bool,
    simulate_unreachable: bool,
    byzantine: bool,
    corruption_type: String,
    token_budget: Option<u32>,
    token_warning_threshold: f32,
    budget_exhaustion_action: BudgetAction,
    adaptive_complexity: bool,
    execution_mode: ExecutionMode,
    role: Option<String>,
    capabilities: Vec<String>,
    auto_publish_results: bool,
    agent_label: Option<String>,
    result_value: Option<f64>,
    result_text: Option<String>,
    result_confidence: f64,
    verification_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentReport {
    agent_id: String,
    #[serde(default)]
    values: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DeliveryGuarantee {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl DeliveryGuarantee {
    fn from_flag(raw: Option<String>) -> Self {
        match raw
            .unwrap_or_else(|| "at_most_once".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "at_least_once" | "at-least-once" | "atleastonce" => Self::AtLeastOnce,
            "exactly_once" | "exactly-once" | "exactlyonce" => Self::ExactlyOnce,
            _ => Self::AtMostOnce,
        }
    }

    fn as_label(&self) -> &'static str {
        match self {
            Self::AtMostOnce => "at_most_once",
            Self::AtLeastOnce => "at_least_once",
            Self::ExactlyOnce => "exactly_once",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentMessage {
    message_id: String,
    sender_session_id: String,
    recipient_session_id: String,
    payload: String,
    created_at: String,
    timestamp_ms: u64,
    delivery: DeliveryGuarantee,
    attempts: u32,
    acknowledged: bool,
    #[serde(default)]
    acked_at_ms: Option<u64>,
    #[serde(default)]
    dedupe_key: Option<String>,
    ttl_ms: u64,
    expires_at_ms: u64,
    #[serde(default)]
    deferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SessionMailbox {
    session_id: String,
    #[serde(default)]
    unread: Vec<AgentMessage>,
    #[serde(default)]
    read: Vec<AgentMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeadLetterMessage {
    dead_letter_id: String,
    message: AgentMessage,
    reason: String,
    moved_at: String,
    moved_at_ms: u64,
    retryable: bool,
    #[serde(default)]
    retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelMessage {
    message_id: String,
    sender_session_id: String,
    payload: String,
    timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageChannel {
    channel_id: String,
    name: String,
    participants: Vec<String>,
    created_at: String,
    #[serde(default)]
    messages: Vec<ChannelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceInstance {
    session_id: String,
    role: String,
    #[serde(default)]
    capabilities: Vec<String>,
    healthy: bool,
    registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum ResultPayload {
    Calculation { value: f64 },
    Text { content: String },
    Structured { schema: String, data: Value },
}

impl ResultPayload {
    fn field_value(&self, field: &str) -> Option<Value> {
        match self {
            Self::Calculation { value } if matches!(field, "value" | "calculation") => {
                Some(json!(value))
            }
            Self::Text { content } if matches!(field, "value" | "text" | "content") => {
                Some(json!(content))
            }
            Self::Structured { data, .. } => data.get(field).cloned(),
            _ => None,
        }
    }

    fn numeric_value(&self) -> Option<f64> {
        match self {
            Self::Calculation { value } => Some(*value),
            Self::Structured { data, .. } => data.get("value").and_then(Value::as_f64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentResult {
    result_id: String,
    session_id: String,
    agent_label: String,
    agent_role: String,
    task_id: String,
    payload: ResultPayload,
    data: Value,
    confidence: f64,
    verification_status: String,
    timestamp_ms: u64,
    created_at: String,
}

#[derive(Debug, Clone, Default)]
struct ResultFilters {
    label_pattern: Option<String>,
    role: Option<String>,
    task_id: Option<String>,
    session_id: Option<String>,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops swarm-runtime status [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime spawn [--task=<text>] [--session-id=<parent>] [--recursive=1|0] [--levels=<n>] [--max-depth=<n>] [--verify=1|0] [--timeout-sec=<seconds>] [--metrics=<none|detailed>] [--byzantine=1|0] [--corruption-type=<id>] [--token-budget=<n>|--max-tokens=<n>] [--token-warning-at=<0..1>] [--on-budget-exhausted=<fail|warn|compact>] [--adaptive-complexity=1|0] [--execution-mode=<task|persistent|background>] [--role=<name>] [--capabilities=<csv>] [--lifespan-sec=<n>] [--check-in-interval-sec=<n>] [--report-mode=<always|anomalies|final>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime tick [--advance-ms=<n>] [--max-check-ins=<n>] [--state-path=<path>]");
    println!(
        "  protheus-ops swarm-runtime byzantine-test <enable|disable|status> [--state-path=<path>]"
    );
    println!("  protheus-ops swarm-runtime consensus-check [--task-id=<id>] [--threshold=<0..1>] [--fields=<csv>] [--reports-json=<json>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime test recursive [--levels=<n>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime test byzantine [--agents=<n>] [--corrupt=<n>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime test concurrency [--agents=<n>] [--metrics=detailed] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime test budget [--budget=<n>] [--warning-at=<0..1>] [--on-budget-exhausted=<fail|warn|compact>] [--assert-hard-enforcement=1|0] [--expect-fail=1|0] [--task=<text>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime test persistent [--lifespan-sec=<n>] [--check-in-interval-sec=<n>] [--advance-ms=<n>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime thorn <status|quarantine|release> [flags]");
    println!("  protheus-ops swarm-runtime budget-report --session-id=<id> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions budget-report --session-id=<id> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions wake --session-id=<id> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions terminate --session-id=<id> [--graceful=1|0] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions metrics --session-id=<id> [--timeline=1|0] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions state --session-id=<id> [--timeline=1|0] [--tool-history-limit=<n>] [--state-path=<path>]");
    println!(
        "  protheus-ops swarm-runtime sessions bootstrap --session-id=<id> [--state-path=<path>]"
    );
    println!("  protheus-ops swarm-runtime sessions handoff --session-id=<sender> --target-session-id=<recipient> --reason=<text> [--importance=<0..1>] [--context-json=<json>] [--network-id=<id>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions context-put --session-id=<id> --context-json=<json> [--merge=1|0] [--state-path=<path>]");
    println!(
        "  protheus-ops swarm-runtime sessions context-get --session-id=<id> [--state-path=<path>]"
    );
    println!(
        "  protheus-ops swarm-runtime sessions anomalies --session-id=<id> [--state-path=<path>]"
    );
    println!("  protheus-ops swarm-runtime sessions send --sender-id=<session|coordinator> --session-id=<recipient> --message=<text> [--delivery=<at_most_once|at_least_once|exactly_once>] [--ttl-ms=<n>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions receive --session-id=<id> [--limit=<n>] [--mark-read=1|0] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions ack --session-id=<id> --message-id=<id> [--state-path=<path>]");
    println!(
        "  protheus-ops swarm-runtime sessions resume --session-id=<id> [--state-path=<path>]"
    );
    println!("  protheus-ops swarm-runtime sessions dead-letter [--session-id=<id>] [--retryable=1|0] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions retry-dead-letter --message-id=<id> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions discover --role=<name> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime sessions send-role --sender-id=<session|coordinator> --role=<name> --message=<text> [--delivery=<at_most_once|at_least_once|exactly_once>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime background <start|status|stop> [flags]");
    println!("  protheus-ops swarm-runtime scheduled <add|status|run-due> [flags]");
    println!("  protheus-ops swarm-runtime channels <create|publish|poll|monitor> [flags]");
    println!(
        "  protheus-ops swarm-runtime results <publish|query|wait|show|consensus|outliers> [flags]"
    );
    println!("  protheus-ops swarm-runtime tools register-json-schema --session-id=<id> --tool-name=<name> --schema-json=<json> --bridge-path=<path> --entrypoint=<name> [--description=<text>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime tools invoke --session-id=<id> --tool-name=<name> [--args-json=<json>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime stream emit --session-id=<id> [--turn-id=<id>] [--agent-label=<label>] --chunks-json=<json> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime stream render --session-id=<id> [--turn-id=<id>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime turns run --session-id=<id> --turns-json=<json> [--label=<text>] [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime turns show --session-id=<id> --run-id=<id> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime networks create [--session-id=<owner>] --spec-json=<json> [--state-path=<path>]");
    println!("  protheus-ops swarm-runtime networks status --network-id=<id> [--session-id=<owner>] [--state-path=<path>]");
    println!(
        "  protheus-ops swarm-runtime metrics queue [--format=<json|prometheus>] [--state-path=<path>]"
    );
    println!("  protheus-ops swarm-runtime test heterogeneous [--label-pattern=<glob>] [--min-count=<n>] [--timeout-sec=<n>] [--state-path=<path>]");
}

fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let key_pref = format!("--{key}=");
    let key_exact = format!("--{key}");
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(value) = token.strip_prefix(&key_pref) {
            return Some(value.to_string());
        }
        if token == key_exact && idx + 1 < argv.len() {
            return Some(argv[idx + 1].clone());
        }
        idx += 1;
    }
    None
}

fn parse_first_flag(argv: &[String], keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| parse_flag(argv, key))
}

fn parse_bool_flag(argv: &[String], key: &str, fallback: bool) -> bool {
    match parse_flag(argv, key) {
        Some(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        None => fallback,
    }
}

fn parse_u8_flag(argv: &[String], key: &str, fallback: u8) -> u8 {
    parse_flag(argv, key)
        .and_then(|v| v.trim().parse::<u8>().ok())
        .unwrap_or(fallback)
}

fn parse_u64_flag(argv: &[String], key: &str, fallback: u64) -> u64 {
    parse_flag(argv, key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn parse_f64_flag(argv: &[String], key: &str, fallback: f64) -> f64 {
    parse_flag(argv, key)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

fn parse_json_flag(argv: &[String], key: &str) -> Option<Value> {
    parse_flag(argv, key).and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.trim().chars().take(max_len).collect()
}

fn json_size_bytes(value: &Value) -> usize {
    serde_json::to_vec(value).map(|row| row.len()).unwrap_or(0)
}
