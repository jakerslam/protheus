
    #[test]
    fn api_fetch_rejects_unknown_explicit_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://example.com",
                "provider": "firecrawl"
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
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("skipped_validation")
        );
        assert_eq!(
            out.get("cache_skip_reason").and_then(Value::as_str),
            Some("unknown_fetch_provider")
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("unknown_fetch_provider")
        );
        assert_eq!(
            out.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry/strategy").and_then(Value::as_str),
            Some("use_supported_provider_or_auto")
        );
        assert_eq!(
            out.pointer("/retry/reason").and_then(Value::as_str),
            Some("unknown_fetch_provider")
        );
        assert_eq!(
            out.pointer("/retry/contract_version").and_then(Value::as_str),
            Some("v1")
        );
        assert!(out
            .get("fetch_provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("provider").and_then(Value::as_str) == Some("direct_http")))
            .unwrap_or(false));
    }

    #[test]
    fn fetch_localhost_ssrf_block_sets_preflight_contract_fields() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(tmp.path(), &json!({"url": "http://localhost"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("blocked_hostname")
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_fetch")
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("skipped_validation")
        );
        assert_eq!(
            out.get("cache_skip_reason").and_then(Value::as_str),
            Some("ssrf_blocked")
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("ssrf_blocked")
        );
        assert!(out.get("fetch_provider_catalog").is_some());
        assert_eq!(
            out.get("fetch_url_shape_route_hint").and_then(Value::as_str),
            Some("web_fetch")
        );
        assert_eq!(
            out.pointer("/retry/strategy").and_then(Value::as_str),
            Some("use_public_http_or_https_target")
        );
        assert_eq!(
            out.pointer("/retry/reason").and_then(Value::as_str),
            Some("ssrf_blocked")
        );
        assert_eq!(
            out.pointer("/retry/contract_version").and_then(Value::as_str),
            Some("v1")
        );
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn fetch_early_validation_blocks_meta_conversational_input() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(tmp.path(), &json!({"url": "that was just a test"}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("non_fetch_meta_query")
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_fetch")
        );
        assert_eq!(
            out.get("meta_query_blocked").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("tool_execution_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("cache_status").and_then(Value::as_str),
            Some("blocked_meta_query")
        );
        assert_eq!(
            out.get("cache_write_attempted").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_execution_gate/reason")
                .and_then(Value::as_str),
            Some("meta_query_blocked")
        );
        assert_eq!(
            out.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/retry/strategy").and_then(Value::as_str),
            Some("answer_directly_without_web_fetch")
        );
        assert_eq!(
            out.pointer("/retry/reason").and_then(Value::as_str),
            Some("non_fetch_meta_query")
        );
        assert_eq!(
            out.pointer("/retry/contract_version").and_then(Value::as_str),
            Some("v1")
        );
        assert_eq!(
            out.pointer("/retry/lane").and_then(Value::as_str),
            Some("web_fetch")
        );
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn fetch_meta_conversational_input_can_be_overridden() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({
                "url": "that was just a test",
                "force_web_fetch": true
            }),
        );
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("invalid_fetch_url")
        );
    }
