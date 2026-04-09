#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetPacingGateInput {
    pub est_tokens: f64,
    pub value_signal_score: f64,
    #[serde(default)]
    pub risk: Option<String>,
    pub snapshot_tight: bool,
    pub snapshot_autopause_active: bool,
    pub snapshot_remaining_ratio: f64,
    #[serde(default)]
    pub snapshot_pressure: Option<String>,
    pub execution_floor_deficit: bool,
    pub execution_reserve_enabled: bool,
    pub execution_reserve_remaining: f64,
    pub execution_reserve_min_value_signal: f64,
    pub budget_pacing_enabled: bool,
    pub min_remaining_ratio: f64,
    pub high_token_threshold: f64,
    pub min_value_signal_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetPacingGateOutput {
    pub pass: bool,
    #[serde(default)]
    pub reason: Option<String>,
    pub execution_reserve_bypass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCapInput {
    #[serde(default)]
    pub caps: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub primary_key: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCapOutput {
    #[serde(default)]
    pub cap: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensForCandidateInput {
    pub direct_est_tokens: f64,
    pub route_tokens_est: f64,
    pub fallback_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensForCandidateOutput {
    pub est_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneUsageEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub selection_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneUsageInput {
    #[serde(default)]
    pub events: Vec<QosLaneUsageEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneUsageOutput {
    pub critical: u32,
    pub standard: u32,
    pub explore: u32,
    pub quarantine: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneShareCapExceededInput {
    #[serde(default)]
    pub lane: Option<String>,
    pub explore_usage: f64,
    pub quarantine_usage: f64,
    pub executed_count: f64,
    pub explore_max_share: f64,
    pub quarantine_max_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneShareCapExceededOutput {
    pub exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneFromCandidateInput {
    pub queue_underflow_backfill: bool,
    pub pulse_tier: i64,
    #[serde(default)]
    pub proposal_type: Option<String>,
    pub deprioritized_source: bool,
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneFromCandidateOutput {
    pub lane: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub evidence_ref: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeWindowCountInput {
    #[serde(default)]
    pub events: Vec<EyeOutcomeEventInput>,
    #[serde(default)]
    pub eye_ref: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub end_date_str: Option<String>,
    #[serde(default)]
    pub days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeWindowCountOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeLastHoursCountInput {
    #[serde(default)]
    pub events: Vec<EyeOutcomeEventInput>,
    #[serde(default)]
    pub eye_ref: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub hours: Option<f64>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeLastHoursCountOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoProgressResultInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoProgressResultOutput {
    pub is_no_progress: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptRunEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttemptRunEventOutput {
    pub is_attempt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafetyStopRunEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SafetyStopRunEventOutput {
    pub is_safety_stop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldCategoryInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub route_block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldCategoryOutput {
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldReasonInput {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub route_block_reason: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonYieldReasonOutput {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTypeFromRunEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub capability_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTypeFromRunEventOutput {
    pub proposal_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventObjectiveIdInput {
    #[serde(default)]
    pub directive_pulse_present: Option<bool>,
    #[serde(default)]
    pub directive_pulse_objective_id: Option<String>,
    #[serde(default)]
    pub objective_id_present: Option<bool>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub objective_binding_present: Option<bool>,
    #[serde(default)]
    pub objective_binding_objective_id: Option<String>,
    #[serde(default)]
    pub top_escalation_present: Option<bool>,
    #[serde(default)]
    pub top_escalation_objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventObjectiveIdOutput {
    pub objective_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventProposalIdInput {
    #[serde(default)]
    pub proposal_id_present: Option<bool>,
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub selected_proposal_id_present: Option<bool>,
    #[serde(default)]
    pub selected_proposal_id: Option<String>,
    #[serde(default)]
    pub top_escalation_present: Option<bool>,
    #[serde(default)]
    pub top_escalation_proposal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventProposalIdOutput {
    pub proposal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptEventInput {
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
pub struct CapacityCountedAttemptEventOutput {
    pub capacity_counted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatGateAnchorInput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub objective_binding_present: Option<bool>,
    #[serde(default)]
    pub objective_binding_pass: Option<bool>,
    #[serde(default)]
    pub objective_binding_required: Option<bool>,
    #[serde(default)]
    pub objective_binding_source: Option<String>,
    #[serde(default)]
    pub objective_binding_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatGateAnchorBindingOutput {
    pub pass: bool,
    pub required: bool,
    pub objective_id: String,
    pub source: String,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatGateAnchorOutput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub objective_binding: Option<RepeatGateAnchorBindingOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteExecutionPolicyHoldInput {
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub gate_decision: Option<String>,
    #[serde(default)]
    pub route_decision_raw: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub needs_manual_review: Option<bool>,
    #[serde(default)]
    pub executable: Option<bool>,
    #[serde(default)]
    pub budget_block_reason: Option<String>,
    #[serde(default)]
    pub budget_enforcement_reason: Option<String>,
    #[serde(default)]
    pub budget_global_reason: Option<String>,
    #[serde(default)]
    pub summary_reason: Option<String>,
    #[serde(default)]
    pub route_reason: Option<String>,
    #[serde(default)]
    pub budget_blocked: Option<bool>,
    #[serde(default)]
    pub budget_global_blocked: Option<bool>,
    #[serde(default)]
    pub budget_enforcement_blocked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPressureEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub ts_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPressureInput {
    #[serde(default)]
    pub events: Vec<PolicyHoldPressureEventInput>,
    #[serde(default)]
    pub window_hours: Option<f64>,
    #[serde(default)]
    pub min_samples: Option<f64>,
    #[serde(default)]
    pub now_ms: Option<f64>,
    #[serde(default)]
    pub warn_rate: Option<f64>,
    #[serde(default)]
    pub hard_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPressureOutput {
    pub window_hours: f64,
    pub min_samples: f64,
    pub samples: u32,
    pub policy_holds: u32,
    pub rate: f64,
    pub level: String,
    pub applicable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPatternEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub route_block_reason: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub ts_ms: Option<f64>,
}
