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

    fn run_request_with_fixture(fixture: Value, request: &Value) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || api_batch_query(tmp.path(), request))
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

    fn summary_lowered(out: &Value) -> String {
        out.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
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
    fn multi_term_current_queries_reject_single_term_false_positives() {
        let query = "scientific breakthroughs 2026";
        let false_positive = candidate(
            "https://www.desmos.com/scientific",
            "A free online scientific calculator with trigonometry, statistics, and logarithms.",
        );
        assert!(
            !candidate_passes_relevance_gate(query, &false_positive, false),
            "single-term overlap should not become evidence for a multi-term current query"
        );
    }

    #[test]
    fn broad_current_query_drops_off_topic_provider_results_before_evidence() {
        let query = "scientific breakthroughs 2026";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "provider": "bing_rss",
                    "summary": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "content": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "links": ["https://www.desmos.com/scientific"],
                    "requested_url": "https://www.bing.com/search?q=scientific+breakthroughs+2026&format=rss",
                    "status_code": 200
                },
                format!("bing_rss::{query}"): {
                    "ok": true,
                    "provider": "bing_rss",
                    "summary": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "content": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "links": ["https://www.desmos.com/scientific"],
                    "requested_url": "https://www.bing.com/search?q=scientific+breakthroughs+2026&format=rss",
                    "status_code": 200
                },
                format!("duckduckgo_instant::{query}"): {"ok": false, "error": "duckduckgo_instant_no_usable_summary"}
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        assert_eq!(
            out.pointer("/tool_result_quality/evidence_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert!(
            quality_flags(&out)
                .iter()
                .any(|flag| flag == "low_relevance_filtered"),
            "{out}"
        );
    }

    #[test]
    fn provider_result_artifact_marks_low_relevance_payload_as_not_usable() {
        let query = "scientific breakthroughs 2026";
        let out = run_request_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "provider": "bing_rss",
                    "summary": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "content": "Scientific Calculator - Desmos — https://www.desmos.com/scientific — A free online scientific calculator with trigonometry and statistics.",
                    "links": ["https://www.desmos.com/scientific"],
                    "requested_url": "https://www.bing.com/search?q=scientific+breakthroughs+2026&format=rss",
                    "status_code": 200
                }
            }),
            &json!({"source":"web","queries":[query],"aperture":"small"}),
        );
        let provider_result = out
            .get("provider_results")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .expect("provider result");
        assert_eq!(
            provider_result.get("provider_transport_ok").and_then(Value::as_bool),
            Some(true),
            "{out:#}"
        );
        assert_eq!(provider_result.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            provider_result.get("result_quality").and_then(Value::as_str),
            Some("low_relevance")
        );
        assert_eq!(
            provider_result
                .get("synthesis_candidate_count")
                .and_then(Value::as_u64),
            Some(0)
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
    fn current_year_counts_as_current_web_intent() {
        assert!(current_web_intent("scientific breakthroughs 2026"));
        assert!(current_web_intent("materials science publications 2026"));
    }

    #[test]
    fn degraded_provider_issue_survives_when_one_candidate_is_retained() {
        let query = "CrewAI automation workforce training";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": false,
                    "error": "web_search_tool_surface_degraded",
                    "summary": "Web search tooling is degraded (provider readiness mismatch). Retry after credentials or provider runtime are repaired."
                },
                format!("bing_rss::{query}"): {
                    "ok": true,
                    "summary": "AI and Automation Impact on Workforce Training | .Training - crewai.io",
                    "content": "AI and Automation Impact on Workforce Training | .Training - crewai.io — https://www.crewai.io/lander — AI and automation are revolutionizing workforce training by reshaping job roles, necessitating reskilling, and enhancing learning experiences.",
                    "requested_url": "https://www.crewai.io/lander",
                    "status_code": 200
                }
            }),
            query,
        );
        assert!(
            quality_flags(&out)
                .iter()
                .any(|flag| flag == "provider_degraded"),
            "{out:#}"
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retry/reason")
                .and_then(Value::as_str),
            Some("provider_degraded")
        );
        assert!(
            out.pointer("/provider_results/0/summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("provider readiness mismatch")
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

    #[test]
    fn weak_single_research_source_recommends_agent_retry() {
        let report = web_tool_quality_report(
            "CrewAI multi agent framework documentation",
            "ok",
            1,
            1,
            &[],
            &[],
            &[(
                candidate(
                    "https://www.crewai.io/lander",
                    "AI and automation are revolutionizing workforce training by reshaping job roles, necessitating reskilling, and enhancing learning experiences.",
                ),
                0.52,
            )],
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(flags.iter().any(|flag| flag == "weak_single_source"), "{flags:?}");
        assert_eq!(
            report.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("weak_single_source")
        );
    }

    #[test]
    fn cached_weak_single_source_is_not_replayed_as_clean_success() {
        let report = cached_web_tool_quality_report(
            "CrewAI multi agent framework documentation",
            "ok",
            &json!([]),
            &json!([
                {
                    "title": "AI and Automation Impact on Workforce Training | .Training - crewai.io",
                    "locator": "https://www.crewai.io/lander",
                    "score": 0.52
                }
            ]),
        );
        assert_eq!(
            report.get("version").and_then(Value::as_str),
            Some(web_tool_quality_version())
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("weak_single_source")
        );
        assert_eq!(
            report.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn comparison_research_requires_more_than_one_evidence_source() {
        let report = web_tool_quality_report(
            "compare CrewAI and LangGraph agent frameworks",
            "ok",
            1,
            1,
            &[],
            &[],
            &[(
                candidate(
                    "https://www.langchain.com/langgraph",
                    "LangGraph is an agent orchestration framework for reliable AI agents.",
                ),
                0.92,
            )],
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            flags
                .iter()
                .any(|flag| flag == "comparison_evidence_insufficient"),
            "{flags:?}"
        );
        assert_eq!(
            report.pointer("/retry/recommended").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("comparison_evidence_insufficient")
        );
    }

    #[test]
    fn comparison_guard_keeps_hidden_search_results_when_only_one_side_retrieves() {
        let out = run_request_with_fixture(
            json!({
                "LangGraph official docs reliability deployment": {
                    "ok": true,
                    "summary": "LangGraph documentation covers durable execution, checkpoints, deployment controls, and human-in-the-loop review for reliable agents.",
                    "requested_url": "https://docs.langchain.com/langgraph",
                    "status_code": 200
                },
                "CrewAI official docs reliability deployment": {
                    "ok": false,
                    "error": "query_result_mismatch"
                }
            }),
            &json!({
                "source":"web",
                "query":"Compare LangGraph vs CrewAI on reliability and deployment",
                "queries":[
                    "LangGraph official docs reliability deployment",
                    "CrewAI official docs reliability deployment"
                ],
                "aperture":"medium"
            }),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        assert!(summary_lowered(&out).contains("retrieval-quality miss"));
        assert_eq!(
            out.pointer("/search_results/0/title").and_then(Value::as_str),
            Some("Web result from docs.langchain.com")
        );
        assert!(
            out.pointer("/search_results/0/snippet")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("langgraph")
        );
    }

    #[test]
    fn comparison_guard_keeps_ranked_preview_when_official_docs_are_too_generic_for_evidence() {
        let out = run_request_with_fixture(
            json!({
                "LangGraph official docs reliability observability human-in-the-loop deployment": {
                    "ok": true,
                    "summary": "LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain",
                    "requested_url": "https://www.langchain.com/langgraph",
                    "status_code": 200
                },
                "CrewAI official docs reliability observability human-in-the-loop deployment": {
                    "ok": false,
                    "error": "query_result_mismatch"
                }
            }),
            &json!({
                "source":"web",
                "query":"Compare LangGraph vs CrewAI on reliability, observability, human-in-the-loop, and deployment.",
                "queries":[
                    "LangGraph official docs reliability observability human-in-the-loop deployment",
                    "CrewAI official docs reliability observability human-in-the-loop deployment"
                ],
                "aperture":"medium"
            }),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        assert_eq!(
            out.pointer("/tool_result_quality/evidence_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            out.pointer("/search_results/0/locator").and_then(Value::as_str),
            Some("https://www.langchain.com/langgraph")
        );
        assert!(
            out.pointer("/tool_result_quality/flags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .any(|flag| flag == "comparison_evidence_insufficient")
        );
    }
}
