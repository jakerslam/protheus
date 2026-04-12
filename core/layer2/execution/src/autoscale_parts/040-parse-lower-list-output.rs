#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ParseLowerListOutput {
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanaryFailedChecksAllowedInput {
    #[serde(default)]
    pub failed_checks: Vec<String>,
    #[serde(default)]
    pub allowed_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanaryFailedChecksAllowedOutput {
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTextBlobEvidenceEntryInput {
    #[serde(default)]
    pub evidence_ref: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTextBlobInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub suggested_next_command: Option<String>,
    #[serde(default)]
    pub suggested_command: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub evidence: Vec<ProposalTextBlobEvidenceEntryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalTextBlobOutput {
    pub blob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PercentMentionsFromTextInput {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PercentMentionsFromTextOutput {
    #[serde(default)]
    pub values: Vec<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationMinDeltaPercentInput {
    #[serde(default)]
    pub high_accuracy_mode: bool,
    #[serde(default)]
    pub high_accuracy_value: f64,
    #[serde(default)]
    pub base_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationMinDeltaPercentOutput {
    pub min_delta_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEyeRefInput {
    #[serde(default)]
    pub meta_source_eye: Option<String>,
    #[serde(default)]
    pub first_evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEyeRefOutput {
    pub eye_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedRiskInput {
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedRiskOutput {
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseIsoTsInput {
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseIsoTsOutput {
    #[serde(default)]
    pub timestamp_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractObjectiveIdTokenInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractObjectiveIdTokenOutput {
    #[serde(default)]
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeValueCurrencyTokenInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub allowed_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeValueCurrencyTokenOutput {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListValueCurrenciesInput {
    #[serde(default)]
    pub value_list: Vec<String>,
    #[serde(default)]
    pub value_csv: Option<String>,
    #[serde(default)]
    pub allowed_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListValueCurrenciesOutput {
    #[serde(default)]
    pub currencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferValueCurrenciesFromDirectiveBitsInput {
    #[serde(default)]
    pub bits: Vec<String>,
    #[serde(default)]
    pub allowed_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferValueCurrenciesFromDirectiveBitsOutput {
    #[serde(default)]
    pub currencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasLinkedObjectiveEntryInput {
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub directive_objective_id: Option<String>,
    #[serde(default)]
    pub directive: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasLinkedObjectiveEntryOutput {
    pub linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedEntryOutcomeInput {
    #[serde(default)]
    pub outcome_verified: bool,
    #[serde(default)]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedEntryOutcomeOutput {
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedRevenueActionInput {
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub outcome_verified: bool,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifiedRevenueActionOutput {
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinutesUntilNextUtcDayInput {
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinutesUntilNextUtcDayOutput {
    pub minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgeHoursInput {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgeHoursOutput {
    pub age_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrlDomainInput {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UrlDomainOutput {
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainAllowedInput {
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainAllowedOutput {
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsExecuteModeInput {
    #[serde(default)]
    pub execution_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsExecuteModeOutput {
    pub execute_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionAllowedByFeatureFlagInput {
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub shadow_only: bool,
    #[serde(default)]
    pub autonomy_enabled: bool,
    #[serde(default)]
    pub canary_allow_with_flag_off: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionAllowedByFeatureFlagOutput {
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsTier1ObjectiveIdInput {
    #[serde(default)]
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsTier1ObjectiveIdOutput {
    pub tier1: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsTier1CandidateObjectiveInput {
    #[serde(default)]
    pub objective_binding_objective_id: Option<String>,
    #[serde(default)]
    pub directive_pulse_tier: Option<f64>,
    #[serde(default)]
    pub directive_pulse_objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsTier1CandidateObjectiveOutput {
    pub tier1: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeedsExecutionQuotaInput {
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub shadow_only: bool,
    #[serde(default)]
    pub executed_today: f64,
    #[serde(default)]
    pub min_daily_executions: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NeedsExecutionQuotaOutput {
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeCriteriaMetricInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeCriteriaMetricOutput {
    pub metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EscapeRegExpInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EscapeRegExpOutput {
    pub escaped: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolTokenMentionedInput {
    #[serde(default)]
    pub blob: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolTokenMentionedOutput {
    pub mentioned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldReasonFromEventInput {
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub route_block_reason: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldReasonFromEventOutput {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyMarkerTokensInput {
    #[serde(default)]
    pub objective_primary: Option<String>,
    #[serde(default)]
    pub objective_fitness_metric: Option<String>,
    #[serde(default)]
    pub objective_secondary: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyMarkerTokensOutput {
    #[serde(default)]
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCooldownKeyInput {
    #[serde(default)]
    pub capability_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityCooldownKeyOutput {
    pub cooldown_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadinessRetryCooldownKeyInput {
    #[serde(default)]
    pub strategy_id: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadinessRetryCooldownKeyOutput {
    pub cooldown_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEyeIdInput {
    #[serde(default)]
    pub eye_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEyeIdOutput {
    pub eye_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeprioritizedSourceProposalInput {
    #[serde(default)]
    pub eye_id: Option<String>,
    #[serde(default)]
    pub deprioritized_eye_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeprioritizedSourceProposalOutput {
    pub deprioritized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositeEligibilityMinInput {
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub base_min: f64,
    #[serde(default)]
    pub canary_low_risk_relax: f64,
}
