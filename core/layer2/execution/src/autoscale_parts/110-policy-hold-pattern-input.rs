#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPatternInput {
    #[serde(default)]
    pub events: Vec<PolicyHoldPatternEventInput>,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub window_hours: Option<f64>,
    #[serde(default)]
    pub repeat_threshold: Option<f64>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldPatternOutput {
    pub objective_id: Option<String>,
    pub window_hours: f64,
    pub repeat_threshold: f64,
    pub total_holds: u32,
    pub top_reason: Option<String>,
    pub top_count: u32,
    pub by_reason: std::collections::BTreeMap<String, u32>,
    pub should_dampen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldLatestEventEntryInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub policy_hold: Option<bool>,
    #[serde(default)]
    pub ts_ms: Option<f64>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub hold_reason: Option<String>,
    #[serde(default)]
    pub route_block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldLatestEventInput {
    #[serde(default)]
    pub events: Vec<PolicyHoldLatestEventEntryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldLatestEventOutput {
    pub found: bool,
    pub event_index: Option<u32>,
    pub result: Option<String>,
    pub ts: Option<String>,
    pub ts_ms: Option<f64>,
    pub hold_reason: Option<String>,
    pub route_block_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldCooldownInput {
    #[serde(default)]
    pub base_minutes: Option<f64>,
    #[serde(default)]
    pub pressure_level: Option<String>,
    #[serde(default)]
    pub pressure_applicable: Option<bool>,
    #[serde(default)]
    pub last_result: Option<String>,
    #[serde(default)]
    pub now_ms: Option<f64>,
    #[serde(default)]
    pub cooldown_warn_minutes: Option<f64>,
    #[serde(default)]
    pub cooldown_hard_minutes: Option<f64>,
    #[serde(default)]
    pub cooldown_cap_minutes: Option<f64>,
    #[serde(default)]
    pub cooldown_manual_review_minutes: Option<f64>,
    #[serde(default)]
    pub cooldown_unchanged_state_minutes: Option<f64>,
    #[serde(default)]
    pub readiness_retry_minutes: Option<f64>,
    #[serde(default)]
    pub until_next_day_caps: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyHoldCooldownOutput {
    pub cooldown_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DodEvidenceDiffInput {
    #[serde(default)]
    pub before_artifacts: Option<f64>,
    #[serde(default)]
    pub before_entries: Option<f64>,
    #[serde(default)]
    pub before_revenue_actions: Option<f64>,
    #[serde(default)]
    pub before_registry_total: Option<f64>,
    #[serde(default)]
    pub before_registry_active: Option<f64>,
    #[serde(default)]
    pub before_registry_candidate: Option<f64>,
    #[serde(default)]
    pub before_habit_runs: Option<f64>,
    #[serde(default)]
    pub before_habit_errors: Option<f64>,
    #[serde(default)]
    pub after_artifacts: Option<f64>,
    #[serde(default)]
    pub after_entries: Option<f64>,
    #[serde(default)]
    pub after_revenue_actions: Option<f64>,
    #[serde(default)]
    pub after_registry_total: Option<f64>,
    #[serde(default)]
    pub after_registry_active: Option<f64>,
    #[serde(default)]
    pub after_registry_candidate: Option<f64>,
    #[serde(default)]
    pub after_habit_runs: Option<f64>,
    #[serde(default)]
    pub after_habit_errors: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DodEvidenceDiffOutput {
    pub artifacts_delta: f64,
    pub entries_delta: f64,
    pub revenue_actions_delta: f64,
    pub registry_total_delta: f64,
    pub registry_active_delta: f64,
    pub registry_candidate_delta: f64,
    pub habit_runs_delta: f64,
    pub habit_errors_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptVerdictInput {
    pub decision: String,
    pub exec_ok: bool,
    pub postconditions_ok: bool,
    pub dod_passed: bool,
    pub success_criteria_required: bool,
    pub success_criteria_passed: bool,
    pub queue_outcome_logged: bool,
    #[serde(default)]
    pub route_attestation_status: String,
    #[serde(default)]
    pub route_attestation_expected_model: String,
    #[serde(default)]
    pub success_criteria_primary_failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptCheck {
    pub name: String,
    pub pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptVerdictOutput {
    pub exec_check_name: String,
    pub checks: Vec<ReceiptCheck>,
    pub failed: Vec<String>,
    pub passed: bool,
    pub outcome: String,
    pub primary_failure: Option<String>,
    pub route_attestation_mismatch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultBacklogAutoscaleStateInput {
    #[serde(default)]
    pub module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultBacklogAutoscaleStateOutput {
    pub schema_id: String,
    pub schema_version: String,
    pub module: String,
    pub current_cells: f64,
    pub target_cells: f64,
    pub last_run_ts: Option<String>,
    pub last_high_pressure_ts: Option<String>,
    pub last_action: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeBacklogAutoscaleStateInput {
    #[serde(default)]
    pub module: String,
    #[serde(default)]
    pub src: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeBacklogAutoscaleStateOutput {
    pub schema_id: String,
    pub schema_version: String,
    pub module: String,
    pub current_cells: f64,
    pub target_cells: f64,
    pub last_run_ts: Option<String>,
    pub last_high_pressure_ts: Option<String>,
    pub last_action: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnAllocatedCellsInput {
    #[serde(default)]
    pub active_cells: Option<f64>,
    #[serde(default)]
    pub current_cells: Option<f64>,
    #[serde(default)]
    pub allocated_cells: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnAllocatedCellsOutput {
    pub active_cells: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnCapacityBoostRowInput {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
    #[serde(default)]
    pub granted_cells: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnCapacityBoostSnapshotInput {
    pub enabled: bool,
    pub lookback_minutes: f64,
    pub min_granted_cells: f64,
    pub now_ms: f64,
    #[serde(default)]
    pub rows: Vec<SpawnCapacityBoostRowInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnCapacityBoostSnapshotOutput {
    pub enabled: bool,
    pub active: bool,
    pub lookback_minutes: f64,
    pub min_granted_cells: f64,
    pub grant_count: i64,
    pub granted_cells: f64,
    pub latest_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InversionMaturityScoreInput {
    pub total_tests: f64,
    pub passed_tests: f64,
    pub destructive_failures: f64,
    pub target_test_count: f64,
    pub weight_pass_rate: f64,
    pub weight_non_destructive_rate: f64,
    pub weight_experience: f64,
    pub band_novice: f64,
    pub band_developing: f64,
    pub band_mature: f64,
    pub band_seasoned: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InversionMaturityScoreOutput {
    pub score: f64,
    pub band: String,
    pub pass_rate: f64,
    pub non_destructive_rate: f64,
    pub experience: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultCriteriaPatternMemoryInput {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultCriteriaPatternMemoryOutput {
    pub version: String,
    pub updated_at: Option<String>,
    pub patterns: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyExecutionModeEffectiveInput {
    #[serde(default)]
    pub strategy_mode: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyExecutionModeEffectiveOutput {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCanaryExecLimitEffectiveInput {
    #[serde(default)]
    pub strategy_limit: Option<serde_json::Value>,
    #[serde(default)]
    pub fallback: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCanaryExecLimitEffectiveOutput {
    #[serde(default)]
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyExplorationEffectiveInput {
    #[serde(default)]
    pub strategy_exploration: Option<serde_json::Value>,
    #[serde(default)]
    pub default_fraction: Option<f64>,
    #[serde(default)]
    pub default_every_n: Option<f64>,
    #[serde(default)]
    pub default_min_eligible: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyExplorationEffectiveOutput {
    pub fraction: f64,
    pub every_n: f64,
    pub min_eligible: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyBudgetEffectiveInput {
    #[serde(default)]
    pub caps: Option<serde_json::Value>,
    #[serde(default)]
    pub hard_runs: Option<f64>,
    #[serde(default)]
    pub hard_tokens: Option<f64>,
    #[serde(default)]
    pub hard_per_action: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyBudgetEffectiveOutput {
    pub budget: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreexecVerdictFromSignalsInput {
    #[serde(default)]
    pub blockers: Vec<serde_json::Value>,
    #[serde(default)]
    pub signals: Option<serde_json::Value>,
    #[serde(default)]
    pub next_runnable_at: Option<String>,
    #[serde(default)]
    pub now_iso: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreexecVerdictFromSignalsOutput {
    pub verdict: String,
    pub confidence: f64,
    pub blocker_count: u32,
    pub blocker_codes: Vec<String>,
    pub manual_action_required: bool,
    #[serde(default)]
    pub next_runnable_at: Option<String>,
    pub signals: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyProposalChurnInput {
    #[serde(default)]
    pub prior_runs: Vec<serde_json::Value>,
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub window_hours: Option<f64>,
    #[serde(default)]
    pub now_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreOnlyProposalChurnOutput {
    pub count: u32,
    pub streak: u32,
    #[serde(default)]
    pub first_ts: Option<String>,
    #[serde(default)]
    pub last_ts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaQualityAuditInput {
    #[serde(default)]
    pub verification: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessCriteriaQualityAuditOutput {
    pub verification: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectEyesTerminologyDriftInput {
    #[serde(default)]
    pub proposals: Vec<serde_json::Value>,
    #[serde(default)]
    pub tool_capability_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectEyesTerminologyDriftWarning {
    #[serde(default)]
    pub proposal_id: Option<String>,
    pub reason: String,
    pub matched_tools: Vec<String>,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectEyesTerminologyDriftOutput {
    pub warnings: Vec<DetectEyesTerminologyDriftWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeStoredProposalRowInput {
    #[serde(default)]
    pub proposal: Option<serde_json::Value>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default)]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub proposal_type_source: Option<String>,
    #[serde(default)]
    pub proposal_type_inferred: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizeStoredProposalRowOutput {
    pub proposal: serde_json::Value,
}
