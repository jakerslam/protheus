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
    let contract_surface = probe.contract_surface.expect("contract surface");
    assert_eq!(contract_surface.operations, vec!["search".to_string()]);
    assert_eq!(contract_surface.default_extraction_type, "markdown");
    assert_eq!(contract_surface.default_timeout_ms, 30_000);
    assert!(contract_surface.request_fingerprint_dedupe);
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
    let web_search_surface = web_search
        .contract_surface
        .as_ref()
        .expect("web_search contract surface");
    assert_eq!(web_search_surface.operations, vec!["search".to_string()]);
    assert!(web_search_surface
        .optional_args
        .contains(&"aperture".to_string()));
    assert!(web_search_surface
        .optional_args
        .contains(&"source_scope".to_string()));
    assert_eq!(web_search_surface.default_extraction_type, "markdown");
    assert_eq!(web_search_surface.cost_tier, "low");
    assert_eq!(web_search_surface.default_timeout_ms, 30_000);
    assert!(!web_search_surface.dynamic_page_allowed);
    assert_eq!(web_search_surface.selector_hint_fallback_mode, "whole_page");
    assert!(!web_search_surface.include_artifact_refs);
    assert_eq!(web_search_surface.per_domain_concurrency_default, 0);
    assert!(!web_search_surface.blocked_response_retry_allowed);
    assert_eq!(web_search_surface.blocked_domains_source, "none");
    assert_eq!(web_search_surface.ad_block_profile_default, None);
    assert_eq!(web_search_surface.session_state_scope, "stateless");
    assert_eq!(web_search_surface.session_pooling_mode, "none");
    assert!(!web_search_surface.implicit_session_on_invoke);
    assert_eq!(web_search_surface.session_handle_arg, None);
    let batch_query = catalog
        .iter()
        .find(|row| row.tool_name == "batch_query")
        .expect("batch_query capability");
    assert_eq!(batch_query.retrieval_mode.as_deref(), Some("search_pack"));
    let batch_query_surface = batch_query
        .contract_surface
        .as_ref()
        .expect("batch_query contract surface");
    assert_eq!(
        batch_query_surface.operations,
        vec!["search_pack".to_string()]
    );
    assert!(batch_query_surface.supports_bulk);
    assert_eq!(batch_query_surface.max_bulk_items, 6);
    assert_eq!(batch_query_surface.per_domain_concurrency_default, 2);
    assert!(batch_query_surface.blocked_response_retry_allowed);
    assert_eq!(batch_query_surface.max_blocked_retries_default, 2);
    assert_eq!(batch_query_surface.session_max_parallel_items_default, 6);
    let web_fetch = catalog
        .iter()
        .find(|row| row.tool_name == "web_fetch")
        .expect("web_fetch capability");
    let web_fetch_surface = web_fetch
        .contract_surface
        .as_ref()
        .expect("web_fetch contract surface");
    assert_eq!(web_fetch_surface.operations, vec!["fetch".to_string()]);
    assert!(web_fetch_surface.include_artifact_refs);
    assert!(web_fetch_surface
        .allowed_artifact_kinds
        .contains(&"screenshot".to_string()));
    assert!(web_fetch_surface
        .optional_args
        .contains(&"selector_hint".to_string()));
    assert!(web_fetch_surface
        .optional_args
        .contains(&"source_scope".to_string()));
    assert_eq!(web_fetch_surface.per_domain_concurrency_default, 1);
    assert!(web_fetch_surface.blocked_response_retry_allowed);
    assert_eq!(web_fetch_surface.max_blocked_retries_default, 2);
    assert!(web_fetch_surface.dynamic_page_allowed);
    assert_eq!(web_fetch_surface.selector_hint_fallback_mode, "whole_page");
    assert!(web_fetch_surface.disable_resources_allowed);
    assert!(web_fetch_surface.block_ads_allowed);
    assert!(web_fetch_surface.blocked_domains_allowed);
    assert_eq!(
        web_fetch_surface.blocked_domains_source,
        "profile_or_custom"
    );
    assert_eq!(
        web_fetch_surface.ad_block_profile_default.as_deref(),
        Some("built_in_ad_domains")
    );
    assert_eq!(web_fetch_surface.session_state_scope, "session_context");
    assert!(web_fetch_surface.session_reuse_allowed);
    assert_eq!(web_fetch_surface.session_pooling_mode, "serial_reuse");
    assert_eq!(web_fetch_surface.session_max_pages_default, 1);
    assert!(web_fetch_surface.session_request_overrides_allowed);
    assert!(web_fetch_surface.implicit_session_on_invoke);
    assert_eq!(
        web_fetch_surface.session_handle_arg.as_deref(),
        Some("session_id")
    );
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
