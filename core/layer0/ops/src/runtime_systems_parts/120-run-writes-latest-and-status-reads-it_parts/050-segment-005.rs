    #[test]
    fn roi_sweep_defaults_to_400_and_orders_by_roi_score() {
        let root = runtime_temp_root();
        let out = roi_sweep_payload(root.path(), &[]).expect("roi sweep should run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("limit_requested").and_then(Value::as_u64),
            Some(400)
        );
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(400));
        let executed = out
            .get("executed")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(executed.len(), 400);
        let mut prev = i64::MAX;
        for row in executed {
            let score = row.get("roi_score").and_then(Value::as_i64).unwrap_or(0);
            assert!(score <= prev, "roi scores should be descending");
            prev = score;
        }
    }

    #[test]
    fn roi_sweep_respects_limit_and_read_only_apply_flag() {
        let root = runtime_temp_root();
        let out = roi_sweep_payload(
            root.path(),
            &[
                "--limit=7".to_string(),
                "--apply=0".to_string(),
                "--strict=1".to_string(),
            ],
        )
        .expect("roi sweep should run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("selected_count").and_then(Value::as_u64), Some(7));
        assert_eq!(out.get("apply").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn infring_detach_bootstrap_assimilates_nursery_and_rewrites_policy_root() {
        let root = runtime_temp_root();
        let source = root.path().join("legacy_infring_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::create_dir_all(source.join("cron")).expect("mkdir cron");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::create_dir_all(source.join("cron/runs")).expect("mkdir cron runs");
        fs::create_dir_all(source.join("subagents")).expect("mkdir subagents");
        fs::create_dir_all(source.join("memory")).expect("mkdir memory");
        fs::create_dir_all(source.join("local/state/sensory/eyes")).expect("mkdir eyes");
        fs::create_dir_all(source.join("client/local/memory")).expect("mkdir client local memory");
        fs::create_dir_all(source.join("agents/main/agent")).expect("mkdir agent main");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::write(source.join("infring.json"), "{\"ok\":true}").expect("write infring.json");
        fs::write(source.join("cron/jobs.json"), "{\"jobs\":[]}").expect("write jobs");
        fs::write(
            source.join("cron/runs/example.jsonl"),
            "{\"ts\":\"2026-03-24T00:00:00Z\",\"status\":\"ok\"}\n",
        )
        .expect("write cron run");
        fs::write(source.join("subagents/runs.json"), "{\"runs\":[]}")
            .expect("write subagent runs");
        fs::write(source.join("memory/main.sqlite"), "sqlite-bytes").expect("write memory sqlite");
        fs::write(
            source.join("agents/main/agent/state.json"),
            "{\"status\":\"ready\"}",
        )
        .expect("write agent state");
        fs::write(
            source.join("agents/main/agent/models.json"),
            "{\"provider\":\"ollama\"}",
        )
        .expect("write agent models");
        fs::write(
            source.join("agents/main/agent/routing-policy.json"),
            "{\"default\":\"local\"}",
        )
        .expect("write agent routing policy");
        fs::write(
            source.join("agents/main/sessions/sessions.json"),
            "{\"active_session\":\"abc\",\"sessions\":[\"abc\"]}",
        )
        .expect("write sessions index");
        fs::write(
            source.join("agents/main/sessions/abc.jsonl"),
            "{\"ts\":\"2026-03-24T00:00:00Z\",\"role\":\"user\",\"content\":\"hi\"}\n",
        )
        .expect("write session transcript");
        fs::write(
            source.join("local/state/sensory/eyes/collector_rate_state.json"),
            "{\"rates\":[]}",
        )
        .expect("write collector rate state");
        fs::write(
            source.join("client/local/memory/.rebuild_delta_cache.json"),
            "{\"delta\":0}",
        )
        .expect("write rebuild delta cache");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":25}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/containment/policy-gates.json"),
            "{\"execution_mode\":\"sandboxed\"}",
        )
        .expect("write policy gates");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tiny\",\"provider\":\"ollama\",\"model\":\"tinyllama\",\"required\":true}]}",
        )
        .expect("write seed manifest");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-INFRING-DETACH-001.1",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach bootstrap should succeed");

