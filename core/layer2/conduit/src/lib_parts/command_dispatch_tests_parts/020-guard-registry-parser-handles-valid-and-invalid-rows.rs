
    #[test]
    fn guard_registry_parser_handles_valid_and_invalid_inputs() {
        let valid = serde_json::json!({
            "merge_guard": {
                "checks": [
                    {"id": "contract_check"},
                    {"id": "formal_invariant_engine"}
                ]
            }
        })
        .to_string();
        let ids = super::parse_guard_registry_check_ids(&valid).expect("valid guard registry");
        assert!(ids.contains("contract_check"));
        assert!(ids.contains("formal_invariant_engine"));

        let invalid_json = super::parse_guard_registry_check_ids("{bad-json");
        assert_eq!(
            invalid_json.expect_err("invalid json must fail"),
            "guard_registry_invalid_json"
        );

        let missing_checks = super::parse_guard_registry_check_ids("{}");
        assert_eq!(
            missing_checks.expect_err("missing checks must fail"),
            "guard_registry_checks_missing"
        );
    }

    #[test]
    fn parse_json_payload_supports_full_and_tail_json() {
        let direct = super::parse_json_payload("{\"ok\":true,\"type\":\"x\"}").expect("direct");
        assert_eq!(direct.get("ok").and_then(Value::as_bool), Some(true));

        let tailed = super::parse_json_payload("noise line\n{\"ok\":false,\"reason\":\"x\"}\n")
            .expect("tail object");
        assert_eq!(tailed.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(tailed.get("reason").and_then(Value::as_str), Some("x"));

        assert!(super::parse_json_payload("noise only\nline2").is_none());
    }

    #[test]
    fn bridge_timeout_ms_uses_default_and_clamps_env_values() {
        let _guard = env_lock().lock().expect("env lock");

        std::env::remove_var("INFRING_OPS_BRIDGE_TIMEOUT_MS");
        assert_eq!(super::bridge_command_timeout_ms(), 110_000);

        std::env::set_var("INFRING_OPS_BRIDGE_TIMEOUT_MS", "1");
        assert_eq!(super::bridge_command_timeout_ms(), 1_000);

        std::env::set_var("INFRING_OPS_BRIDGE_TIMEOUT_MS", "999999999");
        assert_eq!(super::bridge_command_timeout_ms(), 15 * 60 * 1_000);

        std::env::remove_var("INFRING_OPS_BRIDGE_TIMEOUT_MS");
    }

    #[test]
    fn resolve_infring_ops_command_honors_explicit_bin_and_fallbacks() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

        std::env::set_var("INFRING_OPS_BIN", "/tmp/infring-ops-explicit");
        let (command, args) = super::resolve_infring_ops_command(&root, "spine");
        assert_eq!(command, "/tmp/infring-ops-explicit");
        assert_eq!(args, vec!["spine".to_string()]);
        std::env::remove_var("INFRING_OPS_BIN");

        let (fallback_command, fallback_args) = super::resolve_infring_ops_command(&root, "spine");
        assert_eq!(fallback_command, "cargo");
        assert!(fallback_args.contains(&"--manifest-path".to_string()));
        assert_eq!(fallback_args.last().map(String::as_str), Some("spine"));
    }

    #[test]
    fn execute_ops_bridge_command_reports_spawn_error_when_binary_missing() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var(
            "INFRING_OPS_BIN",
            "/definitely/missing/infring-ops-bridge-bin",
        );
        let detail = super::execute_ops_bridge_command("spine", &[], None);
        std::env::remove_var("INFRING_OPS_BIN");

        assert_eq!(detail.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            detail.get("type").and_then(Value::as_str),
            Some("spine_bridge_spawn_error")
        );
        assert_eq!(detail.get("exit_code").and_then(Value::as_i64), Some(1));
        assert!(
            detail
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .starts_with("spine_bridge_spawn_failed:"),
            "expected explicit spawn failure reason: {detail}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn execute_ops_bridge_command_reports_timeout_when_child_exceeds_budget() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let script_path = temp.path().join("sleepy_bridge.sh");
        fs::write(&script_path, "#!/bin/sh\nsleep 2\n").expect("write script");
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).expect("chmod script");

        std::env::set_var("INFRING_OPS_BIN", script_path.display().to_string());
        std::env::set_var("INFRING_OPS_BRIDGE_TIMEOUT_MS", "1000");
        let detail = super::execute_ops_bridge_command("spine", &[], None);
        std::env::remove_var("INFRING_OPS_BIN");
        std::env::remove_var("INFRING_OPS_BRIDGE_TIMEOUT_MS");

        assert_eq!(detail.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            detail.get("type").and_then(Value::as_str),
            Some("spine_bridge_timeout")
        );
        assert_eq!(detail.get("exit_code").and_then(Value::as_i64), Some(124));
        assert!(
            detail
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("spine_bridge_timeout:1000"),
            "expected timeout reason to include budget: {detail}"
        );
    }

    #[test]
    fn load_cockpit_summary_reports_missing_invalid_and_valid_sources() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let cockpit_file = temp.path().join("cockpit_latest.json");
        std::env::set_var("COCKPIT_INBOX_LATEST_PATH", &cockpit_file);

        let missing = super::load_cockpit_summary(&root);
        assert_eq!(
            missing.get("available").and_then(Value::as_bool),
            Some(false)
        );

        fs::write(&cockpit_file, "{ invalid-json").expect("write invalid");
        let invalid = super::load_cockpit_summary(&root);
        assert_eq!(
            invalid.get("available").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            invalid.get("reason").and_then(Value::as_str),
            Some("cockpit_latest_invalid_json")
        );

        fs::write(&cockpit_file, "[]").expect("write non-object");
        let not_object = super::load_cockpit_summary(&root);
        assert_eq!(
            not_object.get("reason").and_then(Value::as_str),
            Some("cockpit_latest_not_object")
        );

        fs::write(
            &cockpit_file,
            serde_json::json!({
                "ts": "2026-03-19T00:00:00Z",
                "sequence": 7,
                "consumer_id": "test",
                "attention": {"batch_count": 2, "queue_depth": 3},
                "receipt_hash": "abc"
            })
            .to_string(),
        )
        .expect("write valid object");
        let valid = super::load_cockpit_summary(&root);
        assert_eq!(valid.get("available").and_then(Value::as_bool), Some(true));
        assert_eq!(valid.get("sequence").and_then(Value::as_i64), Some(7));
        assert_eq!(
            valid.get("attention_batch_count").and_then(Value::as_i64),
            Some(2)
        );

        std::env::remove_var("COCKPIT_INBOX_LATEST_PATH");
    }

    #[test]
    fn legacy_lane_receipt_and_edge_decode_cover_error_paths() {
        let invalid_lane = super::build_legacy_lane_receipt("  !!! ");
        assert_eq!(invalid_lane.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            invalid_lane.get("error").and_then(Value::as_str),
            Some("lane_id_missing_or_invalid")
        );

        let valid_lane = super::build_legacy_lane_receipt(" lane-42 ");
        assert_eq!(valid_lane.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            valid_lane.get("lane_id").and_then(Value::as_str),
            Some("LANE-42")
        );

        assert!(super::decode_edge_bridge_message("no_prefix")
            .expect("decode")
            .is_none());
        assert!(super::decode_edge_bridge_message("edge_json:{bad")
            .expect_err("invalid edge json")
            .starts_with("edge_bridge_json_invalid:"));
    }

    #[test]
    fn ops_domain_message_rejects_missing_domain() {
        let event =
            super::execute_edge_bridge_message(super::EdgeBridgeMessage::OpsDomainCommand {
                domain: "  ".to_string(),
                args: vec!["status".to_string()],
                run_context: None,
            });
        match event {
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            } => {
                assert_eq!(status, "ops_domain_bridge_error");
                assert_eq!(violation_reason.as_deref(), Some("missing_domain"));
                assert_eq!(
                    detail.get("reason").and_then(Value::as_str),
                    Some("missing_domain")
                );
            }
            _ => panic!("expected system feedback"),
        }
    }

    #[test]
    fn edge_bridge_domain_variants_execute_ops_bridge_paths() {
        let _guard = env_lock().lock().expect("env lock");
        std::env::set_var(
            "INFRING_OPS_BIN",
            "/definitely/missing/infring-ops-bridge-bin",
        );

        let cases = vec![
            (
                super::EdgeBridgeMessage::SpineCommand {
                    args: vec!["status".to_string()],
                    run_context: Some("rc-test".to_string()),
                },
                "spine_bridge_spawn_error",
            ),
            (
                super::EdgeBridgeMessage::AttentionCommand {
                    args: vec!["status".to_string()],
                },
                "attention-queue_bridge_spawn_error",
            ),
            (
                super::EdgeBridgeMessage::PersonaAmbientCommand {
                    args: vec!["status".to_string()],
                },
                "persona-ambient_bridge_spawn_error",
            ),
            (
                super::EdgeBridgeMessage::DopamineAmbientCommand {
                    args: vec!["status".to_string()],
                },
                "dopamine-ambient_bridge_spawn_error",
            ),
            (
                super::EdgeBridgeMessage::MemoryAmbientCommand {
                    args: vec!["status".to_string()],
                },
                "memory-ambient_bridge_spawn_error",
            ),
            (
                super::EdgeBridgeMessage::OpsDomainCommand {
                    domain: "skills-plane".to_string(),
                    args: vec!["status".to_string()],
                    run_context: Some("rc-test".to_string()),
                },
                "skills-plane_bridge_spawn_error",
            ),
        ];

        for (message, expected_status) in cases {
            match super::execute_edge_bridge_message(message) {
                RustEvent::SystemFeedback {
                    status,
                    detail,
                    violation_reason,
                } => {
                    assert_eq!(status, expected_status);
                    assert_eq!(detail.get("ok").and_then(Value::as_bool), Some(false));
                    assert_eq!(
                        detail.get("type").and_then(Value::as_str),
                        Some(expected_status)
                    );
                    assert!(violation_reason
                        .as_deref()
                        .unwrap_or_default()
                        .contains("bridge_spawn_failed"));
                }
                other => panic!("expected system feedback event, got {other:?}"),
            }
        }

        std::env::remove_var("INFRING_OPS_BIN");
    }

    #[test]
    fn rate_limiting_fails_closed() {
        let mut policy = test_policy();
        policy.rate_limit = RateLimitPolicy {
            window_ms: 10_000,
            per_client_max: 2,
            per_client_command_max: 1,
        };

        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let mut handler = EchoCommandHandler;

        let c1 = signed_envelope(&policy, TsCommand::GetSystemStatus);
        let c2 = signed_envelope(&policy, TsCommand::GetSystemStatus);

        let first = process_command(&c1, &gate, &mut security, &mut handler);
        assert!(first.validation.ok);

        let second = process_command(&c2, &gate, &mut security, &mut handler);
        assert!(!second.validation.ok);
        assert!(second.validation.reason.starts_with("rate_limited:"));
    }

    #[test]
    fn registry_policy_denies_when_constitution_missing_marker() {
        let temp = tempfile::tempdir().expect("tempdir");
        let constitution = temp.path().join("constitution.md");
        fs::write(&constitution, "missing markers").expect("constitution");

        let guard_registry = temp.path().join("guard_registry.json");
        fs::write(
            &guard_registry,
            serde_json::json!({"merge_guard":{"checks":[{"id":"contract_check"}]}}).to_string(),
        )
        .expect("guard registry");

        let policy = ConduitPolicy {
            constitution_path: constitution.to_string_lossy().to_string(),
            guard_registry_path: guard_registry.to_string_lossy().to_string(),
            ..ConduitPolicy::default()
        };
        let gate = RegistryPolicyGate::new(policy);

        let decision = gate.evaluate(&TsCommand::GetSystemStatus);
        assert!(!decision.allow);
        assert!(decision.reason.starts_with("constitution_marker_missing:"));
    }

    #[test]
    fn registry_policy_denies_when_command_capability_mapping_cardinality_mismatches() {
        let mut policy = test_policy();
        policy.command_required_capabilities.remove("start_agent");
        let gate = RegistryPolicyGate::new(policy);

        let decision = gate.evaluate(&TsCommand::GetSystemStatus);
        assert!(!decision.allow);
        assert_eq!(
            decision.reason,
            "command_capability_mapping_cardinality_mismatch"
        );
    }

    #[test]
    fn registry_policy_denies_when_required_command_mapping_is_missing() {
        let mut policy = test_policy();
        policy.command_required_capabilities.remove("start_agent");
        policy.command_required_capabilities.insert(
            "synthetic_command".to_string(),
            "agent.lifecycle".to_string(),
        );
        let gate = RegistryPolicyGate::new(policy);

        let decision = gate.evaluate(&TsCommand::GetSystemStatus);
        assert!(!decision.allow);
        assert_eq!(
            decision.reason,
            "policy_missing_command_capability_mapping:start_agent"
        );
    }

    #[test]
    fn registry_policy_denies_policy_updates_without_constitution_safe_prefix() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy);
        let decision = gate.evaluate(&TsCommand::ApplyPolicyUpdate {
            patch_id: "unsafe/runtime_patch".to_string(),
            patch: serde_json::json!({"safe": false}),
        });
        assert!(!decision.allow);
        assert_eq!(decision.reason, "policy_update_must_be_constitution_safe");
    }
