use protheus_nexus_core_v1::{
    DefaultNexusPolicy, DeliveryAuthorizationInput, LeaseIssueRequest, MainNexusControlPlane,
    NexusFeatureFlags, TrustClass, VerityClass,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const CONTEXT_STACKS_POLICY_REL: &str = "client/runtime/config/context_stacks_policy.json";
const CONTEXT_STACKS_STATE_REL: &str = "client/runtime/local/state/memory/context_stacks/state.json";
const CONTEXT_STACKS_RECEIPTS_REL: &str =
    "client/runtime/local/state/memory/context_stacks/receipts.jsonl";
const CONTEXT_STACKS_DIGESTION_LOG_REL: &str =
    "client/runtime/local/state/memory/context_stacks/digestion-log.yaml";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CachePolicy {
    #[serde(rename = "none")]
    NoCache,
    Auto,
    ExplicitBreakpoint,
    MultiBreakpoint,
}

impl CachePolicy {
    fn from_raw(raw: &str) -> Self {
        match clean(raw, 64).to_ascii_lowercase().as_str() {
            "none" | "no_cache" => CachePolicy::NoCache,
            "explicit_breakpoint" | "explicit-breakpoint" => CachePolicy::ExplicitBreakpoint,
            "multi_breakpoint" | "multi-breakpoint" => CachePolicy::MultiBreakpoint,
            _ => CachePolicy::Auto,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            CachePolicy::NoCache => "none",
            CachePolicy::Auto => "auto",
            CachePolicy::ExplicitBreakpoint => "explicit_breakpoint",
            CachePolicy::MultiBreakpoint => "multi_breakpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BatchLane {
    LiveMicrobatch,
    ProviderBatch,
}

impl BatchLane {
    fn from_raw(raw: &str) -> Self {
        match clean(raw, 64).to_ascii_lowercase().as_str() {
            "provider_batch" | "provider-batch" => BatchLane::ProviderBatch,
            _ => BatchLane::LiveMicrobatch,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            BatchLane::LiveMicrobatch => "live_microbatch",
            BatchLane::ProviderBatch => "provider_batch",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct StableHead {
    system_prompt: String,
    tools: Vec<String>,
    ordered_stable_nodes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SemanticSnapshot {
    semantic_snapshot_id: String,
    stable_head: StableHead,
    volatile_metadata: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct RenderPlan {
    render_plan_id: String,
    provider: String,
    model: String,
    tool_choice: String,
    thinking_mode: String,
    image_presence: String,
    response_mode: String,
    cache_policy: CachePolicy,
    ttl_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ProviderSnapshot {
    render_fingerprint: String,
    semantic_snapshot_id: String,
    render_plan_id: String,
    provider: String,
    model: String,
    serialized_prefix: String,
    derived_disposable: bool,
    created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct DeltaTailEntry {
    kind: String,
    text: String,
    ts: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct DeltaTail {
    tail_id: String,
    stack_id: String,
    session_id: String,
    current_objective: String,
    entries: Vec<DeltaTailEntry>,
    created_at: String,
    updated_at: String,
    last_promoted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ContextStackManifest {
    stack_id: String,
    semantic_snapshot_id: String,
    active_delta_tail_ids: Vec<String>,
    archived: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct BatchClass {
    lane: BatchLane,
    provider: String,
    model: String,
    render_fingerprint: String,
    tool_choice: String,
    thinking_mode: String,
    image_presence: String,
    response_mode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ContextStacksPolicy {
    version: String,
    cache_threshold_tokens: u64,
    seed_then_fanout_min_cohort: usize,
    lookback_window_tokens: u64,
    allow_provider_batch_lane: bool,
    allow_multi_breakpoint: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct ContextStacksState {
    version: String,
    manifests: Vec<ContextStackManifest>,
    semantic_snapshots: Vec<SemanticSnapshot>,
    render_plans: Vec<RenderPlan>,
    provider_snapshots: Vec<ProviderSnapshot>,
    delta_tails: Vec<DeltaTail>,
    batch_classes: Vec<BatchClass>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SchedulerEdgeCaseDecision {
    scheduler_mode: String,
    cache_hit: bool,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
    seed_then_fanout: bool,
    breakpoint_mode: Option<String>,
}

fn clean(raw: impl AsRef<str>, max_len: usize) -> String {
    raw.as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect()
}

fn truthy(raw: Option<&String>) -> bool {
    raw.map(|value| clean(value, 32).to_ascii_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn parse_csv(raw: Option<&String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split(',')
            .map(|token| clean(token, 240))
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn parse_json_array(raw: Option<&String>) -> Vec<String> {
    let Some(encoded) = raw else {
        return Vec::new();
    };
    serde_json::from_str::<Value>(encoded)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean(row, 800))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_json_value(raw: Option<&String>) -> Value {
    raw.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or_else(|| json!({}))
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn semantic_snapshot_id_for(stable_head: &StableHead) -> String {
    let payload = serde_json::to_string(stable_head).unwrap_or_else(|_| "{}".to_string());
    let digest = sha256_hex(&payload);
    format!("semantic_{}", &digest[..24])
}

fn render_fingerprint_for(snapshot: &SemanticSnapshot, plan: &RenderPlan) -> String {
    let payload = json!({
        "semantic_snapshot_id": snapshot.semantic_snapshot_id,
        "provider": plan.provider,
        "model": plan.model,
        "tool_choice": plan.tool_choice,
        "thinking_mode": plan.thinking_mode,
        "image_presence": plan.image_presence,
        "response_mode": plan.response_mode,
    });
    let digest = sha256_hex(
        &serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    );
    format!("render_{}", &digest[..24])
}

fn batch_class_id_for(batch_class: &BatchClass) -> String {
    let payload = serde_json::to_string(batch_class).unwrap_or_else(|_| "{}".to_string());
    let digest = sha256_hex(&payload);
    format!("batch_{}", &digest[..24])
}

fn default_render_plan(semantic_snapshot_id: &str) -> RenderPlan {
    let payload = json!({
        "semantic_snapshot_id": semantic_snapshot_id,
        "provider": "default",
        "model": "default",
        "tool_choice": "auto",
        "thinking_mode": "default",
        "image_presence": "none",
        "response_mode": "chat",
        "cache_policy": "auto",
        "ttl_class": "session",
    });
    let render_plan_id = format!(
        "render_plan_{}",
        &sha256_hex(&serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()))[..24]
    );
    RenderPlan {
        render_plan_id,
        provider: "default".to_string(),
        model: "default".to_string(),
        tool_choice: "auto".to_string(),
        thinking_mode: "default".to_string(),
        image_presence: "none".to_string(),
        response_mode: "chat".to_string(),
        cache_policy: CachePolicy::Auto,
        ttl_class: "session".to_string(),
    }
}

fn provider_snapshot_serialized_prefix(snapshot: &SemanticSnapshot, plan: &RenderPlan) -> String {
    serde_json::to_string(&json!({
        "semantic_snapshot_id": snapshot.semantic_snapshot_id,
        "stable_head": snapshot.stable_head,
        "render_plan": {
            "provider": plan.provider,
            "model": plan.model,
            "tool_choice": plan.tool_choice,
            "thinking_mode": plan.thinking_mode,
            "image_presence": plan.image_presence,
            "response_mode": plan.response_mode,
            "cache_policy": plan.cache_policy.as_str(),
            "ttl_class": plan.ttl_class
        }
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn receipt_hash(payload: &Value) -> String {
    crate::deterministic_receipt_hash(payload)
}

fn now_iso() -> String {
    crate::now_iso()
}

fn ensure_workspace_root(root: &Path) -> &Path {
    root
}
