    #[test]
    fn strict_mode_rejects_unknown_contract_ids() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V8-UNKNOWN-404.1",
            "run",
            &["--strict=1".to_string()],
        )
        .expect_err("unknown contract should fail");
        assert!(
            err.contains("unknown_runtime_contract_id"),
            "expected strict unknown id error, got {err}"
        );
    }

    #[test]
    fn manifest_exposes_actionable_contract_registry() {
        let out = manifest_payload();
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("counts")
                .and_then(Value::as_object)
                .and_then(|m| m.get("contracts"))
                .and_then(Value::as_u64),
            Some(actionable_ids().len() as u64)
        );
    }

    #[test]
    fn actionable_contract_ids_emit_profile_and_receipts() {
        let root = runtime_temp_root();
        for &id in actionable_ids() {
            let out = run_payload(
                root.path(),
                id,
                "run",
                &["--strict=1".to_string(), "--apply=0".to_string()],
            )
            .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            assert_eq!(
                out.get("contract_profile")
                    .and_then(Value::as_object)
                    .and_then(|m| m.get("id"))
                    .and_then(Value::as_str),
                Some(id)
            );
            let has_claim = out
                .get("claim_evidence")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .any(|row| row.get("id").and_then(Value::as_str) == Some(id))
                })
                .unwrap_or(false);
            assert!(has_claim, "missing contract claim evidence for {id}");
        }
    }

    #[test]
    fn v5_contract_families_persist_stateful_artifacts() {
        let root = runtime_temp_root();
        for id in ["V5-HOLD-001", "V5-RUST-HYB-001", "V5-RUST-PROD-001"] {
            let out = run_payload(
                root.path(),
                id,
                "run",
                &["--strict=1".to_string(), "--apply=1".to_string()],
            )
            .expect("contract run should succeed");
            assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
            let artifacts = out
                .get("artifacts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            assert!(
                !artifacts.is_empty(),
                "contract artifacts should be emitted"
            );
            let state_file = artifacts[0].as_str().unwrap_or_default().to_string();
            assert!(
                root.path().join(state_file).exists(),
                "expected contract state artifact to exist"
            );
        }
    }

    #[test]
    fn v9_audit_contract_family_persists_state_and_claims() {
        let root = runtime_temp_root();
        let out = run_payload(
            root.path(),
            "V9-AUDIT-026.1",
            "run",
            &["--strict=1".to_string(), "--apply=1".to_string()],
        )
        .expect("contract run should succeed");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("contract_profile")
                .and_then(Value::as_object)
                .and_then(|m| m.get("family"))
                .and_then(Value::as_str),
            Some("audit_self_healing_stack")
        );
        let artifacts = out
            .get("artifacts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!artifacts.is_empty());
        let state_file = artifacts[0].as_str().unwrap_or_default().to_string();
        assert!(root.path().join(state_file).exists());
    }

    #[test]
    fn v9_audit_contract_family_fails_closed_on_threshold_violation() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"verification_agents\":1,\"poll_interval_minutes\":30}"
                    .to_string(),
            ],
        )
        .expect_err("strict threshold violation should fail");
        assert!(
            err.contains("family_contract_gate_failed"),
            "expected family gate failure, got {err}"
        );
    }

    #[test]
    fn v9_audit_self_healing_requires_all_actions() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.2",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"self_healing_actions\":[\"refresh_spine_receipt\"]}".to_string(),
            ],
        )
        .expect_err("strict missing self-healing actions should fail");
        assert!(
            err.contains("specific_missing_self_healing_actions"),
            "expected self-healing action gate failure, got {err}"
        );
    }

    #[test]
    fn v9_audit_cross_agent_requires_strict_consensus_mode() {
        let root = runtime_temp_root();
        let err = run_payload(
            root.path(),
            "V9-AUDIT-026.4",
            "run",
            &[
                "--strict=1".to_string(),
                "--payload-json={\"consensus_mode\":\"weighted\"}".to_string(),
            ],
        )
        .expect_err("strict non-matching consensus mode should fail");
        assert!(
            err.contains("specific_consensus_mode_mismatch"),
            "expected consensus mode gate failure, got {err}"
        );
    }

