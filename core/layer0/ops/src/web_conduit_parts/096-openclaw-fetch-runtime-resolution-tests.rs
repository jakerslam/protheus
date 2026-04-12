#[cfg(test)]
mod openclaw_fetch_runtime_resolution_tests {
    use super::*;

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
    }
}
