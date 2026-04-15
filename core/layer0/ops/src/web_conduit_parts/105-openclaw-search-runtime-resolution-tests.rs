#[cfg(test)]
mod openclaw_search_runtime_resolution_tests {
    use super::*;

    #[test]
    fn openclaw_search_runtime_resolution_flags_invalid_top_level_search_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "firecrawl",
                    "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/configured_provider_input")
                .and_then(Value::as_str),
            Some("firecrawl")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selection_fallback_reason")
                .and_then(Value::as_str),
            Some("invalid_configured_provider")
        );
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert!(out
            .pointer("/runtime_web_tools_metadata/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| {
                row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_PROVIDER_INVALID_AUTODETECT")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_search_runtime_resolution_prefers_valid_top_level_provider_in_default_chain() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "bing_rss",
                    "search_provider_order": ["duckduckgo", "duckduckgo_lite", "bing_rss"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/provider_source")
                .and_then(Value::as_str),
            Some("configured")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/tool_catalog/0/default_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_snapshot_prefers_request_hint_scope() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({"provider": "ddg"}),
            "ddg",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_hint")
        );
        assert_eq!(
            out.pointer("/requested_provider_hint")
                .and_then(Value::as_str),
            Some("ddg")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            out.pointer("/allow_fallback").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/resolution_contract/prefer_runtime_providers")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/fallback_runtime_resolver")
                .and_then(Value::as_str),
            Some("resolvePluginWebSearchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_runtime_resolver")
                .and_then(Value::as_str),
            Some("resolveBundledWebSearchProvidersFromPublicArtifacts")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/manifest_contract_owner_resolver")
                .and_then(Value::as_str),
            Some("resolveManifestContractOwnerPluginId")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/runtime_registry_resolver")
                .and_then(Value::as_str),
            Some("resolveRuntimeWebSearchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/candidate_plugin_contract/contract")
                .and_then(Value::as_str),
            Some("webSearchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_sort_contract/auto_detect_sorter")
                .and_then(Value::as_str),
            Some("sortPluginProvidersForAutoDetect")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/provider_context_type")
                .and_then(Value::as_str),
            Some("WebSearchProviderContext")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/runtime_metadata_context_type")
                .and_then(Value::as_str),
            Some("WebSearchRuntimeMetadataContext")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_type_contract/provider_entry_type")
                .and_then(Value::as_str),
            Some("PluginWebSearchProviderEntry")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/credential_presence_contract/resolver")
                .and_then(Value::as_str),
            Some("hasConfiguredWebSearchCredential")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_config_contract/forced_provider_wrapper")
                .and_then(Value::as_str),
            Some("withForcedProvider")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_config_contract/scoped_credential_accessors/0")
                .and_then(Value::as_str),
            Some("getScopedCredentialValue")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_credential_resolution_contract/resolver")
                .and_then(Value::as_str),
            Some("resolveWebSearchProviderCredential")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_credential_resolution_contract/resolution_order/1")
                .and_then(Value::as_str),
            Some("config_secret_ref_env_value")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_common_runtime_contract/default_search_count")
                .and_then(Value::as_u64),
            Some(5)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_common_runtime_contract/max_search_count")
                .and_then(Value::as_u64),
            Some(10)
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/provider_common_runtime_contract/trusted_json_post_wrapper")
                .and_then(Value::as_str),
            Some("postTrustedWebToolsJson")
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
            Some("sortWebSearchProviders")
        );
        assert_eq!(
            out.pointer("/openclaw_runtime_contract/public_artifact_resolution_contract/bundled_resolution_config_resolver")
                .and_then(Value::as_str),
            Some("resolveBundledWebSearchResolutionConfig")
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
                row.as_str() == Some("WEB_SEARCH_KEY_UNRESOLVED_NO_FALLBACK")
            }))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_search_runtime_resolution_prefers_runtime_metadata_provider_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "runtimeWebSearch": {
                    "selectedProvider": "bing_rss"
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
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/runtime_provider_preferred")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/allow_fallback").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn openclaw_search_runtime_resolution_accepts_camel_case_provider_chain_alias() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "enabled": true,
                "search_provider_order": ["duckduckgo", "bing_rss"]
            }
        });

        let out = crate::web_conduit_provider_runtime::search_provider_resolution_snapshot(
            tmp.path(),
            &policy,
            &json!({
                "providerChain": ["duckduckgo_lite", "bing_rss"]
            }),
            "auto",
        );
        assert_eq!(
            out.pointer("/selection_scope").and_then(Value::as_str),
            Some("request_provider_chain")
        );
        assert_eq!(
            out.pointer("/provider_chain/0").and_then(Value::as_str),
            Some("duckduckgo_lite")
        );
    }

    #[test]
    fn api_search_does_not_fallback_when_policy_provider_is_explicit_and_circuit_open() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider": "bing_rss",
                    "search_provider_order": ["duckduckgo", "duckduckgo_lite", "bing_rss"],
                    "provider_circuit_breaker": {
                        "enabled": true,
                        "failure_threshold": 1,
                        "open_for_secs": 60
                    }
                }
            }),
        )
        .expect("write policy");
        let (policy, _) = load_policy(tmp.path());
        record_provider_attempt(tmp.path(), "bing_rss", false, "timeout", &policy);

        let out = api_search(tmp.path(), &json!({"query": "agent reliability"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("provider_circuit_open")
        );
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/provider_resolution/selection_scope")
                .and_then(Value::as_str),
            Some("policy_configured")
        );
        assert_eq!(
            out.pointer("/provider_resolution/allow_fallback")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/provider_resolution/selected_provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/providers_skipped/0/provider")
                .and_then(Value::as_str),
            Some("bing_rss")
        );
        assert_eq!(
            out.pointer("/providers_skipped/0/reason")
                .and_then(Value::as_str),
            Some("circuit_open")
        );
        assert!(out
            .get("providers_attempted")
            .and_then(Value::as_array)
            .map(|rows| rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn empty_duckduckgo_metadata_shell_is_treated_as_low_signal_search_payload() {
        let payload = json!({
            "ok": true,
            "summary": "Key findings: {\"Abstract\":\"\",\"AbstractSource\":\"\",\"AbstractText\":\"\",\"AbstractURL\":\"\",\"Answer\":\"\",\"AnswerType\":\"\",\"Definition\":\"\",\"DefinitionSource\":\"\",\"DefinitionURL\":\"\",\"Entity\":\"\",\"Heading\":\"\",\"RelatedTopics\":[],\"Results\":[],\"Type\":\"\",\"url\":\"https://duck.",
            "content": "{\"Abstract\":\"\",\"AbstractSource\":\"\",\"AbstractText\":\"\",\"AbstractURL\":\"\",\"Answer\":\"\",\"AnswerType\":\"\",\"Definition\":\"\",\"DefinitionSource\":\"\",\"DefinitionURL\":\"\",\"Entity\":\"\",\"Heading\":\"\",\"RelatedTopics\":[],\"Results\":[],\"Type\":\"\"}"
        });
        assert!(payload_looks_low_signal_search(&payload));
        assert!(!search_payload_usable(&payload));
        assert_eq!(search_payload_error(&payload), "low_signal_search_payload");
    }
}
