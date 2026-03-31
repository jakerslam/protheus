// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Duration, NaiveDate, SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuePressure {
    pub pressure: String,
    pub pending: f64,
    pub pending_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanInput {
    pub queue_pressure: QueuePressure,
    pub min_cells: u32,
    pub max_cells: u32,
    pub current_cells: u32,
    pub run_interval_minutes: f64,
    pub idle_release_minutes: f64,
    pub autopause_active: bool,
    pub last_run_minutes_ago: Option<f64>,
    pub last_high_pressure_minutes_ago: Option<f64>,
    pub trit_shadow_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanOutput {
    pub action: String,
    pub reason: String,
    pub pressure: String,
    pub pending: f64,
    pub pending_ratio: f64,
    pub current_cells: u32,
    pub target_cells: u32,
    pub warning_pressure: bool,
    pub high_pressure: bool,
    pub pressure_active: bool,
    pub cooldown_active: bool,
    pub idle_release_ready: bool,
    pub budget_blocked: bool,
    pub trit_shadow_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchMaxInput {
    pub enabled: bool,
    pub max_batch: u32,
    pub daily_remaining: Option<u32>,
    pub pressure: String,
    pub current_cells: u32,
    pub budget_blocked: bool,
    pub trit_shadow_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchMaxOutput {
    pub max: u32,
    pub reason: String,
    pub pressure: String,
    pub current_cells: u32,
    pub budget_blocked: bool,
    pub trit_shadow_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicCapsInput {
    pub enabled: bool,
    pub base_daily_cap: u32,
    #[serde(default)]
    pub base_canary_cap: Option<u32>,
    pub candidate_pool_size: u32,
    pub queue_pressure: String,
    pub policy_hold_level: String,
    pub policy_hold_applicable: bool,
    pub spawn_boost_enabled: bool,
    pub spawn_boost_active: bool,
    pub shipped_today: f64,
    pub no_progress_streak: f64,
    pub gate_exhaustion_streak: f64,
    pub warn_factor: f64,
    pub critical_factor: f64,
    pub min_input_pool: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicCapsOutput {
    pub enabled: bool,
    pub daily_runs_cap: u32,
    pub canary_daily_exec_cap: Option<u32>,
    pub input_candidates_cap: Option<u32>,
    #[serde(rename = "inputCandidateCap")]
    pub input_candidate_cap_alias: Option<u32>,
    pub low_yield: bool,
    pub high_yield: bool,
    pub spawn_reset_active: bool,
    pub queue_pressure: String,
    pub policy_hold_level: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsageInput {
    #[serde(default)]
    pub selected_model_tokens_est: Option<f64>,
    #[serde(default)]
    pub route_budget_request_tokens_est: Option<f64>,
    #[serde(default)]
    pub route_tokens_est: Option<f64>,
    #[serde(default)]
    pub fallback_est_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_prompt_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_input_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_completion_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_output_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_total_tokens: Option<f64>,
    #[serde(default)]
    pub metrics_tokens_used: Option<f64>,
    #[serde(default)]
    pub metrics_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsageOutput {
    pub available: bool,
    pub source: String,
    pub actual_prompt_tokens: Option<f64>,
    pub actual_completion_tokens: Option<f64>,
    pub actual_total_tokens: Option<f64>,
    pub estimated_tokens: f64,
    pub effective_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeQueueInput {
    #[serde(default)]
    pub pressure: Option<String>,
    #[serde(default)]
    pub pending: Option<f64>,
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub pending_ratio: Option<f64>,
    pub warn_pending_count: f64,
    pub critical_pending_count: f64,
    pub warn_pending_ratio: f64,
    pub critical_pending_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeQueueOutput {
    pub pressure: String,
    pub pending: f64,
    pub total: f64,
    pub pending_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaGateInput {
    #[serde(default)]
    pub min_count: Option<f64>,
    #[serde(default)]
    pub total_count: Option<f64>,
    #[serde(default)]
    pub contract_not_allowed_count: Option<f64>,
    #[serde(default)]
    pub unsupported_count: Option<f64>,
    #[serde(default)]
    pub structurally_supported_count: Option<f64>,
    #[serde(default)]
    pub contract_violation_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaGateOutput {
    pub pass: bool,
    pub reasons: Vec<String>,
    pub min_count: f64,
    pub total_count: f64,
    pub supported_count: f64,
    pub unsupported_count: f64,
    pub contract_violation_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralPreviewCriteriaFailureInput {
    #[serde(default)]
    pub primary_failure: Option<String>,
    #[serde(default)]
    pub contract_not_allowed_count: Option<f64>,
    #[serde(default)]
    pub unsupported_count: Option<f64>,
    #[serde(default)]
    pub total_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuralPreviewCriteriaFailureOutput {
    pub has_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldInput {
    pub target: String,
    pub gate_decision: String,
    pub route_decision: String,
    pub needs_manual_review: bool,
    pub executable: bool,
    #[serde(default)]
    pub budget_reason: String,
    #[serde(default)]
    pub route_reason: String,
    pub budget_blocked_flag: bool,
    pub budget_global_blocked: bool,
    pub budget_enforcement_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldOutput {
    pub hold: bool,
    pub hold_scope: Option<String>,
    pub hold_reason: Option<String>,
    pub route_block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldResultInput {
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldResultOutput {
    pub is_policy_hold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldRunEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldRunEventOutput {
    pub is_policy_hold_run_event: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyResultInput {
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyResultOutput {
    pub is_score_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyFailureLikeInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub preview_verification_present: Option<bool>,
    #[serde(default)]
    pub preview_verification_passed: Option<bool>,
    #[serde(default)]
    pub preview_verification_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyFailureLikeOutput {
    pub is_failure_like: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateExhaustedAttemptInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateExhaustedAttemptOutput {
    pub is_gate_exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveGateExhaustedAttemptEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveGateExhaustedAttemptsInput {
    #[serde(default)]
    pub events: Vec<ConsecutiveGateExhaustedAttemptEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveGateExhaustedAttemptsOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunsSinceResetEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunsSinceResetIndexInput {
    #[serde(default)]
    pub events: Vec<RunsSinceResetEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunsSinceResetIndexOutput {
    pub start_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptEventIndexEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptEventIndicesInput {
    #[serde(default)]
    pub events: Vec<AttemptEventIndexEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptEventIndicesOutput {
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptIndexEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub proposal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptIndicesInput {
    #[serde(default)]
    pub events: Vec<CapacityCountedAttemptIndexEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptIndicesOutput {
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveNoProgressEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveNoProgressRunsInput {
    #[serde(default)]
    pub events: Vec<ConsecutiveNoProgressEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsecutiveNoProgressRunsOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShippedCountEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShippedCountInput {
    #[serde(default)]
    pub events: Vec<ShippedCountEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShippedCountOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutedCountByRiskEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub proposal_risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutedCountByRiskInput {
    #[serde(default)]
    pub events: Vec<ExecutedCountByRiskEventInput>,
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutedCountByRiskOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResultTallyEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResultTallyInput {
    #[serde(default)]
    pub events: Vec<RunResultTallyEventInput>,
}
