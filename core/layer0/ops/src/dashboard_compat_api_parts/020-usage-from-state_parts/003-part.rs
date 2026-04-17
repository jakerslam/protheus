    fn memory_hygiene_flags_snapshot_history_bloat() {
        let temp = tempdir().expect("tempdir");
        let snapshot_path = temp
            .path()
            .join("client/runtime/local/state/ui/infring_dashboard/snapshot_history.jsonl");
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent).expect("mkdirs");
        }
        fs::write(&snapshot_path, vec![b'x'; 101 * 1024 * 1024]).expect("write large snapshot");

        let out = proactive_telemetry_alerts_payload(
            temp.path(),
            &json!({
                "ok": true,
                "health": {
                    "dashboard_metrics": {
                        "queue_depth": { "value": 0 }
                    },
                    "alerts": { "count": 0 }
                }
            }),
        );
        assert_eq!(
            out.pointer("/memory_hygiene/snapshot_history_over_soft_cap")
                .and_then(Value::as_bool),
            Some(true)
        );
        let alert_rows = out
            .get("alerts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = alert_rows
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"snapshot_history_bloat"));
    }

    #[test]
    fn dashboard_runtime_version_info_prefers_latest_git_tag_over_stale_contract_files() {
        let temp = tempdir().expect("tempdir");
        write_json(
            &temp.path().join("package.json"),
            &json!({
                "version": "0.2.1-alpha.1"
            }),
        );
        write_json(
            &temp
                .path()
                .join("client/runtime/config/runtime_version.json"),
            &json!({
                "version": "0.2.1-alpha.1",
                "tag": "v0.2.1-alpha.1",
                "source": "runtime_version_contract"
            }),
        );
        fs::write(temp.path().join("README.md"), "demo\n").expect("write readme");
        run_git(temp.path(), &["init"]);
        run_git(temp.path(), &["config", "user.email", "tests@example.com"]);
        run_git(temp.path(), &["config", "user.name", "Dashboard Tests"]);
        run_git(temp.path(), &["add", "."]);
        run_git(temp.path(), &["commit", "-m", "test repo"]);
        run_git(temp.path(), &["tag", "v0.3.10-alpha"]);

        let payload = dashboard_runtime_version_info(temp.path());
        assert_eq!(
            payload.get("version").and_then(Value::as_str),
            Some("0.3.10-alpha")
        );
        assert_eq!(
            payload.get("tag").and_then(Value::as_str),
            Some("v0.3.10-alpha")
        );
        assert_eq!(
            payload.get("source").and_then(Value::as_str),
            Some("git_latest_tag")
        );
    }
}
