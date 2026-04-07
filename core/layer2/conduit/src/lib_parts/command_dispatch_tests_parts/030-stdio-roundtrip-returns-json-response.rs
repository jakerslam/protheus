
    #[test]
    fn stdio_roundtrip_returns_json_response() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);

        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "agent-alpha".to_string(),
            },
        );

        let mut payload = serde_json::to_string(&command).expect("serialize command");
        payload.push('\n');

        let cursor = Cursor::new(payload.into_bytes());
        let reader = BufReader::new(cursor);
        let mut writer = Vec::new();
        let mut handler = EchoCommandHandler;

        let wrote = run_stdio_once(reader, &mut writer, &gate, &mut security, &mut handler)
            .expect("stdio call should succeed");
        assert!(wrote);

        let text = String::from_utf8(writer).expect("utf8 response");
        let response: super::ResponseEnvelope =
            serde_json::from_str(text.trim()).expect("json response");
        assert!(response.validation.ok);
        assert_eq!(response.request_id, "req-test");
    }

    #[test]
    fn stdio_once_returns_false_on_eof() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        let mut writer = Vec::new();
        let mut handler = EchoCommandHandler;

        let wrote = run_stdio_once(reader, &mut writer, &gate, &mut security, &mut handler)
            .expect("eof should not fail");
        assert!(!wrote);
        assert!(writer.is_empty());
    }

    #[test]
    fn stdio_once_rejects_invalid_json_payload() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let reader = BufReader::new(Cursor::new(b"not-json\n".to_vec()));
        let mut writer = Vec::new();
        let mut handler = EchoCommandHandler;

        let err = run_stdio_once(reader, &mut writer, &gate, &mut security, &mut handler)
            .expect_err("invalid json must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn conduit_contract_budget_rejects_zero_budget() {
        let err = validate_conduit_contract_budget(0).expect_err("zero budget must be rejected");
        assert_eq!(err, "conduit_message_budget_invalid_zero");
    }

    #[test]
    fn install_extension_invalid_sha_is_fail_closed() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::InstallExtension {
                extension_id: "ext-bad-sha".to_string(),
                wasm_sha256: "badsha".to_string(),
                capabilities: vec!["metrics.read".to_string()],
                plugin_type: Some("substrate_adapter".to_string()),
                version: Some("0.1.0".to_string()),
                wasm_component_path: Some(
                    "adapters/protocol/wasm_adapter_skeleton.wasm".to_string(),
                ),
                signature: None,
                provenance: None,
                recovery_max_attempts: None,
                recovery_backoff_ms: None,
            },
        );

        let mut handler = EchoCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(!response.validation.ok);
        assert!(response.validation.fail_closed);
        assert_eq!(response.validation.reason, "extension_wasm_sha256_invalid");
    }

    #[test]
    fn install_extension_autoheals_and_quarantines_after_retries() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let registry_path = temp.path().join("plugin_registry.json");
        let receipts_path = temp.path().join("plugin_runtime_receipts.jsonl");
        let component_path = temp.path().join("plugin.wasm");
        fs::write(&component_path, b"wasm-ok").expect("write component");
        let wasm_sha256 = super::hash_file_sha256(&component_path).expect("hash component");

        std::env::set_var(
            "INFRING_PLUGIN_REGISTRY_PATH",
            registry_path.to_string_lossy().to_string(),
        );
        std::env::set_var(
            "INFRING_PLUGIN_RUNTIME_RECEIPTS_PATH",
            receipts_path.to_string_lossy().to_string(),
        );

        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);

        let install = signed_envelope(
            &policy,
            TsCommand::InstallExtension {
                extension_id: "plugin-alpha".to_string(),
                wasm_sha256,
                capabilities: vec!["metrics.read".to_string()],
                plugin_type: Some("substrate_adapter".to_string()),
                version: Some("0.1.0".to_string()),
                wasm_component_path: Some(component_path.to_string_lossy().to_string()),
                signature: None,
                provenance: None,
                recovery_max_attempts: Some(2),
                recovery_backoff_ms: Some(500),
            },
        );
        let mut handler = EchoCommandHandler;
        let install_response = process_command(&install, &gate, &mut security, &mut handler);
        assert!(install_response.validation.ok);

        fs::write(&component_path, b"wasm-corrupt").expect("corrupt component");

        let status = signed_envelope(&policy, TsCommand::GetSystemStatus);
        let mut handler = EchoCommandHandler;
        let _ = process_command(&status, &gate, &mut security, &mut handler);
        let mut handler = EchoCommandHandler;
        let status_response = process_command(&status, &gate, &mut security, &mut handler);
        assert!(status_response.validation.ok);

        let (quarantined, healing) = match status_response.event {
            RustEvent::SystemFeedback { detail, .. } => {
                let runtime = detail
                    .get("plugin_runtime")
                    .and_then(Value::as_object)
                    .expect("plugin_runtime payload");
                let quarantined = runtime
                    .get("quarantined_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let healing = runtime
                    .get("healing_count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                (quarantined, healing)
            }
            other => panic!("unexpected event: {other:?}"),
        };
        assert!(quarantined >= 1 || healing >= 1);
        assert!(registry_path.exists(), "plugin registry must exist");
        assert!(receipts_path.exists(), "plugin receipt log must exist");

        std::env::remove_var("INFRING_PLUGIN_REGISTRY_PATH");
        std::env::remove_var("INFRING_PLUGIN_RUNTIME_RECEIPTS_PATH");
    }

    #[test]
    fn install_extension_reports_runtime_save_failures() {
        let _guard = env_lock().lock().expect("env lock");
        let temp = tempfile::tempdir().expect("tempdir");
        let blocked_parent = temp.path().join("blocked-parent");
        fs::write(&blocked_parent, "file-not-dir").expect("write blocker file");
        let registry_path = blocked_parent.join("plugin_registry.json");
        let receipts_path = temp.path().join("plugin_runtime_receipts.jsonl");
        let component_path = temp.path().join("plugin-ok.wasm");
        fs::write(&component_path, b"wasm-ok").expect("write component");
        let wasm_sha256 = super::hash_file_sha256(&component_path).expect("hash component");

        std::env::set_var(
            "INFRING_PLUGIN_REGISTRY_PATH",
            registry_path.to_string_lossy().to_string(),
        );
        std::env::set_var(
            "INFRING_PLUGIN_RUNTIME_RECEIPTS_PATH",
            receipts_path.to_string_lossy().to_string(),
        );

        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::InstallExtension {
                extension_id: "plugin-save-fail".to_string(),
                wasm_sha256,
                capabilities: vec!["metrics.read".to_string()],
                plugin_type: Some("substrate_adapter".to_string()),
                version: Some("0.1.0".to_string()),
                wasm_component_path: Some(component_path.to_string_lossy().to_string()),
                signature: None,
                provenance: None,
                recovery_max_attempts: None,
                recovery_backoff_ms: None,
            },
        );
        let mut handler = EchoCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(
            response.validation.ok,
            "command should validate before runtime write"
        );
        match response.event {
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            } => {
                assert_eq!(status, "extension_install_failed");
                assert_eq!(
                    violation_reason.as_deref(),
                    Some("extension_runtime_registration_failed")
                );
                assert_eq!(
                    detail.get("extension_id").and_then(Value::as_str),
                    Some("plugin-save-fail")
                );
            }
            other => panic!("expected system feedback event, got {other:?}"),
        }

        std::env::remove_var("INFRING_PLUGIN_REGISTRY_PATH");
        std::env::remove_var("INFRING_PLUGIN_RUNTIME_RECEIPTS_PATH");
    }

    #[test]
    fn kernel_lane_handler_returns_lane_receipt_for_lane_start() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "lane:SYSTEMS-ASSIMILATION-ASSIMILATION-CONTROLLER".to_string(),
            },
        );

        let mut handler = KernelLaneCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(response.validation.ok);

        match response.event {
            RustEvent::SystemFeedback {
                status,
                detail,
                violation_reason,
            } => {
                assert_eq!(status, "legacy_lane_receipt");
                assert_eq!(violation_reason, None);
                let lane_receipt = detail
                    .get("lane_receipt")
                    .and_then(serde_json::Value::as_object)
                    .expect("lane receipt object");
                assert_eq!(
                    lane_receipt.get("ok").and_then(serde_json::Value::as_bool),
                    Some(true)
                );
                assert_eq!(
                    lane_receipt
                        .get("lane_id")
                        .and_then(serde_json::Value::as_str),
                    Some("SYSTEMS-ASSIMILATION-ASSIMILATION-CONTROLLER")
                );
                assert!(lane_receipt.contains_key("receipt_hash"));
            }
            _ => panic!("expected system_feedback event"),
        }
    }

    #[test]
    fn kernel_lane_handler_returns_edge_status_payload() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "edge_status".to_string(),
            },
        );

        let mut handler = KernelLaneCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(response.validation.ok);

        match response.event {
            RustEvent::SystemFeedback { status, detail, .. } => {
                assert_eq!(status, "edge_status");
                assert_eq!(
                    detail.get("type").and_then(Value::as_str),
                    Some("edge_status")
                );
                assert!(detail.get("receipt_hash").and_then(Value::as_str).is_some());
            }
            _ => panic!("expected system_feedback event"),
        }
    }

    #[test]
    fn kernel_lane_handler_accepts_edge_json_inference_contract() {
        let policy = test_policy();
        let gate = RegistryPolicyGate::new(policy.clone());
        let mut security = test_security(&policy);
        let command = signed_envelope(
            &policy,
            TsCommand::StartAgent {
                agent_id: "edge_json:{\"type\":\"edge_inference\",\"prompt\":\"hello tiny edge world\",\"max_tokens\":3}".to_string(),
            },
        );

        let mut handler = KernelLaneCommandHandler;
        let response = process_command(&command, &gate, &mut security, &mut handler);
        assert!(response.validation.ok);

        match response.event {
            RustEvent::SystemFeedback { status, detail, .. } => {
                if cfg!(feature = "edge") {
                    assert_eq!(status, "edge_inference");
                    assert_eq!(
                        detail
                            .get("output")
                            .and_then(Value::as_object)
                            .and_then(|o| o.get("token_count"))
                            .and_then(Value::as_u64),
                        Some(3)
                    );
                } else {
                    assert_eq!(status, "edge_backend_unavailable");
                    assert_eq!(
                        detail.get("reason").and_then(Value::as_str),
                        Some("edge_feature_disabled")
                    );
                }
                assert!(detail.get("receipt_hash").and_then(Value::as_str).is_some());
            }
            _ => panic!("expected system_feedback event"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn unix_socket_server_roundtrip_returns_validated_response() {
        use std::os::unix::net::UnixStream;
        use std::thread;

        let policy = test_policy();
        let socket_dir = tempfile::tempdir().expect("tempdir");
        let socket_path = socket_dir.path().join("conduit-test.sock");
        let envelope = signed_envelope(&policy, TsCommand::GetSystemStatus);
        let envelope_json = serde_json::to_string(&envelope).expect("serialize envelope");
        let socket_path_for_server = socket_path.clone();
        let policy_for_server = policy.clone();

        let server = thread::spawn(move || {
            let gate = RegistryPolicyGate::new(policy_for_server.clone());
            let mut security = test_security(&policy_for_server);
            let mut handler = EchoCommandHandler;
            super::run_unix_socket_server(
                &socket_path_for_server,
                &gate,
                &mut security,
                &mut handler,
            )
            .expect("unix socket server run");
        });

        let mut client = None;
        for _ in 0..30 {
            match UnixStream::connect(&socket_path) {
                Ok(stream) => {
                    client = Some(stream);
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(20)),
            }
        }
        let mut stream = client.expect("connect unix socket");
        stream
            .write_all(format!("{envelope_json}\n").as_bytes())
            .expect("write command");
        stream
            .shutdown(std::net::Shutdown::Write)
            .expect("shutdown write");

        let mut response_line = String::new();
        let mut reader = BufReader::new(stream);
        reader.read_line(&mut response_line).expect("read response");
        let response: super::ResponseEnvelope =
            serde_json::from_str(response_line.trim()).expect("response json");
        assert!(response.validation.ok);
        assert_eq!(response.request_id, envelope.request_id);

        server.join().expect("server join");
    }
