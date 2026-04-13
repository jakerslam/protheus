mod quality_tests {
    use super::*;
    use std::sync::Mutex;

    static QUALITY_TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _guard = QUALITY_TEST_ENV_MUTEX.lock().expect("lock");
        let _fixture = ScopedEnvVar::set(
            "INFRING_BATCH_QUERY_TEST_FIXTURE_JSON",
            &serde_json::to_string(&fixture).expect("encode fixture"),
        );
        run()
    }

    fn run_query(root: &Path, query: &str, aperture: &str) -> Value {
        api_batch_query(
            root,
            &json!({
                "source":"web",
                "query": query,
                "aperture": aperture
            }),
        )
    }

    fn run_query_with_fixture(fixture: Value, query: &str, aperture: &str) -> Value {
        let tmp = tempfile::tempdir().expect("tempdir");
        with_fixture(fixture, || run_query(tmp.path(), query, aperture))
    }

    fn summary_lowered(out: &Value) -> String {
        out.get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase()
    }

    #[test]
    fn comparison_guard_summary_marks_retrieval_quality_miss() {
        let out = run_query_with_fixture(
            json!({
                "compare infring vs openclaw": {
                    "ok": true,
                    "summary": "OpenClaw overview and architecture notes without side-by-side comparison details.",
                    "requested_url": "https://example.com/openclaw-overview",
                    "status_code": 200
                }
            }),
            "compare infring vs openclaw",
            "medium",
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("retrieval-quality miss"));
        assert!(lowered.contains("not proof the systems are equivalent"));
    }

    #[test]
    fn cached_placeholder_summary_is_rewritten_to_actionable_low_signal_guidance() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let key = cache_key("web", "top AI agent frameworks", "medium", &policy);
        let now_ts = chrono::Utc::now().timestamp();
        let payload = json!({
            "version": 1,
            "entries": {
                key: {
                    "stored_at": now_ts,
                    "expires_at": now_ts + 120,
                    "response": {
                        "status": "no_results",
                        "summary": "Search returned no useful information.",
                        "evidence_refs": [],
                        "rewrite_set": [],
                        "parallel_retrieval_used": true,
                        "partial_failure_details": [
                            "top ai agent frameworks overview:primary:fetch_candidate_low_relevance"
                        ]
                    }
                }
            }
        });
        write_json_atomic(&cache_path(tmp.path()), &payload).expect("write cache");

        let out = api_batch_query(
            tmp.path(),
            &json!({
                "source":"web",
                "query":"top AI agent frameworks",
                "aperture":"medium"
            }),
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("catalog-style framework evidence"));
        assert!(!lowered.contains("search returned no useful information"));
    }

    #[test]
    fn cached_generic_no_findings_placeholder_is_rewritten_for_web_hits() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let key = cache_key("web", "top AI agentic frameworks", "medium", &policy);
        let now_ts = chrono::Utc::now().timestamp();
        let payload = json!({
            "version": 1,
            "entries": {
                key: {
                    "stored_at": now_ts,
                    "expires_at": now_ts + 120,
                    "response": {
                        "status": "no_results",
                        "summary": crate::tool_output_match_filter::no_findings_user_copy(),
                        "evidence_refs": [],
                        "rewrite_set": ["top AI agentic frameworks overview"],
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
                "query":"top AI agentic frameworks",
                "aperture":"medium"
            }),
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("catalog-style framework evidence"));
        assert!(!lowered.contains("usable tool findings from this turn yet"));
        assert_eq!(out.get("cache_status").and_then(Value::as_str), Some("hit"));
    }

    #[test]
    fn cached_comparison_placeholder_is_rewritten_for_local_subject_queries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = load_policy(tmp.path());
        let key = cache_key("web", "compare this system to openclaw", "medium", &policy);
        let now_ts = chrono::Utc::now().timestamp();
        let payload = json!({
            "version": 1,
            "entries": {
                key: {
                    "stored_at": now_ts,
                    "expires_at": now_ts + 120,
                    "response": {
                        "status": "no_results",
                        "summary": "Search returned no useful comparison findings for infring vs openclaw.",
                        "evidence_refs": [],
                        "rewrite_set": ["compare infring to openclaw overview"],
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
                "query":"compare this system to openclaw",
                "aperture":"medium"
            }),
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("web retrieval alone cannot compare this local workspace/system"));
        assert!(lowered.contains("workspace analysis"));
        assert!(!lowered.contains("search returned no useful comparison findings"));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("local_subject_requires_workspace_analysis")
        );
    }

    #[test]
    fn framework_catalog_query_uses_comparison_rewrite_and_synthesizes_ranked_frameworks() {
        let query = "top AI agentic frameworks";
        let out = run_query_with_fixture(
            json!({
                query: {
                    "ok": true,
                    "summary": "top ai agentic frameworks at DuckDuckGo All Regions Safe Search Any Time",
                    "content": "",
                    "requested_url": "https://duckduckgo.com/html/?q=top+AI+agentic+frameworks",
                    "status_code": 200
                },
                "ai agentic frameworks comparison": {
                    "ok": true,
                    "summary": "LangGraph, OpenAI Agents SDK, and AutoGen are widely used AI agentic frameworks for orchestrating tool-using agents.",
                    "requested_url": "https://example.com/agent-framework-comparison",
                    "status_code": 200
                }
            }),
            query,
            "medium",
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        let rewrite_set = out
            .get("rewrite_set")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let rewrites = rewrite_set
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        assert!(rewrites.iter().any(|row| *row == "ai agentic frameworks comparison"));
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("langgraph"));
        assert!(lowered.contains("openai agents sdk"));
        assert!(lowered.contains("autogen"));
    }

    #[test]
    fn local_subject_comparison_query_returns_workspace_guidance_before_web_retrieval() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = api_batch_query(
            tmp.path(),
            &json!({
                "source":"web",
                "query":"compare this system to openclaw",
                "aperture":"medium"
            }),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("local_subject_requires_workspace_analysis")
        );
        let lowered = summary_lowered(&out);
        assert!(lowered.contains("workspace analysis"));
        assert!(lowered.contains("web retrieval"));
        assert!(!lowered.contains("no useful comparison findings"));
    }
}
