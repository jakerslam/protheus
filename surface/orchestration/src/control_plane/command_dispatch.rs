// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use super::{SubdomainBoundary, SubdomainContract};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommandFamily {
    Navigation,
    SessionControl,
    WorkspaceTooling,
    RuntimeStatus,
    NetworkStatus,
    ModelProvider,
    Telemetry,
    Diagnostics,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommandAction {
    ShellProjectionOnly,
    RequestToolRoute { tool_family: String },
    RequestRuntimeOperation { operation: String },
    RequestOrchestrationWorkflow { workflow_hint: String },
    Clarify { prompt: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandDispatch {
    pub command: String,
    pub args: String,
    pub family: ShellCommandFamily,
    pub action: ShellCommandAction,
}

pub struct CommandDispatchContract;

impl SubdomainContract for CommandDispatchContract {
    fn boundary() -> SubdomainBoundary {
        boundary()
    }
}

pub fn boundary() -> SubdomainBoundary {
    SubdomainBoundary {
        id: "command_dispatch",
        legacy_module_bindings: &[
            "chat_slash_command_helpers",
            "chat_slash_alias_helpers",
            "chat_slash_telemetry_helpers",
            "chat_slash_apikey_helpers",
            "chat_memprobe_helpers",
        ],
        allowed_kernel_inputs: &[
            "typed_request_snapshot",
            "core_probe_envelope",
            "capability_probe_snapshot",
            "policy_scope_snapshot",
        ],
        allowed_kernel_outputs: &[
            "tool_route_recommendation_envelope",
            "task_fabric_proposal_envelope",
            "clarification_request_envelope",
        ],
        message_boundaries: &[
            "command_to_shell_projection_boundary",
            "command_to_tool_route_boundary",
            "command_to_workflow_boundary",
        ],
    }
}

pub fn dispatch_shell_command(input: &str) -> ShellCommandDispatch {
    let (command, args) = split_command(input);
    let normalized = command.to_lowercase();
    match normalized.as_str() {
        "/help" | "/agents" | "/clear" | "/exit" => dispatch(
            command,
            args,
            ShellCommandFamily::Navigation,
            ShellCommandAction::ShellProjectionOnly,
        ),
        "/new" | "/compact" | "/stop" | "/queue" | "/context" | "/think" | "/verbose" => dispatch(
            command,
            args,
            ShellCommandFamily::SessionControl,
            ShellCommandAction::RequestRuntimeOperation {
                operation: normalized.trim_start_matches('/').to_string(),
            },
        ),
        "/file" => workspace_tool(command, args, "workspace_read"),
        "/folder" => workspace_tool(command, args, "workspace_export"),
        "/status" | "/usage" | "/budget" => dispatch(
            command,
            args,
            ShellCommandFamily::RuntimeStatus,
            ShellCommandAction::RequestRuntimeOperation {
                operation: normalized.trim_start_matches('/').to_string(),
            },
        ),
        "/peers" | "/a2a" => dispatch(
            command,
            args,
            ShellCommandFamily::NetworkStatus,
            ShellCommandAction::RequestRuntimeOperation {
                operation: normalized.trim_start_matches('/').to_string(),
            },
        ),
        "/model" | "/apikey" => dispatch(
            command,
            args,
            ShellCommandFamily::ModelProvider,
            ShellCommandAction::RequestOrchestrationWorkflow {
                workflow_hint: "model_provider_coordination".to_string(),
            },
        ),
        "/alerts" | "/next" | "/memory" | "/continuity" | "/opt" => dispatch(
            command,
            args,
            ShellCommandFamily::Telemetry,
            ShellCommandAction::RequestOrchestrationWorkflow {
                workflow_hint: "telemetry_continuity_packaging".to_string(),
            },
        ),
        "/aliases" | "/alias" | "/memprobe" => dispatch(
            command,
            args,
            ShellCommandFamily::Diagnostics,
            ShellCommandAction::RequestRuntimeOperation {
                operation: normalized.trim_start_matches('/').to_string(),
            },
        ),
        _ => dispatch(
            command,
            args,
            ShellCommandFamily::Unknown,
            ShellCommandAction::Clarify {
                prompt:
                    "unknown slash command; choose a registered command or submit as normal text"
                        .to_string(),
            },
        ),
    }
}

fn workspace_tool(command: String, args: String, tool_family: &str) -> ShellCommandDispatch {
    if args.trim().is_empty() {
        return dispatch(
            command,
            args,
            ShellCommandFamily::WorkspaceTooling,
            ShellCommandAction::Clarify {
                prompt: "workspace command requires a target path".to_string(),
            },
        );
    }
    dispatch(
        command,
        args,
        ShellCommandFamily::WorkspaceTooling,
        ShellCommandAction::RequestToolRoute {
            tool_family: tool_family.to_string(),
        },
    )
}

fn dispatch(
    command: String,
    args: String,
    family: ShellCommandFamily,
    action: ShellCommandAction,
) -> ShellCommandDispatch {
    ShellCommandDispatch {
        command,
        args,
        family,
        action,
    }
}

fn split_command(input: &str) -> (String, String) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or("").trim().to_string();
    let args = parts.next().unwrap_or("").trim().to_string();
    (command, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_file_command_routes_to_workspace_tool_family() {
        let dispatch = dispatch_shell_command("/file docs/workspace/SRS.md");

        assert_eq!(dispatch.family, ShellCommandFamily::WorkspaceTooling);
        assert_eq!(
            dispatch.action,
            ShellCommandAction::RequestToolRoute {
                tool_family: "workspace_read".to_string()
            }
        );
    }

    #[test]
    fn workspace_command_without_path_clarifies() {
        let dispatch = dispatch_shell_command("/folder");

        assert_eq!(dispatch.family, ShellCommandFamily::WorkspaceTooling);
        assert_eq!(
            dispatch.action,
            ShellCommandAction::Clarify {
                prompt: "workspace command requires a target path".to_string()
            }
        );
    }

    #[test]
    fn model_commands_route_to_orchestration_workflow() {
        let dispatch = dispatch_shell_command("/model anthropic/claude");

        assert_eq!(dispatch.family, ShellCommandFamily::ModelProvider);
        assert_eq!(
            dispatch.action,
            ShellCommandAction::RequestOrchestrationWorkflow {
                workflow_hint: "model_provider_coordination".to_string()
            }
        );
    }

    #[test]
    fn unknown_commands_clarify_instead_of_shell_guessing() {
        let dispatch = dispatch_shell_command("/definitely-not-a-command");

        assert_eq!(dispatch.family, ShellCommandFamily::Unknown);
        assert!(matches!(
            dispatch.action,
            ShellCommandAction::Clarify { .. }
        ));
    }
}
