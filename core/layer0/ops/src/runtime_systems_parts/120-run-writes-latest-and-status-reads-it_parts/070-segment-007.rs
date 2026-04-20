        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            root.path()
                .join("config/infring_assimilation/cron/jobs.json")
                .exists(),
            "expected source-controlled cron jobs mirror"
        );
        assert!(
            root.path()
                .join("config/infring_assimilation/nursery/containment/permissions.json")
                .exists(),
            "expected source-controlled nursery containment mirror"
        );
        assert!(
            root.path()
                .join("config/infring_assimilation/agents/main/sessions/sessions.json")
                .exists(),
            "expected source-controlled agent session index mirror"
        );
    }

    #[test]
    fn infring_detach_llm_registry_materializes_ranked_models() {
        let root = runtime_temp_root();
        let source = root.path().join("legacy_infring_home");
        fs::create_dir_all(source.join("nursery/manifests")).expect("mkdir manifests");
        fs::write(
            source.join("nursery/manifests/seed_manifest.json"),
            "{\"artifacts\":[{\"id\":\"tiny\",\"provider\":\"ollama\",\"model\":\"qwen2.5-coder:3b\"},{\"id\":\"big\",\"provider\":\"openai\",\"model\":\"gpt-5.4-128k\"}]}",
        )
        .expect("write seed manifest");
        fs::create_dir_all(source.join("nursery/containment")).expect("mkdir containment");
        fs::write(
            source.join("nursery/containment/permissions.json"),
            "{\"max_train_minutes\":30}",
        )
        .expect("write permissions");
        let policy_path = root
            .path()
            .join("client/runtime/config/nursery_policy.json");
        fs::create_dir_all(policy_path.parent().expect("policy parent")).expect("mkdir policy");
        fs::write(&policy_path, "{\"version\":\"1.0\",\"containment\":{}}").expect("write policy");

        let payload = json!({ "source_root": source.display().to_string(), "max_assimilation_copy_mb": 2048 });
        let out = run_payload(
            root.path(),
            "V6-INFRING-DETACH-001.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                format!("--payload-json={}", payload),
            ],
        )
        .expect("detach llm registry should succeed");

        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let registry_path = root
            .path()
            .join("local/state/llm_runtime/model_registry.json");
        assert!(registry_path.exists(), "expected llm runtime registry");
        let registry = lane_utils::read_json(&registry_path).expect("read llm registry");
        let models = registry
            .get("models")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            models.len() >= 2,
            "expected llm model registry rows from seed manifest"
        );
        let power_values = models
            .iter()
            .filter_map(|row| row.get("power_score_1_to_5").and_then(Value::as_u64))
            .collect::<Vec<_>>();
        assert!(power_values.contains(&1));
        assert!(power_values.contains(&5));
    }
}
