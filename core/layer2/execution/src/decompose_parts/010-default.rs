// SPDX-License-Identifier: Apache-2.0
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

fn parse_payload_json<T: DeserializeOwned>(payload: &str, error_prefix: &str) -> Result<T, String> {
    serde_json::from_str::<T>(payload).map_err(|err| format!("{error_prefix}_payload_parse_failed:{err}"))
}

fn serialize_payload_json<T: Serialize>(value: &T, error_prefix: &str) -> Result<String, String> {
    serde_json::to_string(value)
        .map_err(|err| format!("{error_prefix}_payload_serialize_failed:{err}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposePolicy {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_max_micro_tasks")]
    pub max_micro_tasks: usize,
    #[serde(default = "default_max_words_per_leaf")]
    pub max_words_per_leaf: usize,
    #[serde(default = "default_min_minutes")]
    pub min_minutes: usize,
    #[serde(default = "default_max_minutes")]
    pub max_minutes: usize,
    #[serde(default = "default_max_groups")]
    pub max_groups: usize,
    #[serde(default = "default_lane")]
    pub default_lane: String,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
    #[serde(default)]
    pub human_lane_keywords: Vec<String>,
    #[serde(default)]
    pub autonomous_lane_keywords: Vec<String>,
    #[serde(default = "default_min_storm_share")]
    pub min_storm_share: f64,
}

impl Default for DecomposePolicy {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            max_micro_tasks: default_max_micro_tasks(),
            max_words_per_leaf: default_max_words_per_leaf(),
            min_minutes: default_min_minutes(),
            max_minutes: default_max_minutes(),
            max_groups: default_max_groups(),
            default_lane: default_lane(),
            storm_lane: default_storm_lane(),
            human_lane_keywords: Vec::new(),
            autonomous_lane_keywords: Vec::new(),
            min_storm_share: default_min_storm_share(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecomposeRequest {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub goal_text: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub creator_id: Option<String>,
    #[serde(default)]
    pub policy: DecomposePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub capability_id: String,
    pub adapter_kind: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseTask {
    pub micro_task_id: String,
    pub goal_id: String,
    pub objective_id: Option<String>,
    pub parent_id: Option<String>,
    pub depth: usize,
    pub index: usize,
    pub title: String,
    pub task_text: String,
    pub estimated_minutes: usize,
    pub success_criteria: Vec<String>,
    pub required_capability: String,
    pub profile_id: String,
    pub capability: Capability,
    pub suggested_lane: String,
    pub parallel_group: usize,
    pub parallel_priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposeResponse {
    pub ok: bool,
    pub tasks: Vec<BaseTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposePolicy {
    #[serde(default = "default_min_minutes")]
    pub min_minutes: usize,
    #[serde(default = "default_max_minutes")]
    pub max_minutes: usize,
    #[serde(default = "default_max_groups")]
    pub max_groups: usize,
    #[serde(default = "default_lane")]
    pub default_lane: String,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
}

impl Default for ComposePolicy {
    fn default() -> Self {
        Self {
            min_minutes: default_min_minutes(),
            max_minutes: default_max_minutes(),
            max_groups: default_max_groups(),
            default_lane: default_lane(),
            storm_lane: default_storm_lane(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComposeRequest {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub goal_text: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub creator_id: Option<String>,
    #[serde(default)]
    pub policy: ComposePolicy,
    #[serde(default)]
    pub tasks: Vec<BaseTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeResponse {
    pub ok: bool,
    pub tasks: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskSummaryRequest {
    #[serde(default)]
    pub tasks: Vec<Value>,
    #[serde(default)]
    pub shadow_only: bool,
    #[serde(default)]
    pub apply_executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummaryResponse {
    pub ok: bool,
    pub summary: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DispatchSummaryRequest {
    #[serde(default)]
    pub rows: Vec<Value>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchSummaryResponse {
    pub ok: bool,
    pub summary: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueRowsRequest {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub shadow_only: bool,
    #[serde(default)]
    pub passport_id: Option<String>,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
    #[serde(default)]
    pub tasks: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRowsResponse {
    pub ok: bool,
    pub weaver: Vec<Value>,
    pub storm: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DispatchRowsRequest {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub shadow_only: bool,
    #[serde(default)]
    pub apply_executed: bool,
    #[serde(default)]
    pub passport_id: Option<String>,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
    #[serde(default = "default_autonomous_executor")]
    pub autonomous_executor: String,
    #[serde(default = "default_storm_executor")]
    pub storm_executor: String,
    #[serde(default)]
    pub tasks: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRowsResponse {
    pub ok: bool,
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceApplyPolicy {
    #[serde(default = "default_lane")]
    pub default_lane: String,
    #[serde(default = "default_storm_lane")]
    pub storm_lane: String,
    #[serde(default = "default_min_storm_share")]
    pub min_storm_share: f64,
    #[serde(default = "default_block_on_constitution_deny")]
    pub block_on_constitution_deny: bool,
}

impl Default for GovernanceApplyPolicy {
    fn default() -> Self {
        Self {
            default_lane: default_lane(),
            storm_lane: default_storm_lane(),
            min_storm_share: default_min_storm_share(),
            block_on_constitution_deny: default_block_on_constitution_deny(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernanceApplyRequest {
    #[serde(default)]
    pub policy: GovernanceApplyPolicy,
    #[serde(default)]
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceApplyResponse {
    pub ok: bool,
    pub tasks: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectiveGateRequest {
    #[serde(default)]
    pub task_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveGateResponse {
    pub ok: bool,
    pub decision: String,
    pub risk: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutePrimitivesRequest {
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub tokens_est: i64,
    #[serde(default)]
    pub repeats_14d: i64,
    #[serde(default)]
    pub errors_30d: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteThresholdA {
    pub repeats_14d_min: i64,
    pub tokens_min: i64,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteThresholdB {
    pub tokens_min: i64,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteThresholdC {
    pub errors_30d_min: i64,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteThresholds {
    #[serde(rename = "A")]
    pub a: RouteThresholdA,
    #[serde(rename = "B")]
    pub b: RouteThresholdB,
    #[serde(rename = "C")]
    pub c: RouteThresholdC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePrimitivesResponse {
    pub ok: bool,
    pub intent_key: String,
    pub intent: String,
    pub predicted_habit_id: String,
    pub trigger_a: bool,
    pub trigger_b: bool,
    pub trigger_c: bool,
    pub any_trigger: bool,
    pub which_met: Vec<String>,
    pub thresholds: RouteThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteMatchHabit {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteMatchRequest {
    #[serde(default)]
    pub intent_key: String,
    #[serde(default)]
    pub skip_habit_id: String,
    #[serde(default)]
    pub habits: Vec<RouteMatchHabit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatchResponse {
    pub ok: bool,
    pub matched_habit_id: Option<String>,
    pub match_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteReflexRoutine {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteReflexMatchRequest {
    #[serde(default)]
    pub intent_key: String,
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub routines: Vec<RouteReflexRoutine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteReflexMatchResponse {
    pub ok: bool,
    pub matched_reflex_id: Option<String>,
    pub match_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteComplexityRequest {
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub tokens_est: i64,
    #[serde(default)]
    pub has_match: bool,
    #[serde(default)]
    pub any_trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteComplexityResponse {
    pub ok: bool,
    pub complexity: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteEvaluateRequest {
    #[serde(default)]
    pub task_text: String,
    #[serde(default)]
    pub tokens_est: i64,
    #[serde(default)]
    pub repeats_14d: i64,
    #[serde(default)]
    pub errors_30d: i64,
    #[serde(default)]
    pub skip_habit_id: String,
    #[serde(default)]
    pub habits: Vec<RouteMatchHabit>,
    #[serde(default)]
    pub reflex_routines: Vec<RouteReflexRoutine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEvaluateResponse {
    pub ok: bool,
    pub intent_key: String,
    pub intent: String,
    pub predicted_habit_id: String,
    pub trigger_a: bool,
    pub trigger_b: bool,
    pub trigger_c: bool,
    pub any_trigger: bool,
    pub which_met: Vec<String>,
    pub thresholds: RouteThresholds,
    pub matched_habit_id: Option<String>,
    pub matched_habit_strategy: String,
    pub matched_reflex_id: Option<String>,
    pub matched_reflex_strategy: String,
    pub complexity: String,
    pub complexity_reason: String,
}
