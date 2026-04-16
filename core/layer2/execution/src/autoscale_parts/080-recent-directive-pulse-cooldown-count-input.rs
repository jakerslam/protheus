
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentDirectivePulseCooldownCountInput {
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub hours: Option<f64>,
    #[serde(default, alias = "nowMs")]
    pub now_ms: Option<f64>,
    #[serde(default, alias = "recentEvents")]
    pub events: Vec<RecentDirectivePulseCooldownEventInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentDirectivePulseCooldownCountOutput {
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDirectiveTextInput {
    #[serde(default)]
    pub proposal: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDirectiveTextOutput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectiveIdsFromPulseContextInput {
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
    #[serde(default, alias = "fallbackEnabled")]
    pub fallback_enabled: bool,
    #[serde(default, alias = "fallbackIds")]
    pub fallback_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectiveIdsFromPulseContextOutput {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldObjectiveContextInput {
    #[serde(default, alias = "candidateObjectiveIds")]
    pub candidate_objective_ids: Vec<String>,
    #[serde(default, alias = "poolObjectiveIds")]
    pub pool_objective_ids: Vec<String>,
    #[serde(default, alias = "dominantObjectiveId")]
    pub dominant_objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldObjectiveContextOutput {
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default, alias = "objectiveSource")]
    pub objective_source: Option<String>,
    #[serde(default, alias = "objectiveIds")]
    pub objective_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalSemanticObjectiveIdInput {
    #[serde(default)]
    pub proposal: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalSemanticObjectiveIdOutput {
    pub objective_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternKeysRowInput {
    #[serde(default)]
    pub metric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternKeysInput {
    #[serde(default)]
    pub capability_key_hint: Option<String>,
    #[serde(default)]
    pub capability_descriptor_key: Option<String>,
    #[serde(default)]
    pub rows: Vec<CriteriaPatternKeysRowInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CriteriaPatternKeysOutput {
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaRequirementInput {
    #[serde(default, alias = "requireSuccessCriteria")]
    pub require_success_criteria: Option<bool>,
    #[serde(default, alias = "minSuccessCriteriaCount")]
    pub min_success_criteria_count: Option<f64>,
    #[serde(default, alias = "policyExemptTypes")]
    pub policy_exempt_types: Vec<String>,
    #[serde(default, alias = "envExemptTypes")]
    pub env_exempt_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaRequirementOutput {
    pub required: bool,
    pub min_count: f64,
    pub exempt_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaPolicyForProposalInput {
    #[serde(default)]
    pub base_required: bool,
    #[serde(default)]
    pub base_min_count: f64,
    #[serde(default)]
    pub base_exempt_types: Vec<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaPolicyForProposalOutput {
    pub required: bool,
    pub min_count: f64,
    pub exempt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityDescriptorInput {
    #[serde(default, alias = "actuationKind")]
    pub actuation_kind: Option<String>,
    #[serde(default, alias = "proposalType")]
    pub proposal_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityDescriptorOutput {
    pub key: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeTokenUsageShapeInput {
    #[serde(default, alias = "promptTokens")]
    pub prompt_tokens: Option<f64>,
    #[serde(default, alias = "inputTokens")]
    pub input_tokens: Option<f64>,
    #[serde(default, alias = "completionTokens")]
    pub completion_tokens: Option<f64>,
    #[serde(default, alias = "outputTokens")]
    pub output_tokens: Option<f64>,
    #[serde(default, alias = "totalTokens")]
    pub total_tokens: Option<f64>,
    #[serde(default, alias = "tokensUsed")]
    pub tokens_used: Option<f64>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeTokenUsageShapeValueOutput {
    #[serde(default)]
    pub prompt_tokens: Option<f64>,
    #[serde(default)]
    pub completion_tokens: Option<f64>,
    #[serde(default)]
    pub total_tokens: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeTokenUsageShapeOutput {
    pub has_value: bool,
    #[serde(default)]
    pub usage: Option<NormalizeTokenUsageShapeValueOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsDirectiveClarificationProposalInput {
    #[serde(default)]
    pub proposal_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsDirectiveClarificationProposalOutput {
    pub is_clarification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsDirectiveDecompositionProposalInput {
    #[serde(default)]
    pub proposal_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsDirectiveDecompositionProposalOutput {
    pub is_decomposition: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SanitizeDirectiveObjectiveIdInput {
    #[serde(default, alias = "objectiveId")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SanitizeDirectiveObjectiveIdOutput {
    pub objective_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SanitizedDirectiveIdListInput {
    #[serde(default)]
    pub rows: Vec<String>,
    #[serde(default)]
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SanitizedDirectiveIdListOutput {
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseFirstJsonLineInput {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseFirstJsonLineOutput {
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseJsonObjectsFromTextInput {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub max_objects: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseJsonObjectsFromTextOutput {
    #[serde(default)]
    pub objects: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadPathValueInput {
    #[serde(default)]
    pub obj: Option<serde_json::Value>,
    #[serde(default)]
    pub path_expr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadPathValueOutput {
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberOrNullInput {
    #[serde(default)]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberOrNullOutput {
    #[serde(default)]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChooseEvidenceSelectionModeRunInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChooseEvidenceSelectionModeInput {
    #[serde(default)]
    pub eligible_len: Option<f64>,
    #[serde(default)]
    pub prior_runs: Vec<ChooseEvidenceSelectionModeRunInput>,
    #[serde(default)]
    pub evidence_sample_window: Option<f64>,
    #[serde(default)]
    pub mode_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChooseEvidenceSelectionModeOutput {
    pub mode: String,
    pub index: u32,
    pub sample_window: u32,
    pub sample_cursor: u32,
    pub prior_evidence_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruthyFlagInput {
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruthyFlagOutput {
    pub value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StableSelectionIndexInput {
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StableSelectionIndexOutput {
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsStringArrayInput {
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsStringArrayOutput {
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniqSortedInput {
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UniqSortedOutput {
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeModelIdsInput {
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeModelIdsOutput {
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectedModelFromRunEventInput {
    #[serde(default)]
    pub route_summary: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectedModelFromRunEventOutput {
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadFirstNumericMetricInput {
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    #[serde(default)]
    pub path_exprs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadFirstNumericMetricOutput {
    #[serde(default)]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseArgInput {
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseArgOutput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateArgOrTodayInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub today: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DateArgOrTodayOutput {
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasEnvNumericOverrideInput {
    pub present: bool,
    #[serde(default)]
    pub raw_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HasEnvNumericOverrideOutput {
    pub has_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoalesceNumericInput {
    #[serde(default)]
    pub primary: Option<f64>,
    #[serde(default)]
    pub fallback: Option<f64>,
    #[serde(default)]
    pub null_fallback: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoalesceNumericOutput {
    #[serde(default)]
    pub value: Option<f64>,
}
