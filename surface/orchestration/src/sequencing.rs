use crate::contracts::{
    CoreContractCall, OrchestrationPlanStep, OrchestrationRequest, RequestClass,
};

pub fn build_steps(
    _request: &OrchestrationRequest,
    request_class: RequestClass,
) -> Vec<OrchestrationPlanStep> {
    match request_class {
        RequestClass::ToolCall => vec![
            OrchestrationPlanStep {
                step_id: "step_tool_capability_probe".to_string(),
                operation: "probe_tool_capability".to_string(),
                target_contract: CoreContractCall::ToolCapabilityProbe,
            },
            OrchestrationPlanStep {
                step_id: "step_tool_broker_request".to_string(),
                operation: "route_tool_call".to_string(),
                target_contract: CoreContractCall::ToolBrokerRequest,
            },
        ],
        RequestClass::Assimilation => vec![
            OrchestrationPlanStep {
                step_id: "step_assimilation_plan".to_string(),
                operation: "request_assimilation_plan".to_string(),
                target_contract: CoreContractCall::AssimilationPlanRequest,
            },
            OrchestrationPlanStep {
                step_id: "step_task_fabric_proposal".to_string(),
                operation: "propose_assimilation_task".to_string(),
                target_contract: CoreContractCall::TaskFabricProposal,
            },
        ],
        RequestClass::TaskProposal | RequestClass::Mutation => vec![OrchestrationPlanStep {
            step_id: "step_task_fabric_proposal".to_string(),
            operation: "propose_task_graph_update".to_string(),
            target_contract: CoreContractCall::TaskFabricProposal,
        }],
        RequestClass::ReadOnly => vec![OrchestrationPlanStep {
            step_id: "step_memory_read".to_string(),
            operation: "request_materialized_view".to_string(),
            target_contract: CoreContractCall::UnifiedMemoryRead,
        }],
    }
}
