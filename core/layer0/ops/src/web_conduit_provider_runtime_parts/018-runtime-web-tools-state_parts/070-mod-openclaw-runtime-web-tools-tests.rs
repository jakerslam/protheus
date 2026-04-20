
#[cfg(test)]
mod openclaw_runtime_web_tools_tests {
    use super::*;

    #[test]
    fn runtime_web_tools_snapshot_persists_active_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/fetch/selected_provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_load_is_defensive_clone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _snapshot = runtime_web_tools_snapshot(tmp.path(), &policy);
        let mut loaded = load_active_runtime_web_tools_metadata(tmp.path());
        if let Some(search) = loaded.pointer_mut("/search").and_then(Value::as_object_mut) {
            search.insert("selected_provider".to_string(), json!("brave"));
            search.insert("provider_configured".to_string(), json!("brave"));
        }
        let reloaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            reloaded
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            reloaded
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_exposes_openclaw_contract_markers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo", "bing_rss"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/resolution_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            metadata
                .pointer("/fetch/resolution_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/module_entrypoint")
                .and_then(Value::as_str),
            Some("src/agents/tools/web-tools.ts")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/exports/0")
                .and_then(Value::as_str),
            Some("createWebFetchTool")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/exports/1")
                .and_then(Value::as_str),
            Some("extractReadableContent")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/exports/readability_helper")
                .and_then(Value::as_str),
            Some("extractReadableContent")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/web_fetch_enabled_by_default_non_sandbox")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/web_search_runtime_provider_override_supported")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/runtime_web_search_metadata_fields/2")
                .and_then(Value::as_str),
            Some("selectedProvider")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/default_enablement/runtime_metadata_provider_override_field")
                .and_then(Value::as_str),
            Some("runtimeWebSearch.selectedProvider")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/headers_factory_entrypoint")
                .and_then(Value::as_str),
            Some("makeFetchHeaders")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/headers_lookup_contract")
                .and_then(Value::as_str),
            Some("map[normalizeLowercaseStringOrEmpty(key)] ?? null")
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/base_test_defaults/cache_ttl_minutes")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/base_test_optional_overrides/max_response_bytes_added_only_when_truthy")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/openclaw_web_tools_contract/fetch_unit_test_harness/readability_test_mock_entrypoint")
                .and_then(Value::as_str),
            Some("web-fetch.test-mocks.ts")
        );
        assert!(metadata
            .pointer("/diagnostics")
            .and_then(Value::as_array)
            .is_some());
    }

    #[test]
    fn runtime_web_tools_snapshot_flags_invalid_search_provider_tokens() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["perplexity", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("auto-detect")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_PROVIDER_INVALID_AUTODETECT")))
            .unwrap_or(false));
    }

    #[test]
    fn clear_active_runtime_web_tools_metadata_removes_persisted_snapshot() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let _metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert!(runtime_web_tools_state_path(tmp.path()).exists());
        clear_active_runtime_web_tools_metadata(tmp.path());
        assert!(!runtime_web_tools_state_path(tmp.path()).exists());
        let loaded = load_active_runtime_web_tools_metadata(tmp.path());
        assert_eq!(
            loaded
                .pointer("/search/provider_source")
                .and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn runtime_web_tools_snapshot_reports_missing_key_fallback() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = json!({
            "web_conduit": {
                "search_provider_order": ["serperdev", "duckduckgo"],
                "fetch_provider_order": ["direct_http"]
            }
        });
        let metadata = runtime_web_tools_snapshot(tmp.path(), &policy);
        assert_eq!(
            metadata
                .pointer("/search/provider_configured")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            metadata
                .pointer("/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert!(metadata
            .pointer("/search/diagnostics")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.get("code").and_then(Value::as_str)
                    == Some("WEB_SEARCH_KEY_UNRESOLVED_FALLBACK_USED")))
            .unwrap_or(false));
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/status")
                .and_then(Value::as_str),
            Some("degraded")
        );
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/blocking_reason")
                .and_then(Value::as_str),
            Some("configured_provider_credential_unresolved")
        );
        assert_eq!(
            metadata
                .pointer("/search/tool_surface_health/selected_provider_ready")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .pointer("/tool_surface_health/search_status")
                .and_then(Value::as_str),
            Some("degraded")
        );
    }
}
