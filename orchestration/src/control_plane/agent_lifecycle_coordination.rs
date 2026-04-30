// Layer ownership: orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLifecycleIntent {
    Select,
    Stop,
    Archive,
    ArchiveBatch,
    DeleteArchived,
    Revive,
    Rebind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLifecycleSnapshot {
    Active,
    Missing,
    Archived,
    Terminated,
    SystemThread,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLifecycleRequest {
    pub intent: AgentLifecycleIntent,
    pub agent_id: Option<String>,
    pub snapshot: AgentLifecycleSnapshot,
    pub batch_target_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLifecycleOperation {
    RequestCoreLifecycleMutation {
        operation: String,
        agent_id: Option<String>,
    },
    RequestBatchLifecycleMutation {
        operation: String,
        target_count: usize,
    },
    RequestRebindPlan {
        agent_id: String,
    },
    RenderProjectionOnly {
        reason: String,
    },
    Clarify {
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLifecycleCoordinationPlan {
    pub operation: AgentLifecycleOperation,
    pub telemetry_note: String,
}

pub struct AgentLifecycleCoordinationContract;

impl SubdomainContract for AgentLifecycleCoordinationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "agent_lifecycle_coordination",
        legacy_module_bindings: &[
            "agents_lifecycle_archive_helpers",
            "agents_detail_control_helpers",
            "chat_agent_lifecycle_helpers",
            "chat_agent_selection_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "agent_lifecycle_recommendation",
            "agent_rebind_projection",
            "agent_batch_lifecycle_projection",
        ],
        message_boundaries: &[
            "agent_lifecycle_to_shell_projection_boundary",
            "agent_lifecycle_to_core_mutation_boundary",
            "agent_lifecycle_to_rebind_boundary",
        ],
    }
}

pub fn coordinate_agent_lifecycle(
    request: &AgentLifecycleRequest,
) -> AgentLifecycleCoordinationPlan {
    if request.intent != AgentLifecycleIntent::ArchiveBatch && request.agent_id.is_none() {
        return AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::Clarify {
                prompt: "agent lifecycle intent requires an agent id".to_string(),
            },
            telemetry_note: "missing agent id for lifecycle request".to_string(),
        };
    }

    match request.intent {
        AgentLifecycleIntent::Select => coordinate_select(request),
        AgentLifecycleIntent::Stop => mutation("stop", request),
        AgentLifecycleIntent::Archive => mutation("archive", request),
        AgentLifecycleIntent::DeleteArchived => mutation("delete_archived", request),
        AgentLifecycleIntent::Revive => coordinate_revive(request),
        AgentLifecycleIntent::Rebind => coordinate_rebind(request),
        AgentLifecycleIntent::ArchiveBatch => coordinate_batch_archive(request),
    }
}

fn coordinate_select(request: &AgentLifecycleRequest) -> AgentLifecycleCoordinationPlan {
    match request.snapshot {
        AgentLifecycleSnapshot::SystemThread => AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::RenderProjectionOnly {
                reason: "system thread activation is a shell projection of backend state"
                    .to_string(),
            },
            telemetry_note: "system thread selection does not mutate lifecycle truth".to_string(),
        },
        AgentLifecycleSnapshot::Missing => AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::Clarify {
                prompt: "selected agent is missing from lifecycle snapshot".to_string(),
            },
            telemetry_note: "selection rejected because agent is missing".to_string(),
        },
        AgentLifecycleSnapshot::Archived | AgentLifecycleSnapshot::Terminated => {
            coordinate_rebind(request)
        }
        AgentLifecycleSnapshot::Active => AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::RenderProjectionOnly {
                reason: "active agent selection is a projection update".to_string(),
            },
            telemetry_note: "active agent selection should reconcile against backend snapshot"
                .to_string(),
        },
    }
}

fn coordinate_revive(request: &AgentLifecycleRequest) -> AgentLifecycleCoordinationPlan {
    if !matches!(
        request.snapshot,
        AgentLifecycleSnapshot::Archived | AgentLifecycleSnapshot::Terminated
    ) {
        return AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::Clarify {
                prompt: "revive requires an archived or terminated agent".to_string(),
            },
            telemetry_note: "revive rejected for non-archived lifecycle state".to_string(),
        };
    }
    mutation("revive", request)
}

fn coordinate_rebind(request: &AgentLifecycleRequest) -> AgentLifecycleCoordinationPlan {
    let agent_id = request.agent_id.clone().unwrap_or_default();
    AgentLifecycleCoordinationPlan {
        operation: AgentLifecycleOperation::RequestRebindPlan { agent_id },
        telemetry_note: "request backend rebind plan instead of shell inference".to_string(),
    }
}

fn coordinate_batch_archive(request: &AgentLifecycleRequest) -> AgentLifecycleCoordinationPlan {
    if request.batch_target_count == 0 {
        return AgentLifecycleCoordinationPlan {
            operation: AgentLifecycleOperation::Clarify {
                prompt: "batch archive requires at least one target agent".to_string(),
            },
            telemetry_note: "batch archive rejected because target set is empty".to_string(),
        };
    }
    AgentLifecycleCoordinationPlan {
        operation: AgentLifecycleOperation::RequestBatchLifecycleMutation {
            operation: "archive_batch".to_string(),
            target_count: request.batch_target_count,
        },
        telemetry_note: "request backend batch lifecycle mutation with receipts".to_string(),
    }
}

fn mutation(operation: &str, request: &AgentLifecycleRequest) -> AgentLifecycleCoordinationPlan {
    AgentLifecycleCoordinationPlan {
        operation: AgentLifecycleOperation::RequestCoreLifecycleMutation {
            operation: operation.to_string(),
            agent_id: request.agent_id.clone(),
        },
        telemetry_note: "request core/runtime lifecycle mutation with receipt".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        intent: AgentLifecycleIntent,
        snapshot: AgentLifecycleSnapshot,
    ) -> AgentLifecycleRequest {
        AgentLifecycleRequest {
            intent,
            agent_id: Some("misty".to_string()),
            snapshot,
            batch_target_count: 0,
        }
    }

    #[test]
    fn missing_agent_clarifies_instead_of_shell_rebinding() {
        let plan = coordinate_agent_lifecycle(&request(
            AgentLifecycleIntent::Select,
            AgentLifecycleSnapshot::Missing,
        ));

        assert_eq!(
            plan.operation,
            AgentLifecycleOperation::Clarify {
                prompt: "selected agent is missing from lifecycle snapshot".to_string()
            }
        );
    }

    #[test]
    fn archived_agent_selection_requests_rebind_plan() {
        let plan = coordinate_agent_lifecycle(&request(
            AgentLifecycleIntent::Select,
            AgentLifecycleSnapshot::Archived,
        ));

        assert_eq!(
            plan.operation,
            AgentLifecycleOperation::RequestRebindPlan {
                agent_id: "misty".to_string()
            }
        );
    }

    #[test]
    fn revive_archived_agent_requests_core_mutation() {
        let plan = coordinate_agent_lifecycle(&request(
            AgentLifecycleIntent::Revive,
            AgentLifecycleSnapshot::Archived,
        ));

        assert_eq!(
            plan.operation,
            AgentLifecycleOperation::RequestCoreLifecycleMutation {
                operation: "revive".to_string(),
                agent_id: Some("misty".to_string())
            }
        );
    }

    #[test]
    fn revive_active_agent_clarifies() {
        let plan = coordinate_agent_lifecycle(&request(
            AgentLifecycleIntent::Revive,
            AgentLifecycleSnapshot::Active,
        ));

        assert!(matches!(
            plan.operation,
            AgentLifecycleOperation::Clarify { .. }
        ));
    }

    #[test]
    fn batch_archive_requests_backend_batch_mutation() {
        let plan = coordinate_agent_lifecycle(&AgentLifecycleRequest {
            intent: AgentLifecycleIntent::ArchiveBatch,
            agent_id: None,
            snapshot: AgentLifecycleSnapshot::Active,
            batch_target_count: 3,
        });

        assert_eq!(
            plan.operation,
            AgentLifecycleOperation::RequestBatchLifecycleMutation {
                operation: "archive_batch".to_string(),
                target_count: 3
            }
        );
    }
}
