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
    #[serde(default)]
    pub optional_args: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    pub retrieval: ToolRetrievalContract,
    pub execution_policy: ToolExecutionPolicyContract,
    pub extraction: ToolExtractionContract,
    pub readiness: ToolReadinessContract,
    pub resource_policy: ToolResourcePolicyContract,
    pub session_policy: ToolSessionPolicyContract,
    pub lifecycle: ToolLifecycleContract,
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
pub struct ToolExecutionPolicyContract {
    pub request_fingerprint_dedupe: bool,
    pub fingerprint_identity_fields: Vec<String>,
    pub fingerprint_include_request_options_default: bool,
    pub fingerprint_include_headers_default: bool,
    pub fingerprint_keep_url_fragments_default: bool,
    pub per_domain_concurrency_default: usize,
    pub request_delay_ms_default: u64,
    pub blocked_response_retry_allowed: bool,
    pub max_blocked_retries_default: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolExtractionContract {
    pub default_type: String,
    pub allowed_types: Vec<String>,
    pub selector_hint_allowed: bool,
    pub selector_hint_fallback_mode: String,
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
    pub blocked_domains_source: String,
    #[serde(default)]
    pub ad_block_profile_default: Option<String>,
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
pub struct ToolLifecycleContract {
    pub implicit_session_on_invoke: bool,
    pub explicit_close_supported: bool,
    pub explicit_list_supported: bool,
    #[serde(default)]
    pub session_handle_arg: Option<String>,
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
    #[serde(default)]
    pub include_artifact_refs: bool,
    #[serde(default)]
    pub allowed_artifact_kinds: Vec<String>,
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
    pub proxy_error_fragments: Vec<String>,
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
    if contract.operations.is_empty() {
        return Err(format!("operations_required:{tool_id}"));
    }
    let mut seen_arg_names = BTreeSet::<String>::new();
    for arg in &contract.required_args {
        let normalized = normalized_key(arg);
        if normalized.is_empty() {
            return Err(format!("required_arg_name_required:{tool_id}"));
        }
        if !seen_arg_names.insert(normalized) {
            return Err(format!("duplicate_arg_name:{tool_id}"));
        }
    }
    for arg in &contract.optional_args {
        let normalized = normalized_key(arg);
        if normalized.is_empty() {
            return Err(format!("optional_arg_name_required:{tool_id}"));
        }
        if !seen_arg_names.insert(normalized) {
            return Err(format!("duplicate_arg_name:{tool_id}"));
        }
    }
    for operation in &contract.operations {
        if normalized_key(operation).is_empty() {
            return Err(format!("operation_name_required:{tool_id}"));
        }
    }
    if normalized_key(&contract.retrieval.mode).is_empty() {
        return Err(format!("retrieval_mode_required:{tool_id}"));
    }
    if contract.retrieval.max_bulk_items == 0 {
        return Err(format!("max_bulk_items_required:{tool_id}"));
    }
    if contract.execution_policy.request_fingerprint_dedupe
        && contract
            .execution_policy
            .fingerprint_identity_fields
            .iter()
            .all(|field| normalized_key(field).is_empty())
    {
        return Err(format!("fingerprint_identity_fields_required:{tool_id}"));
    }
    if contract.execution_policy.blocked_response_retry_allowed
        && contract.execution_policy.max_blocked_retries_default == 0
    {
        return Err(format!("blocked_retry_budget_required:{tool_id}"));
    }
    if !contract.execution_policy.blocked_response_retry_allowed
        && contract.execution_policy.max_blocked_retries_default != 0
    {
        return Err(format!("blocked_retry_budget_must_be_zero:{tool_id}"));
    }
    if contract.extraction.allowed_types.is_empty()
        || !contract
            .extraction
            .allowed_types
            .contains(&contract.extraction.default_type)
    {
        return Err(format!("extraction_default_must_be_allowed:{tool_id}"));
    }
    if contract.extraction.selector_hint_allowed
        && normalized_key(&contract.extraction.selector_hint_fallback_mode).is_empty()
    {
        return Err(format!("selector_hint_fallback_required:{tool_id}"));
    }
    if contract.readiness.default_timeout_ms == 0 {
        return Err(format!("readiness_timeout_required:{tool_id}"));
    }
    if normalized_key(&contract.session_policy.state_scope).is_empty() {
        return Err(format!("session_state_scope_required:{tool_id}"));
    }
    if normalized_key(&contract.resource_policy.blocked_domains_source).is_empty() {
        return Err(format!("blocked_domains_source_required:{tool_id}"));
    }
    if contract.resource_policy.block_ads_allowed
        && contract
            .resource_policy
            .ad_block_profile_default
            .as_deref()
            .map(normalized_key)
            .unwrap_or_default()
            .is_empty()
    {
        return Err(format!("ad_block_profile_required:{tool_id}"));
    }
    if normalized_key(&contract.session_policy.pooling_mode).is_empty() {
        return Err(format!("session_pooling_mode_required:{tool_id}"));
    }
    if contract.session_policy.max_parallel_items_default == 0 {
        return Err(format!("session_parallel_items_required:{tool_id}"));
    }
    let has_session_handle_arg = contract
        .lifecycle
        .session_handle_arg
        .as_ref()
        .map(|value| !normalized_key(value).is_empty())
        .unwrap_or(false);
    if contract.session_policy.reuse_allowed && !has_session_handle_arg {
        return Err(format!("session_handle_arg_required:{tool_id}"));
    }
    if (contract.lifecycle.explicit_close_supported || contract.lifecycle.explicit_list_supported)
        && !has_session_handle_arg
    {
        return Err(format!("session_handle_arg_required:{tool_id}"));
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
    if contract.evidence_packaging.include_artifact_refs
        && contract
            .evidence_packaging
            .allowed_artifact_kinds
            .is_empty()
    {
        return Err(format!("artifact_kinds_required:{tool_id}"));
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
        assert_eq!(
            contract.extraction.selector_hint_fallback_mode,
            "whole_page"
        );
        assert!(contract.safety.ssrf_guard_required);
        assert!(contract.safety.sanitization.hidden_content_removed);
        assert!(!contract.visibility.raw_payload_chat_visible);
        assert!(contract.execution_policy.request_fingerprint_dedupe);
        assert_eq!(
            contract.execution_policy.fingerprint_identity_fields,
            vec![
                "session_scope".to_string(),
                "http_method".to_string(),
                "request_body".to_string(),
                "canonical_url".to_string()
            ]
        );
        assert!(
            !contract
                .execution_policy
                .fingerprint_include_request_options_default
        );
        assert!(
            !contract
                .execution_policy
                .fingerprint_include_headers_default
        );
        assert!(
            !contract
                .execution_policy
                .fingerprint_keep_url_fragments_default
        );
        assert_eq!(contract.operations, vec!["search".to_string()]);
        assert!(contract.optional_args.contains(&"aperture".to_string()));
        assert!(contract.optional_args.contains(&"source_scope".to_string()));
        assert_eq!(contract.resource_policy.blocked_domains_source, "none");
        assert!(contract
            .quality_classification
            .blocked_status_codes
            .contains(&403));
        assert!(contract
            .quality_classification
            .proxy_error_fragments
            .iter()
            .any(|row| row == "net::err_proxy"));
        assert!(contract
            .quality_classification
            .retryable_status_codes
            .contains(&429));
        assert!(contract
            .quality_classification
            .blocked_text_fragments
            .iter()
            .any(|row| row == "captcha"));
        assert!(!contract.evidence_packaging.include_artifact_refs);
        assert!(contract.quality_lanes.contains(&"low_signal".to_string()));
        assert!(contract
            .quality_lanes
            .contains(&"needs_dynamic".to_string()));
        assert_eq!(contract.session_policy.state_scope, "stateless");
        assert_eq!(contract.session_policy.pooling_mode, "none");
        assert_eq!(contract.session_policy.max_parallel_items_default, 1);
        assert!(!contract.execution_policy.blocked_response_retry_allowed);
        assert!(!contract.lifecycle.implicit_session_on_invoke);
        assert_eq!(contract.lifecycle.session_handle_arg, None);
        let fetch = tool_cd_contract_for("web_fetch").expect("web_fetch contract");
        assert!(fetch.evidence_packaging.include_artifact_refs);
        assert!(fetch
            .evidence_packaging
            .allowed_artifact_kinds
            .contains(&"screenshot".to_string()));
        assert_eq!(fetch.operations, vec!["fetch".to_string()]);
        assert!(fetch.optional_args.contains(&"selector_hint".to_string()));
        assert!(fetch.optional_args.contains(&"source_scope".to_string()));
        assert!(fetch.execution_policy.blocked_response_retry_allowed);
        assert_eq!(fetch.execution_policy.max_blocked_retries_default, 2);
        assert_eq!(
            fetch.execution_policy.fingerprint_identity_fields,
            vec![
                "session_scope".to_string(),
                "http_method".to_string(),
                "request_body".to_string(),
                "canonical_url".to_string()
            ]
        );
        assert!(
            !fetch
                .execution_policy
                .fingerprint_include_request_options_default
        );
        assert!(!fetch.execution_policy.fingerprint_include_headers_default);
        assert!(
            !fetch
                .execution_policy
                .fingerprint_keep_url_fragments_default
        );
        assert!(fetch.lifecycle.implicit_session_on_invoke);
        assert_eq!(
            fetch.resource_policy.blocked_domains_source,
            "profile_or_custom"
        );
        assert_eq!(
            fetch.resource_policy.ad_block_profile_default.as_deref(),
            Some("built_in_ad_domains")
        );
        assert_eq!(
            fetch.lifecycle.session_handle_arg.as_deref(),
            Some("session_id")
        );
        let batch = tool_cd_contract_for("batch_query").expect("batch_query contract");
        assert_eq!(batch.operations, vec!["search_pack".to_string()]);
        assert_eq!(batch.execution_policy.per_domain_concurrency_default, 2);
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
