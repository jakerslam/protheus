#[cfg(test)]
mod openclaw_runtime_provider_proof_tests {
    use super::*;

    fn search_provider_row<'a>(out: &'a Value, provider: &str) -> &'a Value {
        out.get("search_providers")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| row.get("provider").and_then(Value::as_str) == Some(provider))
            })
            .expect("search provider row")
    }

    #[test]
    fn openclaw_runtime_contract_search_runtime_prefers_keyless_fallback_without_credentials() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "search_provider_order": ["serperdev", "duckduckgo", "bing_rss"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/default_search_provider_chain/0")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            out.pointer("/runtime_web_tools_metadata/search/selected_provider")
                .and_then(Value::as_str),
            Some("duckduckgo")
        );
        assert_eq!(
            search_provider_row(&out, "serperdev")
                .get("credential_present")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn openclaw_runtime_contract_fetch_runtime_falls_back_to_direct_http_for_invalid_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json_atomic(
            &policy_path(tmp.path()),
            &json!({
                "web_conduit": {
                    "enabled": true,
                    "fetch_provider_order": ["firecrawl"]
                }
            }),
        )
        .expect("write policy");

        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/default_fetch_provider_chain/0")
                .and_then(Value::as_str),
            Some("direct_http")
        );
        assert_eq!(
            out.pointer("/fetch_providers/0/provider")
                .and_then(Value::as_str),
            Some("direct_http")
        );
    }

    #[test]
    fn openclaw_runtime_contract_provider_web_search_registration_contract_is_built_in_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(
            out.pointer("/search_provider_registration_contract/supported_provider_ids/0")
                .and_then(Value::as_str),
            Some("serperdev")
        );
        assert_eq!(
            out.pointer("/search_provider_registration_contract/unsupported_provider_examples/0")
                .and_then(Value::as_str),
            Some("brave")
        );
        assert_eq!(
            out.pointer("/search_provider_registration_contract/unsupported_provider_examples/7")
                .and_then(Value::as_str),
            Some("xai")
        );
        assert_eq!(
            out.pointer("/search_provider_registration_contract/public_artifact_contract/runtime_mode")
                .and_then(Value::as_str),
            Some("built_in_only")
        );
    }

    #[test]
    fn openclaw_runtime_contract_brave_search_contract_fails_closed_outside_allowlist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let providers = api_providers(tmp.path());
        assert!(!providers
            .pointer("/search_provider_registration_contract/supported_provider_ids")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("brave")))
            .unwrap_or(false));
        assert!(providers
            .pointer("/search_provider_registration_contract/unsupported_provider_examples")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("brave")))
            .unwrap_or(false));
        let out = api_search(tmp.path(), &json!({"query": "agent reliability", "provider": "brave"}));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("unknown_search_provider"));
    }

    #[test]
    fn openclaw_runtime_contract_duckduckgo_search_contract_is_keyless_and_allowlisted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        let row = search_provider_row(&out, "duckduckgo");
        assert_eq!(
            row.get("requires_credential").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            row.pointer("/contract_fields/credential_contract/type")
                .and_then(Value::as_str),
            Some("none")
        );
        assert!(row
            .get("aliases")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|alias| alias.as_str() == Some("ddg")))
            .unwrap_or(false));
        assert!(out
            .pointer("/search_provider_registration_contract/supported_provider_ids")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.as_str() == Some("duckduckgo")))
            .unwrap_or(false));
    }

    #[test]
    fn openclaw_runtime_contract_exa_search_contract_fails_closed_outside_allowlist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(tmp.path(), &json!({"query": "agent reliability", "provider": "exa"}));
        assert_eq!(out.get("error").and_then(Value::as_str), Some("unknown_search_provider"));
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("exa")
        );
    }

    #[test]
    fn openclaw_runtime_contract_firecrawl_search_contract_fails_closed_outside_allowlist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "agent reliability", "provider": "firecrawl"}),
        );
        assert_eq!(out.get("error").and_then(Value::as_str), Some("unknown_search_provider"));
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("firecrawl")
        );
    }

    #[test]
    fn openclaw_runtime_contract_google_search_contract_fails_closed_outside_allowlist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "agent reliability", "provider": "google"}),
        );
        assert_eq!(out.get("error").and_then(Value::as_str), Some("unknown_search_provider"));
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("google")
        );
    }
}
