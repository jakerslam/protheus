use crate::tool_contracts::ToolCdContract;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCapabilityContractSurface {
    pub supports_bulk: bool,
    pub max_bulk_items: usize,
    pub cost_tier: String,
    pub requires_network: bool,
    pub default_extraction_type: String,
    pub allowed_extraction_types: Vec<String>,
    pub selector_hint_allowed: bool,
    pub main_content_only_default: bool,
    pub max_chars: usize,
    pub readiness_supported_fields: Vec<String>,
    pub default_timeout_ms: u64,
    pub dynamic_page_allowed: bool,
    pub disable_resources_allowed: bool,
    pub block_ads_allowed: bool,
    pub blocked_domains_allowed: bool,
    pub session_state_scope: String,
    pub session_reuse_allowed: bool,
    pub session_pooling_mode: String,
    pub session_max_pages_default: usize,
    pub session_max_parallel_items_default: usize,
    pub session_request_overrides_allowed: bool,
    pub session_close_on_complete_default: bool,
}

pub fn capability_contract_surface(contract: &ToolCdContract) -> ToolCapabilityContractSurface {
    ToolCapabilityContractSurface {
        supports_bulk: contract.retrieval.supports_bulk,
        max_bulk_items: contract.retrieval.max_bulk_items,
        cost_tier: contract.retrieval.cost_tier.clone(),
        requires_network: contract.retrieval.requires_network,
        default_extraction_type: contract.extraction.default_type.clone(),
        allowed_extraction_types: contract.extraction.allowed_types.clone(),
        selector_hint_allowed: contract.extraction.selector_hint_allowed,
        main_content_only_default: contract.extraction.main_content_only_default,
        max_chars: contract.extraction.max_chars,
        readiness_supported_fields: contract.readiness.supported_fields.clone(),
        default_timeout_ms: contract.readiness.default_timeout_ms,
        dynamic_page_allowed: contract.readiness.dynamic_page_allowed,
        disable_resources_allowed: contract.resource_policy.disable_resources_allowed,
        block_ads_allowed: contract.resource_policy.block_ads_allowed,
        blocked_domains_allowed: contract.resource_policy.blocked_domains_allowed,
        session_state_scope: contract.session_policy.state_scope.clone(),
        session_reuse_allowed: contract.session_policy.reuse_allowed,
        session_pooling_mode: contract.session_policy.pooling_mode.clone(),
        session_max_pages_default: contract.session_policy.max_pages_default,
        session_max_parallel_items_default: contract.session_policy.max_parallel_items_default,
        session_request_overrides_allowed: contract.session_policy.request_overrides_allowed,
        session_close_on_complete_default: contract.session_policy.close_on_complete_default,
    }
}
