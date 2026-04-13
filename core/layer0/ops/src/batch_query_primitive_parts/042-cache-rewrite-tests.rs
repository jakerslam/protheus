mod cache_rewrite_tests {
    use super::*;
    use std::sync::Mutex;

    static CACHE_REWRITE_TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = CACHE_REWRITE_TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_request_with_fixture(fixture: Value, request: &Value) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || api_batch_query(tmp.path(), request))
    }

    #[test]
    fn cached_framework_summary_is_rewritten_from_evidence_refs_on_hit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let query = "top AI agentic frameworks";
        let key = cache_key_with_query_plan(
            "web",
            query,
            "medium",
            &policy,
            &[
                "top AI agentic frameworks".to_string(),
                "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                    .to_string(),
            ],
        );
        let now_ts = chrono::Utc::now().timestamp();
        let payload = json!({
            "version": 1,
            "entries": {
                key: {
                    "stored_at": now_ts,
                    "expires_at": now_ts + 120,
                    "response": {
                        "status": "ok",
                        "summary": "Key findings: langchain.com: LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain",
                        "evidence_refs": [
                            {"title":"Web result from langchain.com","locator":"https://www.langchain.com/langgraph","score":0.78},
                            {"title":"Web result from crewai.com","locator":"https://crewai.com/","score":0.66}
                        ],
                        "rewrite_set": ["AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"],
                        "query_plan": [
                            "top AI agentic frameworks",
                            "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                        ],
                        "query_plan_source": "explicit_request_pack",
                        "parallel_retrieval_used": true,
                        "partial_failure_details": []
                    }
                }
            }
        });
        write_json_atomic(&cache_path(tmp.path()), &payload).expect("write cache");

        let out = api_batch_query(
            tmp.path(),
            &json!({
                "source":"web",
                "query": query,
                "queries": [
                    "top AI agentic frameworks",
                    "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                ],
                "aperture":"medium"
            }),
        );

        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        let lowered = summary.to_ascii_lowercase();
        assert!(lowered.contains("langgraph"), "{summary}");
        assert!(lowered.contains("crewai"), "{summary}");
        assert!(!lowered.contains("zhihu.com"), "{summary}");
    }

    #[test]
    fn framework_catalog_query_plan_preserves_official_domain_queries() {
        let payload = json!({
            "source": "web",
            "query": "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
            "queries": [
                "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
                "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
                "site:langchain.com LangGraph agent framework overview",
                "site:openai.github.io/openai-agents-python OpenAI Agents SDK overview",
                "site:crewai.com CrewAI agent framework overview",
                "site:microsoft.github.io AutoGen framework overview",
                "site:github.com huggingface/smolagents smolagents framework overview",
                "OpenAI Agents SDK official docs overview"
            ],
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(&payload, &query, aperture_budget("medium").expect("budget"));
        assert_eq!(plan.query_plan_source, "explicit_request_pack");
        assert!(plan.queries.len() >= 8, "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:openai.github.io/openai-agents-python")), "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:microsoft.github.io")), "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:github.com huggingface/smolagents")), "{:?}", plan.queries);
    }

    #[test]
    fn derived_framework_catalog_query_plan_expands_official_domain_queries() {
        let payload = json!({
            "source": "web",
            "query": "top AI agentic frameworks",
            "aperture": "medium"
        });
        let query = request_query_text(&payload, 600);
        let plan = resolve_query_plan(&payload, &query, aperture_budget("medium").expect("budget"));
        assert_eq!(plan.query_plan_source, "derived_rewrite");
        assert!(plan.queries.len() >= 6, "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:openai.github.io/openai-agents-python")), "{:?}", plan.queries);
        assert!(plan
            .queries
            .iter()
            .any(|row| row.contains("site:crewai.com")), "{:?}", plan.queries);
    }

    #[test]
    fn framework_catalog_source_adjustment_penalizes_support_noise() {
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: "Contact Us - Microsoft Support".to_string(),
            locator: "https://support.microsoft.com/en-us/contactus".to_string(),
            snippet: "Contact Microsoft Support. Find solutions to common problems, or get help from a support agent.".to_string(),
            excerpt_hash: "support-noise".to_string(),
            timestamp: None,
            permissions: None,
            status_code: 200,
        };
        assert!(framework_catalog_source_adjustment(&candidate) < 0.0);
    }

    #[test]
    fn framework_catalog_source_adjustment_penalizes_mirror_domains() {
        let candidate = Candidate {
            source_kind: "web".to_string(),
            title: "LangGraph - LangChain Framework".to_string(),
            locator: "https://langgraph.com.cn/index.html".to_string(),
            snippet: "LangGraph mirror documentation in Chinese.".to_string(),
            excerpt_hash: "mirror-noise".to_string(),
            timestamp: None,
            permissions: None,
            status_code: 200,
        };
        assert!(framework_catalog_source_adjustment(&candidate) < 0.0);
    }

    #[test]
    fn rendered_search_payload_extracts_multiple_framework_candidates() {
        let payload = json!({
            "ok": true,
            "content": concat!(
                "LangGraph: Agent Orchestration Framework for Reliable AI Agents — https://www.langchain.com/langgraph — LangGraph sets the foundation for reliable agent workflows.\n",
                "OpenAI Agents SDK overview — https://openai.github.io/openai-agents-python/ — OpenAI Agents SDK helps build tool-using agents.\n",
                "crewAI — https://crewai.com/ — CrewAI enables multiple agents to collaborate on tasks."
            ),
            "status_code": 200
        });
        let candidates =
            candidates_from_rendered_search_payload("top AI agentic frameworks", &payload, 4);
        assert!(candidates.len() >= 3, "{candidates:?}");
        let joined = candidates
            .iter()
            .map(|row| format!("{} {}", row.title, row.locator))
            .collect::<Vec<_>>()
            .join(" | ")
            .to_ascii_lowercase();
        assert!(joined.contains("langchain.com"), "{joined}");
        assert!(joined.contains("openai.github.io"), "{joined}");
        assert!(joined.contains("crewai.com"), "{joined}");
    }

    #[test]
    fn batch_query_synthesizes_multiple_frameworks_from_single_search_payload() {
        let out = run_request_with_fixture(
            json!({
                "top AI agentic frameworks": {
                    "ok": true,
                    "summary": "top ai agentic frameworks official docs",
                    "content": concat!(
                        "LangGraph: Agent Orchestration Framework for Reliable AI Agents — https://www.langchain.com/langgraph — LangGraph sets the foundation for reliable agent workflows.\n",
                        "OpenAI Agents SDK overview — https://openai.github.io/openai-agents-python/ — OpenAI Agents SDK helps build tool-using agents.\n",
                        "crewAI — https://crewai.com/ — CrewAI enables multiple agents to collaborate on tasks."
                    ),
                    "links": [
                        "https://www.langchain.com/langgraph",
                        "https://openai.github.io/openai-agents-python/",
                        "https://crewai.com/"
                    ],
                    "requested_url": "https://example.com/frameworks",
                    "status_code": 200
                }
            }),
            &json!({
                "source":"web",
                "query":"top AI agentic frameworks",
                "aperture":"medium"
            }),
        );
        let lowered = out
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(lowered.contains("langgraph"), "{lowered}");
        assert!(lowered.contains("openai agents sdk"), "{lowered}");
        assert!(lowered.contains("crewai"), "{lowered}");
    }
}
