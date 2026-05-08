fn tool_capability_status_color(
    status: crate::infring_tooling_core_v1_bridge::ToolCapabilityStatus,
    read_only: bool,
) -> &'static str {
    match status {
        crate::infring_tooling_core_v1_bridge::ToolCapabilityStatus::Available => {
            if read_only {
                "green"
            } else {
                "yellow"
            }
        }
        crate::infring_tooling_core_v1_bridge::ToolCapabilityStatus::Degraded => "yellow",
        crate::infring_tooling_core_v1_bridge::ToolCapabilityStatus::Blocked => "red",
        crate::infring_tooling_core_v1_bridge::ToolCapabilityStatus::Unavailable => "gray",
    }
}

fn capabilities_status_payload(root: &Path) -> Value {
    let policy = tool_governance_policy(root);
    let broker = crate::infring_tooling_core_v1_bridge::ToolBroker::default();
    let catalog = broker.capability_catalog();
    let grouped_catalog = broker.grouped_capability_catalog();
    json!({
        "ok": true,
        "type": "tool_capability_tiers",
        "policy": policy,
        "catalog_contract": "domain_grouped_tool_catalog_v1",
        "catalog_default_workflow": "complex_prompt_chain_v1",
        "catalog_domains": grouped_catalog,
        "tools": catalog.iter().map(|row| {
            json!({
                "tool": row.tool_name,
                "tier": tool_capability_status_color(row.status, row.read_only),
                "domain": row.domain,
                "backend": row.backend,
                "read_only": row.read_only,
                "discoverable": row.discoverable,
                "capability_family": row.capability_family,
                "retrieval_mode": row.retrieval_mode,
                "quality_lanes": row.quality_lanes,
                "contract_surface": row.contract_surface,
            })
        }).collect::<Vec<_>>()
    })
}
