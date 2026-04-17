#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetPacingGateInput {
    #[serde(alias = "estTokens", alias = "projected_tokens")]
    pub est_tokens: f64,
    #[serde(alias = "valueSignalScore", alias = "value_signal")]
    pub value_signal_score: f64,
    #[serde(default, alias = "riskLevel")]
    pub risk: Option<String>,
    #[serde(alias = "snapshotTight")]
    pub snapshot_tight: bool,
    #[serde(alias = "snapshotAutopauseActive", alias = "autopauseActive")]
    pub snapshot_autopause_active: bool,
    #[serde(alias = "snapshotRemainingRatio")]
    pub snapshot_remaining_ratio: f64,
    #[serde(default, alias = "snapshotPressure")]
    pub snapshot_pressure: Option<String>,
    #[serde(alias = "executionFloorDeficit")]
    pub execution_floor_deficit: bool,
    #[serde(alias = "executionReserveEnabled")]
    pub execution_reserve_enabled: bool,
    #[serde(alias = "executionReserveRemaining")]
    pub execution_reserve_remaining: f64,
    #[serde(alias = "executionReserveMinValueSignal")]
    pub execution_reserve_min_value_signal: f64,
    #[serde(alias = "budgetPacingEnabled")]
    pub budget_pacing_enabled: bool,
    #[serde(alias = "minRemainingRatio")]
    pub min_remaining_ratio: f64,
    #[serde(alias = "highTokenThreshold")]
    pub high_token_threshold: f64,
    #[serde(alias = "minValueSignalScore")]
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
    #[serde(default, alias = "capMap")]
    pub caps: std::collections::BTreeMap<String, f64>,
    #[serde(default, alias = "primaryKey")]
    pub primary_key: Option<String>,
    #[serde(default, alias = "aliasKeys")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCapOutput {
    #[serde(default)]
    pub cap: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensForCandidateInput {
    #[serde(alias = "directEstTokens")]
    pub direct_est_tokens: f64,
    #[serde(alias = "routeTokensEst")]
    pub route_tokens_est: f64,
    #[serde(alias = "fallbackEstimate")]
    pub fallback_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensForCandidateOutput {
    pub est_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneUsageEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "selectionMode")]
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
    #[serde(alias = "exploreUsage")]
    pub explore_usage: f64,
    #[serde(alias = "quarantineUsage")]
    pub quarantine_usage: f64,
    #[serde(alias = "executedCount")]
    pub executed_count: f64,
    #[serde(alias = "exploreMaxShare")]
    pub explore_max_share: f64,
    #[serde(alias = "quarantineMaxShare")]
    pub quarantine_max_share: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneShareCapExceededOutput {
    pub exceeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneFromCandidateInput {
    #[serde(alias = "queueUnderflowBackfill")]
    pub queue_underflow_backfill: bool,
    #[serde(alias = "pulseTier")]
    pub pulse_tier: i64,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(alias = "deprioritizedSource")]
    pub deprioritized_source: bool,
    #[serde(default, alias = "riskLevel")]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneFromCandidateOutput {
    pub lane: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default, alias = "evidenceRef")]
    pub evidence_ref: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeWindowCountInput {
    #[serde(default)]
    pub events: Vec<EyeOutcomeEventInput>,
    #[serde(default, alias = "eyeRef")]
    pub eye_ref: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default, alias = "endDateStr")]
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
    #[serde(default, alias = "eyeRef")]
    pub eye_ref: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub hours: Option<f64>,
    #[serde(default, alias = "nowMs")]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EyeOutcomeLastHoursCountOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoProgressResultInput {
    #[serde(default, alias = "eventType")]
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
    #[serde(default, alias = "eventType")]
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
    #[serde(default, alias = "eventType")]
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
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default, alias = "policyHold")]
    pub policy_hold: Option<bool>,
    #[serde(default, alias = "holdReason")]
    pub hold_reason: Option<String>,
    #[serde(default, alias = "routeBlockReason")]
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
    #[serde(default, alias = "holdReason")]
    pub hold_reason: Option<String>,
    #[serde(default, alias = "routeBlockReason")]
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
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default, alias = "proposalType")]
    pub proposal_type: Option<String>,
    #[serde(default, alias = "capabilityKey")]
    pub capability_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTypeFromRunEventOutput {
    pub proposal_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventObjectiveIdInput {
    #[serde(default, alias = "directivePulsePresent")]
    pub directive_pulse_present: Option<bool>,
    #[serde(default, alias = "directivePulseObjectiveId")]
    pub directive_pulse_objective_id: Option<String>,
    #[serde(default, alias = "objectiveIdPresent")]
    pub objective_id_present: Option<bool>,
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default, alias = "objectiveBindingPresent")]
    pub objective_binding_present: Option<bool>,
    #[serde(default, alias = "objectiveBindingObjectiveId")]
    pub objective_binding_objective_id: Option<String>,
    #[serde(default, alias = "topEscalationPresent")]
    pub top_escalation_present: Option<bool>,
    #[serde(default, alias = "topEscalationObjectiveId")]
    pub top_escalation_objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventObjectiveIdOutput {
    pub objective_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventProposalIdInput {
    #[serde(default, alias = "proposalIdPresent")]
    pub proposal_id_present: Option<bool>,
    #[serde(default, alias = "proposalId")]
    pub proposal_id: Option<String>,
    #[serde(default, alias = "selectedProposalIdPresent")]
    pub selected_proposal_id_present: Option<bool>,
    #[serde(default, alias = "selectedProposalId")]
    pub selected_proposal_id: Option<String>,
    #[serde(default, alias = "topEscalationPresent")]
    pub top_escalation_present: Option<bool>,
    #[serde(default, alias = "topEscalationProposalId")]
    pub top_escalation_proposal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunEventProposalIdOutput {
    pub proposal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "policyHold")]
    pub policy_hold: Option<bool>,
    #[serde(default, alias = "proposalId")]
    pub proposal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityCountedAttemptEventOutput {
    pub capacity_counted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepeatGateAnchorInput {
    #[serde(default, alias = "proposalId")]
    pub proposal_id: Option<String>,
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default, alias = "objectiveBindingPresent")]
    pub objective_binding_present: Option<bool>,
    #[serde(default, alias = "objectiveBindingPass")]
    pub objective_binding_pass: Option<bool>,
    #[serde(default, alias = "objectiveBindingRequired")]
    pub objective_binding_required: Option<bool>,
    #[serde(default, alias = "objectiveBindingSource")]
    pub objective_binding_source: Option<String>,
    #[serde(default, alias = "objectiveBindingValid")]
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
    #[serde(default, alias = "gateDecision")]
    pub gate_decision: Option<String>,
    #[serde(default, alias = "routeDecisionRaw")]
    pub route_decision_raw: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default, alias = "needsManualReview")]
    pub needs_manual_review: Option<bool>,
    #[serde(default)]
    pub executable: Option<bool>,
    #[serde(default, alias = "budgetBlockReason")]
    pub budget_block_reason: Option<String>,
    #[serde(default, alias = "budgetEnforcementReason")]
    pub budget_enforcement_reason: Option<String>,
    #[serde(default, alias = "budgetGlobalReason")]
    pub budget_global_reason: Option<String>,
    #[serde(default, alias = "summaryReason")]
    pub summary_reason: Option<String>,
    #[serde(default, alias = "routeReason")]
    pub route_reason: Option<String>,
    #[serde(default, alias = "budgetBlocked")]
    pub budget_blocked: Option<bool>,
    #[serde(default, alias = "budgetGlobalBlocked")]
    pub budget_global_blocked: Option<bool>,
    #[serde(default, alias = "budgetEnforcementBlocked")]
    pub budget_enforcement_blocked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPressureEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "policyHold")]
    pub policy_hold: Option<bool>,
    #[serde(default, alias = "tsMs")]
    pub ts_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPressureInput {
    #[serde(default)]
    pub events: Vec<PolicyHoldPressureEventInput>,
    #[serde(default, alias = "windowHours")]
    pub window_hours: Option<f64>,
    #[serde(default, alias = "minSamples")]
    pub min_samples: Option<f64>,
    #[serde(default, alias = "nowMs")]
    pub now_ms: Option<f64>,
    #[serde(default, alias = "warnRate")]
    pub warn_rate: Option<f64>,
    #[serde(default, alias = "hardRate")]
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
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default, alias = "holdReason")]
    pub hold_reason: Option<String>,
    #[serde(default, alias = "routeBlockReason")]
    pub route_block_reason: Option<String>,
    #[serde(default, alias = "policyHold")]
    pub policy_hold: Option<bool>,
    #[serde(default, alias = "tsMs")]
    pub ts_ms: Option<f64>,
}
