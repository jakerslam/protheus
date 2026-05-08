use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const WEB_RETRIEVAL_TOOL_CD: &str = include_str!("../tool_cds/web_retrieval_v0.tool.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCdCatalog {
    pub version: String,
    pub source: String,
    pub contracts: Vec<ToolCdContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCdContract {
    pub tool_id: String,
    pub capability_family: String,
    pub required_args: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub retrieval: ToolRetrievalContract,
    pub extraction: ToolExtractionContract,
    pub readiness: ToolReadinessContract,
    pub resource_policy: ToolResourcePolicyContract,
    pub session_policy: ToolSessionPolicyContract,
    pub safety: ToolSafetyContract,
    pub evidence_packaging: ToolEvidencePackagingContract,
    pub quality_classification: ToolQualityClassificationContract,
    pub quality_lanes: Vec<String>,
    pub visibility: ToolVisibilityContract,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolRetrievalContract {
    pub mode: String,
    pub supports_bulk: bool,
    pub max_bulk_items: usize,
    pub cost_tier: String,
    pub requires_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolExtractionContract {
    pub default_type: String,
    pub allowed_types: Vec<String>,
    pub selector_hint_allowed: bool,
    pub main_content_only_default: bool,
    pub max_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolReadinessContract {
    pub supported_fields: Vec<String>,
    pub default_timeout_ms: u64,
    pub dynamic_page_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResourcePolicyContract {
    pub disable_resources_allowed: bool,
    pub block_ads_allowed: bool,
    pub blocked_domains_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSessionPolicyContract {
    pub state_scope: String,
    pub reuse_allowed: bool,
    pub pooling_mode: String,
    pub max_pages_default: usize,
    pub max_parallel_items_default: usize,
    pub request_overrides_allowed: bool,
    pub close_on_complete_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSafetyContract {
    pub ssrf_guard_required: bool,
    pub redirect_policy: String,
    pub prompt_injection_sanitizer_required: bool,
    pub sanitization: ToolSanitizationContract,
    pub raw_payload_chat_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSanitizationContract {
    pub hidden_content_removed: bool,
    pub template_content_removed: bool,
    pub html_comments_removed: bool,
    pub zero_width_chars_removed: bool,
    pub script_style_noise_removed: bool,
    pub raw_payload_quarantined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolEvidencePackagingContract {
    pub include_content_excerpt: bool,
    pub include_source_url: bool,
    pub include_status: bool,
    pub include_fetch_mode: bool,
    pub include_extraction_type: bool,
    pub include_quality_flags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolQualityClassificationContract {
    pub classifier: String,
    pub status_fields: Vec<String>,
    pub content_fields: Vec<String>,
    pub blocked_status_codes: Vec<u16>,
    #[serde(default)]
    pub retryable_status_codes: Vec<u16>,
    #[serde(default)]
    pub blocked_error_fragments: Vec<String>,
    #[serde(default)]
    pub blocked_text_fragments: Vec<String>,
    #[serde(default)]
    pub needs_dynamic_text_fragments: Vec<String>,
    #[serde(default)]
    pub low_signal_text_fragments: Vec<String>,
    #[serde(default)]
    pub irrelevant_text_fragments: Vec<String>,
    #[serde(default)]
    pub proxy_metadata_fields: Vec<String>,
    pub min_partial_content_chars: usize,
    pub min_usable_content_chars: usize,
    pub emit_on_normalized_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolVisibilityContract {
    pub raw_payload_chat_visible: bool,
    pub tool_trace_chat_visible: bool,
    pub final_answer_requires_synthesis: bool,
}

pub fn published_tool_cd_catalog_v1() -> ToolCdCatalog {
    let catalog = serde_json::from_str::<ToolCdCatalog>(WEB_RETRIEVAL_TOOL_CD)
        .expect("bundled web retrieval Tool CD must parse");
    validate_tool_cd_catalog(&catalog).expect("bundled web retrieval Tool CD must validate");
    catalog
}

pub fn tool_cd_contract_for(tool_id_or_alias: &str) -> Option<ToolCdContract> {
    let requested = normalized_key(tool_id_or_alias);
    published_tool_cd_catalog_v1()
        .contracts
        .into_iter()
        .find(|contract| {
            normalized_key(&contract.tool_id) == requested
                || contract
                    .aliases
                    .iter()
                    .any(|alias| normalized_key(alias) == requested)
        })
}

pub fn tool_cd_contract_index_v1() -> BTreeMap<String, ToolCdContract> {
    published_tool_cd_catalog_v1()
        .contracts
        .into_iter()
        .map(|contract| (contract.tool_id.clone(), contract))
        .collect()
}

pub fn validate_tool_cd_catalog(catalog: &ToolCdCatalog) -> Result<(), String> {
    if normalized_key(&catalog.version).is_empty() {
        return Err("version_required".to_string());
    }
    let mut seen = BTreeSet::<String>::new();
    for contract in &catalog.contracts {
        validate_contract(contract)?;
        if !seen.insert(normalized_key(&contract.tool_id)) {
            return Err(format!("duplicate_tool_id:{}", contract.tool_id));
        }
    }
    Ok(())
}

fn validate_contract(contract: &ToolCdContract) -> Result<(), String> {
    let tool_id = normalized_key(&contract.tool_id);
    if tool_id.is_empty() {
        return Err("tool_id_required".to_string());
    }
    if normalized_key(&contract.capability_family).is_empty() {
        return Err(format!("capability_family_required:{tool_id}"));
    }
    if contract.required_args.is_empty() {
        return Err(format!("required_args_required:{tool_id}"));
    }
    if normalized_key(&contract.retrieval.mode).is_empty() {
        return Err(format!("retrieval_mode_required:{tool_id}"));
    }
    if contract.retrieval.max_bulk_items == 0 {
        return Err(format!("max_bulk_items_required:{tool_id}"));
    }
    if contract.extraction.allowed_types.is_empty()
        || !contract
            .extraction
            .allowed_types
            .contains(&contract.extraction.default_type)
    {
        return Err(format!("extraction_default_must_be_allowed:{tool_id}"));
    }
    if contract.readiness.default_timeout_ms == 0 {
        return Err(format!("readiness_timeout_required:{tool_id}"));
    }
    if normalized_key(&contract.session_policy.state_scope).is_empty() {
        return Err(format!("session_state_scope_required:{tool_id}"));
    }
    if normalized_key(&contract.session_policy.pooling_mode).is_empty() {
        return Err(format!("session_pooling_mode_required:{tool_id}"));
    }
    if contract.session_policy.max_parallel_items_default == 0 {
        return Err(format!("session_parallel_items_required:{tool_id}"));
    }
    if !contract.safety.ssrf_guard_required {
        return Err(format!("ssrf_guard_required:{tool_id}"));
    }
    if contract.safety.raw_payload_chat_visible || contract.visibility.raw_payload_chat_visible {
        return Err(format!("raw_payload_must_not_be_chat_visible:{tool_id}"));
    }
    if !contract.safety.prompt_injection_sanitizer_required
        || !contract.safety.sanitization.hidden_content_removed
        || !contract.safety.sanitization.template_content_removed
        || !contract.safety.sanitization.html_comments_removed
        || !contract.safety.sanitization.zero_width_chars_removed
        || !contract.safety.sanitization.script_style_noise_removed
        || !contract.safety.sanitization.raw_payload_quarantined
    {
        return Err(format!("sanitization_contract_incomplete:{tool_id}"));
    }
    if contract.visibility.tool_trace_chat_visible {
        return Err(format!("tool_trace_must_not_be_chat_visible:{tool_id}"));
    }
    if !contract.visibility.final_answer_requires_synthesis {
        return Err(format!("final_answer_synthesis_required:{tool_id}"));
    }
    for lane in [
        "usable",
        "partial",
        "low_signal",
        "irrelevant_or_off_topic",
        "blocked",
        "needs_js",
        "needs_dynamic",
        "absent",
    ] {
        if !contract.quality_lanes.iter().any(|row| row == lane) {
            return Err(format!("missing_quality_lane:{tool_id}:{lane}"));
        }
    }
    if !contract.evidence_packaging.include_content_excerpt
        || !contract.evidence_packaging.include_source_url
        || !contract.evidence_packaging.include_quality_flags
    {
        return Err(format!("evidence_packaging_incomplete:{tool_id}"));
    }
    if normalized_key(&contract.quality_classification.classifier).is_empty()
        || contract.quality_classification.status_fields.is_empty()
        || contract.quality_classification.content_fields.is_empty()
        || contract
            .quality_classification
            .blocked_status_codes
            .is_empty()
        || contract.quality_classification.min_partial_content_chars == 0
        || contract.quality_classification.min_usable_content_chars
            < contract.quality_classification.min_partial_content_chars
        || !contract.quality_classification.emit_on_normalized_result
    {
        return Err(format!("quality_classification_incomplete:{tool_id}"));
    }
    Ok(())
}

fn normalized_key(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_tool_cd_contract_declares_general_retrieval_and_quality_fields() {
        let contract = tool_cd_contract_for("web_search").expect("web_search contract");
        assert_eq!(contract.capability_family, "web_retrieval");
        assert_eq!(contract.retrieval.mode, "search");
        assert!(contract
            .extraction
            .allowed_types
            .contains(&"markdown".to_string()));
        assert!(contract.extraction.selector_hint_allowed);
        assert!(contract.safety.ssrf_guard_required);
        assert!(contract.safety.sanitization.hidden_content_removed);
        assert!(!contract.visibility.raw_payload_chat_visible);
        assert!(contract
            .quality_classification
            .blocked_status_codes
            .contains(&403));
        assert!(contract
            .quality_classification
            .retryable_status_codes
            .contains(&429));
        assert!(contract
            .quality_classification
            .blocked_text_fragments
            .iter()
            .any(|row| row == "captcha"));
        assert!(contract.quality_lanes.contains(&"low_signal".to_string()));
        assert!(contract
            .quality_lanes
            .contains(&"needs_dynamic".to_string()));
        assert_eq!(contract.session_policy.state_scope, "stateless");
        assert_eq!(contract.session_policy.pooling_mode, "none");
        assert_eq!(contract.session_policy.max_parallel_items_default, 1);
    }

    #[test]
    fn web_tool_cd_contract_lookup_normalizes_aliases() {
        let search = tool_cd_contract_for("web_lookup").expect("web_lookup alias");
        let research = tool_cd_contract_for("web_research").expect("web_research alias");
        assert_eq!(search.tool_id, "web_search");
        assert_eq!(research.tool_id, "batch_query");
    }

    #[test]
    fn bundled_web_tool_cd_catalog_validates() {
        let catalog = published_tool_cd_catalog_v1();
        validate_tool_cd_catalog(&catalog).expect("valid Tool CD");
        assert_eq!(catalog.contracts.len(), 3);
        assert!(tool_cd_contract_index_v1().contains_key("web_fetch"));
    }
}
