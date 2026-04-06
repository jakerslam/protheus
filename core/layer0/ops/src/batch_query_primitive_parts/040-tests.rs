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
        assert!(!summary.to_ascii_lowercase().contains("key findings for"));
        assert!(!summary.to_ascii_lowercase().contains("potential sources:"));
    }

    #[test]
    fn no_results_path_returns_clean_no_results_status() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "batch query no results":{"ok":false,"error":"provider_network_policy_blocked"},
                "bing_rss::batch query no results":{"ok":false,"error":"bing_rss_search_failed"}
            }),
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
    fn source_only_scaffold_is_filtered_and_returns_no_results() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({
                "infring competitors": {
                    "ok": true,
                    "summary": "Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai, iot-analytics.com, linkedin.com.",
                    "requested_url": "https://duckduckgo.com/html/?q=infring+competitors",
                    "status_code": 200
                },
                "bing_rss::infring competitors": {
                    "ok": false,
                    "error": "bing_rss_search_failed"
                },
                "duckduckgo_instant::infring competitors": {
                    "ok": false,
                    "error": "duckduckgo_instant_no_usable_summary"
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"infring competitors","aperture":"small"}),
                )
            },
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(!lowered.contains("key findings for"));
        assert!(!lowered.contains("potential sources:"));
        assert!(lowered.contains("no useful information"));
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
    fn instructional_prompt_rewrite_focuses_query_topic() {
        let query = "Verify web search and fetch capabilities by researching current AI agent framework benchmarks. Report top 3 performance metrics found.";
        let rewrite = normalize_instructional_query(query).expect("rewrite");
        assert!(rewrite.contains("agent"));
        assert!(rewrite.contains("benchmarks"));
        assert!(rewrite.contains("metrics"));
        assert!(!rewrite.contains("verify"));
        assert!(!rewrite.contains("report"));
        let budget = aperture_budget("medium").expect("budget");
        let (plan, rewrite_set, rewrite_applied) = build_query_plan(query, budget);
        assert!(rewrite_applied);
        assert_eq!(plan.len(), 2);
        assert_eq!(rewrite_set.len(), 1);
        assert_eq!(plan[1], rewrite);
    }

    #[test]
    fn benchmark_intent_skips_definition_noise_and_synthesizes_metrics() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "Verify web search and fetch capabilities by researching current AI agent framework benchmarks. Report top 3 performance metrics found.";
        let rewrite = normalize_instructional_query(query).expect("rewrite");
        let mut fixture = Map::new();
        fixture.insert(
            query.to_string(),
            json!({
                "ok": true,
                "summary": "VERIFY Definition & Meaning - Merriam-Webster",
                "requested_url": "https://www.merriam-webster.com/dictionary/verify",
                "status_code": 200
            }),
        );
        fixture.insert(
            rewrite,
            json!({
                "ok": true,
                "summary": "Latest benchmark run reports median latency 820ms, throughput 48 tokens/s, and task success rate 86% across top agent frameworks.",
                "requested_url": "https://artificialanalysis.ai/benchmarks/agent-frameworks",
                "status_code": 200
            }),
        );
        let out = with_fixture(Value::Object(fixture), || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":query,"aperture":"medium"}),
            )
        });
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("web benchmark synthesis"));
        assert!(lowered.contains("latency") || lowered.contains("tokens/s"));
        assert!(!lowered.contains("merriam-webster"));
        let evidence = out
            .get("evidence_refs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!evidence.is_empty());
        let has_metric_locator = evidence.iter().any(|row| {
            row.get("locator")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("artificialanalysis.ai")
        });
        assert!(has_metric_locator);
    }

    #[test]
    fn compare_query_resolves_deictic_framework_and_blocks_grammar_noise() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "compare this framework to openclaw";
        let out = with_fixture(
            json!({
                "compare infring to openclaw": {
                    "ok": true,
                    "summary": "bing.com: compare [A with B] vs compare A [with B] | WordReference Forums",
                    "requested_url": "https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"medium"}),
                )
            },
        );
        assert_eq!(
            out.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("comparison findings"));
        assert!(lowered.contains("infring"));
        assert!(lowered.contains("openclaw"));
        assert!(!lowered.contains("wordreference"));
    }

    #[test]
    fn compare_query_prefers_entities_coverage_for_synthesis() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let query = "compare this framework to openclaw";
        let out = with_fixture(
            json!({
                "compare infring to openclaw": {
                    "ok": true,
                    "summary": "Independent benchmark: Infring achieved 52 ops/sec while OpenClaw achieved 47 ops/sec with median latency 840ms vs 910ms.",
                    "requested_url": "https://example.com/agent-framework-benchmark",
                    "status_code": 200
                },
                "compare infring to openclaw overview": {
                    "ok": true,
                    "summary": "bing.com: compare [A with B] vs compare A [with B] | WordReference Forums",
                    "requested_url": "https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
                    "status_code": 200
                }
            }),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":query,"aperture":"medium"}),
                )
            },
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("infring"));
        assert!(lowered.contains("openclaw"));
        assert!(lowered.contains("ops/sec") || lowered.contains("latency"));
        assert!(!lowered.contains("wordreference"));
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
                "bing_rss::current technology news": {
                    "ok": false,
                    "error": "bing_rss_search_failed"
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

    #[test]
    fn batch_query_emits_nexus_connection_metadata() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"nexus route test":{"ok":true,"summary":"Route metadata fixture.","requested_url":"https://example.com/nexus","status_code":200}}),
            || {
                api_batch_query(
                    tmp.path(),
                    &json!({"source":"web","query":"nexus route test","aperture":"small"}),
                )
            },
        );
        assert_eq!(
            out.pointer("/nexus_connection/source")
                .and_then(Value::as_str),
            Some("client_ingress")
        );
        assert_eq!(
            out.pointer("/nexus_connection/target")
                .and_then(Value::as_str),
            Some("context_stacks")
        );
    }

    #[test]
    fn batch_query_fails_closed_when_ingress_lifecycle_blocks_new_leases() {
        let _guard = TEST_ENV_MUTEX.lock().expect("lock");
        std::env::set_var(
            "PROTHEUS_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE",
            "quiesced",
        );
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({"source":"web","query":"lifecycle deny path","aperture":"small"}),
        );
        std::env::remove_var("PROTHEUS_HIERARCHICAL_NEXUS_CLIENT_INGRESS_LIFECYCLE");
        assert_eq!(out.get("status").and_then(Value::as_str), Some("blocked"));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("batch_query_nexus_delivery_denied")
        );
    }
}
