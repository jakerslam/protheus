#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeBandMapOutput {
    pub novice: f64,
    pub developing: f64,
    pub mature: f64,
    pub seasoned: f64,
    pub legendary: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeImpactMapInput {
    #[serde(default)]
    pub raw: Option<Value>,
    #[serde(default)]
    pub base: Option<Value>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeImpactMapOutput {
    pub low: f64,
    pub medium: f64,
    pub high: f64,
    pub critical: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTargetMapInput {
    #[serde(default)]
    pub raw: Option<Value>,
    #[serde(default)]
    pub base: Option<Value>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeTargetMapOutput {
    pub tactical: f64,
    pub belief: f64,
    pub identity: f64,
    pub directive: f64,
    pub constitution: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTargetPolicyInput {
    #[serde(default)]
    pub raw: Option<Value>,
    #[serde(default)]
    pub base: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTargetPolicyOutput {
    pub rank: i64,
    pub live_enabled: bool,
    pub test_enabled: bool,
    pub require_human_veto_live: bool,
    pub min_shadow_hours: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WindowDaysForTargetInput {
    #[serde(default)]
    pub window_map: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub fallback: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WindowDaysForTargetOutput {
    pub days: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TierRetentionDaysInput {
    #[serde(default)]
    pub policy: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TierRetentionDaysOutput {
    pub days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InversionCandidateRow {
    pub id: String,
    pub filters: Vec<String>,
    pub source: String,
    pub probability: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseCandidateListFromLlmPayloadInput {
    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParseCandidateListFromLlmPayloadOutput {
    pub candidates: Vec<InversionCandidateRow>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HeuristicFilterCandidatesInput {
    #[serde(default)]
    pub objective: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HeuristicFilterCandidatesOutput {
    pub candidates: Vec<InversionCandidateRow>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ScoreTrialInput {
    #[serde(default)]
    pub decision: Option<Value>,
    #[serde(default)]
    pub candidate: Option<Value>,
    #[serde(default)]
    pub trial_cfg: Option<Value>,
    #[serde(default)]
    pub runtime_probe_pass: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScoreTrialOutput {
    pub score: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MutateTrialCandidatesInput {
    #[serde(default)]
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MutateTrialCandidatesOutput {
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeIsoEventsInput {
    #[serde(default)]
    pub src: Vec<Value>,
    #[serde(default)]
    pub max_rows: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeIsoEventsOutput {
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExpandLegacyCountToEventsInput {
    #[serde(default)]
    pub count: Option<Value>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExpandLegacyCountToEventsOutput {
    pub events: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTierEventMapInput {
    #[serde(default)]
    pub src: Option<Value>,
    #[serde(default)]
    pub fallback: Option<Value>,
    #[serde(default)]
    pub legacy_counts: Option<Value>,
    #[serde(default)]
    pub legacy_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTierEventMapOutput {
    pub map: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultTierScopeInput {
    #[serde(default)]
    pub legacy: Option<Value>,
    #[serde(default)]
    pub legacy_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultTierScopeOutput {
    pub scope: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTierScopeInput {
    #[serde(default)]
    pub scope: Option<Value>,
    #[serde(default)]
    pub legacy: Option<Value>,
    #[serde(default)]
    pub legacy_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTierScopeOutput {
    pub scope: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultTierGovernanceStateInput {
    #[serde(default)]
    pub policy_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultTierGovernanceStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CloneTierScopeInput {
    #[serde(default)]
    pub scope: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CloneTierScopeOutput {
    pub scope: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PruneTierScopeEventsInput {
    #[serde(default)]
    pub scope: Option<Value>,
    #[serde(default)]
    pub retention_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PruneTierScopeEventsOutput {
    pub scope: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CountTierEventsInput {
    #[serde(default)]
    pub scope: Option<Value>,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub window_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CountTierEventsOutput {
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EffectiveWindowDaysForTargetInput {
    #[serde(default)]
    pub window_map: Option<Value>,
    #[serde(default)]
    pub minimum_window_map: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub fallback: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EffectiveWindowDaysForTargetOutput {
    pub days: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ToDateInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToDateOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ParseTsMsInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ParseTsMsOutput {
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AddMinutesInput {
    #[serde(default)]
    pub iso_ts: Option<String>,
    #[serde(default)]
    pub minutes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AddMinutesOutput {
    pub iso_ts: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClampIntInput {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub lo: Option<i64>,
    #[serde(default)]
    pub hi: Option<i64>,
    #[serde(default)]
    pub fallback: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClampIntOutput {
    pub value: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClampNumberInput {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClampNumberOutput {
    pub value: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ToBoolInput {
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub fallback: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToBoolOutput {
    pub value: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CleanTextInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub max_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CleanTextOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeTokenInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub max_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeTokenOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeWordTokenInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub max_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeWordTokenOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BandToIndexInput {
    #[serde(default)]
    pub band: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BandToIndexOutput {
    pub index: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EscapeRegexInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EscapeRegexOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PatternToWordRegexInput {
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub max_len: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PatternToWordRegexOutput {
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StableIdInput {
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StableIdOutput {
    pub id: String,
}
