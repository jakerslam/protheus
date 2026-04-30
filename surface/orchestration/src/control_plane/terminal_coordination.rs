// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalActor {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalIntentKind {
    UserCommand,
    AgentCommand,
    SessionStart,
    RetryAfterMissingSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalIntent {
    pub kind: TerminalIntentKind,
    pub actor: TerminalActor,
    pub target_agent_id: Option<String>,
    pub session_id: Option<String>,
    pub command_present: bool,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRuntimeState {
    pub active_send_in_progress: bool,
    pub websocket_ready: bool,
    pub backend_terminal_endpoint_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalReceiptStatus {
    Success,
    Error,
    Blocked,
    LowSignal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalReceiptProjection {
    pub status: TerminalReceiptStatus,
    pub exit_code: Option<i32>,
    pub stdout_present: bool,
    pub stderr_present: bool,
    pub permission_gate_present: bool,
    pub recovery_hint_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalCoordinationAction {
    RequestBackendTerminalIntent {
        route: String,
    },
    Hold {
        reason: String,
    },
    Clarify {
        prompt: String,
    },
    PackageReceipt {
        status: TerminalReceiptStatus,
        renderable_output_present: bool,
        recovery_hint_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCoordinationPlan {
    pub action: TerminalCoordinationAction,
    pub telemetry_note: String,
}

pub struct TerminalCoordinationContract;

impl SubdomainContract for TerminalCoordinationContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "terminal_coordination",
        legacy_module_bindings: &[
            "chat_terminal_session_helpers",
            "chat_terminal_compose_helpers",
            "chat_ws_terminal_event_helpers",
            "chat_message_source_run_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "execution_observation_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "terminal_intent_projection",
            "terminal_receipt_projection",
            "terminal_recovery_recommendation",
        ],
        message_boundaries: &[
            "terminal_to_shell_projection_boundary",
            "terminal_to_core_admission_boundary",
            "terminal_to_recovery_boundary",
        ],
    }
}

pub fn coordinate_terminal_intent(
    intent: &TerminalIntent,
    state: &TerminalRuntimeState,
) -> TerminalCoordinationPlan {
    if state.active_send_in_progress {
        return hold(
            "active send in progress",
            "hold terminal intent while another send is active",
        );
    }
    if !intent.command_present && intent.kind != TerminalIntentKind::SessionStart {
        return TerminalCoordinationPlan {
            action: TerminalCoordinationAction::Clarify {
                prompt: "terminal command intent requires a command payload".to_string(),
            },
            telemetry_note: "terminal command missing command payload".to_string(),
        };
    }
    if intent.actor == TerminalActor::Agent && intent.target_agent_id.is_none() {
        return TerminalCoordinationPlan {
            action: TerminalCoordinationAction::Clarify {
                prompt: "agent terminal intent requires a target agent id".to_string(),
            },
            telemetry_note: "agent terminal intent missing target agent".to_string(),
        };
    }
    if !state.websocket_ready && !state.backend_terminal_endpoint_ready {
        return hold(
            "terminal transport unavailable",
            "hold terminal intent until an admitted backend route is available",
        );
    }

    TerminalCoordinationPlan {
        action: TerminalCoordinationAction::RequestBackendTerminalIntent {
            route: terminal_route(intent, state),
        },
        telemetry_note: "request backend terminal intent instead of shell execution".to_string(),
    }
}

pub fn package_terminal_receipt(receipt: &TerminalReceiptProjection) -> TerminalCoordinationPlan {
    let output_present = receipt.stdout_present || receipt.stderr_present;
    let renderable_output_present =
        output_present || receipt.permission_gate_present || receipt.recovery_hint_count > 0;

    TerminalCoordinationPlan {
        action: TerminalCoordinationAction::PackageReceipt {
            status: receipt.status.clone(),
            renderable_output_present,
            recovery_hint_count: receipt.recovery_hint_count,
        },
        telemetry_note: receipt_note(receipt),
    }
}

fn terminal_route(intent: &TerminalIntent, state: &TerminalRuntimeState) -> String {
    if intent.actor == TerminalActor::System {
        return "system_terminal_intent".to_string();
    }
    if state.websocket_ready {
        return "agent_terminal_websocket_intent".to_string();
    }
    "agent_terminal_http_intent".to_string()
}

fn receipt_note(receipt: &TerminalReceiptProjection) -> String {
    match receipt.status {
        TerminalReceiptStatus::Success => "package successful terminal receipt".to_string(),
        TerminalReceiptStatus::Error => "package terminal error as receipt projection".to_string(),
        TerminalReceiptStatus::Blocked => {
            "package permission-gated terminal receipt without shell interpretation".to_string()
        }
        TerminalReceiptStatus::LowSignal => {
            "package low-signal terminal receipt with recovery telemetry".to_string()
        }
    }
}

fn hold(reason: &str, note: &str) -> TerminalCoordinationPlan {
    TerminalCoordinationPlan {
        action: TerminalCoordinationAction::Hold {
            reason: reason.to_string(),
        },
        telemetry_note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_state() -> TerminalRuntimeState {
        TerminalRuntimeState {
            active_send_in_progress: false,
            websocket_ready: true,
            backend_terminal_endpoint_ready: true,
        }
    }

    fn user_intent() -> TerminalIntent {
        TerminalIntent {
            kind: TerminalIntentKind::UserCommand,
            actor: TerminalActor::User,
            target_agent_id: Some("misty".to_string()),
            session_id: Some("session-1".to_string()),
            command_present: true,
            cwd: "/workspace".to_string(),
        }
    }

    #[test]
    fn user_command_requests_backend_terminal_intent() {
        let plan = coordinate_terminal_intent(&user_intent(), &ready_state());

        assert_eq!(
            plan.action,
            TerminalCoordinationAction::RequestBackendTerminalIntent {
                route: "agent_terminal_websocket_intent".to_string()
            }
        );
    }

    #[test]
    fn agent_command_requires_target_agent() {
        let intent = TerminalIntent {
            kind: TerminalIntentKind::AgentCommand,
            actor: TerminalActor::Agent,
            target_agent_id: None,
            session_id: None,
            command_present: true,
            cwd: "/workspace".to_string(),
        };

        assert_eq!(
            coordinate_terminal_intent(&intent, &ready_state()).action,
            TerminalCoordinationAction::Clarify {
                prompt: "agent terminal intent requires a target agent id".to_string()
            }
        );
    }

    #[test]
    fn active_send_holds_terminal_intent() {
        let state = TerminalRuntimeState {
            active_send_in_progress: true,
            websocket_ready: true,
            backend_terminal_endpoint_ready: true,
        };

        assert_eq!(
            coordinate_terminal_intent(&user_intent(), &state).action,
            TerminalCoordinationAction::Hold {
                reason: "active send in progress".to_string()
            }
        );
    }

    #[test]
    fn blocked_receipt_is_packaged_without_shell_permission_interpretation() {
        let receipt = TerminalReceiptProjection {
            status: TerminalReceiptStatus::Blocked,
            exit_code: None,
            stdout_present: false,
            stderr_present: false,
            permission_gate_present: true,
            recovery_hint_count: 1,
        };

        assert_eq!(
            package_terminal_receipt(&receipt).action,
            TerminalCoordinationAction::PackageReceipt {
                status: TerminalReceiptStatus::Blocked,
                renderable_output_present: true,
                recovery_hint_count: 1
            }
        );
    }

    #[test]
    fn low_signal_receipt_keeps_recovery_as_telemetry_packaging() {
        let receipt = TerminalReceiptProjection {
            status: TerminalReceiptStatus::LowSignal,
            exit_code: Some(0),
            stdout_present: false,
            stderr_present: false,
            permission_gate_present: false,
            recovery_hint_count: 2,
        };

        let plan = package_terminal_receipt(&receipt);

        assert_eq!(
            plan.action,
            TerminalCoordinationAction::PackageReceipt {
                status: TerminalReceiptStatus::LowSignal,
                renderable_output_present: true,
                recovery_hint_count: 2
            }
        );
        assert!(plan.telemetry_note.contains("low-signal"));
    }
}
