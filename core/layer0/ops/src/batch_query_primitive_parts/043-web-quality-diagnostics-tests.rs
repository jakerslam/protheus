mod web_quality_diagnostics_tests {
    use super::*;
    use std::sync::Mutex;

    static WEB_QUALITY_TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct ScopedEnvVar {
        key: &'static str,
        previous: Option<String>,
    }

    impl ScopedEnvVar {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    fn with_fixture<T>(fixture: Value, run: impl FnOnce() -> T) -> T {
        let _guard = WEB_QUALITY_TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_query_with_fixture(fixture: Value, query: &str) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || {
            api_batch_query(
                tmp.path(),
                &json!({"source":"web","query":query,"aperture":"small"}),
            )
        })
    }

    fn quality_flags(out: &Value) -> Vec<String> {
        out.pointer("/tool_result_quality/flags")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn candidate(locator: &str, snippet: &str) -> Candidate {
        Candidate {
            source_kind: "web".to_string(),
            title: format!("Web result from {locator}"),
            locator: locator.to_string(),
            snippet: snippet.to_string(),
            excerpt_hash: sha256_hex(snippet),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        }
    }

    #[test]
    fn anti_bot_failures_emit_structured_quality_retry() {
        let query = "latest technology news today";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Unfortunately, bots use DuckDuckGo too. Please complete the following challenge.",
                    "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {"ok": false, "error": "bing_rss_search_failed"},
                format!("duckduckgo_instant::{query}"): {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        let flags = quality_flags(&out);
        assert!(flags.iter().any(|flag| flag == "anti_bot_filtered"));
        assert!(flags.iter().any(|flag| flag == "insufficient_evidence"));
        assert_eq!(
            out.pointer("/tool_result_quality/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn junk_pages_are_filtered_before_synthesis_and_diagnosed() {
        let query = "current agent framework releases";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Please enable JavaScript and cookies to continue. Access denied.",
                    "requested_url": "https://example.com/blocked",
                    "status_code": 403
                },
                format!("bing_rss::{query}"): {"ok": false, "error": "bing_rss_search_failed"},
                format!("duckduckgo_instant::{query}"): {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        let flags = quality_flags(&out);
        assert!(flags.iter().any(|flag| flag == "junk_filtered"), "{flags:?}");
        assert!(!out.get("summary").and_then(Value::as_str).unwrap_or("").contains("Access denied"));
    }

    #[test]
    fn diverse_ranked_selection_avoids_same_domain_monoculture_first() {
        let ranked = vec![
            (
                candidate(
                    "https://news.example.com/one",
                    "Agent framework release notes include current benchmark data for May 2026.",
                ),
                0.94,
            ),
            (
                candidate(
                    "https://news.example.com/two",
                    "Agent framework release notes include current benchmark data for May 2026.",
                ),
                0.93,
            ),
            (
                candidate(
                    "https://docs.example.org/agent-frameworks",
                    "Agent framework documentation describes current tooling support in May 2026.",
                ),
                0.82,
            ),
        ];
        let selected = select_diverse_ranked_candidates(ranked, 2);
        let domains = selected
            .iter()
            .map(|(row, _)| candidate_domain_hint(row))
            .collect::<Vec<_>>();
        assert_eq!(domains.len(), 2);
        assert_ne!(domains[0], domains[1]);
    }

    #[test]
    fn source_trust_scoring_prefers_primary_sources_over_forums() {
        let query = "current AI agent framework release notes";
        let official = candidate(
            "https://docs.example.com/agent-framework/releases",
            "Agent framework release notes list May 2026 tool support and current APIs.",
        );
        let forum = candidate(
            "https://reddit.com/r/LocalLLaMA/comments/example",
            "A forum thread discusses agent frameworks with anecdotes and no source links.",
        );
        assert!(
            rerank_score(query, &official) > rerank_score(query, &forum),
            "official={} forum={}",
            rerank_score(query, &official),
            rerank_score(query, &forum)
        );
    }

    #[test]
    fn fallback_links_are_ranked_before_followup_fetch() {
        let payload = json!({
            "links": [
                "https://reddit.com/r/agents/comments/example",
                "https://docs.example.com/agent-framework/releases",
                "https://www.bing.com/search?q=agent+frameworks",
                "https://news.example.com/ai-agent-frameworks-2026"
            ]
        });
        let links = payload_links_for_fallback(
            "current AI agent framework release notes",
            &payload,
            2,
        );
        assert_eq!(
            links.first().map(String::as_str),
            Some("https://docs.example.com/agent-framework/releases")
        );
        assert!(!links.iter().any(|link| link.contains("bing.com")));
    }

    #[test]
    fn quality_report_keeps_retry_query_authority_with_agent() {
        let out = run_query_with_fixture(
            json!({
                "current agent frameworks": {
                    "ok": false,
                    "error": "provider_timeout"
                },
                "bing_rss::current agent frameworks": {"ok": false, "error": "bing_rss_search_failed"},
                "duckduckgo_instant::current agent frameworks": {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            "current agent frameworks",
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/input_contract/authority")
                .and_then(Value::as_str),
            Some("agent_submitted")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/input_contract/hidden_query_expansion")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/freshness/current_intent")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn successful_web_result_exports_synthesis_quality_bundle() {
        let query = "current AI agent frameworks May 2026";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "LangGraph, OpenAI Agents SDK, CrewAI, and AutoGen publish official 2026 documentation for agent framework tool use and orchestration patterns.",
                    "requested_url": "https://docs.example.com/agent-frameworks-2026",
                    "status_code": 200
                }
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        assert_eq!(
            out.pointer("/tool_result_quality/synthesis_contract/authority")
                .and_then(Value::as_str),
            Some("agent_authored")
        );
        assert!(out
            .pointer("/tool_result_quality/candidate_quality/0/snippet_preview")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("LangGraph"));
        assert_eq!(
            out.pointer("/tool_result_quality/retry/recommended")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn quality_report_marks_comparison_sources_for_careful_synthesis() {
        let ranked = vec![
            (
                candidate(
                    "https://docs.example.com/agent-a",
                    "Agent A is faster and stronger for multi-agent task execution in 2026.",
                ),
                0.88,
            ),
            (
                candidate(
                    "https://docs.example.org/agent-b",
                    "Agent B has limitations and is slower but offers stronger integrations.",
                ),
                0.84,
            ),
        ];
        let report = web_tool_quality_report(
            "compare agent A vs agent B in 2026",
            "ok",
            2,
            2,
            &[],
            &[],
            &ranked,
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(flags.iter().any(|flag| flag == "comparative_synthesis_required"));
        assert!(flags.iter().any(|flag| flag == "potential_source_conflict"));
        assert!(report.pointer("/candidate_quality/0/snippet_preview").is_some());
    }
}
