#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bootstraps_default_policy_and_receipts_surface() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_status(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(out.get("policy").is_some());
        assert!(out
            .get("fetch_provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("default_fetch_provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn providers_surface_returns_ranked_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_providers(tmp.path());
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_providers")
        );
        let providers = out
            .get("providers")
            .and_then(Value::as_array)
            .expect("provider catalog");
        assert!(providers
            .iter()
            .any(|row| row.get("provider").and_then(Value::as_str) == Some("duckduckgo")));
        assert!(providers.iter().all(|row| row.get("auto_detect_rank").is_some()));
        let fetch_providers = out
            .get("fetch_providers")
            .and_then(Value::as_array)
            .expect("fetch provider catalog");
        assert!(fetch_providers.iter().any(|row| {
            row.get("provider").and_then(Value::as_str) == Some("direct_http")
        }));
    }

    #[test]
    fn sensitive_domain_requires_explicit_human_approval() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/policy_decision/reason")
                .and_then(Value::as_str),
            Some("human_approval_required_for_sensitive_domain")
        );
        assert_eq!(
            out.get("approval_required").and_then(Value::as_bool),
            Some(true)
        );
        assert!(out.pointer("/approval/id").is_some());
    }

    #[test]
    fn approved_token_allows_sensitive_domain_policy_gate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let first = api_fetch(
            tmp.path(),
            &json!({"url": "https://accounts.google.com/login", "human_approved": false}),
        );
        let approval_id = first
            .pointer("/approval/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(!approval_id.is_empty());

        let mut approvals = load_approvals(tmp.path());
        if let Some(row) = approvals.iter_mut().find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
        }) {
            row["status"] = json!("approved");
            row["updated_at"] = json!(crate::now_iso());
        }
        save_approvals(tmp.path(), &approvals).expect("save approvals");

        let second = api_fetch(
            tmp.path(),
            &json!({
                "url": "https://accounts.google.com/login",
                "approval_id": approval_id,
                "summary_only": true
            }),
        );
        assert_eq!(
            second
                .pointer("/policy_decision/allow")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn fetch_example_com_and_summarize_smoke() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_fetch(
            tmp.path(),
            &json!({"url": "https://example.com", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("provider").and_then(Value::as_str),
            Some("direct_http")
        );
        assert!(out
            .get("provider_chain")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            assert!(out
                .get("summary")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false));
        } else {
            assert!(out.get("error").is_some());
        }
    }

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
        assert!(out
            .get("fetch_provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("provider").and_then(Value::as_str) == Some("direct_http")))
            .unwrap_or(false));
    }

    #[test]
    fn search_requires_query() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(tmp.path(), &json!({"query": ""}));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("query_required")
        );
        assert!(out.get("receipt").is_some());
    }

    #[test]
    fn search_smoke_records_receipt() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({"query": "example domain", "summary_only": true}),
        );
        assert!(out.get("receipt").is_some());
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("web_conduit_search")
        );
        assert!(
            matches!(
                out.get("provider").and_then(Value::as_str),
                Some("duckduckgo")
                    | Some("duckduckgo_lite")
                    | Some("bing_rss")
                    | Some("serperdev")
                    | Some("none")
            ),
            "unexpected provider: {:?}",
            out.get("provider")
        );
        assert!(out.get("provider_chain").is_some());
    }

    #[test]
    fn api_search_rejects_unknown_explicit_provider() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_search(
            tmp.path(),
            &json!({
                "query": "agent reliability benchmarks",
                "provider": "perplexity"
            }),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("unknown_search_provider")
        );
        assert_eq!(
            out.get("requested_provider").and_then(Value::as_str),
            Some("perplexity")
        );
        assert!(out
            .get("provider_catalog")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row.get("provider").and_then(Value::as_str) == Some("duckduckgo")))
            .unwrap_or(false));
    }

    #[test]
    fn challenge_detector_flags_anomaly_copy() {
        assert!(looks_like_search_challenge_payload(
            "Unfortunately, bots use DuckDuckGo too.",
            "Please complete the following challenge and select all squares containing a duck."
        ));
    }

    #[test]
    fn challenge_detector_ignores_normal_results() {
        assert!(!looks_like_search_challenge_payload(
            "Tech News | Today's Latest Technology News | Reuters",
            "www.reuters.com/technology/ Find latest technology news from every corner of the globe."
        ));
    }

    #[test]
    fn scoped_search_query_applies_domain_filters() {
        let scoped = scoped_search_query(
            "agent reliability",
            &vec!["github.com".to_string(), "docs.rs".to_string()],
            false,
        );
        assert!(scoped.contains("site:github.com"));
        assert!(scoped.contains("site:docs.rs"));
        assert!(scoped.contains("agent reliability"));
    }

    #[test]
    fn scoped_search_query_leaves_plain_query_when_domains_empty() {
        let scoped = scoped_search_query("agent reliability", &[], false);
        assert_eq!(scoped, "agent reliability");
    }

    #[test]
    fn normalize_allowed_domains_sanitizes_urls_and_duplicates() {
        let domains = normalize_allowed_domains(&json!([
            "https://www.github.com/openai",
            "docs.rs",
            "github.com",
            "not a domain"
        ]));
        assert_eq!(
            domains,
            vec!["github.com".to_string(), "docs.rs".to_string()]
        );
    }

    #[test]
    fn scoped_search_query_supports_exact_domain_mode() {
        let scoped =
            scoped_search_query("agent reliability", &vec!["example.com".to_string()], true);
        assert!(scoped.contains("site:example.com"));
        assert!(scoped.contains("-site:*.example.com"));
    }

    #[test]
    fn normalize_allowed_domains_supports_comma_string() {
        let domains =
            normalize_allowed_domains(&json!("https://www.github.com, docs.rs *.example.com"));
        assert_eq!(
            domains,
            vec![
                "github.com".to_string(),
                "docs.rs".to_string(),
                "example.com".to_string()
            ]
        );
    }

    #[test]
    fn domain_allowed_scope_respects_exact_domain_mode() {
        let filters = vec!["example.com".to_string()];
        assert!(domain_allowed_for_scope(
            "https://example.com/docs",
            &filters,
            true
        ));
        assert!(!domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            true
        ));
        assert!(domain_allowed_for_scope(
            "https://blog.example.com/post",
            &filters,
            false
        ));
    }

    #[test]
    fn render_serper_payload_filters_domains_and_builds_content() {
        let body = serde_json::to_string(&json!({
            "organic": [
                {
                    "title": "Main",
                    "link": "https://example.com/main",
                    "snippet": "Main domain snippet"
                },
                {
                    "title": "Subdomain",
                    "link": "https://blog.example.com/post",
                    "snippet": "Subdomain snippet"
                },
                {
                    "title": "Other",
                    "link": "https://other.com/page",
                    "snippet": "Other domain snippet"
                }
            ]
        }))
        .expect("encode");
        let rendered =
            render_serper_payload(&body, &vec!["example.com".to_string()], true, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            rendered.get("provider_raw_count").and_then(Value::as_u64),
            Some(3)
        );
        assert_eq!(
            rendered
                .get("provider_filtered_count")
                .and_then(Value::as_u64),
            Some(1)
        );
        let links = rendered
            .get("links")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].as_str(), Some("https://example.com/main"));
    }

    #[test]
    fn render_serper_payload_handles_invalid_json() {
        let rendered = render_serper_payload("not-json", &[], false, 8, 12_000);
        assert_eq!(rendered.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            rendered.get("error").and_then(Value::as_str),
            Some("serper_decode_failed")
        );
    }
    include!("080-tests.tail.rs");
}
