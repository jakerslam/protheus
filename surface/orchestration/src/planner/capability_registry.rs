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
            primary_steps: vec![step(
                Capability::ReadMemory,
                "step_memory_read",
                "request_materialized_view",
                CoreContractCall::UnifiedMemoryRead,
            )],
            degraded_steps: Vec::new(),
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
        Capability::ExecuteTool => CapabilitySpec {
            requires: vec![
                Precondition::ToolAvailable,
                Precondition::TransportAvailable,
            ],
            primary_steps: vec![
                step(
                    Capability::ExecuteTool,
                    "step_tool_capability_probe",
                    "probe_tool_capability",
                    CoreContractCall::ToolCapabilityProbe,
                ),
                step(
                    Capability::ExecuteTool,
                    "step_tool_broker_request",
                    "route_tool_call",
                    CoreContractCall::ToolBrokerRequest,
                ),
            ],
            degraded_steps: vec![step(
                Capability::ExecuteTool,
                "step_memory_fallback",
                "request_materialized_view",
                CoreContractCall::UnifiedMemoryRead,
            )],
        },
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
                    "request_materialized_view",
                    CoreContractCall::UnifiedMemoryRead,
                ),
                step(
                    Capability::VerifyClaim,
                    "step_claim_verifier_request",
                    "verify_claim_bundle",
                    CoreContractCall::VerifierRequest,
                ),
            ],
            degraded_steps: vec![step(
                Capability::VerifyClaim,
                "step_claim_verification_fallback",
                "request_materialized_view",
                CoreContractCall::UnifiedMemoryRead,
            )],
        },
    }
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
        expected_contract_ref: format!("expect_{step_id}"),
        blocked_on: Vec::new(),
    }
}
