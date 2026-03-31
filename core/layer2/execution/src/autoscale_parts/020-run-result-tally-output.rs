#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResultTallyOutput {
    pub counts: std::collections::BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortedCountsInput {
    #[serde(default)]
    pub counts: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortedCountItem {
    pub result: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SortedCountsOutput {
    pub items: Vec<SortedCountItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeProposalStatusInput {
    #[serde(default)]
    pub raw_status: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeProposalStatusOutput {
    pub normalized_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalStatusForQueuePressureInput {
    #[serde(default)]
    pub overlay_decision: Option<String>,
    #[serde(default)]
    pub proposal_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalStatusForQueuePressureOutput {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalStatusInput {
    #[serde(default)]
    pub overlay_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalStatusOutput {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinutesSinceTsInput {
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinutesSinceTsOutput {
    #[serde(default)]
    pub minutes_since: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateWindowInput {
    #[serde(default)]
    pub end_date_str: Option<String>,
    #[serde(default)]
    pub days: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateWindowOutput {
    #[serde(default)]
    pub dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InWindowInput {
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub end_date_str: Option<String>,
    #[serde(default)]
    pub days: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InWindowOutput {
    pub in_window: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecWindowMatchInput {
    #[serde(default)]
    pub ts_ms: Option<f64>,
    #[serde(default)]
    pub start_ms: Option<f64>,
    #[serde(default)]
    pub end_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecWindowMatchOutput {
    pub in_window: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartOfNextUtcDayInput {
    #[serde(default)]
    pub date_str: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartOfNextUtcDayOutput {
    #[serde(default)]
    pub iso_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsoAfterMinutesInput {
    #[serde(default)]
    pub minutes: Option<f64>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsoAfterMinutesOutput {
    #[serde(default)]
    pub iso_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceHistoryMatchInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub event_capability_key: Option<String>,
    #[serde(default)]
    pub event_proposal_type: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub capability_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceHistoryMatchOutput {
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceCooldownKeyInput {
    #[serde(default)]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceCooldownKeyOutput {
    pub cooldown_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneWeightsInput {
    #[serde(default)]
    pub pressure: Option<String>,
    pub critical_weight: f64,
    pub standard_weight: f64,
    pub explore_weight: f64,
    pub quarantine_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QosLaneWeightsOutput {
    pub critical: f64,
    pub standard: f64,
    pub explore: f64,
    pub quarantine: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalOutcomeStatusInput {
    #[serde(default)]
    pub overlay_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalOutcomeStatusOutput {
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueUnderflowBackfillInput {
    pub underflow_backfill_max: f64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub overlay_outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueUnderflowBackfillOutput {
    pub allow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalRiskScoreInput {
    #[serde(default)]
    pub explicit_risk_score: Option<f64>,
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalRiskScoreOutput {
    pub risk_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalScoreInput {
    pub impact_weight: f64,
    pub risk_penalty: f64,
    pub age_hours: f64,
    pub is_stub: bool,
    pub no_change_count: f64,
    pub reverted_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalScoreOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalAdmissionPreviewInput {
    #[serde(default)]
    pub admission_preview: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalAdmissionPreviewOutput {
    #[serde(default)]
    pub preview: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpactWeightInput {
    #[serde(default)]
    pub expected_impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpactWeightOutput {
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskPenaltyInput {
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskPenaltyOutput {
    pub penalty: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensInput {
    #[serde(default)]
    pub expected_impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstimateTokensOutput {
    pub est_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalRemediationDepthInput {
    #[serde(default)]
    pub remediation_depth: Option<f64>,
    #[serde(default)]
    pub trigger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalRemediationDepthOutput {
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDedupKeyInput {
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub source_eye_id: Option<String>,
    #[serde(default)]
    pub remediation_kind: Option<String>,
    #[serde(default)]
    pub proposal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDedupKeyOutput {
    pub dedup_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalSemanticFingerprintInput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub source_eye: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub text_blob: Option<String>,
    #[serde(default)]
    pub stopwords: Vec<String>,
    #[serde(default)]
    pub min_tokens: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalSemanticFingerprintOutput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    pub proposal_type: String,
    #[serde(default)]
    pub source_eye: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub token_stems: Vec<String>,
    pub token_count: u32,
    pub eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticTokenSimilarityInput {
    #[serde(default)]
    pub left_tokens: Vec<String>,
    #[serde(default)]
    pub right_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticTokenSimilarityOutput {
    pub similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticContextComparableInput {
    #[serde(default)]
    pub left_proposal_type: Option<String>,
    #[serde(default)]
    pub right_proposal_type: Option<String>,
    #[serde(default)]
    pub left_source_eye: Option<String>,
    #[serde(default)]
    pub right_source_eye: Option<String>,
    #[serde(default)]
    pub left_objective_id: Option<String>,
    #[serde(default)]
    pub right_objective_id: Option<String>,
    #[serde(default)]
    pub require_same_type: bool,
    #[serde(default)]
    pub require_shared_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticContextComparableOutput {
    pub comparable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticNearDuplicateFingerprintInput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub source_eye: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub token_stems: Vec<String>,
    #[serde(default)]
    pub eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticNearDuplicateMatchInput {
    pub fingerprint: SemanticNearDuplicateFingerprintInput,
    #[serde(default)]
    pub seen_fingerprints: Vec<SemanticNearDuplicateFingerprintInput>,
    #[serde(default)]
    pub min_similarity: f64,
    #[serde(default)]
    pub require_same_type: bool,
    #[serde(default)]
    pub require_shared_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticNearDuplicateMatchOutput {
    pub matched: bool,
    pub similarity: f64,
    pub proposal_id: Option<String>,
    pub proposal_type: Option<String>,
    pub source_eye: Option<String>,
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyRankScoreInput {
    pub composite_weight: f64,
    pub actionability_weight: f64,
    pub directive_fit_weight: f64,
    pub signal_quality_weight: f64,
    pub expected_value_weight: f64,
    pub value_density_weight: f64,
    pub risk_penalty_weight: f64,
    pub time_to_value_weight: f64,
    pub composite: f64,
    pub actionability: f64,
    pub directive_fit: f64,
    pub signal_quality: f64,
    pub expected_value: f64,
    pub value_density: f64,
    pub risk_penalty: f64,
    pub time_to_value: f64,
    pub non_yield_penalty: f64,
    pub collective_shadow_penalty: f64,
    pub collective_shadow_bonus: f64,
}
