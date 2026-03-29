#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClampNumberInput {
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClampNumberOutput {
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListProposalFilesInput {
    #[serde(default)]
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListProposalFilesOutput {
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatestProposalDateInput {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub max_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatestProposalDateOutput {
    #[serde(default)]
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseDirectiveFileArgInput {
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseDirectiveFileArgOutput {
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseDirectiveObjectiveArgInput {
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseDirectiveObjectiveArgOutput {
    pub objective_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NowIsoInput {
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NowIsoOutput {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodayStrInput {
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodayStrOutput {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanCanaryOverrideApprovalPhraseInput {
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub date_str: Option<String>,
    #[serde(default)]
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanCanaryOverrideApprovalPhraseOutput {
    pub phrase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseHumanCanaryOverrideStateInput {
    #[serde(default)]
    pub record: Option<serde_json::Value>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseHumanCanaryOverrideStateOutput {
    pub active: bool,
    pub reason: String,
    #[serde(default)]
    pub expired: Option<bool>,
    #[serde(default)]
    pub remaining: Option<f64>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub require_execution_mode: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyBudgetPathInput {
    #[serde(default)]
    pub state_dir: Option<String>,
    #[serde(default)]
    pub date_str: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyBudgetPathOutput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunsPathForInput {
    #[serde(default)]
    pub runs_dir: Option<String>,
    #[serde(default)]
    pub date_str: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunsPathForOutput {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectiveTier1PolicyInput {
    #[serde(default)]
    pub execution_mode: Option<String>,
    pub tier1_burn_rate_multiplier: f64,
    pub tier1_canary_burn_rate_multiplier: f64,
    pub tier1_min_projected_tokens_for_burn_check: f64,
    pub tier1_canary_min_projected_tokens_for_burn_check: f64,
    pub tier1_drift_min_samples: f64,
    pub tier1_canary_drift_min_samples: f64,
    pub tier1_alignment_threshold: f64,
    pub tier1_canary_alignment_threshold: f64,
    pub tier1_canary_suppress_alignment_blocker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectiveTier1PolicyOutput {
    #[serde(default)]
    pub execution_mode: Option<String>,
    pub canary_relaxed: bool,
    pub burn_rate_multiplier: f64,
    pub min_projected_tokens_for_burn_check: f64,
    pub drift_min_samples: f64,
    pub alignment_threshold: f64,
    pub suppress_alignment_blocker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactTier1ExceptionInput {
    #[serde(default)]
    pub tracked: Option<bool>,
    #[serde(default)]
    pub novel: Option<bool>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub count: Option<f64>,
    #[serde(default)]
    pub recovery: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactTier1ExceptionOutput {
    pub has_value: bool,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NextHumanEscalationClearAtInput {
    #[serde(default)]
    pub rows: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NextHumanEscalationClearAtOutput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCatalogCanaryThresholdsInput {
    pub min_samples: f64,
    pub max_fail_rate: f64,
    pub max_route_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelCatalogCanaryThresholdsOutput {
    pub min_samples: f64,
    pub max_fail_rate: f64,
    pub max_route_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveClarificationExecSpecInput {
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub meta_directive_objective_id: Option<String>,
    #[serde(default)]
    pub suggested_next_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveClarificationExecSpecOutput {
    pub applicable: bool,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveDecompositionExecSpecInput {
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub meta_directive_objective_id: Option<String>,
    #[serde(default)]
    pub suggested_next_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectiveDecompositionExecSpecOutput {
    pub applicable: bool,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseActuationSpecInput {
    #[serde(default)]
    pub proposal: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseActuationSpecMutationGuard {
    pub applies: bool,
    pub pass: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub reasons: Vec<serde_json::Value>,
    #[serde(default)]
    pub controls: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseActuationSpecContext {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub safety_attestation_id: Option<String>,
    #[serde(default)]
    pub rollback_receipt_id: Option<String>,
    #[serde(default)]
    pub adaptive_mutation_guard_receipt_id: Option<String>,
    pub mutation_guard: ParseActuationSpecMutationGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseActuationSpecOutput {
    pub has_spec: bool,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub context: Option<ParseActuationSpecContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskFromProposalInput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskFromProposalOutput {
    pub task: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseObjectiveIdFromEvidenceRefsInput {
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub objective_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseObjectiveIdFromEvidenceRefsOutput {
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseObjectiveIdFromCommandInput {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub objective_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseObjectiveIdFromCommandOutput {
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectiveIdForExecutionInput {
    #[serde(default)]
    pub objective_binding_id: Option<String>,
    #[serde(default)]
    pub directive_pulse_id: Option<String>,
    #[serde(default)]
    pub directive_action_id: Option<String>,
    #[serde(default)]
    pub meta_objective_id: Option<String>,
    #[serde(default)]
    pub meta_directive_objective_id: Option<String>,
    #[serde(default)]
    pub action_spec_objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectiveIdForExecutionOutput {
    #[serde(default)]
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortTextInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub max_len: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortTextOutput {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedSignalStatusInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedSignalStatusOutput {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionReserveSnapshotInput {
    pub cap: f64,
    pub used: f64,
    pub reserve_enabled: bool,
    pub reserve_ratio: f64,
    pub reserve_min_tokens: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionReserveSnapshotOutput {
    pub enabled: bool,
    pub reserve_tokens: f64,
    pub reserve_remaining: f64,
}
