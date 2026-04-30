// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusEventKind {
    WorkflowPhase,
    AgentActivity,
    ThinkingBubble,
    ContextWarning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSourceAuthority {
    CoreRuntime,
    Orchestration,
    ShellOptimistic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivityState {
    Idle,
    Working,
    Typing,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusProjectionAction {
    ProjectPhase {
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectActivity {
        activity: AgentActivityState,
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectThinkingBubble {
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectContextWarning {
        display_label: String,
        source: StatusSourceAuthority,
    },
    RejectShellAuthoredInference {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEventEnvelope {
    pub kind: StatusEventKind,
    pub display_label: String,
    pub source: StatusSourceAuthority,
    pub activity: Option<AgentActivityState>,
    pub backend_event_id: Option<String>,
    pub optimistic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProjectionPlan {
    pub action: StatusProjectionAction,
    pub telemetry_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellStatusProjection {
    pub projection_type: &'static str,
    pub display_label: String,
    pub status_text: String,
    pub source: StatusSourceAuthority,
    pub activity: Option<AgentActivityState>,
    pub backend_event_id: Option<String>,
    pub optimistic: bool,
    pub telemetry_note: String,
}

pub struct StatusPhaseProjectionContract;

impl SubdomainContract for StatusPhaseProjectionContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "status_phase_projection",
        legacy_module_bindings: &[
            "chat_agent_live_status_helpers",
            "chat_lifecycle_init_helpers",
            "chat_ws_phase_event_helpers",
            "chat_ws_typing_event_helpers",
            "chat_context_warning_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "status_phase_projection",
            "agent_activity_projection",
            "context_warning_projection",
        ],
        message_boundaries: &[
            "status_to_shell_projection_boundary",
            "status_to_workflow_event_boundary",
            "status_to_telemetry_boundary",
        ],
    }
}

pub fn project_status_event(event: &StatusEventEnvelope) -> StatusProjectionPlan {
    if event.display_label.trim().is_empty() {
        return reject("status projection requires a backend supplied display label");
    }

    if event.source == StatusSourceAuthority::ShellOptimistic {
        return project_shell_optimistic(event);
    }

    if event
        .backend_event_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return reject("backend status projections require a backend event id");
    }

    match event.kind {
        StatusEventKind::WorkflowPhase => StatusProjectionPlan {
            action: StatusProjectionAction::ProjectPhase {
                display_label: event.display_label.clone(),
                source: event.source.clone(),
            },
            telemetry_note: "project backend workflow phase label".to_string(),
        },
        StatusEventKind::AgentActivity => {
            let Some(activity) = event.activity.clone() else {
                return reject(
                    "agent activity projection requires explicit backend activity state",
                );
            };
            StatusProjectionPlan {
                action: StatusProjectionAction::ProjectActivity {
                    activity,
                    display_label: event.display_label.clone(),
                    source: event.source.clone(),
                },
                telemetry_note: "project backend agent activity label".to_string(),
            }
        }
        StatusEventKind::ThinkingBubble => StatusProjectionPlan {
            action: StatusProjectionAction::ProjectThinkingBubble {
                display_label: event.display_label.clone(),
                source: event.source.clone(),
            },
            telemetry_note: "project backend thinking-bubble progress label".to_string(),
        },
        StatusEventKind::ContextWarning => StatusProjectionPlan {
            action: StatusProjectionAction::ProjectContextWarning {
                display_label: event.display_label.clone(),
                source: event.source.clone(),
            },
            telemetry_note: "project backend context warning label".to_string(),
        },
    }
}

pub fn project_status_event_for_shell(
    event: &StatusEventEnvelope,
) -> Result<ShellStatusProjection, String> {
    let plan = project_status_event(event);
    match &plan.action {
        StatusProjectionAction::ProjectPhase {
            display_label,
            source,
        } => Ok(shell_projection(
            "status_phase_projection",
            display_label,
            source,
            None,
            event,
            &plan.telemetry_note,
        )),
        StatusProjectionAction::ProjectActivity {
            activity,
            display_label,
            source,
        } => Ok(shell_projection(
            "agent_activity_projection",
            display_label,
            source,
            Some(activity.clone()),
            event,
            &plan.telemetry_note,
        )),
        StatusProjectionAction::ProjectThinkingBubble {
            display_label,
            source,
        } => Ok(shell_projection(
            "thinking_bubble_projection",
            display_label,
            source,
            None,
            event,
            &plan.telemetry_note,
        )),
        StatusProjectionAction::ProjectContextWarning {
            display_label,
            source,
        } => Ok(shell_projection(
            "context_warning_projection",
            display_label,
            source,
            None,
            event,
            &plan.telemetry_note,
        )),
        StatusProjectionAction::RejectShellAuthoredInference { reason } => Err(reason.clone()),
    }
}

fn project_shell_optimistic(event: &StatusEventEnvelope) -> StatusProjectionPlan {
    if !event.optimistic {
        return reject("shell-origin status labels must be explicitly marked optimistic");
    }
    if event.kind == StatusEventKind::ContextWarning {
        return reject("context warnings must come from backend event payloads");
    }

    let display_label = format!("{} (optimistic)", event.display_label.trim());
    match event.kind {
        StatusEventKind::WorkflowPhase | StatusEventKind::ThinkingBubble => StatusProjectionPlan {
            action: StatusProjectionAction::ProjectThinkingBubble {
                display_label,
                source: StatusSourceAuthority::ShellOptimistic,
            },
            telemetry_note: "project clearly marked optimistic shell progress label".to_string(),
        },
        StatusEventKind::AgentActivity => {
            let activity = event
                .activity
                .clone()
                .unwrap_or(AgentActivityState::Working);
            StatusProjectionPlan {
                action: StatusProjectionAction::ProjectActivity {
                    activity,
                    display_label,
                    source: StatusSourceAuthority::ShellOptimistic,
                },
                telemetry_note: "project clearly marked optimistic shell activity label"
                    .to_string(),
            }
        }
        StatusEventKind::ContextWarning => unreachable!("context warnings rejected above"),
    }
}

fn shell_projection(
    projection_type: &'static str,
    display_label: &str,
    source: &StatusSourceAuthority,
    activity: Option<AgentActivityState>,
    event: &StatusEventEnvelope,
    telemetry_note: &str,
) -> ShellStatusProjection {
    ShellStatusProjection {
        projection_type,
        display_label: display_label.to_string(),
        status_text: display_label.to_string(),
        source: source.clone(),
        activity,
        backend_event_id: event.backend_event_id.clone(),
        optimistic: event.optimistic,
        telemetry_note: telemetry_note.to_string(),
    }
}

fn reject(reason: &str) -> StatusProjectionPlan {
    StatusProjectionPlan {
        action: StatusProjectionAction::RejectShellAuthoredInference {
            reason: reason.to_string(),
        },
        telemetry_note: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend_event(kind: StatusEventKind, label: &str) -> StatusEventEnvelope {
        StatusEventEnvelope {
            kind,
            display_label: label.to_string(),
            source: StatusSourceAuthority::Orchestration,
            activity: None,
            backend_event_id: Some("evt_123".to_string()),
            optimistic: false,
        }
    }

    #[test]
    fn phase_events_project_backend_label_and_source() {
        let event = backend_event(StatusEventKind::WorkflowPhase, "Using web search");

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::ProjectPhase {
                display_label: "Using web search".to_string(),
                source: StatusSourceAuthority::Orchestration
            }
        );
    }

    #[test]
    fn thinking_bubble_uses_backend_status_text() {
        let event = backend_event(StatusEventKind::ThinkingBubble, "Searching rust docs");

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::ProjectThinkingBubble {
                display_label: "Searching rust docs".to_string(),
                source: StatusSourceAuthority::Orchestration
            }
        );
    }

    #[test]
    fn idle_transition_requires_explicit_activity_state() {
        let mut event = backend_event(StatusEventKind::AgentActivity, "Idle");
        event.source = StatusSourceAuthority::CoreRuntime;
        event.activity = Some(AgentActivityState::Idle);

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::ProjectActivity {
                activity: AgentActivityState::Idle,
                display_label: "Idle".to_string(),
                source: StatusSourceAuthority::CoreRuntime
            }
        );
    }

    #[test]
    fn context_warning_must_be_backend_sourced() {
        let event = StatusEventEnvelope {
            kind: StatusEventKind::ContextWarning,
            display_label: "Context window is getting tight".to_string(),
            source: StatusSourceAuthority::ShellOptimistic,
            activity: None,
            backend_event_id: None,
            optimistic: true,
        };

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::RejectShellAuthoredInference {
                reason: "context warnings must come from backend event payloads".to_string()
            }
        );
    }

    #[test]
    fn shell_status_labels_must_be_marked_optimistic() {
        let event = StatusEventEnvelope {
            kind: StatusEventKind::AgentActivity,
            display_label: "Working".to_string(),
            source: StatusSourceAuthority::ShellOptimistic,
            activity: Some(AgentActivityState::Working),
            backend_event_id: None,
            optimistic: false,
        };

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::RejectShellAuthoredInference {
                reason: "shell-origin status labels must be explicitly marked optimistic"
                    .to_string()
            }
        );
    }

    #[test]
    fn backend_status_without_event_id_is_rejected() {
        let mut event = backend_event(StatusEventKind::WorkflowPhase, "Planning");
        event.backend_event_id = None;

        assert_eq!(
            project_status_event(&event).action,
            StatusProjectionAction::RejectShellAuthoredInference {
                reason: "backend status projections require a backend event id".to_string()
            }
        );
    }

    #[test]
    fn backend_phase_exports_shell_ready_projection_envelope() {
        let event = backend_event(StatusEventKind::WorkflowPhase, "Coordinating tools");

        let projection =
            project_status_event_for_shell(&event).expect("phase projection should be shell ready");

        assert_eq!(projection.projection_type, "status_phase_projection");
        assert_eq!(projection.display_label, "Coordinating tools");
        assert_eq!(projection.status_text, "Coordinating tools");
        assert_eq!(projection.source, StatusSourceAuthority::Orchestration);
        assert_eq!(projection.activity, None);
        assert_eq!(projection.backend_event_id.as_deref(), Some("evt_123"));
        assert!(!projection.optimistic);
    }

    #[test]
    fn shell_rejections_do_not_export_visible_projection_envelopes() {
        let mut event = backend_event(StatusEventKind::WorkflowPhase, "Planning");
        event.backend_event_id = None;

        assert_eq!(
            project_status_event_for_shell(&event),
            Err("backend status projections require a backend event id".to_string())
        );
    }

    #[test]
    fn shell_projection_serializes_stable_snake_case_values() {
        let mut event = backend_event(StatusEventKind::AgentActivity, "Working");
        event.source = StatusSourceAuthority::CoreRuntime;
        event.activity = Some(AgentActivityState::Working);

        let projection = project_status_event_for_shell(&event)
            .expect("activity projection should be shell ready");
        let value = serde_json::to_value(projection).expect("projection envelope should serialize");

        assert_eq!(value["projection_type"], "agent_activity_projection");
        assert_eq!(value["source"], "core_runtime");
        assert_eq!(value["activity"], "working");
        assert_eq!(value["status_text"], "Working");
    }
}
