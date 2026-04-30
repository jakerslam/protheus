// Layer ownership: orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSurfaceOrigin {
    UserAuthored,
    FinalLlmOutput,
    RuntimeDiagnostic,
    SystemNotice,
    SlashCommandOutput,
    ToolDiagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSurfaceCandidate {
    pub origin: ChatSurfaceOrigin,
    pub text_present: bool,
    pub final_llm_synthesized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatVisibilityAction {
    AllowVisibleChat { reason: String },
    RouteToTelemetry { reason: String },
    RouteToNoticeRail { reason: String },
    Reject { reason: String },
}

pub struct ChatVisibilityContract;

impl SubdomainContract for ChatVisibilityContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "chat_visibility",
        legacy_module_bindings: &[
            "chat_notice_message_helpers",
            "chat_ws_error_event_helpers",
            "chat_ws_phase_event_helpers",
            "chat_model_usage_notice_helpers",
            "chat_proactive_telemetry_helpers",
            "chat_agent_lifecycle_helpers",
            "chat_slash_command_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "chat_visibility_projection",
            "diagnostic_telemetry_projection",
            "notice_rail_projection",
        ],
        message_boundaries: &[
            "visibility_to_shell_projection_boundary",
            "visibility_to_workflow_finalization_boundary",
            "visibility_to_telemetry_boundary",
        ],
    }
}

pub fn route_chat_surface_candidate(candidate: &ChatSurfaceCandidate) -> ChatVisibilityAction {
    if !candidate.text_present {
        return ChatVisibilityAction::Reject {
            reason: "empty text is not chat-visible".to_string(),
        };
    }

    match candidate.origin {
        ChatSurfaceOrigin::UserAuthored => ChatVisibilityAction::AllowVisibleChat {
            reason: "user-authored content may appear in chat".to_string(),
        },
        ChatSurfaceOrigin::FinalLlmOutput if candidate.final_llm_synthesized => {
            ChatVisibilityAction::AllowVisibleChat {
                reason: "synthesized final LLM output may appear in chat".to_string(),
            }
        }
        ChatSurfaceOrigin::FinalLlmOutput => ChatVisibilityAction::Reject {
            reason: "non-synthesized final output cannot substitute visible chat".to_string(),
        },
        ChatSurfaceOrigin::SystemNotice => ChatVisibilityAction::RouteToNoticeRail {
            reason: "system notices render outside the chat transcript".to_string(),
        },
        ChatSurfaceOrigin::RuntimeDiagnostic
        | ChatSurfaceOrigin::SlashCommandOutput
        | ChatSurfaceOrigin::ToolDiagnostic => ChatVisibilityAction::RouteToTelemetry {
            reason: "diagnostic output is telemetry, not assistant chat text".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(origin: ChatSurfaceOrigin) -> ChatSurfaceCandidate {
        ChatSurfaceCandidate {
            origin,
            text_present: true,
            final_llm_synthesized: false,
        }
    }

    #[test]
    fn final_llm_synthesized_output_is_chat_visible() {
        let mut row = candidate(ChatSurfaceOrigin::FinalLlmOutput);
        row.final_llm_synthesized = true;

        assert_eq!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::AllowVisibleChat {
                reason: "synthesized final LLM output may appear in chat".to_string()
            }
        );
    }

    #[test]
    fn finalization_edge_does_not_inject_visible_text() {
        let row = candidate(ChatSurfaceOrigin::FinalLlmOutput);

        assert_eq!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::Reject {
                reason: "non-synthesized final output cannot substitute visible chat".to_string()
            }
        );
    }

    #[test]
    fn runtime_errors_route_to_telemetry() {
        let row = candidate(ChatSurfaceOrigin::RuntimeDiagnostic);

        assert_eq!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::RouteToTelemetry {
                reason: "diagnostic output is telemetry, not assistant chat text".to_string()
            }
        );
    }

    #[test]
    fn slash_outputs_route_to_telemetry() {
        let row = candidate(ChatSurfaceOrigin::SlashCommandOutput);

        assert!(matches!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::RouteToTelemetry { .. }
        ));
    }

    #[test]
    fn system_notices_route_to_notice_rail() {
        let row = candidate(ChatSurfaceOrigin::SystemNotice);

        assert_eq!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::RouteToNoticeRail {
                reason: "system notices render outside the chat transcript".to_string()
            }
        );
    }

    #[test]
    fn user_authored_text_is_chat_visible() {
        let row = candidate(ChatSurfaceOrigin::UserAuthored);

        assert!(matches!(
            route_chat_surface_candidate(&row),
            ChatVisibilityAction::AllowVisibleChat { .. }
        ));
    }
}
