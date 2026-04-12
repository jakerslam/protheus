#[cfg(test)]
mod openclaw_search_tool_tests {
    use super::*;

    #[test]
    fn api_search_rejects_unsupported_search_filters() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({
                "query": "agent reliability benchmarks",
                "country": "us"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unsupported_country")
        );
        assert_eq!(
            out.get("unsupported_filter").and_then(Value::as_str),
            Some("country")
        );
        assert!(out.get("provider_catalog").is_some());
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn api_search_count_alias_hits_cached_response() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let request = json!({
            "query": "agent reliability benchmark",
            "count": 4,
            "summary_only": true,
            "cache_ttl_minutes": 9
        });
        let query = clean_text(
            request
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            600,
        );
        let (policy, _) = load_policy(tmp.path());
        let allowed_domains =
            normalize_allowed_domains(request.get("allowed_domains").unwrap_or(&Value::Null));
        let exclude_subdomains = false;
        let scoped_query = scoped_search_query(&query, &allowed_domains, exclude_subdomains);
        let provider_chain = crate::web_conduit_provider_runtime::provider_chain_from_request(
            "auto", &request, &policy,
        );
        let top_k = crate::web_conduit_provider_runtime::resolve_search_count(&request, &policy);
        let key = crate::web_conduit_provider_runtime::search_cache_key(
            &query,
            &scoped_query,
            &allowed_domains,
            exclude_subdomains,
            top_k,
            true,
            &provider_chain,
        );
        crate::web_conduit_provider_runtime::store_search_cache(
            tmp.path(),
            &key,
            &json!({
                "ok": true,
                "summary": "cached search summary",
                "content": "",
                "provider": "duckduckgo"
            }),
            "ok",
            Some(9 * 60),
        );

        let out = api_search(tmp.path(), &request);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
        assert_eq!(out.get("top_k").and_then(Value::as_u64), Some(4));
        assert_eq!(out.get("count").and_then(Value::as_u64), Some(4));
        assert_eq!(
            out.get("cache_ttl_minutes").and_then(Value::as_u64),
            Some(9)
        );
    }

    #[test]
    fn api_providers_reports_tool_catalog_and_search_request_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/search_request_contract/default_count")
                .and_then(Value::as_u64),
            Some(8)
        );
        let tools = out
            .get("tool_catalog")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(tools
            .iter()
            .any(|row| row.get("tool").and_then(Value::as_str) == Some("web_search")));
        assert!(tools
            .iter()
            .any(|row| row.get("tool").and_then(Value::as_str) == Some("web_fetch")));
    }
}
