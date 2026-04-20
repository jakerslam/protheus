        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            root.path()
                .join("local/state/nursery/containment/permissions.json")
                .exists(),
            "expected nursery permissions to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/infring/cron/runs/example.jsonl")
                .exists(),
            "expected cron runs to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/infring/subagents/runs.json")
                .exists(),
            "expected subagent run state to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/infring/memory/main.sqlite")
                .exists(),
            "expected memory sqlite to be assimilated"
        );
        assert!(
            root.path()
                .join("local/state/assimilations/infring/agents/main/sessions/sessions.json")
                .exists(),
            "expected agent sessions index to be assimilated"
        );
        assert!(
            root.path()
                .join("config/infring_assimilation/agents/main/sessions/sessions.json")
                .exists(),
            "expected source-controlled sessions index mirror to be written"
        );
        assert!(
            root.path()
                .join("config/infring_assimilation/cron/jobs.json")
                .exists(),
            "expected source-controlled cron mirror to be written"
        );
        assert!(
            root.path()
                .join("config/infring_assimilation/nursery/manifests/seed_manifest.json")
                .exists(),
            "expected source-controlled nursery mirror to be written"
        );
        let policy = lane_utils::read_json(&policy_path).expect("read synced policy");
        assert_eq!(
            policy.get("root_dir").and_then(Value::as_str),
            Some("local/state/nursery")
        );
    }

    #[test]
    fn infring_detach_specialist_training_materializes_plan() {
        let root = runtime_temp_root();
        let source = root.path().join("legacy_infring_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":30}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tinyllama_seed\",\"provider\":\"ollama\",\"model\":\"tinyllama:1.1b\",\"required\":true},{\"id\":\"red_team_seed\",\"provider\":\"ollama\",\"model\":\"qwen2.5:3b\",\"required\":false}]}",
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
            "V6-INFRING-DETACH-001.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach specialist training should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let plan_path = root
            .path()
            .join("local/state/nursery/promotion/specialist_training_plan.json");
        assert!(plan_path.exists(), "expected specialist training plan");
        let plan = lane_utils::read_json(&plan_path).expect("read plan");
        let specialists = plan
            .get("specialists")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            specialists.len() >= 2,
            "expected specialists from seed manifest"
        );
    }

    #[test]
    fn infring_detach_source_control_mirror_contract_writes_expected_files() {
        let root = runtime_temp_root();
        let source = root.path().join("legacy_infring_home");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::create_dir_all(source.join("cron")).expect("mkdir cron");
        fs::create_dir_all(source.join("agents/main/sessions")).expect("mkdir agent sessions");
        fs::write(
            source.join("cron/jobs.json"),
            "{\"jobs\":[{\"id\":\"heartbeat\"}]}",
        )
        .expect("write jobs");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":35}",
        )
        .expect("write permissions");
        fs::write(
            source.join("nursery/containment/policy-gates.json"),
            "{\"execution_mode\":\"sandboxed\"}",
        )
        .expect("write gates");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"seed\",\"provider\":\"ollama\",\"model\":\"qwen2.5:7b\"}]}",
        )
        .expect("write seed manifest");
        fs::write(
            source.join("agents/main/sessions/sessions.json"),
            "{\"active_session\":\"alpha\"}",
        )
        .expect("write sessions index");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-INFRING-DETACH-001.3",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach source mirror should succeed");

