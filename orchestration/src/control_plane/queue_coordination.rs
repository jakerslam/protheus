// Layer ownership: orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueItemKind {
    Prompt,
    Terminal,
    Steer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueIntent {
    Enqueue,
    Remove,
    Reorder,
    DispatchNext,
    InjectSteer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItemProjection {
    pub queue_id: Option<String>,
    pub kind: QueueItemKind,
    pub text_present: bool,
    pub attachment_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRuntimeState {
    pub sending: bool,
    pub failover_in_progress: bool,
    pub active_agent_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueMutationOperation {
    Enqueue,
    Remove,
    Reorder,
    RequeueFailedSteer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueCoordinationAction {
    RequestBackendQueueMutation {
        operation: QueueMutationOperation,
        queue_id: Option<String>,
    },
    RequestDispatch {
        queue_id: String,
        kind: QueueItemKind,
    },
    Hold {
        reason: String,
    },
    Clarify {
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueCoordinationPlan {
    pub action: QueueCoordinationAction,
    pub telemetry_note: String,
}

pub struct QueueCoordinationContract;

impl SubdomainContract for QueueCoordinationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "queue_coordination",
        legacy_module_bindings: &[
            "chat_prompt_queue_helpers",
            "chat_queue_processing_helpers",
            "chat_send_message_helpers",
            "chat_send_payload_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "queue_sequence_projection",
            "steer_coordination_projection",
            "queue_mutation_recommendation",
        ],
        message_boundaries: &[
            "queue_to_shell_projection_boundary",
            "queue_to_core_admission_boundary",
            "queue_to_workflow_boundary",
        ],
    }
}

pub fn coordinate_queue_intent(
    intent: QueueIntent,
    state: &QueueRuntimeState,
    item: Option<&QueueItemProjection>,
) -> QueueCoordinationPlan {
    match intent {
        QueueIntent::DispatchNext => coordinate_dispatch_next(state, item),
        QueueIntent::InjectSteer => coordinate_steer(state, item),
        QueueIntent::Enqueue => queue_mutation(QueueMutationOperation::Enqueue, item),
        QueueIntent::Remove => queue_mutation(QueueMutationOperation::Remove, item),
        QueueIntent::Reorder => queue_mutation(QueueMutationOperation::Reorder, item),
    }
}

fn coordinate_dispatch_next(
    state: &QueueRuntimeState,
    item: Option<&QueueItemProjection>,
) -> QueueCoordinationPlan {
    if state.sending {
        return hold(
            "active send in progress",
            "hold queue dispatch while a send is active",
        );
    }
    if state.failover_in_progress {
        return hold(
            "model failover in progress",
            "hold queue dispatch while failover is resolving",
        );
    }

    let Some(item) = item else {
        return hold("queue empty", "no queue item available for dispatch");
    };
    if !item.text_present && item.attachment_count == 0 {
        return hold(
            "empty queue item",
            "empty queue item should not be dispatched",
        );
    }

    let Some(queue_id) = item.queue_id.clone() else {
        return QueueCoordinationPlan {
            action: QueueCoordinationAction::Clarify {
                prompt: "queued work requires a backend queue id before dispatch".to_string(),
            },
            telemetry_note: "queue item has no backend id".to_string(),
        };
    };

    QueueCoordinationPlan {
        action: QueueCoordinationAction::RequestDispatch {
            queue_id,
            kind: item.kind.clone(),
        },
        telemetry_note: "request backend dispatch for next queued item".to_string(),
    }
}

fn coordinate_steer(
    state: &QueueRuntimeState,
    item: Option<&QueueItemProjection>,
) -> QueueCoordinationPlan {
    if !state.active_agent_available {
        return QueueCoordinationPlan {
            action: QueueCoordinationAction::Clarify {
                prompt: "steer requires an active agent target".to_string(),
            },
            telemetry_note: "steer blocked because no active agent is available".to_string(),
        };
    }

    let Some(item) = item else {
        return hold(
            "missing steer payload",
            "steer requested without a queue item",
        );
    };
    if item.kind != QueueItemKind::Steer {
        return QueueCoordinationPlan {
            action: QueueCoordinationAction::Clarify {
                prompt: "steer coordination requires a steer queue item".to_string(),
            },
            telemetry_note: "non-steer item rejected for steer injection".to_string(),
        };
    }
    if !item.text_present {
        return hold("empty steer payload", "steer item has no text payload");
    }

    QueueCoordinationPlan {
        action: QueueCoordinationAction::RequestBackendQueueMutation {
            operation: QueueMutationOperation::Enqueue,
            queue_id: item.queue_id.clone(),
        },
        telemetry_note: "request backend steer enqueue instead of shell injection".to_string(),
    }
}

fn queue_mutation(
    operation: QueueMutationOperation,
    item: Option<&QueueItemProjection>,
) -> QueueCoordinationPlan {
    QueueCoordinationPlan {
        action: QueueCoordinationAction::RequestBackendQueueMutation {
            operation,
            queue_id: item.and_then(|row| row.queue_id.clone()),
        },
        telemetry_note: "queue mutation must be requested from backend authority".to_string(),
    }
}

fn hold(reason: &str, note: &str) -> QueueCoordinationPlan {
    QueueCoordinationPlan {
        action: QueueCoordinationAction::Hold {
            reason: reason.to_string(),
        },
        telemetry_note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_state() -> QueueRuntimeState {
        QueueRuntimeState {
            sending: false,
            failover_in_progress: false,
            active_agent_available: true,
        }
    }

    fn prompt_item() -> QueueItemProjection {
        QueueItemProjection {
            queue_id: Some("queue-1".to_string()),
            kind: QueueItemKind::Prompt,
            text_present: true,
            attachment_count: 0,
        }
    }

    #[test]
    fn dispatch_next_holds_while_sending_or_failover() {
        let item = prompt_item();
        let sending = QueueRuntimeState {
            sending: true,
            failover_in_progress: false,
            active_agent_available: true,
        };
        let failover = QueueRuntimeState {
            sending: false,
            failover_in_progress: true,
            active_agent_available: true,
        };

        assert_eq!(
            coordinate_queue_intent(QueueIntent::DispatchNext, &sending, Some(&item)).action,
            QueueCoordinationAction::Hold {
                reason: "active send in progress".to_string()
            }
        );
        assert_eq!(
            coordinate_queue_intent(QueueIntent::DispatchNext, &failover, Some(&item)).action,
            QueueCoordinationAction::Hold {
                reason: "model failover in progress".to_string()
            }
        );
    }

    #[test]
    fn dispatch_next_requests_backend_dispatch_when_ready() {
        let item = prompt_item();

        assert_eq!(
            coordinate_queue_intent(QueueIntent::DispatchNext, &ready_state(), Some(&item)).action,
            QueueCoordinationAction::RequestDispatch {
                queue_id: "queue-1".to_string(),
                kind: QueueItemKind::Prompt
            }
        );
    }

    #[test]
    fn steer_requires_active_agent_or_clarifies() {
        let state = QueueRuntimeState {
            sending: false,
            failover_in_progress: false,
            active_agent_available: false,
        };
        let item = QueueItemProjection {
            queue_id: Some("queue-2".to_string()),
            kind: QueueItemKind::Steer,
            text_present: true,
            attachment_count: 0,
        };

        assert_eq!(
            coordinate_queue_intent(QueueIntent::InjectSteer, &state, Some(&item)).action,
            QueueCoordinationAction::Clarify {
                prompt: "steer requires an active agent target".to_string()
            }
        );
    }

    #[test]
    fn reorder_is_backend_queue_mutation_not_shell_truth() {
        let item = prompt_item();

        assert_eq!(
            coordinate_queue_intent(QueueIntent::Reorder, &ready_state(), Some(&item)).action,
            QueueCoordinationAction::RequestBackendQueueMutation {
                operation: QueueMutationOperation::Reorder,
                queue_id: Some("queue-1".to_string())
            }
        );
    }
}
