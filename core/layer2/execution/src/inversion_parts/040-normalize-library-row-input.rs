#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeLibraryRowInput {
    #[serde(default, alias = "library_row")]
    pub row: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeLibraryRowOutput {
    pub row: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnsureDirInput {
    #[serde(default, alias = "path")]
    pub dir_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnsureDirOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadJsonInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub fallback: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReadJsonOutput {
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadJsonlInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReadJsonlOutput {
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WriteJsonAtomicInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WriteJsonAtomicOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppendJsonlInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub row: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppendJsonlOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReadTextInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReadTextOutput {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LatestJsonFileInDirInput {
    #[serde(default, alias = "path")]
    pub dir_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LatestJsonFileInDirOutput {
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeOutputChannelInput {
    #[serde(default)]
    pub base_out: Option<Value>,
    #[serde(default)]
    pub src_out: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeOutputChannelOutput {
    pub enabled: bool,
    pub live_enabled: bool,
    pub test_enabled: bool,
    pub require_sandbox_verification: bool,
    pub require_explicit_emit: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeRepoPathInput {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default, alias = "workspace_root")]
    pub root: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NormalizeRepoPathOutput {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimePathsInput {
    #[serde(default, alias = "policyPath")]
    pub policy_path: Option<String>,
    #[serde(default, alias = "inversionStateDirEnv")]
    pub inversion_state_dir_env: Option<String>,
    #[serde(default, alias = "dualBrainPolicyPathEnv")]
    pub dual_brain_policy_path_env: Option<String>,
    #[serde(default, alias = "defaultStateDir")]
    pub default_state_dir: Option<String>,
    #[serde(default, alias = "workspace_root")]
    pub root: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuntimePathsOutput {
    pub paths: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeAxiomListInput {
    #[serde(default)]
    pub raw_axioms: Option<Value>,
    #[serde(default)]
    pub base_axioms: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeAxiomListOutput {
    pub axioms: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NormalizeHarnessSuiteInput {
    #[serde(default)]
    pub raw_suite: Option<Value>,
    #[serde(default)]
    pub base_suite: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NormalizeHarnessSuiteOutput {
    pub suite: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadHarnessStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadHarnessStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveHarnessStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveHarnessStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadFirstPrincipleLockStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadFirstPrincipleLockStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveFirstPrincipleLockStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveFirstPrincipleLockStateOutput {
    pub state: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CheckFirstPrincipleDowngradeInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CheckFirstPrincipleDowngradeOutput {
    pub allowed: bool,
    pub reason: Option<String>,
    pub key: String,
    pub lock_state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpsertFirstPrincipleLockInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub session: Option<Value>,
    #[serde(default)]
    pub principle: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpsertFirstPrincipleLockOutput {
    pub state: Value,
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadObserverApprovalsInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadObserverApprovalsOutput {
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppendObserverApprovalInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub observer_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppendObserverApprovalOutput {
    pub row: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CountObserverApprovalsInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub window_days: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CountObserverApprovalsOutput {
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnsureCorrespondenceFileInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub header: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnsureCorrespondenceFileOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadMaturityStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadMaturityStateOutput {
    pub state: Value,
    pub computed: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveMaturityStateInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub policy: Option<Value>,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveMaturityStateOutput {
    pub state: Value,
    pub computed: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoadActiveSessionsInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LoadActiveSessionsOutput {
    pub store: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveActiveSessionsInput {
    #[serde(default, alias = "path")]
    pub file_path: Option<String>,
    #[serde(default)]
    pub store: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SaveActiveSessionsOutput {
    pub store: Value,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EmitEventInput {
    #[serde(default, alias = "eventsDir")]
    pub events_dir: Option<String>,
    #[serde(default)]
    pub date_str: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub emit_events: Option<bool>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EmitEventOutput {
    pub emitted: bool,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppendPersonaLensGateReceiptInput {
    #[serde(default, alias = "stateDir")]
    pub state_dir: Option<String>,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default, alias = "cfgReceiptsPath")]
    pub cfg_receipts_path: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub decision: Option<Value>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppendPersonaLensGateReceiptOutput {
    pub rel_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppendConclaveCorrespondenceInput {
    #[serde(default, alias = "path")]
    pub correspondence_path: Option<String>,
    #[serde(default)]
    pub row: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppendConclaveCorrespondenceOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PersistDecisionInput {
    #[serde(default, alias = "latestPath")]
    pub latest_path: Option<String>,
    #[serde(default, alias = "historyPath")]
    pub history_path: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PersistDecisionOutput {
    pub ok: bool,
}
