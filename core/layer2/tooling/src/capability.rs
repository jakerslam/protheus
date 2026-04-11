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
    ExecutionError,
    PolicyDenied,
    TransportUnavailable,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapability {
    pub tool_name: String,
    pub required_args: Vec<String>,
    pub allowed_callers: Vec<BrokerCaller>,
    pub backend: String,
    pub read_only: bool,
    pub discoverable: bool,
    pub status: ToolCapabilityStatus,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilitySpec {
    tool_name: String,
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
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "file_read".to_string(),
            required_args: vec!["path".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "file_read_many".to_string(),
            required_args: vec!["paths".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "folder_export".to_string(),
            required_args: vec!["path".to_string()],
            backend: "workspace_fs".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "manage_agent".to_string(),
            required_args: vec!["action".to_string(), "agent_id".to_string()],
            backend: "agent_runtime".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "spawn_subagents".to_string(),
            required_args: vec!["objective".to_string()],
            backend: "agent_runtime".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "terminal_exec".to_string(),
            required_args: vec!["command".to_string()],
            backend: "governed_terminal".to_string(),
            read_only: false,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "web_fetch".to_string(),
            required_args: vec!["url".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "web_search".to_string(),
            required_args: vec!["query".to_string()],
            backend: "retrieval_plane".to_string(),
            read_only: true,
            discoverable: true,
            status: ToolCapabilityStatus::Available,
        },
        CapabilitySpec {
            tool_name: "workspace_analyze".to_string(),
            required_args: vec!["path".to_string()],
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
    let mut out = Vec::<ToolCapability>::new();
    for (tool_name, spec) in matrix {
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
            required_args: spec.required_args,
            allowed_callers: callers,
            backend: spec.backend,
            read_only: spec.read_only,
            discoverable: spec.discoverable,
            status: spec.status,
        });
    }
    out
}

pub fn capability_probe_for(
    allowed_tools: &std::collections::HashMap<BrokerCaller, std::collections::HashSet<String>>,
    caller: BrokerCaller,
    tool_name: &str,
) -> ToolCapabilityProbe {
    let normalized = tool_name.trim().to_ascii_lowercase();
    let Some(spec) = capability_matrix().get(&normalized).cloned() else {
        return ToolCapabilityProbe {
            tool_name: normalized,
            caller,
            available: false,
            discoverable: false,
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::UnknownTool,
            reason: "unknown_tool".to_string(),
            required_args: Vec::new(),
            backend: "unknown".to_string(),
        };
    };
    let allowed = allowed_tools
        .get(&caller)
        .map(|set| set.contains(&normalized))
        .unwrap_or(false);
    let (available, status, reason_code, reason) = if allowed {
        (true, spec.status, ToolReasonCode::Ok, "ok".to_string())
    } else {
        (
            false,
            ToolCapabilityStatus::Blocked,
            ToolReasonCode::CallerNotAuthorized,
            "caller_not_authorized".to_string(),
        )
    };
    ToolCapabilityProbe {
        tool_name: normalized,
        caller,
        available,
        discoverable: spec.discoverable,
        status,
        reason_code,
        reason,
        required_args: spec.required_args,
        backend: spec.backend,
    }
}

pub fn required_args_for(tool_name: &str) -> Vec<String> {
    capability_matrix()
        .get(tool_name)
        .map(|spec| spec.required_args.clone())
        .unwrap_or_default()
}
