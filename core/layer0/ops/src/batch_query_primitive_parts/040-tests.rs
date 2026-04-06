mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_fixture<T>(fixture: Value, run: impl FnOnce() -> T) -> T {
        let _guard = TEST_ENV_MUTEX.lock().expect("lock");
        std::env::set_var(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            serde_json::to_string(&fixture).expect("encode fixture"),
        );
        let out = run();
        std::env::remove_var("INFRING_BATCH_QUERY_TEST_FIXTURE_JSON");
        out
    }

    #[test]
    fn large_aperture_is_blocked_without_policy_opt_in() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({"source": "web", "query": "agent systems", "aperture": "large"}),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("blocked"));
    }

    #[test]
    fn web_query_with_results_returns_evidence_and_clean_summary() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"agent systems":{"ok":true,"summary":"Agent systems coordinate tools with deterministic receipts.","requested_url":"https://example.com/agents","status_code":200}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"agent systems","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        assert!(out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(!summary
            .to_ascii_lowercase()
            .contains("web search completed"));
    }

    #[test]
    fn no_results_path_returns_clean_no_results_status() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"batch query no results":{"ok":false,"error":"provider_network_policy_blocked"}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"batch query no results","aperture":"small"}),
                )
            },
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert_eq!(
            out.get("evidence_refs")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(9),
            0
        );
    }

    #[test]
    fn medium_aperture_enables_parallel_retrieval_for_rewrites() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fixture = json!({
            "agent runtime reliability":{"ok":true,"summary":"Primary finding for runtime reliability.","requested_url":"https://example.com/one","status_code":200},
            "agent runtime reliability overview":{"ok":true,"summary":"Secondary finding for runtime reliability.","requested_url":"https://example.com/two","status_code":200}
        });
        let out = with_fixture(fixture, || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":"agent runtime reliability","aperture":"medium"}),
            )
        });
        assert_eq!(
            out.get("parallel_retrieval_used").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("rewrite_set")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn exact_match_query_disables_rewrite_and_parallel() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"\"agent::run\"":{"ok":true,"summary":"Exact symbol lookup result.","requested_url":"https://example.com/symbol","status_code":200}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"\"agent::run\"","aperture":"medium"}),
                )
            },
        );
        assert_eq!(
            out.get("parallel_retrieval_used").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.get("rewrite_set")
                .and_then(Value::as_array)
                .map(|rows| rows.len())
                .unwrap_or(99),
            0
        );
    }

    #[test]
    fn ack_only_summary_is_never_returned_to_user() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"ack leak":{"ok":true,"summary":"Web search completed.","requested_url":"https://example.com/ack","status_code":200}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"ack leak","aperture":"small"}),
                )
            },
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(!summary
            .to_ascii_lowercase()
            .contains("web search completed"));
    }

    #[test]
    fn low_signal_search_summary_falls_back_to_content_source_domains() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "latest technology news today": {
                    "ok": true,
                    "summary": "latest technology news today at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology News - CNBC www.cnbc.com/technology/ Business news related to the technology industry.",
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"latest technology news today","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("reuters.com") || lowered.contains("cnbc.com"));
        assert!(!lowered.contains("all regions"));
        assert!(!lowered.contains("safe search"));
    }

    #[test]
    fn duckduckgo_instant_fallback_recovers_when_primary_search_has_no_findings() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "current technology news": {
                    "ok": true,
                    "summary": "current technology news at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "",
                    "requested_url": "https://duckduckgo.com/html/?q=current+technology+news",
                    "status_code": 200
                },
                "duckduckgo_instant::current technology news": {
                    "ok": true,
                    "status_code": 200,
                    "requested_url": "https://api.duckduckgo.com/?q=current+technology+news&format=json&no_html=1&skip_disambig=1",
                    "summary": "Instant answer payload",
                    "content": "{\"Heading\":\"Technology news\",\"AbstractText\":\"Technology coverage tracks AI launches, developer tools, and product release cadence.\",\"AbstractURL\":\"https://example.com/technology-news\"}"
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"current technology news","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("technology coverage tracks ai launches"));
        let evidence = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!evidence.is_empty());
        let locator = evidence
            .first()
            .and_then(|row| row.get("locator"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(locator.contains("example.com/technology-news"));
    }

    #[test]
    fn html_noise_content_is_slimmed_before_snippet_generation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "runtime dashboard reliability": {
                    "ok": true,
                    "summary": "",
                    "content": "<html><body><script>alert('x')</script><style>.x{display:none}</style><svg><circle/></svg><img src='data:image/png;base64,abc123' /><div class='hero' data-x='1'>Runtime contract verification is healthy and dashboard startup is deterministic.</div></body></html>",
                    "requested_url": "https://example.com/runtime-health",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"runtime dashboard reliability","aperture":"small"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("runtime contract verification is healthy"));
        assert!(!lowered.contains("<script"));
        assert!(!lowered.contains("alert('x')"));
        assert!(!lowered.contains("<svg"));
        assert!(!lowered.contains("data:image"));
    }
}
