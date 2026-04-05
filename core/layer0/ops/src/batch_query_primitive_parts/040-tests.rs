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
            || api_batch_query(tmp.path(), &json!({"source":"web","query":"agent systems","aperture":"small"})),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("ok"));
        assert!(
            out.get("evidence_refs")
                .and_then(Value::as_array)
                .map(|rows| !rows.is_empty())
                .unwrap_or(false)
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(!summary.to_ascii_lowercase().contains("web search completed"));
    }

    #[test]
    fn no_results_path_returns_clean_no_results_status() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = with_fixture(
            json!({"batch query no results":{"ok":false,"error":"provider_network_policy_blocked"}}),
            || api_batch_query(tmp.path(), &json!({"source":"web","query":"batch query no results","aperture":"small"})),
        );
        assert_eq!(out.get("status").and_then(Value::as_str), Some("no_results"));
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
            || api_batch_query(tmp.path(), &json!({"source":"web","query":"\"agent::run\"","aperture":"medium"})),
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
            || api_batch_query(tmp.path(), &json!({"source":"web","query":"ack leak","aperture":"small"})),
        );
        let summary = out.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(!summary.to_ascii_lowercase().contains("web search completed"));
    }
}
