use protheus_tooling_core_v1::{ToolBackendClass, ToolReasonCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    ExecuteTool,
    PlanAssimilation,
    VerifyClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreContractCall {
    ToolCapabilityProbe,
    ToolBrokerRequest,
    TaskFabricProposal,
    UnifiedMemoryRead,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseResult {
    pub typed_request: TypedOrchestrationRequest,
    pub confidence: f32,
    pub ambiguity: Vec<AmbiguityReason>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationReason {
    MissingSessionId,
    AmbiguousOperation,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precondition {
    ToolAvailable,
    TargetExists,
    AuthorizationValid,
    PolicyAllows,
    TransportAvailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationReason {
    ToolUnavailable,
    AuthFailure,
    PolicyDenied,
    MissingTarget,
    TransportFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlanStep {
    pub step_id: String,
    pub operation: String,
    pub target_contract: CoreContractCall,
    pub capability: Capability,
    pub blocked_on: Vec<Precondition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanCandidate {
    pub plan_id: String,
    pub steps: Vec<OrchestrationPlanStep>,
    pub confidence: f32,
    pub requires_clarification: bool,
    pub blocked_on: Vec<Precondition>,
    pub degradation: Option<DegradationReason>,
    pub capabilities: Vec<Capability>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Ready,
    Blocked,
    Degraded,
    Skipped,
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
    ToolUnavailable,
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
    pub reason: DegradationReason,
    pub alternate_path: Vec<CoreContractCall>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionState {
    pub plan_status: PlanStatus,
    pub steps: Vec<StepState>,
    pub recovery: Option<RecoveryState>,
    pub degradation: Option<DegradationState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub request_class: RequestClass,
    pub classification: RequestClassification,
    pub posture: ExecutionPosture,
    pub needs_clarification: bool,
    pub clarification_prompt: Option<String>,
    pub selected_plan: PlanCandidate,
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
}
