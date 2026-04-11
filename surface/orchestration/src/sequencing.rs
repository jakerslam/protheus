use crate::contracts::{
    CoreContractCall, OrchestrationFallbackAction, OrchestrationPlanStep, OrchestrationRequest,
    RequestClass,
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

pub fn fallback_actions(
    request: &OrchestrationRequest,
    request_class: RequestClass,
) -> Vec<OrchestrationFallbackAction> {
    match request_class {
        RequestClass::ToolCall => vec![
            OrchestrationFallbackAction {
                kind: "inspect_tool_capabilities".to_string(),
                label: "Check available tools".to_string(),
                reason: "probe the governed tool surface before retrying".to_string(),
            },
            OrchestrationFallbackAction {
                kind: "narrow_tool_request".to_string(),
                label: "Retry with narrower input".to_string(),
                reason: "reduce ambiguity in the tool payload or query".to_string(),
            },
            OrchestrationFallbackAction {
                kind: if request.intent.to_ascii_lowercase().contains("file")
                    || request.intent.to_ascii_lowercase().contains("workspace")
                {
                    "paste_workspace_context".to_string()
                } else {
                    "ask_for_source_material".to_string()
                },
                label: "Provide direct source context".to_string(),
                reason: "fallback to explicit files, paths, or pasted content when tools are blocked".to_string(),
            },
        ],
        _ => Vec::new(),
    }
}
