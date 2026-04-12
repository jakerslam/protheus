#[cfg(test)]
mod openclaw_fetch_helper_tests {
    use super::*;

    #[test]
    fn html_to_markdown_document_preserves_links_and_title() {
        let (markdown, title) = html_to_markdown_document(
            r#"<html><head><title>Example Title</title></head><body><h1>Hello</h1><p>Read <a href="https://example.com/docs">the docs</a>.</p></body></html>"#,
        );
        assert_eq!(title.as_deref(), Some("Example Title"));
        assert!(markdown.contains("# Hello"));
        assert!(markdown.contains("[the docs](https://example.com/docs)"));
    }

    #[test]
    fn normalize_search_result_link_decodes_duckduckgo_redirects() {
        let normalized = normalize_search_result_link(
            "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fguide",
        );
        assert_eq!(normalized, "https://example.com/guide");
    }

    #[test]
    fn api_fetch_returns_cached_markdown_payload_for_redirect_url() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let raw_url = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fguide";
        let resolved_url = "https://example.com/guide";
        let key = crate::web_conduit_provider_runtime::fetch_cache_key(
            raw_url,
            resolved_url,
            "markdown",
            4096,
            false,
            &vec!["direct_http".to_string()],
        );
        let cached = json!({
            "ok": true,
            "type": "web_conduit_fetch",
            "requested_url": raw_url,
            "resolved_url": resolved_url,
            "provider": "direct_http",
            "provider_chain": ["direct_http"],
            "extract_mode": "markdown",
            "summary": "Example guide.",
            "content": "# Example guide\n\n[Read more](https://example.com/guide)",
            "cache_status": "miss"
        });
        crate::web_conduit_provider_runtime::store_fetch_cache(tmp.path(), &key, &cached, "ok", 15);
        let out = api_fetch(
            tmp.path(),
            &json!({
                "url": raw_url,
                "extract_mode": "markdown",
                "max_chars": 4096,
                "summary_only": false
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
        assert_eq!(
            out.get("resolved_url").and_then(Value::as_str),
            Some(resolved_url)
        );
        assert_eq!(
            out.get("extract_mode").and_then(Value::as_str),
            Some("markdown")
        );
        assert!(out
            .get("content")
            .and_then(Value::as_str)
            .map(|text| text.contains("[Read more](https://example.com/guide)"))
            .unwrap_or(false));
    }
}
