
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RelPathInput {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelPathOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeAxiomPatternInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeAxiomPatternOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeAxiomSignalTermsInput {
    #[serde(default)]
    pub terms: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeAxiomSignalTermsOutput {
    pub terms: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeObserverIdInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeObserverIdOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractNumericInput {
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExtractNumericOutput {
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PickFirstNumericInput {
    #[serde(default)]
    pub candidates: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PickFirstNumericOutput {
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SafeRelPathInput {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SafeRelPathOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NowIsoInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NowIsoOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultTierEventMapInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultTierEventMapOutput {
    pub map: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CoerceTierEventMapInput {
    #[serde(default)]
    pub map: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CoerceTierEventMapOutput {
    pub map: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GetTierScopeInput {
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub policy_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GetTierScopeOutput {
    pub state: Value,
    pub scope: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadTierGovernanceStateInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy_version: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadTierGovernanceStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveTierGovernanceStateInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub policy_version: Option<String>,
    #[serde(default)]
    pub retention_days: Option<i64>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveTierGovernanceStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PushTierEventInput {
    #[serde(default)]
    pub scope_map: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PushTierEventOutput {
    pub map: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AddTierEventInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AddTierEventOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IncrementLiveApplyAttemptInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IncrementLiveApplyAttemptOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IncrementLiveApplySuccessInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IncrementLiveApplySuccessOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct IncrementLiveApplySafeAbortInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct IncrementLiveApplySafeAbortOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateShadowTrialCountersInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub destructive: Option<bool>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpdateShadowTrialCountersOutput {
    pub state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultHarnessStateInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultHarnessStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultFirstPrincipleLockStateInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultFirstPrincipleLockStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DefaultMaturityStateInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DefaultMaturityStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PrincipleKeyForSessionInput {
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PrincipleKeyForSessionOutput {
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeObjectiveArgInput {
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeObjectiveArgOutput {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MaturityBandOrderInput {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MaturityBandOrderOutput {
    pub bands: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CurrentRuntimeModeInput {
    #[serde(default)]
    pub env_mode: Option<String>,
    #[serde(default)]
    pub args_mode: Option<String>,
    #[serde(default)]
    pub policy_runtime_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CurrentRuntimeModeOutput {
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadDriftFromStateFileInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReadDriftFromStateFileOutput {
    pub value: f64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResolveLensGateDriftInput {
    #[serde(default)]
    pub arg_candidates: Vec<Value>,
    #[serde(default)]
    pub probe_path: Option<String>,
    #[serde(default)]
    pub probe_source: Option<String>,
    #[serde(default)]
    pub probe_payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResolveLensGateDriftOutput {
    pub value: f64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResolveParityConfidenceInput {
    #[serde(default)]
    pub arg_candidates: Vec<Value>,
    #[serde(default)]
    pub path_hint: Option<String>,
    #[serde(default)]
    pub path_source: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResolveParityConfidenceOutput {
    pub value: f64,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComputeAttractorScoreInput {
    #[serde(default)]
    pub attractor: Option<Value>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub external_signals_count: Option<Value>,
    #[serde(default)]
    pub evidence_count: Option<Value>,
    #[serde(default)]
    pub effective_certainty: Option<Value>,
    #[serde(default)]
    pub trit: Option<Value>,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ComputeAttractorScoreOutput {
    pub enabled: bool,
    pub score: f64,
    pub required: f64,
    pub pass: bool,
    pub components: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BuildOutputInterfacesInput {
    #[serde(default)]
    pub outputs: Option<Value>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub sandbox_verified: Option<Value>,
    #[serde(default)]
    pub explicit_code_proposal_emit: Option<Value>,
    #[serde(default)]
    pub channel_payloads: Option<Value>,
    #[serde(default)]
    pub base_payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BuildOutputInterfacesOutput {
    pub default_channel: String,
    pub active_channel: Option<String>,
    pub channels: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BuildCodeChangeProposalDraftInput {
    #[serde(default)]
    pub base: Option<Value>,
    #[serde(default)]
    pub args: Option<Value>,
    #[serde(default)]
    pub opts: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BuildCodeChangeProposalDraftOutput {
    pub proposal: Value,
}
