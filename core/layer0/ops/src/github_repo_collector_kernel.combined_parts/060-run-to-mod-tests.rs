
pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.is_empty() || matches!(argv[0].as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let command = argv[0].trim().to_ascii_lowercase();
    let payload = match lane_utils::payload_json(&argv[1..], "github_repo_collector_kernel") {
        Ok(value) => value,
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "github_repo_collector_kernel_error",
                &err,
            ));
            return 1;
        }
    };
    let payload_obj = payload_obj(&payload);

    match dispatch(root, &command, payload_obj) {
        Ok(result) => {
            lane_utils::print_json_line(&json!({ "ok": true, "payload": result }));
            0
        }
        Err(err) => {
            lane_utils::print_json_line(&lane_utils::cli_error(
                "github_repo_collector_kernel_error",
                &err,
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        let nonce = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        root.push(format!("infring_github_repo_kernel_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir temp root");
        root
    }

    #[test]
    fn file_risk_flags_detects_security_and_schema() {
        let rows = vec![
            json!({"filename": "src/security/auth.rs", "changes": 50}),
            json!({"filename": "schema/migrations/2026.sql", "changes": 20}),
        ];
        let flags = support::file_risk_flags(&rows);
        let vals = flags.iter().filter_map(Value::as_str).collect::<Vec<_>>();
        assert!(vals.contains(&"security_sensitive_paths"));
        assert!(vals.contains(&"schema_or_data_migration"));
    }

    #[test]
    fn resolve_run_params_validates_owner_repo_and_mode() {
        let missing = handle_resolve_run_params(payload_obj(&json!({"owner":"", "repo":"demo"})));
        assert_eq!(missing.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            missing.get("error").and_then(Value::as_str),
            Some("missing_owner_or_repo")
        );

        let pr_mode = handle_resolve_run_params(payload_obj(&json!({
            "owner":"acme",
            "repo":"demo",
            "pr": 42,
            "max_items": 999,
            "timeout_ms": 10
        })));
        assert_eq!(pr_mode.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            pr_mode.get("mode").and_then(Value::as_str),
            Some("pr_review")
        );
        assert_eq!(pr_mode.get("max_items").and_then(Value::as_u64), Some(50));
        assert_eq!(
            pr_mode.get("timeout_ms").and_then(Value::as_u64),
            Some(1000)
        );
    }

    #[test]
    fn prepare_repo_activity_respects_cadence() {
        let root = temp_root("cadence");
        let payload = json!({"owner":"acme","repo":"demo","min_hours":4.0,"force":false});
        let key = support::cache_key("acme", "demo");
        let fp = support::cache_path(&root, payload_obj(&payload), &key);
        support::save_cache(
            &fp,
            &json!({"last_run": support::now_iso(), "seen_ids": []}),
        )
        .expect("save cache");

        let out = handle_prepare_repo_activity(&root, payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("skipped").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("cadence"));
    }

    #[test]
    fn build_fetch_plans_emit_expected_keys() {
        let repo_plan = handle_build_repo_activity_fetch_plan(payload_obj(&json!({
            "owner": "acme",
            "repo": "demo"
        })));
        let repo_keys = repo_plan
            .get("requests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.get("key").and_then(Value::as_str).map(str::to_string))
            .collect::<Vec<_>>();
        assert!(repo_keys.contains(&"release".to_string()));
        assert!(repo_keys.contains(&"commits".to_string()));
        assert!(repo_keys.contains(&"pulls".to_string()));

        let pr_plan = handle_build_pr_review_fetch_plan(payload_obj(&json!({
            "owner": "acme",
            "repo": "demo",
            "pr": 42
        })));
        let pr_keys = pr_plan
            .get("requests")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.get("key").and_then(Value::as_str).map(str::to_string))
            .collect::<Vec<_>>();
        assert_eq!(pr_keys, vec!["pr".to_string(), "files".to_string()]);
    }

    #[test]

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

