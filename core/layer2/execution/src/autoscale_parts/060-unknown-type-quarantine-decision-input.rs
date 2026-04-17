#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnknownTypeQuarantineDecisionInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, alias = "proposalType", alias = "type")]
    pub proposal_type: Option<String>,
    #[serde(default, alias = "in_quarantine_set")]
    pub type_in_quarantine_set: bool,
    #[serde(default, alias = "allowDirective")]
    pub allow_directive: bool,
    #[serde(default, alias = "allowTier1")]
    pub allow_tier1: bool,
    #[serde(default, alias = "objectiveId")]
    pub objective_id: Option<String>,
    #[serde(default, alias = "tier1Objective")]
    pub tier1_objective: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnknownTypeQuarantineDecisionOutput {
    pub block: bool,
    pub proposal_type: Option<String>,
    pub reason: Option<String>,
    pub objective_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferOptimizationDeltaInput {
    #[serde(default, alias = "optimizationDeltaPercent")]
    pub optimization_delta_percent: Option<f64>,
    #[serde(default, alias = "expectedOptimizationPercent")]
    pub expected_optimization_percent: Option<f64>,
    #[serde(default, alias = "expectedDeltaPercent")]
    pub expected_delta_percent: Option<f64>,
    #[serde(default, alias = "estimatedImprovementPercent")]
    pub estimated_improvement_percent: Option<f64>,
    #[serde(default, alias = "targetImprovementPercent")]
    pub target_improvement_percent: Option<f64>,
    #[serde(default, alias = "performanceGainPercent")]
    pub performance_gain_percent: Option<f64>,
    #[serde(default)]
    pub text_blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferOptimizationDeltaOutput {
    pub delta_percent: Option<f64>,
    pub delta_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationIntentProposalInput {
    #[serde(default, alias = "proposalType", alias = "type")]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub blob: Option<String>,
    #[serde(default, alias = "hasActuationMeta")]
    pub has_actuation_meta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationIntentProposalOutput {
    pub intent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnlinkedOptimizationAdmissionInput {
    #[serde(default)]
    pub optimization_intent: bool,
    #[serde(default, alias = "proposalType", alias = "type")]
    pub proposal_type: Option<String>,
    #[serde(default)]
    pub exempt_types: Vec<String>,
    #[serde(default)]
    pub linked: bool,
    #[serde(default, alias = "normalizedRisk")]
    pub normalized_risk: Option<String>,
    #[serde(default, alias = "hardBlockHighRisk")]
    pub hard_block_high_risk: bool,
    #[serde(default)]
    pub penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnlinkedOptimizationAdmissionOutput {
    pub applies: bool,
    pub linked: bool,
    pub penalty: f64,
    pub block: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationGoodEnoughInput {
    #[serde(default)]
    pub applies: bool,
    #[serde(default, alias = "minDeltaPercent")]
    pub min_delta_percent: f64,
    #[serde(default, alias = "requireDelta")]
    pub require_delta: bool,
    #[serde(default, alias = "highAccuracyMode")]
    pub high_accuracy_mode: bool,
    #[serde(default, alias = "normalizedRisk")]
    pub normalized_risk: Option<String>,
    #[serde(default, alias = "deltaPercent")]
    pub delta_percent: Option<f64>,
    #[serde(default, alias = "deltaSource")]
    pub delta_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptimizationGoodEnoughOutput {
    pub applies: bool,
    pub pass: bool,
    pub reason: Option<String>,
    pub delta_percent: Option<f64>,
    pub delta_source: Option<String>,
    pub min_delta_percent: f64,
    pub require_delta: bool,
    pub mode: String,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDependencySummaryInput {
    #[serde(default, alias = "proposalId")]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default, alias = "parentObjectiveId")]
    pub parent_objective_id: Option<String>,
    #[serde(default, alias = "createdIds")]
    pub created_ids: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub created_count: Option<f64>,
    #[serde(default)]
    pub quality_ok: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDependencySummaryNode {
    pub id: String,
    pub kind: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDependencySummaryEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalDependencySummaryOutput {
    pub proposal_id: Option<String>,
    pub decision: String,
    pub source: Option<String>,
    pub parent_objective_id: Option<String>,
    pub child_objective_ids: Vec<String>,
    pub edge_count: u32,
    pub nodes: Vec<ProposalDependencySummaryNode>,
    pub edges: Vec<ProposalDependencySummaryEdge>,
    pub chain: Vec<String>,
    pub dry_run: bool,
    pub created_count: f64,
    pub quality_ok: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChooseSelectionModeInput {
    pub eligible_len: u32,
    pub executed_count: u32,
    pub explore_used: u32,
    pub exploit_used: u32,
    pub explore_quota: u32,
    pub every_n: u32,
    pub min_eligible: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChooseSelectionModeOutput {
    pub mode: String,
    pub index: u32,
    pub explore_used: u32,
    pub explore_quota: u32,
    pub exploit_used: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExploreQuotaForDayInput {
    #[serde(default)]
    pub daily_runs_cap: Option<f64>,
    #[serde(default)]
    pub explore_fraction: Option<f64>,
    #[serde(default)]
    pub default_max_runs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExploreQuotaForDayOutput {
    pub quota: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediumRiskThresholdsInput {
    #[serde(default)]
    pub base_min_directive_fit: f64,
    #[serde(default)]
    pub base_min_actionability_score: f64,
    #[serde(default)]
    pub medium_risk_min_composite_eligibility: f64,
    #[serde(default)]
    pub min_composite_eligibility: f64,
    #[serde(default)]
    pub medium_risk_min_directive_fit: f64,
    #[serde(default)]
    pub default_min_directive_fit: f64,
    #[serde(default)]
    pub medium_risk_min_actionability: f64,
    #[serde(default)]
    pub default_min_actionability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediumRiskThresholdsOutput {
    pub composite_min: f64,
    pub directive_fit_min: f64,
    pub actionability_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediumRiskGateDecisionInput {
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub composite_score: f64,
    #[serde(default)]
    pub directive_fit_score: f64,
    #[serde(default)]
    pub actionability_score: f64,
    #[serde(default)]
    pub composite_min: f64,
    #[serde(default)]
    pub directive_fit_min: f64,
    #[serde(default)]
    pub actionability_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediumRiskGateDecisionOutput {
    pub pass: bool,
    pub risk: String,
    pub reasons: Vec<String>,
    pub required: Option<MediumRiskThresholdsOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockPrefilterInput {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub capability_key: Option<String>,
    #[serde(default)]
    pub window_hours: f64,
    #[serde(default)]
    pub min_observations: f64,
    #[serde(default)]
    pub max_block_rate: f64,
    #[serde(default)]
    pub row_present: bool,
    #[serde(default)]
    pub attempts: f64,
    #[serde(default)]
    pub route_blocked: f64,
    #[serde(default)]
    pub route_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockPrefilterOutput {
    pub enabled: bool,
    pub applicable: bool,
    pub pass: bool,
    pub reason: String,
    pub capability_key: Option<String>,
    pub window_hours: f64,
    pub min_observations: f64,
    pub max_block_rate: f64,
    pub attempts: f64,
    pub route_blocked: f64,
    pub route_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteExecutionSampleEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub execution_target: Option<String>,
    #[serde(default)]
    pub route_summary_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteExecutionSampleEventOutput {
    pub is_sample_event: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockTelemetryEventInput {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub execution_target: Option<String>,
    #[serde(default)]
    pub route_summary_present: bool,
    #[serde(default)]
    pub capability_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockTelemetrySummaryInput {
    #[serde(default)]
    pub events: Vec<RouteBlockTelemetryEventInput>,
    #[serde(default)]
    pub window_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockTelemetryCapabilityOutput {
    pub key: String,
    pub attempts: f64,
    pub route_blocked: f64,
    pub route_block_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteBlockTelemetrySummaryOutput {
    pub window_hours: f64,
    pub sample_events: f64,
    pub by_capability: Vec<RouteBlockTelemetryCapabilityOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsStubProposalInput {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsStubProposalOutput {
    pub is_stub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentAutonomyRunEventsInput {
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
    #[serde(default)]
    pub cutoff_ms: f64,
    #[serde(default)]
    pub cap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentAutonomyRunEventsOutput {
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalMetaIndexEntryInput {
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub eye_id: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalMetaIndexInput {
    #[serde(default)]
    pub entries: Vec<ProposalMetaIndexEntryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalMetaIndexEntryOutput {
    pub proposal_id: String,
    pub eye_id: String,
    #[serde(default)]
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalMetaIndexOutput {
    #[serde(default)]
    pub entries: Vec<ProposalMetaIndexEntryOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewLogEventsInput {
    #[serde(default)]
    pub before_run_len: Option<f64>,
    #[serde(default)]
    pub before_error_len: Option<f64>,
    #[serde(default)]
    pub after_runs: Vec<serde_json::Value>,
    #[serde(default)]
    pub after_errors: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewLogEventsOutput {
    #[serde(default)]
    pub runs: Vec<serde_json::Value>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeBucketsInput {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeBucketsOutput {
    pub shipped: f64,
    pub no_change: f64,
    pub reverted: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentRunEventsInput {
    #[serde(default)]
    pub day_events: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentRunEventsOutput {
    #[serde(default)]
    pub events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllDecisionEventsInput {
    #[serde(default)]
    pub day_events: Vec<Vec<serde_json::Value>>,
}
