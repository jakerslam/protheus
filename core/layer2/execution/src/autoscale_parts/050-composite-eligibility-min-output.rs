#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeEligibilityMinOutput {
    pub min_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClampThresholdInput {
    #[serde(default, alias = "threshold_name", alias = "metric_name")]
    pub name: Option<String>,
    #[serde(default, alias = "threshold_value", alias = "metric_value")]
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClampThresholdOutput {
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedThresholdsInput {
    #[serde(default, alias = "base_thresholds")]
    pub base: std::collections::BTreeMap<String, f64>,
    #[serde(default, alias = "delta_thresholds")]
    pub deltas: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedThresholdsOutput {
    #[serde(default)]
    pub thresholds: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractEyeFromEvidenceRefInput {
    #[serde(default, alias = "evidence_ref", alias = "evidenceRef")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractEyeFromEvidenceRefOutput {
    #[serde(default)]
    pub eye_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TotalOutcomesInput {
    #[serde(default, alias = "shipped_count")]
    pub shipped: f64,
    #[serde(default, alias = "no_change_count")]
    pub no_change: f64,
    #[serde(default, alias = "reverted_count")]
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TotalOutcomesOutput {
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeriveEntityBiasInput {
    #[serde(default, alias = "shipped_count")]
    pub shipped: f64,
    #[serde(default, alias = "no_change_count")]
    pub no_change: f64,
    #[serde(default, alias = "reverted_count")]
    pub reverted: f64,
    #[serde(default, alias = "min_samples")]
    pub min_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeriveEntityBiasOutput {
    pub bias: f64,
    pub total: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildOverlayEventInput {
    #[serde(default, alias = "proposalId")]
    pub proposal_id: Option<String>,
    #[serde(default, rename = "type", alias = "event_type", alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default, alias = "decisionReason")]
    pub decision: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default, alias = "decision_reason")]
    pub reason: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default, alias = "evidenceRef")]
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildOverlayInput {
    #[serde(default)]
    pub events: Vec<BuildOverlayEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildOverlayOutcomeCountsOutput {
    pub shipped: u32,
    pub reverted: u32,
    pub no_change: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildOverlayEntryOutput {
    pub proposal_id: String,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub decision_ts: Option<String>,
    #[serde(default)]
    pub decision_reason: Option<String>,
    #[serde(default)]
    pub last_outcome: Option<String>,
    #[serde(default)]
    pub last_outcome_ts: Option<String>,
    #[serde(default)]
    pub last_evidence_ref: Option<String>,
    pub outcomes: BuildOverlayOutcomeCountsOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildOverlayOutput {
    #[serde(default)]
    pub entries: Vec<BuildOverlayEntryOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasAdaptiveMutationSignalInput {
    #[serde(default, alias = "proposalType")]
    pub proposal_type: Option<String>,
    #[serde(default, alias = "adaptiveMutation")]
    pub adaptive_mutation: bool,
    #[serde(default, alias = "mutationProposal")]
    pub mutation_proposal: bool,
    #[serde(default, alias = "topologyMutation")]
    pub topology_mutation: bool,
    #[serde(default, alias = "selfImprovementChange")]
    pub self_improvement_change: bool,
    #[serde(default, alias = "signalBlob")]
    pub signal_blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasAdaptiveMutationSignalOutput {
    pub has_signal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveMutationExecutionGuardInput {
    #[serde(default)]
    pub guard_required: bool,
    #[serde(default)]
    pub applies: bool,
    #[serde(default)]
    pub metadata_applies: bool,
    #[serde(default)]
    pub guard_pass: bool,
    #[serde(default)]
    pub guard_reason: Option<String>,
    #[serde(default)]
    pub safety_attestation: Option<String>,
    #[serde(default)]
    pub rollback_receipt: Option<String>,
    #[serde(default)]
    pub guard_receipt_id: Option<String>,
    #[serde(default)]
    pub mutation_kernel_applies: bool,
    #[serde(default)]
    pub mutation_kernel_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveMutationExecutionGuardControlsOutput {
    #[serde(default)]
    pub safety_attestation: Option<String>,
    #[serde(default)]
    pub rollback_receipt: Option<String>,
    #[serde(default)]
    pub guard_receipt_id: Option<String>,
    pub mutation_kernel_applies: bool,
    pub mutation_kernel_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveMutationExecutionGuardOutput {
    pub required: bool,
    pub applies: bool,
    pub pass: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub controls: AdaptiveMutationExecutionGuardControlsOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategySelectionVariantInput {
    #[serde(default, alias = "strategyId")]
    pub strategy_id: Option<String>,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default, alias = "executionMode")]
    pub execution_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategySelectionInput {
    #[serde(default, alias = "date")]
    pub date_str: Option<String>,
    #[serde(default, alias = "attemptIndex")]
    pub attempt_index: f64,
    #[serde(default)]
    pub canary_enabled: bool,
    #[serde(default, alias = "canaryAllowExecute")]
    pub canary_allow_execute: bool,
    #[serde(default)]
    pub canary_fraction: f64,
    #[serde(default, alias = "maxActive")]
    pub max_active: f64,
    #[serde(default, alias = "fallbackStrategyId")]
    pub fallback_strategy_id: Option<String>,
    #[serde(default)]
    pub variants: Vec<StrategySelectionVariantInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategySelectionRankedOutput {
    pub strategy_id: String,
    pub score: f64,
    pub confidence: f64,
    #[serde(default)]
    pub stage: Option<String>,
    pub execution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategySelectionOutput {
    #[serde(default)]
    pub selected_strategy_id: Option<String>,
    pub mode: String,
    pub canary_enabled: bool,
    pub canary_due: bool,
    #[serde(default)]
    pub canary_every: Option<u32>,
    pub attempt_index: u32,
    pub active_count: u32,
    #[serde(default)]
    pub ranked: Vec<StrategySelectionRankedOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationDeltasInput {
    #[serde(default)]
    pub executed_count: f64,
    #[serde(default)]
    pub shipped_rate: f64,
    #[serde(default)]
    pub no_change_rate: f64,
    #[serde(default)]
    pub reverted_rate: f64,
    #[serde(default)]
    pub exhausted: f64,
    #[serde(default)]
    pub min_executed: f64,
    #[serde(default)]
    pub tighten_min_executed: f64,
    #[serde(default)]
    pub loosen_low_shipped_rate: f64,
    #[serde(default)]
    pub loosen_exhausted_threshold: f64,
    #[serde(default)]
    pub tighten_min_shipped_rate: f64,
    #[serde(default)]
    pub max_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalibrationDeltasOutput {
    pub min_signal_quality: f64,
    pub min_sensory_signal_score: f64,
    pub min_sensory_relevance_score: f64,
    pub min_directive_fit: f64,
    pub min_actionability_score: f64,
    pub min_eye_score_ema: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyAdmissionMutationGuardInput {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub applies: bool,
    #[serde(default)]
    pub pass: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub controls: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyAdmissionDecisionInput {
    #[serde(default)]
    pub require_admission_preview: bool,
    #[serde(default)]
    pub preview_eligible: bool,
    #[serde(default)]
    pub preview_blocked_by: Vec<String>,
    #[serde(default)]
    pub mutation_guard: Option<StrategyAdmissionMutationGuardInput>,
    #[serde(default)]
    pub strategy_type_allowed: bool,
    #[serde(default)]
    pub max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub strategy_max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub hard_max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub risk_score: Option<f64>,
    #[serde(default)]
    pub remediation_check_required: bool,
    #[serde(default)]
    pub remediation_depth: Option<f64>,
    #[serde(default)]
    pub remediation_max_depth: Option<f64>,
    #[serde(default)]
    pub dedup_key: Option<String>,
    #[serde(default)]
    pub duplicate_window_hours: Option<f64>,
    #[serde(default)]
    pub recent_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyAdmissionPreviewOutput {
    pub eligible: bool,
    #[serde(default)]
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyAdmissionDecisionOutput {
    pub allow: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub admission_preview: Option<StrategyAdmissionPreviewOutput>,
    #[serde(default)]
    pub mutation_guard: Option<StrategyAdmissionMutationGuardInput>,
    #[serde(default)]
    pub risk_score: Option<f64>,
    #[serde(default)]
    pub max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub strategy_max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub hard_max_risk_per_action: Option<f64>,
    #[serde(default)]
    pub duplicate_window_hours: Option<f64>,
    #[serde(default)]
    pub recent_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectedValueScoreInput {
    #[serde(default)]
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpectedValueScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestRunBatchMaxInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub batch_max: f64,
    #[serde(default)]
    pub batch_reason: Option<String>,
    #[serde(default)]
    pub daily_remaining: f64,
    #[serde(default)]
    pub autoscale_hint: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestRunBatchMaxOutput {
    pub enabled: bool,
    pub max: f64,
    pub reason: String,
    pub daily_remaining: f64,
    pub autoscale_hint: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacklogAutoscaleSnapshotInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub state: serde_json::Value,
    #[serde(default)]
    pub queue: serde_json::Value,
    #[serde(default)]
    pub current_cells: f64,
    #[serde(default)]
    pub plan: serde_json::Value,
    #[serde(default)]
    pub trit_productivity: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BacklogAutoscaleSnapshotOutput {
    pub enabled: bool,
    pub module: String,
    pub state: serde_json::Value,
    pub queue: serde_json::Value,
    pub current_cells: f64,
    pub plan: serde_json::Value,
    pub trit_productivity: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdmissionSummaryProposalInput {
    #[serde(default)]
    pub preview_eligible: Option<bool>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdmissionSummaryInput {
    #[serde(default)]
    pub proposals: Vec<AdmissionSummaryProposalInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdmissionSummaryOutput {
    pub total: u32,
    pub eligible: u32,
    pub blocked: u32,
    pub blocked_by_reason: std::collections::BTreeMap<String, u32>,
}
