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
