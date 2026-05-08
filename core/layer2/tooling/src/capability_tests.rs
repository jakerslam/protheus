use crate::backend_registry::ToolBackendClass;
use crate::capability::{
    all_capabilities_for_callers, capability_probe_for, grouped_capabilities_for_callers,
    ToolCapabilityDomain, ToolReasonCode,
};
use crate::tool_broker::BrokerCaller;
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
fn web_capability_catalog_exposes_tool_cd_metadata() {
    let mut allowed = HashMap::<BrokerCaller, HashSet<String>>::new();
    allowed.insert(
        BrokerCaller::Client,
        ["web_search", "batch_query", "web_fetch"]
            .iter()
            .map(|row| row.to_string())
            .collect::<HashSet<_>>(),
    );
    let catalog = all_capabilities_for_callers(&allowed);
    let web_search = catalog
        .iter()
        .find(|row| row.tool_name == "web_search")
        .expect("web_search capability");
    assert_eq!(
        web_search.capability_family.as_deref(),
        Some("web_retrieval")
    );
    assert_eq!(web_search.retrieval_mode.as_deref(), Some("search"));
    assert!(web_search.quality_lanes.contains(&"low_signal".to_string()));
    let batch_query = catalog
        .iter()
        .find(|row| row.tool_name == "batch_query")
        .expect("batch_query capability");
    assert_eq!(batch_query.retrieval_mode.as_deref(), Some("search_pack"));
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
    let local_context_probe = capability_probe_for(&allowed, BrokerCaller::Client, "local_context");
    let shell_exec_probe = capability_probe_for(&allowed, BrokerCaller::Client, "shell_exec");
    let read_many_probe =
        capability_probe_for(&allowed, BrokerCaller::Client, "workspace_read_many");
    assert_eq!(local_context_probe.tool_name, "workspace_analyze");
    assert_eq!(shell_exec_probe.tool_name, "terminal_exec");
    assert_eq!(read_many_probe.tool_name, "file_read_many");
}
