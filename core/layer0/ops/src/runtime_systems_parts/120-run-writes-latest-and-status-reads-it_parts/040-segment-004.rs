    #[test]
    fn v6_dashboard_contract_enforcement_respects_auto_terminate_allowed() {
        let root = runtime_temp_root();
        let out = run_payload(
            root.path(),
            "V6-DASHBOARD-007.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=0".to_string(),
                "--payload-json={\"contracts\":[{\"agent_id\":\"main-agent\",\"status\":\"active\",\"auto_terminate_allowed\":false,\"termination_condition\":\"task_or_timeout\",\"remaining_ms\":0,\"idle_for_ms\":900000},{\"agent_id\":\"worker-agent\",\"status\":\"active\",\"auto_terminate_allowed\":true,\"termination_condition\":\"task_or_timeout\",\"remaining_ms\":0,\"idle_for_ms\":900000}],\"idle_threshold\":1,\"idle_termination_ms\":1000,\"idle_batch\":4,\"idle_batch_max\":8,\"idle_since_last_ms\":180000}".to_string(),
            ],
        )
        .expect("dashboard contract enforcement should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let enforcement = out
            .get("contract_execution")
            .and_then(Value::as_object)
            .and_then(|row| row.get("specific_checks"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("dashboard_runtime_authority"))
            .and_then(Value::as_object)
            .and_then(|row| row.get("contract_enforcement"))
            .and_then(Value::as_object)
            .cloned()
            .expect("expected contract_enforcement object");
        let terminations = enforcement
            .get("termination_decisions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            terminations.iter().any(|row| {
                row.get("agent_id").and_then(Value::as_str) == Some("worker-agent")
                    && row.get("reason").and_then(Value::as_str) == Some("timeout")
            }),
            "expected worker-agent timeout termination from rust authority"
        );
        assert!(
            !terminations
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("main-agent")),
            "main-agent should be excluded when auto_terminate_allowed=false"
        );

        let idle_candidates = enforcement
            .get("idle_candidates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            idle_candidates
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("worker-agent")),
            "expected worker-agent idle candidate"
        );
        assert!(
            !idle_candidates
                .iter()
                .any(|row| row.get("agent_id").and_then(Value::as_str) == Some("main-agent")),
            "main-agent must not be present in idle candidates when auto_terminate_allowed=false"
        );
    }

    #[test]
    fn new_v6_contract_families_execute_and_emit_artifacts() {
        let root = runtime_temp_root();
        for id in [
            "V6-EXECUTION-002.1",
            "V6-EXECUTION-003.1",
            "V6-ASSIMILATE-FAST-001.1",
            "V6-WORKFLOW-028.1",
            "V6-MEMORY-CONTEXT-001.1",
            "V6-INTEGRATION-001.1",
            "V6-INFERENCE-005.1",
            "V6-RUNTIME-CLEANUP-001.1",
            "V6-ERP-AGENTIC-001.1",
            "V6-TOOLING-001.1",
            "V6-WORKFLOW-029.1",
        ] {
            let out = run_payload(
                root.path(),
                id,
                "run",
                &["--strict=1".to_string(), "--apply=1".to_string()],
            )
            .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            assert_eq!(
                out.get("contract_profile")
                    .and_then(Value::as_object)
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str),
                Some(id)
            );
        }
    }

    #[test]
    fn execution_worktree_merge_requires_human_veto_in_strict_mode() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V6-EXECUTION-003.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"conflicts\":[\"src/main.rs\"]}".to_string(),
            ],
        )
        .expect_err("strict merge conflict should require veto");
        assert!(
            err.contains("execution_worktree_merge_conflict_requires_human_veto"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn inference_failover_contract_fails_when_sequence_never_succeeds() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V6-INFERENCE-005.3",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"fail_sequence\":[\"timeout\",\"429\",\"500\"]}".to_string(),
            ],
        )
        .expect_err("strict failover should fail when no success step");
        assert!(err.contains("inference_failover_exhausted"));
    }

    #[test]
    fn runtime_cleanup_removes_stale_files_and_tracks_freed_bytes() {
        let root = runtime_temp_root();
        let cleanup_dir = root
            .path()
            .join("client")
            .join("local")
            .join("state")
            .join("runtime_cleanup")
            .join("staging_queues");
        fs::create_dir_all(&cleanup_dir).expect("mkdir cleanup");
        let stale = cleanup_dir.join("stale.tmp");
        fs::write(&stale, "x".repeat(2048)).expect("write stale");
        let out = run_payload(
            root.path(),
            "V6-RUNTIME-CLEANUP-001.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--apply=1".to_string(),
                "--payload-json={\"disk_free_percent\":1.0,\"memory_percent\":95.0}".to_string(),
            ],
        )
        .expect("cleanup run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(
            !stale.exists(),
            "stale cleanup file should be removed under emergency mode"
        );
    }

