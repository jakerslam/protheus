// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use infring_tooling_core_v1::{ToolBackendClass, ToolReasonCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalQualitySignalMode {
    Scored,
    InsufficientSignal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalQualitySignalSnapshot {
    pub quality_ok: bool,
    pub monitor_ok: bool,
    pub evaluation_mode: EvalQualitySignalMode,
    pub predicted_non_info_samples: u64,
    pub minimum_eval_samples: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalCalibrationSnapshot {
    pub calibration_ready: bool,
    pub status: String,
    pub agreement_rate: f64,
    pub agreement_min: f64,
    pub comparable_samples: u64,
    pub minimum_samples: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalQualityGatePolicy {
    pub required_consecutive_passes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalQualityGateHistory {
    pub consecutive_passes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalQualityGateState {
    pub quality_signal_sufficient: bool,
    pub calibration_ready: bool,
    pub current_pass: bool,
    pub soft_blocked: bool,
    pub consecutive_passes: u64,
    pub required_consecutive_passes: u64,
    pub autonomous_escalation_allowed: bool,
    pub remaining_to_unlock: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestClass {
    ReadOnly,
    ToolCall,
    Assimilation,
    TaskProposal,
    Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestKind {
    Direct,
    Comparative,
    Workflow,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Read,
    Search,
    Fetch,
    Compare,
    InspectTooling,
    Assimilate,
    Plan,
    Mutate,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Web,
    Workspace,
    Tooling,
    TaskGraph,
    Memory,
    Mixed,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutability {
    ReadOnly,
    Proposal,
    Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyScope {
    Default,
    WebOnly,
    WorkspaceOnly,
    CoreProposal,
    CrossBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionPosture {
    Ask,
    Act,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadMemory,
    MutateTask,
    WorkspaceRead,
    WorkspaceSearch,
    WebSearch,
    WebFetch,
    ToolRoute,
    /// Legacy compatibility capability retained for older probe payloads.
    ExecuteTool,
    PlanAssimilation,
    VerifyClaim,
}

impl Capability {
    pub fn is_tool_family(&self) -> bool {
        matches!(
            self,
            Capability::WorkspaceRead
                | Capability::WorkspaceSearch
                | Capability::WebSearch
                | Capability::WebFetch
                | Capability::ToolRoute
                | Capability::ExecuteTool
        )
    }

    pub fn probe_keys(&self) -> &'static [&'static str] {
        match self {
            Capability::ReadMemory => &["read_memory"],
            Capability::MutateTask => &["mutate_task"],
            Capability::WorkspaceRead => &["workspace_read"],
            Capability::WorkspaceSearch => &["workspace_search"],
            Capability::WebSearch => &["web_search"],
            Capability::WebFetch => &["web_fetch"],
            Capability::ToolRoute => &["tool_route"],
            Capability::ExecuteTool => &["tool_route"],
            Capability::PlanAssimilation => &["plan_assimilation"],
            Capability::VerifyClaim => &["verify_claim"],
        }
    }

    pub fn primary_tool_for(
        operation_kind: &OperationKind,
        resource_kind: &ResourceKind,
    ) -> Capability {
        match (resource_kind, operation_kind) {
            (ResourceKind::Workspace, OperationKind::Search) => Capability::WorkspaceSearch,
            (ResourceKind::Workspace, OperationKind::Read) => Capability::WorkspaceRead,
            (ResourceKind::Web, OperationKind::Fetch) => Capability::WebFetch,
            (ResourceKind::Web, OperationKind::Search | OperationKind::Compare) => {
                Capability::WebSearch
            }
            (ResourceKind::Tooling, _) => Capability::ToolRoute,
            (ResourceKind::Mixed, OperationKind::Fetch) => Capability::WebFetch,
            (ResourceKind::Mixed, _) => Capability::ToolRoute,
            (ResourceKind::Web, _) => Capability::WebSearch,
            (ResourceKind::Workspace, _) => Capability::WorkspaceRead,
            _ => Capability::ToolRoute,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreContractCall {
    ToolCapabilityProbe,
    ToolBrokerRequest,
    TaskFabricProposal,
    ContextAtomAppend,
    ContextTopologyMaterialize,
    ContextTopologyInspect,
    UnifiedMemoryRead,
    VerifierRequest,
    AssimilationPlanRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationRequest {
    pub session_id: String,
    pub intent: String,
    #[serde(default)]
    pub surface: RequestSurface,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestSurface {
    #[default]
    Legacy,
    Cli,
    Gateway,
    Sdk,
    Dashboard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserConstraint {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetDescriptor {
    WorkspacePath {
        value: String,
    },
    Url {
        value: String,
    },
    TaskId {
        value: String,
    },
    MemoryRef {
        scope: String,
        object_id: Option<String>,
    },
    ToolName {
        value: String,
    },
    Unknown {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedOrchestrationRequest {
    pub session_id: String,
    pub surface: RequestSurface,
    pub legacy_intent: String,
    pub adapted: bool,
    pub payload: Value,
    pub request_kind: RequestKind,
    pub operation_kind: OperationKind,
    pub resource_kind: ResourceKind,
    pub mutability: Mutability,
    pub target_descriptors: Vec<TargetDescriptor>,
    pub target_refs: Vec<String>,
    pub tool_hints: Vec<String>,
    pub policy_scope: PolicyScope,
    pub user_constraints: Vec<UserConstraint>,
    pub core_probe_envelope: Option<CoreProbeEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityProbeSnapshot {
    pub capability: Capability,
    pub tool_available: Option<bool>,
    pub target_supplied: Option<bool>,
    pub target_syntactically_valid: Option<bool>,
    pub target_exists: Option<bool>,
    pub authorization_valid: Option<bool>,
    pub policy_allows: Option<bool>,
    pub transport_available: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreProbeEnvelope {
    pub probes: Vec<CapabilityProbeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreExecutionStepObservation {
    pub step_id: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreExecutionObservation {
    pub plan_status: Option<PlanStatus>,
    pub receipt_ids: Vec<String>,
    pub outcome_refs: Vec<String>,
    pub step_statuses: Vec<CoreExecutionStepObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationExecutionObservationUpdate {
    pub session_id: String,
    pub observation: CoreExecutionObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextAtomAppendSourceKind {
    InteractionUnit,
    ToolResultBundle,
    StatusSummary,
    WorkflowBoundary,
    ExternalReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextAtomAppendCall {
    pub session_id: String,
    pub source_kind: ContextAtomAppendSourceKind,
    pub source_ref: String,
    pub token_count: u32,
    pub task_refs: Vec<String>,
    pub memory_version_refs: Vec<String>,
    pub semantic_boundary: bool,
    pub workflow_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextTopologyMaterializeCall {
    pub session_id: String,
    pub budget_tokens: u32,
    pub pinned_anchor_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityReason {
    UnknownOperation,
    MultipleOperationCandidates,
    MultipleResourceCandidates,
    MissingTargetSignals,
    SurfaceAdapterFallback,
    UnresolvedTargetDomain,
    LowConfidence,
    LegacyCompatOnly,
    TypedProbeContractViolation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseResult {
    pub typed_request: TypedOrchestrationRequest,
    pub confidence: f32,
    pub ambiguity: Vec<AmbiguityReason>,
    pub reasons: Vec<String>,
    pub surface_adapter_used: bool,
    pub surface_adapter_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationReason {
    MissingSessionId,
    AmbiguousOperation,
    TypedProbeContractViolation,
    MissingTargetRefs,
    MutationScopeRequired,
    PlannerGap,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestClassification {
    pub request_class: RequestClass,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub required_capabilities: Vec<Capability>,
    pub clarification_reasons: Vec<ClarificationReason>,
    pub needs_clarification: bool,
    pub surface_adapter_used: bool,
    pub surface_adapter_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precondition {
    ToolAvailable,
    TargetSupplied,
    TargetSyntacticallyValid,
    TargetExists,
    AuthorizationValid,
    PolicyAllows,
    TransportAvailable,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationReason {
    ToolUnavailable,
    AuthFailure,
    PolicyDenied,
    MissingTarget,
    TargetInvalid,
    TargetNotFound,
    TransportFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlanStep {
    pub step_id: String,
    pub operation: String,
    pub target_contract: CoreContractCall,
    pub capability: Capability,
    pub merged_capabilities: Vec<Capability>,
    pub rationale: Vec<String>,
    pub expected_contract_refs: Vec<String>,
    pub blocked_on: Vec<Precondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanVariant {
    Fastest,
    Safest,
    DegradedFallback,
    ClarificationFirst,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanScore {
    pub overall: f32,
    pub authority_cost: f32,
    pub transport_dependency: f32,
    pub mutation_risk: f32,
    pub fallback_quality: f32,
    pub target_specificity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityProbeResult {
    pub capability: Capability,
    pub blocked_on: Vec<Precondition>,
    pub degradation_reasons: Vec<DegradationReason>,
    pub can_degrade: bool,
    pub probe_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanCandidate {
    pub plan_id: String,
    pub variant: PlanVariant,
    pub steps: Vec<OrchestrationPlanStep>,
    pub mutates_session_context: bool,
    pub context_preparation_rationale: Option<String>,
    #[serde(default)]
    pub decomposition_family: String,
    #[serde(default)]
    pub capability_graph: Vec<Capability>,
    #[serde(default)]
    pub contract_family: String,
    pub confidence: f32,
    pub score: PlanScore,
    pub requires_clarification: bool,
    pub blocked_on: Vec<Precondition>,
    pub degradation: Vec<DegradationReason>,
    pub capabilities: Vec<Capability>,
    pub capability_probes: Vec<CapabilityProbeResult>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Planned,
    ClarificationRequired,
    Blocked,
    Degraded,
    Ready,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Ready,
    Blocked,
    Degraded,
    Skipped,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDecision {
    None,
    Clarify,
    Degrade,
    Halt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    MissingTarget,
    TargetInvalid,
    TargetNotFound,
    ToolUnavailable,
    ToolFailureBudgetExceeded,
    AuthorizationFailure,
    PolicyDenied,
    PlannerContradiction,
    TransportFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepState {
    pub step_id: String,
    pub status: StepStatus,
    pub blocked_on: Vec<Precondition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryState {
    pub decision: RecoveryDecision,
    pub reason: Option<RecoveryReason>,
    pub retryable: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DegradationState {
    pub reasons: Vec<DegradationReason>,
    pub alternate_path: Vec<CoreContractCall>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlPlaneDecisionTraceStep {
    pub step_id: String,
    pub inputs: Vec<String>,
    pub chosen_path: String,
    pub alternatives_rejected: Vec<String>,
    pub confidence: f32,
    pub receipt_metadata: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlPlaneDecisionTrace {
    pub chosen: String,
    pub alternatives_rejected: Vec<String>,
    pub confidence: f32,
    pub rationale: Vec<String>,
    #[serde(default)]
    pub receipt_metadata: Vec<String>,
    #[serde(default)]
    pub step_records: Vec<ControlPlaneDecisionTraceStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReceiptDebugMetadata {
    pub decision_trace: ControlPlaneDecisionTrace,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCorrelation {
    pub orchestration_trace_id: String,
    pub expected_core_contract_ids: Vec<String>,
    pub observed_core_receipt_ids: Vec<String>,
    pub observed_core_outcome_refs: Vec<String>,
    pub receipt_metadata: ReceiptDebugMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionState {
    pub plan_status: PlanStatus,
    pub steps: Vec<StepState>,
    pub recovery: Option<RecoveryState>,
    pub degradation: Option<DegradationState>,
    pub correlation: ExecutionCorrelation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub request_class: RequestClass,
    pub classification: RequestClassification,
    pub posture: ExecutionPosture,
    pub needs_clarification: bool,
    pub clarification_prompt: Option<String>,
    pub selected_plan: PlanCandidate,
    pub alternative_plans: Vec<PlanCandidate>,
    pub execution_state: ExecutionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolFallbackContext {
    pub tool_name: String,
    pub backend: String,
    pub backend_class: ToolBackendClass,
    pub reason_code: ToolReasonCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationFallbackAction {
    pub kind: String,
    pub label: String,
    pub reason: String,
    pub backend_class: Option<ToolBackendClass>,
    pub reason_code: Option<ToolReasonCode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeQualitySignals {
    pub candidate_count: u32,
    pub selected_variant: PlanVariant,
    pub selected_plan_degraded: bool,
    pub selected_plan_requires_clarification: bool,
    pub used_heuristic_probe: bool,
    pub heuristic_probe_source_count: u32,
    pub blocked_precondition_count: u32,
    pub executable_candidate_count: u32,
    pub degraded_candidate_count: u32,
    pub clarification_candidate_count: u32,
    pub zero_executable_candidates: bool,
    pub all_candidates_degraded: bool,
    pub all_candidates_require_clarification: bool,
    pub surface_adapter_fallback: bool,
    pub typed_probe_contract_gap_count: u32,
    pub decision_rationale_count: u32,
    pub fallback_action_count: u32,
    pub tool_failure_budget_failed_step_count: u32,
    pub tool_failure_budget_limit: u32,
    pub tool_failure_budget_exceeded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgeCodeWorkflowQualitySignals {
    pub mcp_alias_route_required: bool,
    pub retry_backoff_contract_required: bool,
    pub mcp_transport_fallback_required: bool,
    pub semantic_discovery_route_required: bool,
    pub exact_pattern_search_required: bool,
    pub known_path_direct_read_required: bool,
    pub parallel_independent_tool_calls_required: bool,
    pub grounded_verification_required: bool,
    pub step_checkpointing_required: bool,
    pub completion_hygiene_required: bool,
    pub specialized_tool_usage_required: bool,
    pub shell_terminal_only_usage_required: bool,
    pub simple_lookup_locality_hygiene_required: bool,
    pub subagent_brief_contract_required: bool,
    pub subagent_output_contract_required: bool,
    pub subagent_result_synthesis_required: bool,
    pub mcp_retry_reason_count: u32,
    pub mcp_transport_fallback_action_count: u32,
    pub mcp_retry_recovery_active: bool,
    pub mcp_diagnostic_summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "workflow", content = "signals", rename_all = "snake_case")]
pub enum WorkflowQualitySignals {
    ForgeCode(ForgeCodeWorkflowQualitySignals),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTemplate {
    ClarifyThenCoordinate,
    ResearchSynthesizeVerify,
    PlanExecuteReview,
    DiagnoseRetryEscalate,
    CodexToolingSynthesis,
    ForgeCodeAgentComposition,
    ForgeCodeRawCapabilityAssimilation,
    OpenHandsControlPlaneAssimilation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStage {
    IntakeNormalization,
    DecompositionPlanning,
    CoordinationSequencing,
    RecoveryEscalation,
    ResultPackaging,
    VerificationClosure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStageStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStageState {
    pub stage: WorkflowStage,
    pub status: WorkflowStageStatus,
    pub owner: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneHandoff {
    pub handoff_id: String,
    pub from: String,
    pub to: String,
    pub owner: String,
    pub artifact: String,
    pub status: WorkflowStageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosureState {
    Pending,
    Ready,
    Complete,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneClosureState {
    pub verification: ClosureState,
    pub receipt_correlation: ClosureState,
    pub memory_packaging: ClosureState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneLifecycleState {
    pub owner: String,
    pub template: WorkflowTemplate,
    pub active_stage: WorkflowStage,
    pub stages: Vec<WorkflowStageState>,
    pub handoff_chain: Vec<ControlPlaneHandoff>,
    pub next_actions: Vec<String>,
    pub closure: ControlPlaneClosureState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationResultPackage {
    pub summary: String,
    pub progress_message: String,
    pub execution_state: ExecutionState,
    pub recovery_applied: bool,
    pub fallback_actions: Vec<OrchestrationFallbackAction>,
    pub core_contract_calls: Vec<CoreContractCall>,
    pub requires_core_promotion: bool,
    pub classification: RequestClassification,
    pub selected_plan: PlanCandidate,
    pub alternative_plans: Vec<PlanCandidate>,
    pub runtime_quality: RuntimeQualitySignals,
    pub workflow_quality: Option<WorkflowQualitySignals>,
    pub decision_trace: ControlPlaneDecisionTrace,
    pub workflow_template: WorkflowTemplate,
    pub control_plane_lifecycle: ControlPlaneLifecycleState,
}
