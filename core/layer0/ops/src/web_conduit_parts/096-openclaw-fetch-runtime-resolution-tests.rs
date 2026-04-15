#[cfg(test)]
mod openclaw_fetch_runtime_resolution_tests {
    use super::*;
    use crate::web_conduit::api_fetch;

    #[test]
    fn openclaw_fetch_runtime_resolution_contract_is_bundled_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/fetch_provider_registration_contract/runtime_resolution_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            out.pointer("/fetch_provider_registration_contract/runtime_resolution_contract/prefer_runtime_providers")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/fetch/resolution_contract/bundled_provider_precedence")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_fetch_runtime_resolution_flags_invalid_top_level_fetch_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "fetch_provider": "firecrawl"
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/fetch/configured_provider_input")
                .and_then(Value::as_str),
            Some("firecrawl")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/fetch/selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/fetch/selection_fallback_reason")
                .and_then(Value::as_str),
            Some("invalid_configured_provider")
        );
        assert!(out
            .pointer("/runtime_web_tools_metadata/fetch/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("code").and_then(Value::as_str)
                    == Some("WEB_FETCH_PROVIDER_INVALID_AUTODETECT")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_fetch_runtime_resolution_snapshot_prefers_request_hint_scope() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "fetch_provider_order": ["direct_http"]
            }
        });

        let out = crate::web_conduit_provider_runtime::fetch_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({"provider": "curl"}),
            "curl",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_hint")
        );
        assert_eq!(
            out.pointer("/requested_provider_hint").and_then(Value::as_str),
            Some("curl")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            out.pointer("/resolution_contract/prefer_runtime_providers")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/fallback_runtime_resolver")
                .and_then(Value::as_str),
            Some("resolvePluginWebFetchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_runtime_resolver")
                .and_then(Value::as_str),
            Some("resolveBundledWebFetchProvidersFromPublicArtifacts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/manifest_contract_owner_resolver")
                .and_then(Value::as_str),
            Some("resolveManifestContractOwnerPluginId")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/runtime_registry_resolver")
                .and_then(Value::as_str),
            Some("resolveRuntimeWebFetchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/candidate_plugin_contract/contract")
                .and_then(Value::as_str),
            Some("webFetchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_sort_contract/auto_detect_sorter")
                .and_then(Value::as_str),
            Some("sortPluginProvidersForAutoDetect")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/provider_context_type")
                .and_then(Value::as_str),
            Some("WebFetchProviderContext")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/runtime_metadata_context_type")
                .and_then(Value::as_str),
            Some("WebFetchRuntimeMetadataContext")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/provider_entry_type")
                .and_then(Value::as_str),
            Some("PluginWebFetchProviderEntry")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/credential_presence_contract/resolver")
                .and_then(Value::as_str),
            Some("hasConfiguredWebFetchCredential")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_contract_suite_contract/registry_plugin_filter")
                .and_then(Value::as_str),
            Some("entry.webFetchProviderIds.length > 0")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_contract_suite_contract/base_provider_contract/provider_id_regex")
                .and_then(Value::as_str),
            Some("^[a-z0-9][a-z0-9-]*$")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_contract_suite_contract/credential_roundtrip_contract/configured_roundtrip_optional")
                .and_then(Value::as_str),
            Some("provider.setConfiguredCredentialValue/getConfiguredCredentialValue")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_contract_suite_contract/tool_definition_contract/execute_function_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_discovery_runtime_contract/runtime_module")
                .and_then(Value::as_str),
            Some("src/plugins/provider-discovery.runtime.ts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_discovery_runtime_contract/entry_fast_path_resolver")
                .and_then(Value::as_str),
            Some("resolveProviderDiscoveryEntryPlugins")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_discovery_contract_suite_contract/helper_module")
                .and_then(Value::as_str),
            Some("test/helpers/plugins/provider-discovery-contract.ts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_discovery_contract_suite_contract/contract_targets/0")
                .and_then(Value::as_str),
            Some("cloudflare-ai-gateway")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_helper_contract/helper_module")
                .and_then(Value::as_str),
            Some("test/helpers/plugins/web-fetch-provider-contract.ts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_helper_contract/contract_suite_installer")
                .and_then(Value::as_str),
            Some("installWebFetchProviderContractSuite")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/runtime_web_channel_plugin_contract/runtime_module")
                .and_then(Value::as_str),
            Some("src/plugins/runtime/runtime-web-channel-plugin.ts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/runtime_web_channel_plugin_contract/entry_base_names/1")
                .and_then(Value::as_str),
            Some("runtime-api")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_runtime_core_contract/core_runtime_contract_targets/0")
                .and_then(Value::as_str),
            Some("src/plugins/contracts/tts.provider-runtime.contract.test.ts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_runtime_core_contract/capability_runtime_entrypoints/1")
                .and_then(Value::as_str),
            Some("resolveMemoryEmbeddingProviderRuntime")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/sanitizer_entrypoint")
                .and_then(Value::as_str),
            Some("sanitizeHtml")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/strip_invisible_unicode_entrypoint")
                .and_then(Value::as_str),
            Some("stripInvisibleUnicode")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/always_remove_tags/3")
                .and_then(Value::as_str),
            Some("canvas")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/hidden_class_name_contract/0")
                .and_then(Value::as_str),
            Some("sr-only")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/comment_stripping")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/hidden_class_token_boundary_match_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/visibility_sanitization_contract/invisible_unicode_codepoint_contract/0")
                .and_then(Value::as_str),
            Some("U+200B")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/shared_runtime_contract/timeout_default_seconds")
                .and_then(Value::as_u64),
            Some(30)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/shared_runtime_contract/cache_ttl_default_minutes")
                .and_then(Value::as_u64),
            Some(15)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/shared_runtime_contract/cache_write_helper")
                .and_then(Value::as_str),
            Some("writeCache")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/html_to_markdown_entrypoint")
                .and_then(Value::as_str),
            Some("htmlToMarkdown")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/readable_extraction_entrypoint")
                .and_then(Value::as_str),
            Some("extractReadableContent")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/readability_html_char_guard")
                .and_then(Value::as_u64),
            Some(1_000_000)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/max_chars_enforced_after_wrapping")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/extract_readable_mode_parity/1")
                .and_then(Value::as_str),
            Some("markdown")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/content_extraction_contract/invisible_unicode_stripping_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_fallback_contract/provider_fallback_payload_rewrap_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_fallback_contract/safe_final_url_contract")
                .and_then(Value::as_str),
            Some("unsafe_provider_final_url_replaced_with_requested_url")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/ssrf_guard_contract/redirect_target_revalidation_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/ssrf_guard_contract/rfc2544_benchmark_range_opt_in_flag")
                .and_then(Value::as_str),
            Some("ssrfPolicy.allowRfc2544BenchmarkRange")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/response_and_wrapping_contract/external_content_wrapper_marker_regex")
                .and_then(Value::as_str),
            Some("<<<EXTERNAL_UNTRUSTED_CONTENT id=\\\"[a-f0-9]{16}\\\">>>")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/response_and_wrapping_contract/response_bytes_cap_enforced")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/cf_markdown_contract/accept_header_preference")
                .and_then(Value::as_str),
            Some("text/markdown, text/html;q=0.9, */*;q=0.1")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/cf_markdown_contract/markdown_extractor_id")
                .and_then(Value::as_str),
            Some("cf-markdown")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/cf_markdown_contract/markdown_tokens_logging_requires_url_redaction")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/guarded_endpoint_contract/strict_endpoint_wrapper")
                .and_then(Value::as_str),
            Some("withStrictWebToolsEndpoint")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/guarded_endpoint_contract/trusted_policy/allow_rfc2544_benchmark_range")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/snapshot_cache_contract/cache_key_builder")
                .and_then(Value::as_str),
            Some("buildWebProviderSnapshotCacheKey")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/snapshot_cache_contract/in_flight_registry_load_guard")
                .and_then(Value::as_str),
            Some("does_not_force_fresh_snapshot_load")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/snapshot_cache_contract/active_registry_workspace_inheritance")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/snapshot_cache_contract/workspace_change_invalidation")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/snapshot_cache_contract/cache_key_dimensions/2")
                .and_then(Value::as_str),
            Some("workspace_dir")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_sort_contract/shared_sort_entrypoint")
                .and_then(Value::as_str),
            Some("sortWebFetchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_resolution_contract/bundled_resolution_config_resolver")
                .and_then(Value::as_str),
            Some("resolveBundledWebFetchResolutionConfig")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_resolution_contract/fast_path_skips_manifest_scans_when_only_plugin_ids")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_resolution_contract/requires_public_artifact_for_each_bundled_manifest_contract_provider")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(out
            .pointer("/openclaw_runtime_contract/diagnostic_code_contract")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.as_str() == Some("WEB_FETCH_PROVIDER_KEY_UNRESOLVED_NO_FALLBACK")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_fetch_runtime_resolution_prefers_runtime_metadata_provider_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "fetch_provider_order": ["direct_http"]
            }
        });

        let out = crate::web_conduit_provider_runtime::fetch_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "runtimeWebFetch": {
                    "selectedProvider": "curl"
                }
            }),
            "auto",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("runtime_metadata")
        );
        assert_eq!(
            out.pointer("/runtime_selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            out.pointer("/runtime_provider_preferred")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn openclaw_fetch_runtime_resolution_accepts_camel_case_provider_chain_alias() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "fetch_provider_order": ["direct_http"]
            }
        });

        let out = crate::web_conduit_provider_runtime::fetch_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "fetchProviderChain": ["curl"]
            }),
            "auto",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_chain")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn api_fetch_accepts_camel_case_fetch_provider_hint_alias() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://example.com",
                "fetchProvider": "firecrawl"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unknown_fetch_provider")
        );
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("firecrawl")
        );
    }
}
