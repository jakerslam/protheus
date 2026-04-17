#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllDecisionEventsOutput {
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownActiveStateInput {
    #[serde(default, alias = "untilMs", alias = "until")]
    pub until_ms: Option<f64>,
    #[serde(default, alias = "nowMs", alias = "now")]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownActiveStateOutput {
    pub active: bool,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BumpCountInput {
    #[serde(default, alias = "currentCount", alias = "count")]
    pub current_count: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BumpCountOutput {
    pub count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockAgeMinutesInput {
    #[serde(default, alias = "lockTs", alias = "lockTimestamp")]
    pub lock_ts: Option<String>,
    #[serde(default, alias = "nowMs", alias = "now")]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockAgeMinutesOutput {
    #[serde(default)]
    pub age_minutes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HashObjInput {
    #[serde(default)]
    pub json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HashObjOutput {
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssessSuccessCriteriaQualityCheckInput {
    #[serde(default)]
    pub evaluated: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssessSuccessCriteriaQualityInput {
    #[serde(default)]
    pub checks: Vec<AssessSuccessCriteriaQualityCheckInput>,
    #[serde(default, alias = "totalCount")]
    pub total_count: f64,
    #[serde(default, alias = "unknownCount")]
    pub unknown_count: f64,
    #[serde(default, alias = "isSynthesized")]
    pub synthesized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssessSuccessCriteriaQualityOutput {
    pub insufficient: bool,
    pub reasons: Vec<String>,
    pub total_count: f64,
    pub unknown_count_raw: f64,
    pub unknown_exempt_count: f64,
    pub unknown_count: f64,
    pub unknown_rate: f64,
    pub unsupported_count: f64,
    pub unsupported_rate: f64,
    pub synthesized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManualGatePrefilterInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, alias = "capabilityKey", alias = "capability")]
    pub capability_key: Option<String>,
    #[serde(default, alias = "windowHours")]
    pub window_hours: f64,
    #[serde(default, alias = "minObservations")]
    pub min_observations: f64,
    #[serde(default, alias = "maxManualBlockRate")]
    pub max_manual_block_rate: f64,
    #[serde(default, alias = "rowPresent")]
    pub row_present: bool,
    #[serde(default)]
    pub attempts: f64,
    #[serde(default, alias = "manualBlocked")]
    pub manual_blocked: f64,
    #[serde(default, alias = "manualBlockRate")]
    pub manual_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManualGatePrefilterOutput {
    pub enabled: bool,
    pub applicable: bool,
    pub pass: bool,
    pub reason: String,
    pub capability_key: Option<String>,
    pub window_hours: f64,
    pub min_observations: f64,
    pub max_manual_block_rate: f64,
    pub attempts: f64,
    pub manual_blocked: f64,
    pub manual_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceCooldownActiveInput {
    #[serde(default)]
    pub cooldown_key: Option<String>,
    #[serde(default)]
    pub cooldown_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteConfidenceCooldownActiveOutput {
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopBiasSummaryEntryInput {
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub bias: f64,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub shipped: f64,
    #[serde(default)]
    pub no_change: f64,
    #[serde(default)]
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopBiasesSummaryInput {
    #[serde(default)]
    pub entries: Vec<TopBiasSummaryEntryInput>,
    #[serde(default, alias = "maxRows", alias = "topN")]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopBiasSummaryEntryOutput {
    pub key: String,
    pub bias: f64,
    pub total: f64,
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopBiasesSummaryOutput {
    pub rows: Vec<TopBiasSummaryEntryOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternPenaltyPatternInput {
    pub key: String,
    #[serde(default)]
    pub failures: f64,
    #[serde(default)]
    pub passes: f64,
    #[serde(default)]
    pub last_failure_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternPenaltyInput {
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<CriteriaPatternPenaltyPatternInput>,
    #[serde(default, alias = "failThreshold")]
    pub fail_threshold: f64,
    #[serde(default, alias = "penaltyPerHit")]
    pub penalty_per_hit: f64,
    #[serde(default, alias = "maxPenalty")]
    pub max_penalty: f64,
    #[serde(default, alias = "windowDays")]
    pub window_days: f64,
    #[serde(default, alias = "nowMs")]
    pub now_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternPenaltyHitOutput {
    pub key: String,
    pub failures: f64,
    pub passes: f64,
    pub effective_failures: f64,
    pub penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternPenaltyOutput {
    pub penalty: f64,
    pub hit_patterns: Vec<CriteriaPatternPenaltyHitOutput>,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyThresholdOverridesInput {
    #[serde(default)]
    pub min_signal_quality: Option<f64>,
    #[serde(default)]
    pub min_sensory_signal_score: Option<f64>,
    #[serde(default)]
    pub min_sensory_relevance_score: Option<f64>,
    #[serde(default)]
    pub min_directive_fit: Option<f64>,
    #[serde(default)]
    pub min_actionability_score: Option<f64>,
    #[serde(default)]
    pub min_eye_score_ema: Option<f64>,
    #[serde(default)]
    pub override_min_signal_quality: Option<f64>,
    #[serde(default)]
    pub override_min_sensory_signal_score: Option<f64>,
    #[serde(default)]
    pub override_min_sensory_relevance_score: Option<f64>,
    #[serde(default)]
    pub override_min_directive_fit: Option<f64>,
    #[serde(default)]
    pub override_min_actionability_score: Option<f64>,
    #[serde(default)]
    pub override_min_eye_score_ema: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyThresholdOverridesOutput {
    pub min_signal_quality: f64,
    pub min_sensory_signal_score: f64,
    pub min_sensory_relevance_score: f64,
    pub min_directive_fit: f64,
    pub min_actionability_score: f64,
    pub min_eye_score_ema: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectiveAllowedRisksInput {
    #[serde(default)]
    pub default_risks: Vec<String>,
    #[serde(default)]
    pub strategy_allowed_risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectiveAllowedRisksOutput {
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseContextObjectiveStatInput {
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub tier: Option<f64>,
    #[serde(default)]
    pub attempts: Option<f64>,
    #[serde(default)]
    pub shipped: Option<f64>,
    #[serde(default)]
    pub no_change: Option<f64>,
    #[serde(default)]
    pub reverted: Option<f64>,
    #[serde(default)]
    pub no_progress_streak: Option<f64>,
    #[serde(default)]
    pub last_attempt_ts: Option<String>,
    #[serde(default)]
    pub last_shipped_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseContextInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub window_days: f64,
    #[serde(default)]
    pub urgency_hours: f64,
    #[serde(default)]
    pub no_progress_limit: f64,
    #[serde(default)]
    pub cooldown_hours: f64,
    #[serde(default)]
    pub tier_attempts_today: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub attempts_today: f64,
    #[serde(default)]
    pub objective_stats: Vec<DirectivePulseContextObjectiveStatInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseContextObjectiveStatOutput {
    pub objective_id: String,
    pub tier: u32,
    pub attempts: u32,
    pub shipped: u32,
    pub no_change: u32,
    pub reverted: u32,
    pub no_progress_streak: u32,
    #[serde(default)]
    pub last_attempt_ts: Option<String>,
    #[serde(default)]
    pub last_shipped_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseContextOutput {
    pub enabled: bool,
    pub available: bool,
    pub objectives: Vec<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    pub window_days: f64,
    pub urgency_hours: f64,
    pub no_progress_limit: f64,
    pub cooldown_hours: f64,
    pub tier_attempts_today: std::collections::BTreeMap<String, f64>,
    pub attempts_today: f64,
    pub objective_stats: Vec<DirectivePulseContextObjectiveStatOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseStatsEventInput {
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub tier: Option<f64>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseStatsInput {
    #[serde(default)]
    pub date_str: Option<String>,
    #[serde(default)]
    pub window_days: Option<f64>,
    #[serde(default)]
    pub events: Vec<DirectivePulseStatsEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseStatsOutput {
    pub tier_attempts_today: std::collections::BTreeMap<String, f64>,
    pub attempts_today: f64,
    pub objective_stats: Vec<DirectivePulseContextObjectiveStatOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompileDirectivePulseObjectivesInput {
    #[serde(default)]
    pub directives: Vec<serde_json::Value>,
    #[serde(default)]
    pub stopwords: Vec<String>,
    #[serde(default)]
    pub allowed_value_keys: Vec<String>,
    #[serde(default)]
    pub t1_min_share: Option<f64>,
    #[serde(default)]
    pub t2_min_share: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompileDirectivePulseObjectiveOutput {
    pub id: String,
    pub tier: u32,
    pub title: String,
    pub tier_weight: f64,
    pub min_share: f64,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub tokens: Vec<String>,
    #[serde(default)]
    pub value_currencies: Vec<String>,
    #[serde(default)]
    pub primary_currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompileDirectivePulseObjectivesOutput {
    #[serde(default)]
    pub objectives: Vec<CompileDirectivePulseObjectiveOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseObjectivesProfileInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub load_error: Option<String>,
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectivePulseObjectivesProfileOutput {
    pub enabled: bool,
    pub available: bool,
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentDirectivePulseCooldownEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub sample_objective_id: Option<String>,
}
