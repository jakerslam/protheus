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
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserConstraint {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedOrchestrationRequest {
    pub session_id: String,
    pub legacy_intent: String,
    pub payload: Value,
    pub request_kind: RequestKind,
    pub operation_kind: OperationKind,
    pub resource_kind: ResourceKind,
    pub mutability: Mutability,
    pub target_refs: Vec<String>,
    pub tool_hints: Vec<String>,
    pub policy_scope: PolicyScope,
    pub user_constraints: Vec<UserConstraint>,
    pub parse_confidence: f32,
    pub parse_reasons: Vec<String>,
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
    pub required_contracts: Vec<CoreContractCall>,
    pub clarification_reasons: Vec<ClarificationReason>,
    pub needs_clarification: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlanStep {
    pub step_id: String,
    pub operation: String,
    pub target_contract: CoreContractCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub request_class: RequestClass,
    pub classification: RequestClassification,
    pub posture: ExecutionPosture,
    pub needs_clarification: bool,
    pub clarification_prompt: Option<String>,
    pub steps: Vec<OrchestrationPlanStep>,
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
    pub recovery_applied: bool,
    pub fallback_actions: Vec<OrchestrationFallbackAction>,
    pub core_contract_calls: Vec<CoreContractCall>,
    pub requires_core_promotion: bool,
    pub classification: RequestClassification,
}
