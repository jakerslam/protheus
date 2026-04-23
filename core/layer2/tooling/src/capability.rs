// Layer ownership: core/layer2/tooling (authoritative canonical tool/evidence substrate).
use crate::backend_registry::{live_backend_registry, live_backend_status_for, ToolBackendClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::tool_broker::BrokerCaller;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityStatus {
    Available,
    Blocked,
    Unavailable,
    Degraded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolReasonCode {
    Ok,
    UnknownTool,
    CallerNotAuthorized,
    InvalidArgs,
    AuthRequired,
    DaemonUnavailable,
    WebsocketUnavailable,
    BackendDegraded,
    ExecutionError,
    PolicyDenied,
    TransportUnavailable,
    Timeout,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapabilityDomain {
    Web,
    File,
    Agent,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapability {
    pub tool_name: String,
    pub domain: ToolCapabilityDomain,
    pub required_args: Vec<String>,
    pub allowed_callers: Vec<BrokerCaller>,
    pub backend: String,
    pub backend_class: ToolBackendClass,
    pub read_only: bool,
    pub discoverable: bool,
    pub status: ToolCapabilityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityCatalogGroup {
    pub domain: ToolCapabilityDomain,
    pub description: String,
    pub tool_count: usize,
    pub available_count: usize,
    pub discoverable_count: usize,
    pub tools: Vec<ToolCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityProbe {
    pub tool_name: String,
    pub caller: BrokerCaller,
    pub available: bool,
    pub discoverable: bool,
    pub status: ToolCapabilityStatus,
    pub reason_code: ToolReasonCode,
    pub reason: String,
    pub required_args: Vec<String>,
    pub backend: String,
    pub backend_class: ToolBackendClass,
    pub backend_status: ToolCapabilityStatus,
    pub backend_reason_code: ToolReasonCode,
    pub backend_reason: String,
    pub daemon_healthy: Option<bool>,
    pub ws_healthy: Option<bool>,
    pub auth_healthy: Option<bool>,
    pub resident_ipc_authoritative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilitySpec {
    tool_name: String,
    domain: ToolCapabilityDomain,
    required_args: Vec<String>,
    backend: String,
    read_only: bool,
    discoverable: bool,
    status: ToolCapabilityStatus,
}

fn capability_specs() -> Vec<CapabilitySpec> {
    vec![
        CapabilitySpec {
            tool_name: "batch_query".to_string(),
            domain: ToolCapabilityDomain::Web,
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "file_read".to_string(),
            domain: ToolCapabilityDomain::File,
            required_args: vec!["path".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "file_read_many".to_string(),
            domain: ToolCapabilityDomain::File,
            required_args: vec!["paths".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "folder_export".to_string(),
            domain: ToolCapabilityDomain::File,
            required_args: vec!["path".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "manage_agent".to_string(),
            domain: ToolCapabilityDomain::Agent,
            required_args: vec!["action".to_string(), "agent_id".to_string()],
            backend: "agent_runtime".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "spawn_subagents".to_string(),
            domain: ToolCapabilityDomain::Agent,
            required_args: vec!["objective".to_string()],
            backend: "agent_runtime".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "terminal_exec".to_string(),
            domain: ToolCapabilityDomain::Terminal,
            required_args: vec!["command".to_string()],
            backend: "governed_terminal".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "web_fetch".to_string(),
            domain: ToolCapabilityDomain::Web,
            required_args: vec!["url".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "web_search".to_string(),
            domain: ToolCapabilityDomain::Web,
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "workspace_analyze".to_string(),
            domain: ToolCapabilityDomain::File,
            required_args: vec!["query".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
    ]
}

fn capability_matrix() -> BTreeMap<String, CapabilitySpec> {
    let mut out = BTreeMap::<String, CapabilitySpec>::new();
    for spec in capability_specs() {
        out.insert(spec.tool_name.clone(), spec);
    }
    out
}

pub fn all_capabilities_for_callers(
    allowed_tools: &std::collections::HashMap<BrokerCaller, std::collections::HashSet<String>>,
) -> Vec<ToolCapability> {
    let matrix = capability_matrix();
    let backend_registry = live_backend_registry()
        .into_iter()
        .map(|row| (row.backend.clone(), row))
        .collect::<BTreeMap<_, _>>();
    let mut out = Vec::<ToolCapability>::new();
    for (tool_name, spec) in matrix {
        let backend_health = backend_registry
            .get(&spec.backend)
            .cloned()
            .unwrap_or_else(|| live_backend_status_for(spec.backend.as_str()));
        let mut callers = allowed_tools
            .iter()
            .filter_map(|(caller, tools)| {
                if tools.contains(&tool_name) {
                    Some(*caller)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        callers.sort_by_key(|caller| match caller {
            BrokerCaller::Client => 0,
            BrokerCaller::Worker => 1,
            BrokerCaller::System => 2,
        });
        out.push(ToolCapability {
            tool_name,
            domain: spec.domain,
            required_args: spec.required_args,
            allowed_callers: callers,
            backend: spec.backend,
            backend_class: backend_health.backend_class,
            read_only: spec.read_only,
            discoverable: spec.discoverable,
            status: merge_status(spec.status, backend_health.status),
        });
    }
    out
}

pub fn grouped_capabilities_for_callers(
    allowed_tools: &std::collections::HashMap<BrokerCaller, std::collections::HashSet<String>>,
) -> Vec<ToolCapabilityCatalogGroup> {
    let mut grouped = BTreeMap::<ToolCapabilityDomain, Vec<ToolCapability>>::new();
    for capability in all_capabilities_for_callers(allowed_tools) {
        grouped
            .entry(capability.domain)
            .or_default()
            .push(capability);
    }
    grouped
        .into_iter()
        .map(|(domain, mut tools)| {
            tools.sort_by(|left, right| left.tool_name.cmp(&right.tool_name));
            ToolCapabilityCatalogGroup {
                domain,
                description: capability_domain_description(domain).to_string(),
                tool_count: tools.len(),
                available_count: tools
                    .iter()
                    .filter(|row| matches!(row.status, ToolCapabilityStatus::Available))
                    .count(),
                discoverable_count: tools.iter().filter(|row| row.discoverable).count(),
                tools,
            }
        })
        .collect::<Vec<_>>()
}

pub fn capability_domain_description(domain: ToolCapabilityDomain) -> &'static str {
    match domain {
        ToolCapabilityDomain::Web => "Governed web search, fetch, and retrieval tools.",
        ToolCapabilityDomain::File => "Workspace file, folder, and repository reading tools.",
        ToolCapabilityDomain::Agent => "Agent management and subagent orchestration tools.",
        ToolCapabilityDomain::Terminal => "Governed terminal execution tools.",
    }
}

fn canonical_tool_name(raw: &str) -> String {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "workspace_read" | "read_file" => "file_read".to_string(),
        "workspace_read_many" | "read_many_files" => "file_read_many".to_string(),
        "workspace_search"
        | "file_search"
        | "file_list"
        | "context_search"
        | "context_resolve"
        | "workspace_context"
        | "local_context"
        | "route_tool_call"
        | "tool_route"
        | "execute_tool"
        | "git_status"
        | "worktree_inspect" => "workspace_analyze".to_string(),
        "web_lookup" | "browse_web" => "web_search".to_string(),
        "shell_exec" => "terminal_exec".to_string(),
        _ => normalized,
    }
}

pub fn capability_probe_for(
    allowed_tools: &std::collections::HashMap<BrokerCaller, std::collections::HashSet<String>>,
    caller: BrokerCaller,
    tool_name: &str,
) -> ToolCapabilityProbe {
    let requested = tool_name.trim().to_ascii_lowercase();
    let canonical = canonical_tool_name(requested.as_str());
    let Some(spec) = capability_matrix().get(&canonical).cloned() else {
        return ToolCapabilityProbe {
            tool_name: requested,
            caller,
            available: false,
            discoverable: false,
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::UnknownTool,
            reason: "unknown_tool".to_string(),
            required_args: Vec::new(),
            backend: "unknown".to_string(),
            backend_class: ToolBackendClass::Unknown,
            backend_status: ToolCapabilityStatus::Unavailable,
            backend_reason_code: ToolReasonCode::UnknownTool,
            backend_reason: "unknown_backend".to_string(),
            daemon_healthy: None,
            ws_healthy: None,
            auth_healthy: None,
            resident_ipc_authoritative: true,
        };
    };
    let backend_health = live_backend_status_for(spec.backend.as_str());
    let allowed = allowed_tools
        .get(&caller)
        .map(|set| set.contains(&canonical) || set.contains(&requested))
        .unwrap_or(false);
    let (available, status, reason_code, mut reason) = if !allowed {
        (
            false,
            ToolCapabilityStatus::Blocked,
            ToolReasonCode::CallerNotAuthorized,
            "caller_not_authorized".to_string(),
        )
    } else {
        match merge_status(spec.status, backend_health.status) {
            ToolCapabilityStatus::Available => (
                true,
                ToolCapabilityStatus::Available,
                ToolReasonCode::Ok,
                "ok".to_string(),
            ),
            ToolCapabilityStatus::Degraded => (
                true,
                ToolCapabilityStatus::Degraded,
                ToolReasonCode::BackendDegraded,
                backend_health.reason.clone(),
            ),
            ToolCapabilityStatus::Blocked => (
                false,
                ToolCapabilityStatus::Blocked,
                if matches!(
                    backend_health.reason_code,
                    ToolReasonCode::AuthRequired | ToolReasonCode::PolicyDenied
                ) {
                    backend_health.reason_code
                } else {
                    ToolReasonCode::PolicyDenied
                },
                backend_health.reason.clone(),
            ),
            ToolCapabilityStatus::Unavailable => (
                false,
                ToolCapabilityStatus::Unavailable,
                if matches!(spec.status, ToolCapabilityStatus::Unavailable) {
                    ToolReasonCode::TransportUnavailable
                } else {
                    backend_health.reason_code
                },
                backend_health.reason.clone(),
            ),
        }
    };
    if matches!(reason_code, ToolReasonCode::Ok) && requested != canonical {
        reason = format!("ok_alias:{requested}->{canonical}");
    }
    ToolCapabilityProbe {
        tool_name: canonical,
        caller,
        available,
        discoverable: spec.discoverable,
        status,
        reason_code,
        reason,
        required_args: spec.required_args,
        backend: spec.backend,
        backend_class: backend_health.backend_class,
        backend_status: backend_health.status,
        backend_reason_code: backend_health.reason_code,
        backend_reason: backend_health.reason,
        daemon_healthy: backend_health.daemon_healthy,
        ws_healthy: backend_health.ws_healthy,
        auth_healthy: backend_health.auth_healthy,
        resident_ipc_authoritative: backend_health.resident_ipc_authoritative,
    }
}

fn merge_status(
    static_status: ToolCapabilityStatus,
    backend_status: ToolCapabilityStatus,
) -> ToolCapabilityStatus {
    match (static_status, backend_status) {
        (ToolCapabilityStatus::Unavailable, _) | (_, ToolCapabilityStatus::Unavailable) => {
            ToolCapabilityStatus::Unavailable
        }
        (ToolCapabilityStatus::Blocked, _) | (_, ToolCapabilityStatus::Blocked) => {
            ToolCapabilityStatus::Blocked
        }
        (ToolCapabilityStatus::Degraded, _) | (_, ToolCapabilityStatus::Degraded) => {
            ToolCapabilityStatus::Degraded
        }
        _ => ToolCapabilityStatus::Available,
    }
}

pub fn required_args_for(tool_name: &str) -> Vec<String> {
    capability_matrix()
        .get(tool_name)
        .map(|spec| spec.required_args.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn workspace_analyze_contract_is_query_driven() {
        let mut allowed = HashMap::<BrokerCaller, HashSet<String>>::new();
        allowed.insert(
            BrokerCaller::Client,
            ["workspace_analyze"]
                .iter()
                .map(|row| row.to_string())
                .collect::<HashSet<_>>(),
        );
        let probe = capability_probe_for(&allowed, BrokerCaller::Client, "workspace_analyze");
        assert!(probe.available);
        assert_eq!(probe.required_args, vec!["query".to_string()]);
        assert_eq!(probe.reason_code, ToolReasonCode::Ok);
    }

    #[test]
    fn probe_includes_live_backend_health_fields() {
        let mut allowed = HashMap::<BrokerCaller, HashSet<String>>::new();
        allowed.insert(
            BrokerCaller::Client,
            ["web_search"]
                .iter()
                .map(|row| row.to_string())
                .collect::<HashSet<_>>(),
        );
        let probe = capability_probe_for(&allowed, BrokerCaller::Client, "web_search");
        assert_eq!(probe.backend, "retrieval_plane");
        assert_eq!(probe.backend_class, ToolBackendClass::RetrievalPlane);
        assert!(!probe.backend_reason.is_empty());
    }

    #[test]
    fn grouped_catalog_clusters_tools_by_domain() {
        let mut allowed = HashMap::<BrokerCaller, HashSet<String>>::new();
        allowed.insert(
            BrokerCaller::Client,
            [
                "web_search",
                "web_fetch",
                "file_read",
                "workspace_analyze",
                "spawn_subagents",
                "terminal_exec",
            ]
            .iter()
            .map(|row| row.to_string())
            .collect::<HashSet<_>>(),
        );
        let grouped = grouped_capabilities_for_callers(&allowed);
        assert!(grouped.iter().any(|row| {
            row.domain == ToolCapabilityDomain::Web
                && row.tools.iter().any(|tool| tool.tool_name == "web_search")
        }));
        assert!(grouped.iter().any(|row| {
            row.domain == ToolCapabilityDomain::File
                && row.tools.iter().any(|tool| tool.tool_name == "file_read")
        }));
        assert!(grouped.iter().any(|row| {
            row.domain == ToolCapabilityDomain::Agent
                && row
                    .tools
                    .iter()
                    .any(|tool| tool.tool_name == "spawn_subagents")
        }));
        assert!(grouped.iter().any(|row| {
            row.domain == ToolCapabilityDomain::Terminal
                && row
                    .tools
                    .iter()
                    .any(|tool| tool.tool_name == "terminal_exec")
        }));
    }

    #[test]
    fn capability_probe_normalizes_extended_tool_aliases() {
        let mut allowed = HashMap::<BrokerCaller, HashSet<String>>::new();
        allowed.insert(
            BrokerCaller::Client,
            ["workspace_analyze", "terminal_exec", "file_read_many"]
                .iter()
                .map(|row| row.to_string())
                .collect::<HashSet<_>>(),
        );
        let local_context_probe =
            capability_probe_for(&allowed, BrokerCaller::Client, "local_context");
        let shell_exec_probe = capability_probe_for(&allowed, BrokerCaller::Client, "shell_exec");
        let read_many_probe =
            capability_probe_for(&allowed, BrokerCaller::Client, "workspace_read_many");
        assert_eq!(local_context_probe.tool_name, "workspace_analyze");
        assert_eq!(shell_exec_probe.tool_name, "terminal_exec");
        assert_eq!(read_many_probe.tool_name, "file_read_many");
    }
}
