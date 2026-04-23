// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{Capability, CoreContractCall, OrchestrationPlanStep, Precondition};

#[derive(Debug, Clone)]
pub struct CapabilitySpec {
    pub requires: Vec<Precondition>,
    pub primary_steps: Vec<OrchestrationPlanStep>,
    pub degraded_steps: Vec<OrchestrationPlanStep>,
}

pub fn spec_for(capability: &Capability) -> CapabilitySpec {
    match capability {
        Capability::ReadMemory => CapabilitySpec {
            requires: Vec::new(),
            primary_steps: vec![
                step(
                    Capability::ReadMemory,
                    "step_context_topology_inspect",
                    "inspect_context_topology",
                    CoreContractCall::ContextTopologyInspect,
                ),
                step(
                    Capability::ReadMemory,
                    "step_context_topology_materialize",
                    "request_context_topology_materialization",
                    CoreContractCall::ContextTopologyMaterialize,
                ),
            ],
            degraded_steps: vec![step(
                Capability::ReadMemory,
                "step_memory_read_compat",
                "request_materialized_view_compat",
                CoreContractCall::UnifiedMemoryRead,
            )],
        },
        Capability::MutateTask => CapabilitySpec {
            requires: vec![Precondition::AuthorizationValid, Precondition::PolicyAllows],
            primary_steps: vec![step(
                Capability::MutateTask,
                "step_task_fabric_proposal",
                "propose_task_graph_update",
                CoreContractCall::TaskFabricProposal,
            )],
            degraded_steps: Vec::new(),
        },
        Capability::WorkspaceRead => tool_spec(
            Capability::WorkspaceRead,
            "workspace_read",
            "route_workspace_read",
        ),
        Capability::WorkspaceSearch => tool_spec(
            Capability::WorkspaceSearch,
            "workspace_search",
            "route_workspace_search",
        ),
        Capability::WebSearch => tool_spec(Capability::WebSearch, "web_search", "route_web_search"),
        Capability::WebFetch => tool_spec(Capability::WebFetch, "web_fetch", "route_web_fetch"),
        Capability::ToolRoute => tool_spec(Capability::ToolRoute, "tool_route", "route_tool_call"),
        Capability::ExecuteTool => {
            tool_spec(Capability::ExecuteTool, "tool_route", "route_tool_call")
        }
        Capability::PlanAssimilation => CapabilitySpec {
            requires: vec![Precondition::TargetExists, Precondition::PolicyAllows],
            primary_steps: vec![step(
                Capability::PlanAssimilation,
                "step_assimilation_plan",
                "prepare_assimilation_plan",
                CoreContractCall::AssimilationPlanRequest,
            )],
            degraded_steps: Vec::new(),
        },
        Capability::VerifyClaim => CapabilitySpec {
            requires: vec![Precondition::TransportAvailable],
            primary_steps: vec![
                step(
                    Capability::VerifyClaim,
                    "step_claim_verification_read",
                    "request_context_topology_materialization",
                    CoreContractCall::ContextTopologyMaterialize,
                ),
                step(
                    Capability::VerifyClaim,
                    "step_claim_verifier_request",
                    "verify_claim_bundle",
                    CoreContractCall::VerifierRequest,
                ),
            ],
            degraded_steps: vec![
                step(
                    Capability::VerifyClaim,
                    "step_claim_verification_topology_fallback",
                    "request_context_topology_materialization",
                    CoreContractCall::ContextTopologyMaterialize,
                ),
                step(
                    Capability::VerifyClaim,
                    "step_claim_verification_fallback",
                    "request_materialized_view",
                    CoreContractCall::UnifiedMemoryRead,
                ),
            ],
        },
    }
}

pub fn context_preparation_step() -> OrchestrationPlanStep {
    step(
        Capability::ReadMemory,
        "step_context_atom_append_prepare",
        "append_context_atom",
        CoreContractCall::ContextAtomAppend,
    )
}

fn step(
    capability: Capability,
    step_id: &str,
    operation: &str,
    target_contract: CoreContractCall,
) -> OrchestrationPlanStep {
    let merged_capabilities = vec![capability.clone()];
    let rationale = vec![format!("capability:{capability:?}").to_lowercase()];
    OrchestrationPlanStep {
        step_id: step_id.to_string(),
        operation: operation.to_string(),
        target_contract,
        capability,
        merged_capabilities,
        rationale,
        expected_contract_refs: vec![format!("expect_{step_id}")],
        blocked_on: Vec::new(),
    }
}

fn tool_spec(capability: Capability, step_key: &str, route_operation: &str) -> CapabilitySpec {
    CapabilitySpec {
        requires: vec![
            Precondition::ToolAvailable,
            Precondition::TransportAvailable,
        ],
        primary_steps: vec![
            step(
                capability.clone(),
                &format!("step_{step_key}_capability_probe"),
                "probe_tool_capability",
                CoreContractCall::ToolCapabilityProbe,
            ),
            step(
                capability.clone(),
                &format!("step_{step_key}_broker_request"),
                route_operation,
                CoreContractCall::ToolBrokerRequest,
            ),
        ],
        degraded_steps: vec![step(
            capability,
            &format!("step_{step_key}_memory_fallback"),
            "request_materialized_view",
            CoreContractCall::UnifiedMemoryRead,
        )],
    }
}
