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
    fn source_class_path_rules_do_not_match_url_host_text() {
        let policy = json!({
            "batch_query": {
                "evidence_pack": {
                    "source_class_rules": [
                        {"class": "announcement_or_news", "path_contains": ["/news"]}
                    ]
                }
            }
        });
        let rss_wrapper = candidate(
            "https://news.google.com/rss/articles/example",
            "A search result surfaced through a news aggregator wrapper.",
        );
        let news_article = candidate(
            "https://example.org/news/article",
            "A direct article URL whose path really contains news.",
        );

        assert_eq!(
            evidence_pack_source_class(&policy, &rss_wrapper),
            "general_web"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &news_article),
            "announcement_or_news"
        );
    }

    #[test]
    fn source_class_rules_match_title_and_snippet_hints() {
        let policy = json!({
            "batch_query": {
                "evidence_pack": {
                    "source_class_rules": [
                        {"class": "documentation_or_reference", "title_contains": ["how to", "tutorial"]},
                        {"class": "independent_analysis", "title_contains": ["best ", " vs "]},
                        {"class": "news_or_current", "snippet_contains": ["announced", "release"]}
                    ]
                }
            }
        });
        let guide = Candidate {
            source_kind: "web".to_string(),
            title: "How to build a retrieval agent".to_string(),
            locator: "https://news.google.com/rss/articles/example".to_string(),
            snippet: "Search result surfaced through an aggregator wrapper.".to_string(),
            excerpt_hash: "guide".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };
        let analysis = Candidate {
            source_kind: "web".to_string(),
            title: "Best retrieval tools for AI agents".to_string(),
            locator: "https://example.org/articles/result".to_string(),
            snippet: "A comparison-style article.".to_string(),
            excerpt_hash: "analysis".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };
        let announcement = Candidate {
            source_kind: "web".to_string(),
            title: "Provider update".to_string(),
            locator: "https://example.org/articles/result".to_string(),
            snippet: "The provider announced a new release today.".to_string(),
            excerpt_hash: "announcement".to_string(),
            timestamp: None,
            permissions: Some("public_web".to_string()),
            status_code: 200,
        };

        assert_eq!(
            evidence_pack_source_class(&policy, &guide),
            "documentation_or_reference"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &analysis),
            "independent_analysis"
        );
        assert_eq!(
            evidence_pack_source_class(&policy, &announcement),
            "news_or_current"
        );
    }

    #[test]
    fn provider_source_hint_domain_allows_source_name_parentheses() {
        let row = candidate(
            "https://news.google.com/rss/articles/example",
            "Result text. Source: Amazon Web Services (AWS) (aws.amazon.com).",
        );
        assert_eq!(candidate_domain_hint(&row), "aws.amazon.com");
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
        assert_eq!(
            out.pointer(
                "/tool_result_quality/browser_materialization/recommended_when_policy_allows"
            )
            .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/capability")
                .and_then(Value::as_str),
            Some("browser_materialize_page")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/decision_authority")
                .and_then(Value::as_str),
            Some("tool_cd_and_gateway_policy")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/blocker_taxonomy/primary_class")
                .and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/blocker_class")
                .and_then(Value::as_str),
            Some("anti_bot_challenge")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/evidence_handoff/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/profile_compilation/status")
                .and_then(Value::as_str),
            Some("contract_ready_default_off")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/profile_compilation/raw_launch_args_accepted_from_caller")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/readiness_lifecycle/status")
                .and_then(Value::as_str),
            Some("not_configured_default_off")
        );
        assert_eq!(
            out.pointer("/tool_result_quality/browser_materialization/readiness_lifecycle/ordinary_research_may_install_dependency")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/retry_stop_conditions/status")
                .and_then(Value::as_str),
            Some("continue_with_alternate_provider_if_admitted")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/retry_stop_conditions/stop_conditions/capability_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/action_status")
                .and_then(Value::as_str),
            Some("requires_admitted_alternate_provider_or_browser_retrieval_capability")
        );
    }

    #[test]
    fn blocker_taxonomy_splits_js_rate_limit_and_access_denied_failures() {
        let report = web_tool_quality_report(
            "current public research evidence",
            "no_results",
            0,
            0,
            &[
                "needs_js: please enable javascript before content renders".to_string(),
                "http_429 provider rate limit".to_string(),
                "access denied 403 forbidden".to_string(),
            ],
            &[],
            &[],
        );
        assert_eq!(
            report
                .pointer("/blocker_taxonomy/primary_class")
                .and_then(Value::as_str),
            Some("needs_js")
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("needs_js")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/action_status")
                .and_then(Value::as_str),
            Some("requires_admitted_alternate_provider_or_browser_retrieval_capability")
        );
        let classes = report
            .pointer("/blocker_taxonomy/classes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for expected in ["needs_js", "rate_limited", "access_denied"] {
            assert!(
                classes.iter().any(|row| {
                    row.get("class").and_then(Value::as_str) == Some(expected)
                        && row.get("present").and_then(Value::as_bool) == Some(true)
                }),
                "{report:#?}"
            );
        }
        assert_eq!(
            report
                .pointer("/browser_materialization/recommended_when_policy_allows")
                .and_then(Value::as_bool),
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
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert!(
            out.pointer("/provider_results/0/summary")
                .and_then(Value::as_str)
                .unwrap_or("")
                .contains("provider readiness mismatch")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/provider_normalization/version")
                .and_then(Value::as_str),
            Some("provider_normalization_v1")
        );
        assert!(
            out.pointer("/retrieval_broker/provider_normalization/failure_classes")
                .and_then(Value::as_array)
                .map(|rows| rows
                    .iter()
                    .any(|row| row.as_str() == Some("provider_degraded")))
                .unwrap_or(false),
            "{out:#?}"
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
        assert_eq!(
            out.pointer("/tool_result_quality/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("synthesize_from_evidence")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/retry_stop_conditions/status")
                .and_then(Value::as_str),
            Some("stop_ready_for_synthesis")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/artifact_quarantine/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/retrieval_broker/page_readiness_extraction/status")
                .and_then(Value::as_str),
            Some("evidence_packaged")
        );
    }

    #[test]
    fn evidence_pack_exports_processible_research_context_without_answer_format() {
        let query = "scientific breakthroughs 2026";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "Researchers reported a scientific breakthroughs 2026 update: an April 2026 quantum sensing result improved measurement precision and documented methods, limits, and institutional context.",
                    "requested_url": "https://science.example.edu/research/publications/scientific-breakthroughs-2026",
                    "status_code": 200
                }
            }),
            query,
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let pack = out
            .get("evidence_pack")
            .and_then(Value::as_array)
            .expect("evidence pack");
        let first = pack.first().expect("first evidence item");
        assert_eq!(
            first.get("pack_version").and_then(Value::as_str),
            Some("evidence_pack_v1")
        );
        assert_eq!(
            first.get("source_class").and_then(Value::as_str),
            Some("scholarly_or_research")
        );
        assert_eq!(
            first.get("confidence").and_then(Value::as_str),
            Some("usable")
        );
        assert_eq!(
            first.pointer("/freshness/current_intent").and_then(Value::as_bool),
            Some(true)
        );
        assert!(first
            .get("claim_hints")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(first
            .get("term_hints")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(first.pointer("/score_components/relevance").is_some());
        assert_eq!(
            first.pointer("/promotion/version").and_then(Value::as_str),
            Some("evidence_promotion_v1")
        );
        assert_eq!(
            first.pointer("/promotion/decision").and_then(Value::as_str),
            Some("promoted")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/raw_payload_chat_visible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/url_safety/status")
                .and_then(Value::as_str),
            Some("allowed_public_http_https")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/artifact_quarantine/version")
                .and_then(Value::as_str),
            Some("artifact_quarantine_v1")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/artifact_quarantine/evidence_promotions/0/promotion_decision")
                .and_then(Value::as_str),
            Some("promoted")
        );
        assert_eq!(
            out.pointer("/evidence_pack_quality/status").and_then(Value::as_str),
            Some("thin")
        );
        assert_eq!(
            out.pointer("/source_class_coverage/status").and_then(Value::as_str),
            Some("limited")
        );
        assert_eq!(
            out.pointer("/retrieval_broker/primitive").and_then(Value::as_str),
            Some("web_research")
        );
        assert!(
            out.pointer("/retrieval_broker/lanes")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| {
                    row.get("lane").and_then(Value::as_str) == Some("candidate_enrichment")
                }))
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(
            out.pointer("/retrieval_broker/provider_attempts")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false),
            "{out:#?}"
        );
        assert!(!out
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains("claim_hints"));
    }

    #[test]
    fn evidence_promotion_marks_internal_or_credentialed_candidate_as_caveated() {
        let candidate = candidate(
            "http://user:pass@127.0.0.1/admin",
            "The public science report describes research milestones, publication dates, method limitations, institutional context, and measured outcomes for the requested investigation.",
        );
        let pack = evidence_pack_from_ranked_candidates(
            &default_policy(),
            "public science report research milestones",
            &[],
            1,
            &[(candidate, 0.91)],
            1,
        );
        let first = pack
            .as_array()
            .and_then(|rows| rows.first())
            .expect("evidence item");
        assert_eq!(
            first.pointer("/promotion/decision").and_then(Value::as_str),
            Some("promoted_with_caveats")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/status")
                .and_then(Value::as_str),
            Some("unsafe_or_internal_hint")
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/credentials_in_url")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/internal_host_hint")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            first
                .pointer("/promotion/safety/url_safety/status")
                .and_then(Value::as_str),
            Some("blocked_internal_host_hint")
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
        assert_eq!(
            report.pointer("/coverage/bucket_status").and_then(Value::as_str),
            Some("covered")
        );
    }

    #[test]
    fn usable_evidence_does_not_misclassify_missing_premium_provider_as_starvation() {
        let ranked = vec![
            (
                candidate(
                    "https://news.google.com/rss/articles/example-a",
                    "Published: Mon, 20 Apr 2026 07:00:00 GMT. Source: Nature (www.nature.com). New tools drive scientific discovery with evidence from major breakthroughs, publication metadata, institution context, and direct research findings suitable for bounded synthesis.",
                ),
                0.88,
            ),
            (
                candidate(
                    "https://news.google.com/rss/articles/example-b",
                    "Published: Wed, 01 Apr 2026 07:00:00 GMT. Source: Phys.org (phys.org). A large-scale analysis identifies disruptive innovations in research history, describes the method used to detect breakthroughs, and gives enough context for evidence-backed synthesis.",
                ),
                0.82,
            ),
        ];
        let report = web_tool_quality_report(
            "scientific breakthroughs 2026",
            "partial",
            8,
            2,
            &["serperdev:search_providers_exhausted".to_string()],
            &["serperdev:search_providers_exhausted".to_string()],
            &ranked,
        );
        let flags = report
            .get("flags")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            !flags.iter().any(|flag| flag == "provider_starved"),
            "{flags:?}"
        );
        assert!(
            flags
                .iter()
                .any(|flag| flag == "credentialed_provider_unavailable_nonblocking"),
            "{flags:?}"
        );
        assert_eq!(
            report.pointer("/retry/reason").and_then(Value::as_str),
            Some("none")
        );
    }

    #[test]
    fn tool_recovery_queries_do_not_become_required_coverage_facets() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "scientific breakthroughs 2026",
            &[
                "scientific breakthroughs 2026".to_string(),
                "scientific breakthroughs 2026 primary evidence".to_string(),
                "scientific breakthroughs 2026 official sources".to_string(),
                "scientific breakthroughs 2026 technical reports".to_string(),
            ],
            &BatchQueryKeywordPack {
                keywords: vec![
                    "scientific".to_string(),
                    "breakthroughs".to_string(),
                    "2026".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true}}}),
            budget,
        );
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].requested_text, "scientific breakthroughs 2026");
    }

    #[test]
    fn generated_query_lanes_do_not_expand_declared_coverage_obligations() {
        let budget = aperture_budget("medium").expect("medium budget");
        let facets = infer_research_facets(
            "Compare the current OpenAI Agents SDK with LangChain/LangGraph for production customer-support agents. Focus on tool orchestration, tracing, safety controls, and vendor lock-in.",
            &[
                "OpenAI Agents SDK tool orchestration".to_string(),
                "LangChain tool orchestration".to_string(),
                "LangGraph tool orchestration".to_string(),
                "OpenAI Agents SDK tracing".to_string(),
                "LangChain tracing".to_string(),
                "LangGraph tracing".to_string(),
                "OpenAI Agents SDK safety controls".to_string(),
                "LangChain safety controls".to_string(),
                "LangGraph safety controls".to_string(),
                "OpenAI Agents SDK vendor lock-in".to_string(),
                "LangChain vendor lock-in".to_string(),
            ],
            &BatchQueryKeywordPack {
                keywords: vec![
                    "production".to_string(),
                    "customer-support".to_string(),
                    "agents".to_string(),
                ],
                entities: vec![
                    "OpenAI Agents SDK".to_string(),
                    "LangChain".to_string(),
                    "LangGraph".to_string(),
                ],
                facets: vec![
                    "tool orchestration".to_string(),
                    "tracing".to_string(),
                    "safety controls".to_string(),
                    "vendor lock-in".to_string(),
                ],
                metadata_authority: "tool_structured_from_user_query_terms".to_string(),
                ..BatchQueryKeywordPack::default()
            },
            &json!({"batch_query":{"coverage_aware_evidence":{"enabled":true,"max_facets":8}}}),
            budget,
        );
        let requested = facets
            .iter()
            .map(|facet| facet.requested_text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(facets.len(), 7, "{requested:?}");
        assert_eq!(
            facets.iter().filter(|facet| facet.kind == "entity").count(),
            3
        );
        assert_eq!(
            facets.iter().filter(|facet| facet.kind == "facet").count(),
            4
        );
        for expected in [
            "OpenAI Agents SDK",
            "LangChain",
            "LangGraph",
            "tool orchestration",
            "tracing",
            "safety controls",
            "vendor lock-in",
        ] {
            assert!(requested.contains(&expected), "{requested:?}");
        }
        assert!(
            !requested
                .iter()
                .any(|text| text.contains("OpenAI Agents SDK tool orchestration")),
            "{requested:?}"
        );
    }

    #[test]
    fn coverage_gap_recovery_spreads_budget_across_missing_facets() {
        let budget = aperture_budget("medium").expect("medium budget");
        let policy = json!({
            "batch_query": {
                "coverage_aware_evidence": {
                    "enabled": true,
                    "max_facets": 8
                },
                "coverage_gap_recovery": {
                    "enabled": true,
                    "max_queries": 4,
                    "min_usable_evidence": 3,
                    "min_covered_facets": 3,
                    "min_covered_facet_ratio": 0.75,
                    "templates": [
                        "{facet} source-backed evidence",
                        "{facet} primary or official source",
                        "{facet} independent analysis evidence",
                        "{facet} examples reports data"
                    ]
                }
            }
        });
        let metadata = BatchQueryKeywordPack {
            facets: vec![
                "LangChain".to_string(),
                "tool orchestration".to_string(),
                "tracing".to_string(),
                "safety controls".to_string(),
            ],
            metadata_authority: "tool_structured_from_user_query_terms".to_string(),
            ..BatchQueryKeywordPack::default()
        };
        let facets = infer_research_facets(
            "Compare frameworks for production agents.",
            &[],
            &metadata,
            &policy,
            budget,
        );
        let queries = coverage_gap_recovery_queries(
            &policy,
            "Compare frameworks for production agents.",
            &[],
            &facets,
            &[candidate(
                "https://example.org/noise",
                "Garden irrigation tips and unrelated seasonal watering advice.",
            )],
            budget,
        );
        assert_eq!(
            queries,
            vec![
                "LangChain source-backed evidence",
                "tool orchestration source-backed evidence",
                "tracing source-backed evidence",
                "safety controls source-backed evidence",
            ]
        );
    }

    #[test]
    fn coverage_gap_recovery_uses_compact_entity_context_when_declared() {
        let budget = aperture_budget("medium").expect("medium budget");
        let policy = json!({
            "batch_query": {
                "coverage_aware_evidence": {
                    "enabled": true,
                    "max_facets": 8
                },
                "coverage_gap_recovery": {
                    "enabled": true,
                    "max_queries": 4,
                    "min_usable_evidence": 3,
                    "min_covered_facets": 3,
                    "min_covered_facet_ratio": 1.0,
                    "templates": [
                        "{entities} {facet} official documentation",
                        "{query} {facet} source-backed evidence"
                    ]
                }
            }
        });
        let metadata = BatchQueryKeywordPack {
            entities: vec![
                "OpenAI Agents SDK".to_string(),
                "LangChain".to_string(),
                "LangGraph".to_string(),
            ],
            facets: vec![
                "tool orchestration".to_string(),
                "safety controls".to_string(),
            ],
            metadata_authority: "tool_structured_from_user_query_terms".to_string(),
            ..BatchQueryKeywordPack::default()
        };
        let facets = infer_research_facets(
            "Compare frameworks for production customer support agents.",
            &[],
            &metadata,
            &policy,
            budget,
        );
        let queries = coverage_gap_recovery_queries(
            &policy,
            "Compare frameworks for production customer support agents.",
            &[],
            &facets,
            &[
                candidate(
                    "https://example.org/openai-agents",
                    "OpenAI Agents SDK release notes for production agents.",
                ),
                candidate(
                    "https://example.org/langchain",
                    "LangChain platform documentation for production agents.",
                ),
                candidate(
                    "https://example.org/langgraph",
                    "LangGraph runtime documentation for production agents.",
                ),
            ],
            budget,
        );

        assert_eq!(
            queries.first().map(String::as_str),
            Some("\"OpenAI Agents SDK\" LangChain LangGraph tool orchestration official documentation"),
            "{queries:?}"
        );
        assert!(
            queries
                .iter()
                .any(|query| query.contains("safety controls official documentation")),
            "{queries:?}"
        );
        assert!(
            queries
                .iter()
                .take(2)
                .all(|query| !query.contains("Compare frameworks for production customer support agents")),
            "{queries:?}"
        );
    }

    #[test]
    fn two_word_non_entity_facets_require_more_than_one_generic_term() {
        let mut facets = vec![
            research_facet_from_metadata_text("Exa", 0, "entity").expect("entity facet"),
            research_facet_from_metadata_text("evidence gathering", 1, "facet")
                .expect("coverage facet"),
        ];
        assign_distinctive_facet_terms(&mut facets);
        let evidence_candidate = candidate(
            "https://example.org/evidence",
            "This article mentions evidence but does not discuss collection workflows.",
        );
        let exa_candidate = candidate("https://exa.ai/docs", "Exa search documentation.");

        assert!(candidate_matches_facet(&facets[0], &exa_candidate, 2));
        assert!(
            !candidate_matches_facet(&facets[1], &evidence_candidate, 2),
            "coverage facets should not be satisfied by one generic token"
        );
    }

    #[test]
    fn candidate_truncation_preserves_late_coverage_rows() {
        let mut facets = vec![
            research_facet_from_metadata_text("Firecrawl", 0, "entity").expect("firecrawl"),
            research_facet_from_metadata_text("Tavily", 1, "entity").expect("tavily"),
            research_facet_from_metadata_text("Exa", 2, "entity").expect("exa"),
            research_facet_from_metadata_text("evidence gathering", 3, "facet")
                .expect("coverage facet"),
        ];
        assign_distinctive_facet_terms(&mut facets);
        let mut candidates = (0..10)
            .map(|index| {
                candidate(
                    &format!("https://example.org/noise-{index}"),
                    "Generic search article with no requested product coverage.",
                )
            })
            .collect::<Vec<_>>();
        candidates.push(candidate(
            "https://docs.firecrawl.dev",
            "Firecrawl crawler documentation for web extraction.",
        ));
        candidates.push(candidate(
            "https://docs.tavily.com",
            "Tavily search API documentation for agent retrieval.",
        ));
        candidates.push(candidate(
            "https://docs.exa.ai",
            "Exa neural search documentation for agent retrieval.",
        ));
        candidates.push(candidate(
            "https://example.org/evidence-gathering",
            "Evidence gathering workflows for research agents.",
        ));

        truncate_candidates_preserving_facet_coverage(
            "Firecrawl Tavily Exa evidence gathering",
            &facets,
            &mut candidates,
            6,
            2,
        );
        let joined = candidates
            .iter()
            .map(|row| format!("{} {}", row.locator, row.snippet))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        for expected in ["firecrawl", "tavily", "exa", "evidence gathering"] {
            assert!(joined.contains(expected), "{joined}");
        }
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
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("agent_refine_query_pack")
        );
        assert_eq!(
            report
                .pointer("/retry/query_refinement_signals/hidden_query_generation")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn candidate_without_promoted_evidence_recommends_direct_fetch() {
        let report = web_tool_quality_report(
            "public agency science breakthrough report",
            "no_results",
            2,
            0,
            &[],
            &[],
            &[(
                candidate(
                    "https://agency.example.gov/reports/science-breakthroughs",
                    "Annual science breakthroughs report with program milestones and publication links.",
                ),
                0.71,
            )],
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("direct_fetch_candidate")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/inputs/candidate_url_state")
                .and_then(Value::as_str),
            Some("candidate_url_ref_available")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/candidate_refs/0/url_safety_status")
                .and_then(Value::as_str),
            Some("allowed_public_http_https")
        );
    }

    #[test]
    fn unsafe_candidate_url_blocks_browser_materialization_recommendation() {
        let report = web_tool_quality_report(
            "public agency science breakthrough report",
            "no_results",
            1,
            0,
            &["needs_js: please enable javascript before content renders".to_string()],
            &[],
            &[(
                candidate(
                    "http://user:pass@127.0.0.1/admin",
                    "Public agency science breakthrough report shell requiring JavaScript.",
                ),
                0.74,
            )],
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/decision")
                .and_then(Value::as_str),
            Some("alternate_provider")
        );
        assert_eq!(
            report
                .pointer("/retrieval_decision/inputs/candidate_url_state")
                .and_then(Value::as_str),
            Some("candidate_url_ref_blocked_by_safety")
        );
        assert_eq!(
            report
                .pointer("/browser_materialization/url_safety/materializable_candidate_count")
                .and_then(Value::as_u64),
            Some(0)
        );
        assert_eq!(
            report
                .pointer("/browser_materialization/url_safety/candidate_refs/0/url_safety/status")
                .and_then(Value::as_str),
            Some("blocked_internal_host_hint")
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
    fn comparison_partial_preserves_actionable_evidence_when_two_entities_are_covered() {
        let comparison_entities = vec![
            "Infring".to_string(),
            "LangGraph".to_string(),
            "CrewAI".to_string(),
            "AutoGen".to_string(),
        ];
        let actionable_ranked = vec![
            (
                candidate(
                    "https://docs.langchain.com/langgraph",
                    "LangGraph supports durable execution, observability, and human review.",
                ),
                0.91,
            ),
            (
                candidate(
                    "https://docs.crewai.com/overview",
                    "CrewAI offers multi-agent workflow coordination and deployment guides.",
                ),
                0.88,
            ),
        ];
        let retained_ranked = actionable_ranked.clone();
        assert!(comparison_partial_preserves_actionable_evidence(
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
        ));
    }

    #[test]
    fn comparison_partial_does_not_preserve_when_only_one_entity_is_covered() {
        let comparison_entities = vec!["LangGraph".to_string(), "CrewAI".to_string()];
        let actionable_ranked = vec![(
            candidate(
                "https://docs.langchain.com/langgraph",
                "LangGraph supports durable execution and checkpointing.",
            ),
            0.91,
        )];
        let retained_ranked = actionable_ranked.clone();
        assert!(!comparison_partial_preserves_actionable_evidence(
            &comparison_entities,
            &actionable_ranked,
            &retained_ranked,
        ));
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
