    fn collect_repo_activity_returns_skip_payload_when_cadence_blocks() {
        let root = temp_root("collect_skip");
        let payload = json!({
            "owner":"acme",
            "repo":"demo",
            "min_hours":100.0,
            "force":false
        });
        let key = support::cache_key("acme", "demo");
        let fp = support::cache_path(&root, payload_obj(&payload), &key);
        support::save_cache(
            &fp,
            &json!({"last_run": support::now_iso(), "seen_ids": []}),
        )
        .expect("save cache");
        let out = handle_collect_repo_activity(&root, payload_obj(&payload)).expect("collect repo");
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn collect_pr_review_rejects_missing_required_payload() {
        let out = handle_collect_pr_review(payload_obj(&json!({
            "owner":"acme",
            "repo":"demo",
            "pr": 42
        })));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("missing_required_pr_payload")
        );
    }
}
