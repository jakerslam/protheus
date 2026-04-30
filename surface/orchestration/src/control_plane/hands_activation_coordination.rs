// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandsIntent {
    CheckDependencies,
    InstallDependencies,
    Activate,
    Deactivate,
    LaunchInstance,
    ProjectStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyState {
    Satisfied,
    Missing,
    InstallFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandsActivationRequest {
    pub intent: HandsIntent,
    pub hand_id: Option<String>,
    pub instance_id: Option<String>,
    pub dependency_state: DependencyState,
    pub config_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandsJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandsJobReceipt {
    pub job_id: String,
    pub status: HandsJobStatus,
    pub durable_receipt_present: bool,
    pub progress_event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandsCoordinationAction {
    RequestOrchestrationJob {
        job_kind: String,
        hand_id: Option<String>,
    },
    RequestRuntimeInstanceMutation {
        operation: String,
        instance_id: String,
    },
    ProjectJobReceipt {
        job_id: String,
        status: HandsJobStatus,
    },
    ProjectInstanceStats {
        instance_id: String,
    },
    Clarify {
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandsCoordinationPlan {
    pub action: HandsCoordinationAction,
    pub telemetry_note: String,
}

pub struct HandsActivationCoordinationContract;

impl SubdomainContract for HandsActivationCoordinationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "hands_activation_coordination",
        legacy_module_bindings: &[
            "hands",
            "hands_setup_wizard_helpers",
            "hands_dashboard_viewer_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "hands_activation_job_projection",
            "hands_dependency_projection",
            "hands_instance_receipt_projection",
        ],
        message_boundaries: &[
            "hands_to_shell_projection_boundary",
            "hands_to_runtime_admission_boundary",
            "hands_to_dependency_job_boundary",
        ],
    }
}

pub fn coordinate_hands_intent(request: &HandsActivationRequest) -> HandsCoordinationPlan {
    match request.intent {
        HandsIntent::CheckDependencies => require_hand_job(
            request,
            "check_dependencies",
            "request backend dependency check job",
        ),
        HandsIntent::InstallDependencies => coordinate_install(request),
        HandsIntent::Activate | HandsIntent::LaunchInstance => coordinate_activation(request),
        HandsIntent::Deactivate => coordinate_deactivation(request),
        HandsIntent::ProjectStats => coordinate_stats(request),
    }
}

pub fn project_hands_job_receipt(receipt: &HandsJobReceipt) -> HandsCoordinationPlan {
    if !receipt.durable_receipt_present {
        return HandsCoordinationPlan {
            action: HandsCoordinationAction::Clarify {
                prompt: "hands job projection requires a durable receipt".to_string(),
            },
            telemetry_note: "hands job receipt missing durable receipt marker".to_string(),
        };
    }
    HandsCoordinationPlan {
        action: HandsCoordinationAction::ProjectJobReceipt {
            job_id: receipt.job_id.clone(),
            status: receipt.status.clone(),
        },
        telemetry_note: "project hands job status from durable receipt".to_string(),
    }
}

fn coordinate_install(request: &HandsActivationRequest) -> HandsCoordinationPlan {
    match request.dependency_state {
        DependencyState::Satisfied => HandsCoordinationPlan {
            action: HandsCoordinationAction::ProjectJobReceipt {
                job_id: "dependencies_already_satisfied".to_string(),
                status: HandsJobStatus::Succeeded,
            },
            telemetry_note: "dependency install skipped because backend reports satisfied"
                .to_string(),
        },
        DependencyState::Missing | DependencyState::InstallFailed => require_hand_job(
            request,
            "install_dependencies",
            "request backend dependency install job with progress receipts",
        ),
    }
}

fn coordinate_activation(request: &HandsActivationRequest) -> HandsCoordinationPlan {
    if request.dependency_state != DependencyState::Satisfied {
        return require_hand_job(
            request,
            "check_dependencies",
            "activation blocked until backend dependency check succeeds",
        );
    }
    if !request.config_present {
        return HandsCoordinationPlan {
            action: HandsCoordinationAction::Clarify {
                prompt: "hands activation requires submitted configuration".to_string(),
            },
            telemetry_note: "hands activation missing configuration payload".to_string(),
        };
    }
    require_hand_job(
        request,
        "activate",
        "request backend hands activation job with durable receipt",
    )
}

fn coordinate_deactivation(request: &HandsActivationRequest) -> HandsCoordinationPlan {
    let Some(instance_id) = request
        .instance_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return HandsCoordinationPlan {
            action: HandsCoordinationAction::Clarify {
                prompt: "hands deactivation requires an instance id".to_string(),
            },
            telemetry_note: "hands deactivation missing instance id".to_string(),
        };
    };
    HandsCoordinationPlan {
        action: HandsCoordinationAction::RequestRuntimeInstanceMutation {
            operation: "deactivate".to_string(),
            instance_id: instance_id.to_string(),
        },
        telemetry_note: "request runtime-owned hands deactivation receipt".to_string(),
    }
}

fn coordinate_stats(request: &HandsActivationRequest) -> HandsCoordinationPlan {
    let Some(instance_id) = request
        .instance_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return HandsCoordinationPlan {
            action: HandsCoordinationAction::Clarify {
                prompt: "hands stats projection requires an instance id".to_string(),
            },
            telemetry_note: "hands stats missing instance id".to_string(),
        };
    };
    HandsCoordinationPlan {
        action: HandsCoordinationAction::ProjectInstanceStats {
            instance_id: instance_id.to_string(),
        },
        telemetry_note: "hands stats render from runtime instance projection".to_string(),
    }
}

fn require_hand_job(
    request: &HandsActivationRequest,
    job_kind: &str,
    note: &str,
) -> HandsCoordinationPlan {
    if request
        .hand_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        return HandsCoordinationPlan {
            action: HandsCoordinationAction::Clarify {
                prompt: "hands orchestration job requires a hand id".to_string(),
            },
            telemetry_note: "hands job missing hand id".to_string(),
        };
    }
    HandsCoordinationPlan {
        action: HandsCoordinationAction::RequestOrchestrationJob {
            job_kind: job_kind.to_string(),
            hand_id: request.hand_id.clone(),
        },
        telemetry_note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(intent: HandsIntent, dependency_state: DependencyState) -> HandsActivationRequest {
        HandsActivationRequest {
            intent,
            hand_id: Some("browser".to_string()),
            instance_id: Some("inst-1".to_string()),
            dependency_state,
            config_present: true,
        }
    }

    #[test]
    fn dependency_missing_requests_install_or_check_job() {
        let plan = coordinate_hands_intent(&request(
            HandsIntent::InstallDependencies,
            DependencyState::Missing,
        ));

        assert_eq!(
            plan.action,
            HandsCoordinationAction::RequestOrchestrationJob {
                job_kind: "install_dependencies".to_string(),
                hand_id: Some("browser".to_string())
            }
        );
    }

    #[test]
    fn install_failure_retries_through_backend_job() {
        let plan = coordinate_hands_intent(&request(
            HandsIntent::InstallDependencies,
            DependencyState::InstallFailed,
        ));

        assert!(matches!(
            plan.action,
            HandsCoordinationAction::RequestOrchestrationJob { .. }
        ));
    }

    #[test]
    fn activation_success_path_requests_durable_job() {
        let plan =
            coordinate_hands_intent(&request(HandsIntent::Activate, DependencyState::Satisfied));

        assert_eq!(
            plan.action,
            HandsCoordinationAction::RequestOrchestrationJob {
                job_kind: "activate".to_string(),
                hand_id: Some("browser".to_string())
            }
        );
    }

    #[test]
    fn activation_blocks_when_dependencies_are_missing() {
        let plan =
            coordinate_hands_intent(&request(HandsIntent::Activate, DependencyState::Missing));

        assert_eq!(
            plan.action,
            HandsCoordinationAction::RequestOrchestrationJob {
                job_kind: "check_dependencies".to_string(),
                hand_id: Some("browser".to_string())
            }
        );
    }

    #[test]
    fn deactivate_requests_runtime_receipt() {
        let plan = coordinate_hands_intent(&request(
            HandsIntent::Deactivate,
            DependencyState::Satisfied,
        ));

        assert_eq!(
            plan.action,
            HandsCoordinationAction::RequestRuntimeInstanceMutation {
                operation: "deactivate".to_string(),
                instance_id: "inst-1".to_string()
            }
        );
    }

    #[test]
    fn job_projection_requires_durable_receipt() {
        let plan = project_hands_job_receipt(&HandsJobReceipt {
            job_id: "job-1".to_string(),
            status: HandsJobStatus::Succeeded,
            durable_receipt_present: false,
            progress_event_count: 1,
        });

        assert!(matches!(
            plan.action,
            HandsCoordinationAction::Clarify { .. }
        ));
    }
}
