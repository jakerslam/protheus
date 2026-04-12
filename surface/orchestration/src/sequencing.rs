use crate::contracts::{
    CoreContractCall, OrchestrationFallbackAction, OrchestrationPlanStep, RequestClass,
    RequestClassification, ResourceKind, ToolFallbackContext, TypedOrchestrationRequest,
};
use protheus_tooling_core_v1::{ToolBackendClass, ToolReasonCode};
use serde_json::Value;

pub fn build_steps(
    _request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
) -> Vec<OrchestrationPlanStep> {
    classification
        .required_contracts
        .iter()
        .map(step_for_contract)
        .collect()
}

pub fn fallback_actions(
    request: &TypedOrchestrationRequest,
    request_class: RequestClass,
    tool_context: Option<&ToolFallbackContext>,
) -> Vec<OrchestrationFallbackAction> {
    match request_class {
        RequestClass::ToolCall => tool_fallback_actions(request, tool_context),
        _ => Vec::new(),
    }
}

pub fn tool_fallback_context_from_payload(payload: &Value) -> Option<ToolFallbackContext> {
    let obj = payload.as_object()?;
    let tool_name = obj
        .get("tool_name")
        .or_else(|| obj.get("tool"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let backend = obj
        .get("tool_backend")
        .or_else(|| obj.get("backend"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let backend_class = match obj
        .get("tool_backend_class")
        .or_else(|| obj.get("backend_class"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "retrieval_plane" => ToolBackendClass::RetrievalPlane,
        "workspace_fs" => ToolBackendClass::WorkspaceFs,
        "agent_runtime" => ToolBackendClass::AgentRuntime,
        "governed_terminal" => ToolBackendClass::GovernedTerminal,
        _ => ToolBackendClass::Unknown,
    };
    let reason_code = match obj
        .get("tool_reason_code")
        .or_else(|| obj.get("reason_code"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "caller_not_authorized" => ToolReasonCode::CallerNotAuthorized,
        "invalid_args" => ToolReasonCode::InvalidArgs,
        "auth_required" => ToolReasonCode::AuthRequired,
        "daemon_unavailable" => ToolReasonCode::DaemonUnavailable,
        "websocket_unavailable" => ToolReasonCode::WebsocketUnavailable,
        "backend_degraded" => ToolReasonCode::BackendDegraded,
        "execution_error" => ToolReasonCode::ExecutionError,
        "policy_denied" => ToolReasonCode::PolicyDenied,
        "transport_unavailable" => ToolReasonCode::TransportUnavailable,
        "timeout" => ToolReasonCode::Timeout,
        "ok" => ToolReasonCode::Ok,
        _ => ToolReasonCode::UnknownTool,
    };
    if tool_name.is_empty()
        && backend.is_empty()
        && backend_class == ToolBackendClass::Unknown
        && reason_code == ToolReasonCode::UnknownTool
    {
        return None;
    }
    Some(ToolFallbackContext {
        tool_name,
        backend,
        backend_class,
        reason_code,
    })
}

fn fallback_action(
    kind: &str,
    label: &str,
    reason: &str,
    tool_context: Option<&ToolFallbackContext>,
) -> OrchestrationFallbackAction {
    OrchestrationFallbackAction {
        kind: kind.to_string(),
        label: label.to_string(),
        reason: reason.to_string(),
        backend_class: tool_context.map(|ctx| ctx.backend_class),
        reason_code: tool_context.map(|ctx| ctx.reason_code),
    }
}

fn is_swarm_agent_bridge_tool(tool_context: Option<&ToolFallbackContext>) -> bool {
    matches!(
        tool_context.map(|ctx| ctx.backend_class),
        Some(ToolBackendClass::AgentRuntime)
    ) && tool_context
        .map(|ctx| {
            let tool = ctx.tool_name.to_ascii_lowercase();
            tool.starts_with("sessions_")
                || tool.starts_with("networks_")
                || tool.starts_with("turns_")
                || tool.starts_with("stream_")
        })
        .unwrap_or(false)
}

fn tool_fallback_actions(
    request: &TypedOrchestrationRequest,
    tool_context: Option<&ToolFallbackContext>,
) -> Vec<OrchestrationFallbackAction> {
    let direct_context_kind = match request.resource_kind {
        ResourceKind::Workspace => "paste_workspace_context",
        _ => "ask_for_source_material",
    };
    let mut out = vec![fallback_action(
        "inspect_tool_capabilities",
        "Check available tools",
        "inspect the governed tool surface before retrying",
        tool_context,
    )];
    match tool_context.map(|ctx| (ctx.reason_code, ctx.backend_class)) {
        Some((ToolReasonCode::InvalidArgs, ToolBackendClass::AgentRuntime))
            if is_swarm_agent_bridge_tool(tool_context) =>
        {
            out.push(fallback_action(
                "inspect_agent_bootstrap_contract",
                "Inspect agent bootstrap contract",
                "agent-runtime orchestration args are invalid, so reload the sessions_bootstrap or sessions_state contract before retrying the bridge call",
                tool_context,
            ));
        }
        Some((ToolReasonCode::AuthRequired, ToolBackendClass::RetrievalPlane)) => {
            out.push(fallback_action(
                "configure_provider_access",
                "Configure provider access",
                "retrieval tooling is blocked on provider auth health, so re-auth or switch to a configured provider",
                tool_context,
            ));
        }
        Some((ToolReasonCode::DaemonUnavailable, ToolBackendClass::AgentRuntime)) => {
            if is_swarm_agent_bridge_tool(tool_context) {
                out.push(fallback_action(
                    "inspect_swarm_runtime_status",
                    "Inspect swarm runtime status",
                    "agent orchestration is down at the runtime layer, so query swarm runtime status before retrying message or handoff routes",
                    tool_context,
                ));
            } else {
                out.push(fallback_action(
                    "retry_after_runtime_recovery",
                    "Retry after runtime recovery",
                    "agent runtime health is down, so wait for the daemon to recover before rerunning the tool path",
                    tool_context,
                ));
            }
        }
        Some((ToolReasonCode::WebsocketUnavailable, ToolBackendClass::AgentRuntime)) => {
            if is_swarm_agent_bridge_tool(tool_context) {
                out.push(fallback_action(
                    "inspect_swarm_runtime_status",
                    "Inspect swarm runtime status",
                    "agent messaging is blocked on websocket health, so restore bridge connectivity before retrying the swarm route",
                    tool_context,
                ));
            } else {
                out.push(fallback_action(
                    "retry_after_ws_reconnect",
                    "Retry after websocket recovery",
                    "the runtime is up but the websocket bridge is unavailable, so retry after WS health returns",
                    tool_context,
                ));
            }
        }
        Some((ToolReasonCode::TransportUnavailable, ToolBackendClass::AgentRuntime))
            if is_swarm_agent_bridge_tool(tool_context) =>
        {
            out.push(fallback_action(
                "inspect_swarm_runtime_status",
                "Inspect swarm runtime status",
                "agent orchestration transport is unavailable, so verify resident IPC health before retrying the bridge request",
                tool_context,
            ));
        }
        Some((ToolReasonCode::TransportUnavailable, ToolBackendClass::WorkspaceFs)) => {
            out.push(fallback_action(
                "provide_exact_workspace_path",
                "Provide exact workspace path",
                "workspace inspection is unavailable, so fall back to explicit file paths or pasted file contents",
                tool_context,
            ));
        }
        Some((ToolReasonCode::BackendDegraded, ToolBackendClass::GovernedTerminal)) => {
            out.push(fallback_action(
                "prefer_non_terminal_path",
                "Prefer non-terminal path",
                "terminal execution is degraded, so use read-only routes or explicit source context until resident IPC is healthy",
                tool_context,
            ));
        }
        Some((ToolReasonCode::CallerNotAuthorized, _))
        | Some((ToolReasonCode::PolicyDenied, _))
            if is_swarm_agent_bridge_tool(tool_context) =>
        {
            out.push(fallback_action(
                "verify_lineage_scope",
                "Verify lineage scope",
                "agent orchestration policy denied the request, so confirm the sender and target stay within allowed parent/child or sibling lineage",
                tool_context,
            ));
        }
        Some((ToolReasonCode::CallerNotAuthorized, _))
        | Some((ToolReasonCode::PolicyDenied, _)) => {
            out.push(fallback_action(
                "use_authorized_route",
                "Use authorized route",
                "this tool request is blocked by policy or caller class, so reroute through an allowed governed path",
                tool_context,
            ));
        }
        _ => {
            out.push(fallback_action(
                "narrow_tool_request",
                "Retry with narrower input",
                "reduce ambiguity in the tool payload or query before retrying",
                tool_context,
            ));
        }
    }
    out.push(fallback_action(
        direct_context_kind,
        "Provide direct source context",
        "fallback to explicit files, paths, or pasted content when governed tools are blocked or unavailable",
        tool_context,
    ));
    out
}

fn step_for_contract(contract: &CoreContractCall) -> OrchestrationPlanStep {
    match contract {
        CoreContractCall::ToolCapabilityProbe => OrchestrationPlanStep {
            step_id: "step_tool_capability_probe".to_string(),
            operation: "probe_tool_capability".to_string(),
            target_contract: CoreContractCall::ToolCapabilityProbe,
        },
        CoreContractCall::ToolBrokerRequest => OrchestrationPlanStep {
            step_id: "step_tool_broker_request".to_string(),
            operation: "route_tool_call".to_string(),
            target_contract: CoreContractCall::ToolBrokerRequest,
        },
        CoreContractCall::TaskFabricProposal => OrchestrationPlanStep {
            step_id: "step_task_fabric_proposal".to_string(),
            operation: "propose_task_graph_update".to_string(),
            target_contract: CoreContractCall::TaskFabricProposal,
        },
        CoreContractCall::UnifiedMemoryRead => OrchestrationPlanStep {
            step_id: "step_memory_read".to_string(),
            operation: "request_materialized_view".to_string(),
            target_contract: CoreContractCall::UnifiedMemoryRead,
        },
        CoreContractCall::AssimilationPlanRequest => OrchestrationPlanStep {
            step_id: "step_assimilation_plan".to_string(),
            operation: "request_assimilation_plan".to_string(),
            target_contract: CoreContractCall::AssimilationPlanRequest,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingress;
    use serde_json::json;

    #[test]
    fn fallback_context_reads_reason_and_backend_from_payload() {
        let context = tool_fallback_context_from_payload(&json!({
            "tool_name": "web_search",
            "tool_reason_code": "auth_required",
            "tool_backend_class": "retrieval_plane",
            "tool_backend": "retrieval_plane"
        }))
        .expect("context");
        assert_eq!(context.reason_code, ToolReasonCode::AuthRequired);
        assert_eq!(context.backend_class, ToolBackendClass::RetrievalPlane);
    }

    #[test]
    fn fallback_actions_are_reason_and_backend_aware() {
        let request = ingress::normalize_request(crate::contracts::OrchestrationRequest {
            session_id: "s1".to_string(),
            intent: "search the web".to_string(),
            payload: json!({}),
        });
        let context = ToolFallbackContext {
            tool_name: "web_search".to_string(),
            backend: "retrieval_plane".to_string(),
            backend_class: ToolBackendClass::RetrievalPlane,
            reason_code: ToolReasonCode::AuthRequired,
        };
        let actions = fallback_actions(&request, RequestClass::ToolCall, Some(&context));
        assert!(actions
            .iter()
            .any(|row| row.kind == "configure_provider_access"));
        assert!(actions
            .iter()
            .all(|row| row.reason_code == Some(ToolReasonCode::AuthRequired)));
    }

    #[test]
    fn swarm_agent_runtime_invalid_args_recommends_bootstrap_contract() {
        let request = ingress::normalize_request(crate::contracts::OrchestrationRequest {
            session_id: "s1".to_string(),
            intent: "send directive to child agent".to_string(),
            payload: json!({}),
        });
        let context = ToolFallbackContext {
            tool_name: "sessions_send".to_string(),
            backend: "agent_runtime".to_string(),
            backend_class: ToolBackendClass::AgentRuntime,
            reason_code: ToolReasonCode::InvalidArgs,
        };
        let actions = fallback_actions(&request, RequestClass::ToolCall, Some(&context));
        assert!(actions
            .iter()
            .any(|row| row.kind == "inspect_agent_bootstrap_contract"));
        assert!(actions
            .iter()
            .all(|row| row.reason_code == Some(ToolReasonCode::InvalidArgs)));
    }

    #[test]
    fn workspace_resource_uses_direct_context_fallback() {
        let request = ingress::normalize_request(crate::contracts::OrchestrationRequest {
            session_id: "s1".to_string(),
            intent: "read workspace file".to_string(),
            payload: json!({"path":"README.md"}),
        });
        let actions = fallback_actions(&request, RequestClass::ToolCall, None);
        assert!(actions
            .iter()
            .any(|row| row.kind == "paste_workspace_context"));
    }
}
