#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentProposalKeyCountEventInput {
    #[serde(default, alias = "proposalKey")]
    pub proposal_key: Option<String>,
    #[serde(default, alias = "tsMs")]
    pub ts_ms: Option<f64>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "attempt")]
    pub is_attempt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentProposalKeyCountsInput {
    #[serde(default)]
    pub events: Vec<RecentProposalKeyCountEventInput>,
    #[serde(default, alias = "cutoffMs")]
    pub cutoff_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentProposalKeyCountsOutput {
    pub counts: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityAttemptCountEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default, alias = "capabilityKey")]
    pub capability_key: Option<String>,
    #[serde(default, alias = "attempt")]
    pub is_attempt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityAttemptCountForDateInput {
    #[serde(default)]
    pub events: Vec<CapabilityAttemptCountEventInput>,
    #[serde(default)]
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityAttemptCountForDateOutput {
    pub count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityOutcomeStatsEventInput {
    #[serde(default, alias = "eventType")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default, alias = "capabilityKey")]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityOutcomeStatsInWindowInput {
    #[serde(default)]
    pub events: Vec<CapabilityOutcomeStatsEventInput>,
    #[serde(default)]
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityOutcomeStatsInWindowOutput {
    pub executed: f64,
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceHistoryEventInput {
    pub matched: bool,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceHistoryInput {
    #[serde(alias = "windowDays")]
    pub window_days: f64,
    #[serde(default, alias = "proposalType")]
    pub proposal_type: Option<String>,
    #[serde(default, alias = "capabilityKey")]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub events: Vec<ExecuteConfidenceHistoryEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceHistoryOutput {
    pub window_days: f64,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub capability_key: Option<String>,
    pub matched_events: f64,
    pub confidence_fallback: f64,
    pub route_blocked: f64,
    pub executed: f64,
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
    pub no_change_rate: f64,
    pub reverted_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidencePolicyInput {
    #[serde(default, alias = "proposalType")]
    pub proposal_type: Option<String>,
    #[serde(default, alias = "capabilityKey")]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default, alias = "executionMode")]
    pub execution_mode: Option<String>,
    #[serde(alias = "adaptiveEnabled")]
    pub adaptive_enabled: bool,
    #[serde(alias = "baseCompositeMargin")]
    pub base_composite_margin: f64,
    #[serde(alias = "baseValueMargin")]
    pub base_value_margin: f64,
    #[serde(alias = "lowRiskRelaxComposite")]
    pub low_risk_relax_composite: f64,
    #[serde(alias = "lowRiskRelaxValue")]
    pub low_risk_relax_value: f64,
    #[serde(alias = "fallbackRelaxEvery")]
    pub fallback_relax_every: f64,
    #[serde(alias = "fallbackRelaxStep")]
    pub fallback_relax_step: f64,
    #[serde(alias = "fallbackRelaxMax")]
    pub fallback_relax_max: f64,
    #[serde(alias = "fallbackRelaxMinExecuted")]
    pub fallback_relax_min_executed: f64,
    #[serde(alias = "fallbackRelaxMinShipped")]
    pub fallback_relax_min_shipped: f64,
    #[serde(alias = "fallbackRelaxMinShipRate")]
    pub fallback_relax_min_ship_rate: f64,
    #[serde(alias = "noChangeTightenMinExecuted")]
    pub no_change_tighten_min_executed: f64,
    #[serde(alias = "noChangeTightenThreshold")]
    pub no_change_tighten_threshold: f64,
    #[serde(alias = "noChangeTightenStep")]
    pub no_change_tighten_step: f64,
    #[serde(default)]
    pub history: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidencePolicyOutput {
    pub policy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveFitAssessmentInput {
    #[serde(alias = "minDirectiveFit")]
    pub min_directive_fit: f64,
    #[serde(alias = "profileAvailable")]
    pub profile_available: bool,
    #[serde(default, alias = "activeDirectiveIds")]
    pub active_directive_ids: Vec<String>,
    #[serde(default, alias = "positivePhraseHits")]
    pub positive_phrase_hits: Vec<String>,
    #[serde(default, alias = "positiveTokenHits")]
    pub positive_token_hits: Vec<String>,
    #[serde(default, alias = "strategyHits")]
    pub strategy_hits: Vec<String>,
    #[serde(default, alias = "negativePhraseHits")]
    pub negative_phrase_hits: Vec<String>,
    #[serde(default, alias = "negativeTokenHits")]
    pub negative_token_hits: Vec<String>,
    #[serde(alias = "strategyTokenCount")]
    pub strategy_token_count: f64,
    #[serde(default)]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveFitAssessmentOutput {
    pub pass: bool,
    pub score: f64,
    pub profile_available: bool,
    #[serde(default)]
    pub active_directive_ids: Vec<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub matched_positive: Vec<String>,
    #[serde(default)]
    pub matched_negative: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalQualityAssessmentInput {
    #[serde(alias = "minSignalQuality")]
    pub min_signal_quality: f64,
    #[serde(alias = "minSensorySignal")]
    pub min_sensory_signal: f64,
    #[serde(alias = "minSensoryRelevance")]
    pub min_sensory_relevance: f64,
    #[serde(alias = "minEyeScoreEma")]
    pub min_eye_score_ema: f64,
    #[serde(default)]
    pub eye_id: Option<String>,
    #[serde(default)]
    pub score_source: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub url_scheme: Option<String>,
    #[serde(default)]
    pub title_has_stub: bool,
    #[serde(default)]
    pub combined_item_score: Option<f64>,
    #[serde(default)]
    pub sensory_relevance_score: Option<f64>,
    #[serde(default)]
    pub sensory_relevance_tier: Option<String>,
    #[serde(default)]
    pub sensory_quality_score: Option<f64>,
    #[serde(default)]
    pub sensory_quality_tier: Option<String>,
    #[serde(default)]
    pub eye_known: bool,
    #[serde(default)]
    pub eye_status: Option<String>,
    #[serde(default)]
    pub eye_score_ema: Option<f64>,
    #[serde(default)]
    pub parser_type: Option<String>,
    #[serde(default)]
    pub parser_disallowed: bool,
    #[serde(default)]
    pub domain_allowlist_enforced: bool,
    #[serde(default)]
    pub domain_allowed: bool,
    #[serde(default)]
    pub eye_proposed_total: Option<f64>,
    #[serde(default)]
    pub eye_yield_rate: Option<f64>,
    pub calibration_eye_bias: f64,
    pub calibration_topic_bias: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalQualityAssessmentOutput {
    pub pass: bool,
    pub score: f64,
    pub score_source: String,
    pub eye_id: String,
    #[serde(default)]
    pub sensory_relevance_score: Option<f64>,
    #[serde(default)]
    pub sensory_relevance_tier: Option<String>,
    #[serde(default)]
    pub sensory_quality_score: Option<f64>,
    #[serde(default)]
    pub sensory_quality_tier: Option<String>,
    #[serde(default)]
    pub eye_status: Option<String>,
    #[serde(default)]
    pub eye_score_ema: Option<f64>,
    #[serde(default)]
    pub parser_type: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    pub calibration_eye_bias: f64,
    pub calibration_topic_bias: f64,
    pub calibration_total_bias: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionabilityAssessmentInput {
    pub min_actionability: f64,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
    pub validation_count: f64,
    pub specific_validation_count: f64,
    pub has_next_cmd: bool,
    pub generic_route_task: bool,
    pub next_cmd_has_dry_run: bool,
    pub looks_like_discovery_cmd: bool,
    pub has_action_verb: bool,
    pub has_opportunity: bool,
    pub has_concrete_target: bool,
    pub is_meta_coordination: bool,
    pub is_explainer: bool,
    pub mentions_proposal: bool,
    #[serde(default)]
    pub relevance_score: Option<f64>,
    #[serde(default)]
    pub directive_fit_score: Option<f64>,
    pub criteria_requirement_applied: bool,
    pub criteria_exempt_type: bool,
    pub criteria_min_count: f64,
    pub measurable_criteria_count: f64,
    pub criteria_total_count: f64,
    pub criteria_pattern_penalty: f64,
    #[serde(default)]
    pub criteria_pattern_hits: Option<serde_json::Value>,
    pub is_executable_proposal: bool,
    pub has_rollback_signal: bool,
    pub subdirective_required: bool,
    pub subdirective_has_concrete_target: bool,
    pub subdirective_has_expected_delta: bool,
    pub subdirective_has_verification_step: bool,
    pub subdirective_target_count: f64,
    pub subdirective_verify_count: f64,
    pub subdirective_success_criteria_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionabilityAssessmentOutput {
    pub pass: bool,
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub executable: bool,
    pub rollback_signal: bool,
    pub generic_next_command_template: bool,
    pub subdirective_v2: serde_json::Value,
    pub success_criteria: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyProfileInput {
    #[serde(default)]
    pub strategy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyProfileOutput {
    #[serde(default)]
    pub strategy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveStrategyVariantsInput {
    #[serde(default)]
    pub listed: Vec<serde_json::Value>,
    #[serde(default)]
    pub primary: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveStrategyVariantsOutput {
    #[serde(default)]
    pub variants: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyScorecardSummariesInput {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub summaries: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyScorecardSummaryItemOutput {
    pub score: f64,
    pub confidence: f64,
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyScorecardSummariesOutput {
    pub path: String,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub by_id: std::collections::BTreeMap<String, StrategyScorecardSummaryItemOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeFitnessPolicyInput {
    #[serde(default)]
    pub policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeFitnessPolicyOutput {
    pub policy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadEyesMapInput {
    #[serde(default)]
    pub cfg_eyes: Vec<serde_json::Value>,
    #[serde(default)]
    pub state_eyes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoadEyesMapOutput {
    #[serde(default)]
    pub eyes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FallbackDirectiveObjectiveIdsInput {
    #[serde(default)]
    pub directive_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FallbackDirectiveObjectiveIdsOutput {
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuePressureSnapshotInput {
    #[serde(default)]
    pub statuses: Vec<String>,
    #[serde(default)]
    pub warn_count: f64,
    #[serde(default)]
    pub critical_count: f64,
    #[serde(default)]
    pub warn_ratio: f64,
    #[serde(default)]
    pub critical_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueuePressureSnapshotOutput {
    pub total: u32,
    pub pending: u32,
    pub accepted: u32,
    pub closed: u32,
    pub rejected: u32,
    pub parked: u32,
    pub pending_ratio: f64,
    pub pressure: String,
    pub warn_ratio: f64,
    pub critical_ratio: f64,
    pub warn_count: f64,
    pub critical_count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseSuccessCriteriaRowsInput {
    #[serde(default)]
    pub action_rows: Vec<serde_json::Value>,
    #[serde(default)]
    pub verify_rows: Vec<serde_json::Value>,
    #[serde(default)]
    pub validation_rows: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseSuccessCriteriaRowOutput {
    pub source: String,
    pub metric: String,
    pub target: String,
    pub measurable: bool,
}
