
const DEFAULT_STATE_PATH: &str = "local/state/ops/swarm_runtime/latest.json";
const MAX_EVENT_ROWS: usize = 256;
const MAX_DEAD_LETTER_ROWS: usize = 256;
const DEFAULT_MESSAGE_TTL_MS: u64 = 300_000;
const MAX_MAILBOX_UNREAD: usize = 32;
const DEFAULT_SCALE_MAX_SESSIONS_HARD: usize = 200_000;
const DEFAULT_SCALE_MAX_CHILDREN_PER_PARENT: usize = 256;
const DEFAULT_SCALE_MAX_DEPTH_HARD: u8 = 64;
const DEFAULT_SCALE_TARGET_READY_AGENTS: usize = 100_000;
const SCALE_UTILIZATION_ALERT_THRESHOLD: f64 = 0.85;
const STATE_PRETTY_MAX_SESSIONS: usize = 2_000;
const STATE_PRETTY_MAX_MAILBOX_MESSAGES: usize = 96;
const STATE_PRETTY_MAX_EVENT_ROWS: usize = 128;
const STATE_PRETTY_MAX_DEAD_LETTERS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwarmScalePolicy {
    max_sessions_hard: usize,
    max_children_per_parent: usize,
    max_depth_hard: u8,
    target_ready_agents: usize,
    enforce_session_cap: bool,
    enforce_parent_capacity: bool,
}

impl Default for SwarmScalePolicy {
    fn default() -> Self {
        Self {
            max_sessions_hard: DEFAULT_SCALE_MAX_SESSIONS_HARD,
            max_children_per_parent: DEFAULT_SCALE_MAX_CHILDREN_PER_PARENT,
            max_depth_hard: DEFAULT_SCALE_MAX_DEPTH_HARD,
            target_ready_agents: DEFAULT_SCALE_TARGET_READY_AGENTS,
            enforce_session_cap: true,
            enforce_parent_capacity: true,
        }
    }
}

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
    role_dispatch_cursor: BTreeMap<String, usize>,
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
    plan_registry: BTreeMap<String, SwarmPlanGraph>,
    #[serde(default)]
    events: Vec<Value>,
    #[serde(default)]
    message_sequence: u64,
    #[serde(default)]
    scale_policy: SwarmScalePolicy,
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
            role_dispatch_cursor: BTreeMap::new(),
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
            plan_registry: BTreeMap::new(),
            events: Vec::new(),
            message_sequence: 0,
            scale_policy: SwarmScalePolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwarmPlanGraph {
    plan_id: String,
    goal: String,
    supervisor_session_id: String,
    root_node_id: String,
    status: String,
    recursion_depth_limit: u8,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    active_node_id: Option<String>,
    #[serde(default)]
    nodes: BTreeMap<String, SwarmPlanNode>,
    #[serde(default)]
    checkpoints: BTreeMap<String, PlanCheckpoint>,
    #[serde(default)]
    branch_gates: BTreeMap<String, BranchGateState>,
    #[serde(default)]
    merge_history: Vec<Value>,
    #[serde(default)]
    speaker_stats: BTreeMap<String, SpeakerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SwarmPlanNode {
    node_id: String,
    #[serde(default)]
    parent_id: Option<String>,
    task: String,
    status: String,
    depth: u8,
    #[serde(default)]
    assignee_session_id: Option<String>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    checkpoint_id: Option<String>,
    #[serde(default)]
    branch_state: Option<String>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanCheckpoint {
    checkpoint_id: String,
    node_id: String,
    state: Value,
    created_at: String,
    resumable: bool,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BranchGateState {
    node_id: String,
    requires_user: bool,
    status: String,
    #[serde(default)]
    decision: Option<String>,
    timeout_ms: u64,
    auto_path: String,
    #[serde(default)]
    decided_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpeakerStats {
    session_id: String,
    role: String,
    #[serde(default)]
    expertise_tags: Vec<String>,
    #[serde(default)]
    last_spoke_ms: Option<u64>,
    turns: u64,
    score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoleCard {
    role: String,
    goal: String,
    capability_envelope: Vec<String>,
    source: String,
}
