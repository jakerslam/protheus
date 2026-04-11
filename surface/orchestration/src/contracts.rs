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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlanStep {
    pub step_id: String,
    pub operation: String,
    pub target_contract: CoreContractCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub request_class: RequestClass,
    pub posture: ExecutionPosture,
    pub needs_clarification: bool,
    pub clarification_prompt: Option<String>,
    pub steps: Vec<OrchestrationPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationFallbackAction {
    pub kind: String,
    pub label: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationResultPackage {
    pub summary: String,
    pub progress_message: String,
    pub recovery_applied: bool,
    pub fallback_actions: Vec<OrchestrationFallbackAction>,
    pub core_contract_calls: Vec<CoreContractCall>,
    pub requires_core_promotion: bool,
}
