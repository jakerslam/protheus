    use super::*;

    fn write_required_sentinel_evidence(root: &Path) {
        let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
        fs::create_dir_all(&evidence_dir).unwrap();
        for (file_name, subject, category) in [
            ("kernel_receipts.jsonl", "receipt-1", "ReceiptIntegrity"),
            ("runtime_observations.jsonl", "runtime-1", "RuntimeCorrectness"),
            ("state_mutations.jsonl", "mutation-1", "StateTransition"),
            ("scheduler_admission.jsonl", "admission-1", "CapabilityEnforcement"),
            ("live_recovery.jsonl", "recovery-1", "RuntimeCorrectness"),
            ("boundedness_observations.jsonl", "boundedness-1", "Boundedness"),
            ("release_proof_packs.jsonl", "proof-pack-1", "ReleaseEvidence"),
            ("release_repairs.jsonl", "repair-1", "ReleaseEvidence"),
            ("gateway_health.jsonl", "gateway-1", "GatewayIsolation"),
            ("gateway_quarantine.jsonl", "quarantine-1", "GatewayIsolation"),
            ("gateway_recovery.jsonl", "gateway-recovery-1", "GatewayIsolation"),
            ("gateway_isolation.jsonl", "gateway-isolation-1", "GatewayIsolation"),
            ("queue_backpressure.jsonl", "queue-1", "QueueBackpressure"),
            ("control_plane_eval.jsonl", "eval-1", "RuntimeCorrectness"),
        ] {
            fs::write(
                evidence_dir.join(file_name),
                format!(
                    "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"kind\":\"required_stream_regression\",\"category\":\"{category}\",\"evidence\":[\"fixture://{subject}\"],\"details\":{{\"source_artifact\":\"fixture://{subject}\",\"freshness_age_seconds\":0}}}}"
                ),
            )
            .unwrap();
        }
    }

    #[test]
    fn kernel_sentinel_contract_uses_distinct_canonical_name() {
        let contract = kernel_sentinel_contract();
        assert_eq!(contract["canonical_name"], KERNEL_SENTINEL_NAME);
        assert_eq!(contract["module_id"], KERNEL_SENTINEL_MODULE_ID);
        assert_eq!(contract["cli_domain"], KERNEL_SENTINEL_CLI_DOMAIN);
        let not_names = contract["not_names"].as_array().unwrap();
        assert!(not_names
            .iter()
            .any(|value| value.as_str() == Some("control_plane_eval")));
        assert!(not_names
            .iter()
            .any(|value| value.as_str() == Some("eval_agent")));
        assert_eq!(
            contract["control_plane_eval_relationship"]["may_write_sentinel_verdict"],
            false
        );
    }

    #[test]
    fn kernel_sentinel_contract_exposes_failure_level_taxonomy() {
        let contract = kernel_sentinel_contract();
        let taxonomy = contract["failure_level_taxonomy"].as_array().unwrap();
        let codes = taxonomy
            .iter()
            .map(|entry| entry["code"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                "L0_local_defect",
                "L1_component_regression",
                "L2_boundary_contract_breach",
                "L3_policy_truth_failure",
                "L4_architectural_misalignment",
                "L5_self_model_failure",
            ]
        );
        assert_eq!(taxonomy[3]["remediation_level"], "policy_realignment");
        assert_eq!(taxonomy[4]["remediation_level"], "architectural_refactor");
        assert_eq!(taxonomy[5]["remediation_level"], "self_model_repair");
    }

    #[test]
    fn control_plane_eval_cannot_write_or_waive_sentinel_verdicts() {
        let rule = authority_rule(KernelSentinelEvidenceSource::ControlPlaneEval);
        assert_eq!(
            rule.authority_class,
            KernelSentinelAuthorityClass::AdvisoryWorkflowQuality
        );
        assert!(rule.may_open_finding);
        assert!(!rule.may_write_verdict);
        assert!(!rule.may_block_release);
        assert!(!rule.may_waive_finding);
        assert_eq!(
            rule.reason,
            "control_plane_eval_is_advisory_input_only_for_kernel_sentinel"
        );
    }

    #[test]
    fn kernel_owned_evidence_can_drive_fail_closed_verdicts_without_self_waiver() {
        let rule = authority_rule(KernelSentinelEvidenceSource::KernelReceipt);
        assert_eq!(
            rule.authority_class,
            KernelSentinelAuthorityClass::DeterministicKernelAuthority
        );
        assert!(rule.may_open_finding);
        assert!(rule.may_write_verdict);
        assert!(rule.may_block_release);
        assert!(!rule.may_waive_finding);
    }

    #[test]
    fn finding_schema_rejects_missing_evidence_and_dedupes_by_fingerprint() {
        let mut valid = KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "ks-1".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::CapabilityEnforcement,
            fingerprint: "capability:workspace_search:missing_grant".to_string(),
            evidence: vec!["receipt://tool-attempt/1".to_string()],
            summary: "workspace search lacked grant".to_string(),
            recommended_action: "fail closed before execution".to_string(),
            status: "open".to_string(),
        };
        assert!(validate_finding(&valid).is_ok());
        let serialized = serde_json::to_value(&valid).unwrap();
        assert_eq!(serialized["failure_level"], "L3_policy_truth_failure");
        assert_eq!(serialized["root_frame"], "policy_truth_contradiction");
        assert_eq!(serialized["remediation_level"], "policy_realignment");
        valid.evidence.clear();
        assert_eq!(validate_finding(&valid), Err("missing_evidence".to_string()));
        valid.evidence.push("receipt://tool-attempt/1".to_string());
        let mut critical = valid.clone();
        critical.id = "ks-2".to_string();
        critical.severity = KernelSentinelSeverity::Critical;
        let deduped = dedupe_findings(vec![valid, critical]);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].severity, KernelSentinelSeverity::Critical);
    }

    #[test]
    fn strict_report_fails_on_open_critical_findings() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-test-{}",
            crate::deterministic_receipt_hash(&json!({"test": "strict"}))
        ));
        let findings_path = root.join("findings.jsonl");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &findings_path,
            serde_json::to_string(&KernelSentinelFinding {
                schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
                id: "ks-critical".to_string(),
                severity: KernelSentinelSeverity::Critical,
                category: KernelSentinelFindingCategory::ReceiptIntegrity,
                fingerprint: "receipt:missing:mutation".to_string(),
                evidence: vec!["local://state/mutation".to_string()],
                summary: "mutation missing receipt".to_string(),
                recommended_action: "block release until receipt emission is restored".to_string(),
                status: "open".to_string(),
            })
            .unwrap(),
        )
        .unwrap();
        let args = vec![
            "--strict=1".to_string(),
            format!("--findings-path={}", findings_path.display()),
        ];
        let (_report, verdict, exit) = build_report(&root, &args);
        assert_eq!(exit, 2);
        assert_eq!(verdict["verdict"], "release_fail");
        assert_eq!(verdict["critical_open_count"], 1);
    }

    #[test]
fn evidence_ingestion_adds_runtime_failures_to_report() {
        let root = std::env::temp_dir().join(format!(
            "kernel-sentinel-test-{}",
            crate::deterministic_receipt_hash(&json!({"test": "evidence-ingestion"}))
        ));
        let evidence_dir = root.join("evidence");
        fs::create_dir_all(&evidence_dir).unwrap();
        fs::write(
            evidence_dir.join("runtime_observations.jsonl"),
            r#"{"id":"obs-1","ok":false,"subject":"task-7","kind":"illegal_reopen","summary":"failed task reopened","evidence":["trace://task-7"],"recommended_action":"require rollback receipt before reopen"}"#,
        )
        .unwrap();
        let args = vec![format!("--evidence-dir={}", evidence_dir.display())];
        let (report, verdict, exit) = build_report(&root, &args);
        assert_eq!(exit, 0);
        assert_eq!(verdict["finding_count"], 1);
    assert_eq!(report["evidence_ingestion"]["normalized_record_count"], 1);
    assert_eq!(report["findings"][0]["category"], "runtime_correctness");
}

#[test]
fn missing_evidence_reports_data_starved_release_gate() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-data-starved-{}",
        crate::deterministic_receipt_hash(&json!({"test": "data-starved"}))
    ));
    let (report, verdict, exit) = build_report(&root, &[]);
    assert_eq!(exit, 0);
    assert_eq!(report["evidence_ingestion"]["observation_state"], "data_starved");
    assert_eq!(report["evidence_ingestion"]["data_starved"], true);
    assert_eq!(report["operator_summary"]["data_starved"], true);
    assert_eq!(report["release_gate"]["data_starved"], true);
    assert_eq!(report["release_gate"]["pass"], false);
    assert_eq!(verdict["verdict"], "release_fail");
}

#[test]
fn malformed_evidence_reports_source_path_and_stream_counts() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-malformed-evidence-{}",
        crate::deterministic_receipt_hash(&json!({"test": "malformed-evidence"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(evidence_dir.join("runtime_observations.jsonl"), "{not-json").unwrap();
    let (report, verdict, exit) = build_report(&root, &["--strict=1".to_string()]);
    assert_eq!(exit, 2);
    assert_eq!(report["evidence_ingestion"]["observation_state"], "malformed_evidence");
    assert_eq!(
        report["evidence_ingestion"]["malformed_by_source"]["runtime_observation"],
        1
    );
    assert_eq!(
        report["evidence_ingestion"]["malformed_by_file_name"]["runtime_observations.jsonl"],
        1
    );
    assert_eq!(report["operator_summary"]["malformed_evidence_count"], 1);
    assert_eq!(verdict["verdict"], "invalid");
}

#[test]
fn malformed_receipts_preserve_valid_rows_and_report_cleanly() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-malformed-receipts-{}",
        crate::deterministic_receipt_hash(&json!({"test": "malformed-receipts"}))
    ));
    write_required_sentinel_evidence(&root);
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::write(
        evidence_dir.join("kernel_receipts.jsonl"),
        concat!(
            r#"{"id":"receipt-valid","ok":true,"subject":"receipt-valid","kind":"receipt_check","category":"ReceiptIntegrity","evidence":["receipt://receipt-valid"],"details":{"source_artifact":"fixture://receipt-valid","freshness_age_seconds":0}}"#,
            "\n",
            "{not-json\n"
        ),
    )
    .unwrap();

    let (report, verdict, exit) = build_report(&root, &["--strict=1".to_string()]);
    assert_eq!(exit, 2);
    assert_eq!(verdict["verdict"], "invalid");
    assert_eq!(report["evidence_ingestion"]["observation_state"], "malformed_evidence");
    assert_eq!(report["evidence_ingestion"]["data_starved"], false);
    assert_eq!(report["evidence_ingestion"]["partial_evidence"], false);
    assert_eq!(report["evidence_ingestion"]["normalized_record_count"], 14);
    assert_eq!(report["evidence_ingestion"]["malformed_record_count"], 1);
    assert_eq!(
        report["evidence_ingestion"]["malformed_by_source"]["kernel_receipt"],
        1
    );
    assert_eq!(
        report["evidence_ingestion"]["malformed_by_file_name"]["kernel_receipts.jsonl"],
        1
    );
    assert_eq!(report["operator_summary"]["data_starved"], false);
    assert_eq!(report["operator_summary"]["malformed_evidence_count"], 1);
    assert!(report["evidence_ingestion"]["normalized_records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["subject"] == "receipt-valid"));
}

#[test]
fn stale_evidence_reports_age_threshold_and_blocks_fresh_readiness() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-stale-evidence-{}",
        crate::deterministic_receipt_hash(&json!({"test": "stale-evidence"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(
        evidence_dir.join("runtime_observations.jsonl"),
        r#"{"id":"runtime-stale","ok":true,"subject":"runtime-1","kind":"runtime_status","category":"RuntimeCorrectness","evidence":["fixture://runtime-stale"],"details":{"source_artifact":"fixture://runtime-stale","freshness_age_seconds":7200}}"#,
    )
    .unwrap();
    let args = vec![
        "--strict=1".to_string(),
        "--stale-evidence-seconds=60".to_string(),
    ];
    let (report, _verdict, exit) = build_report(&root, &args);
    assert_eq!(exit, 2);
    assert_eq!(report["evidence_ingestion"]["observation_state"], "stale_evidence");
    assert_eq!(report["evidence_ingestion"]["stale_record_count"], 1);
    assert_eq!(
        report["evidence_ingestion"]["freshness_observed_record_count"],
        1
    );
    assert_eq!(report["evidence_ingestion"]["max_evidence_age_seconds"], 7200);
    assert_eq!(report["operator_summary"]["data_starved"], false);
}

#[test]
fn report_command_writes_sentinel_health_artifact() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-health-report-{}",
        crate::deterministic_receipt_hash(&json!({"test": "health-report"}))
    ));
    write_required_sentinel_evidence(&root);
    let exit = run(&root, &["report".to_string()]);
    assert_eq!(exit, 0);
    let health_path = root.join("local/state/kernel_sentinel/kernel_sentinel_health_current.json");
    let health: Value = serde_json::from_str(&fs::read_to_string(health_path).unwrap()).unwrap();
    assert_eq!(health["type"], "kernel_sentinel_health_report");
    assert_eq!(health["coverage"]["present_required_source_count"], 14);
    assert_eq!(health["coverage"]["source_classes"]["required"]["ready"], true);
    assert_eq!(health["coverage"]["source_classes"]["optional"]["fully_present"], false);
    assert_eq!(health["trend"]["status"], "unavailable");
    assert_eq!(health["trend"]["delta"]["baseline"], "unavailable");
    assert_eq!(health["issue_synthesis"]["issue_draft_count"], 0);
    assert_eq!(health["authority_safety"]["safe_for_observation_authority"], false);
    assert_eq!(health["authority_safety"]["safe_for_automation_authority"], false);
}

#[test]
fn evidence_ingestion_normalizes_major_stream_families() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-stream-families-{}",
        crate::deterministic_receipt_hash(&json!({"test": "stream-families"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    for (file_name, subject, category) in [
        ("kernel_receipts.jsonl", "receipt-1", "ReceiptIntegrity"),
        ("runtime_observations.jsonl", "runtime-1", "RuntimeCorrectness"),
        ("release_proof_packs.jsonl", "proof-pack-1", "ReleaseEvidence"),
        ("gateway_health.jsonl", "gateway-1", "GatewayIsolation"),
        ("queue_backpressure.jsonl", "queue-1", "QueueBackpressure"),
        ("control_plane_eval.jsonl", "eval-1", "RuntimeCorrectness"),
    ] {
        fs::write(
            evidence_dir.join(file_name),
            format!(
                "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"kind\":\"stream_family_regression\",\"category\":\"{category}\",\"evidence\":[\"fixture://{subject}\"],\"details\":{{\"source_artifact\":\"fixture://{subject}\"}}}}"
            ),
        )
        .unwrap();
    }
    let (report, _verdict, exit) = build_report(&root, &[]);
    assert_eq!(exit, 0);
    assert_eq!(report["evidence_ingestion"]["normalized_record_count"], 6);
    assert_eq!(report["evidence_ingestion"]["observation_state"], "partial_evidence");
    assert_eq!(
        report["evidence_ingestion"]["coverage"]["missing_optional_source_count"],
        1
    );
    assert_eq!(
        report["evidence_ingestion"]["advisory_bridge"]["checked_count"],
        1
    );
    assert_eq!(
        report["evidence_ingestion"]["coverage"]["malformed_record_count"],
        0
    );
}

#[test]
fn shell_telemetry_is_observation_only_and_cannot_open_findings() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-shell-observation-{}",
        crate::deterministic_receipt_hash(&json!({"test": "shell-observation"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(
        evidence_dir.join("shell_telemetry.jsonl"),
        r#"{"id":"shell-1","ok":false,"subject":"chat-bubble","kind":"presentation_status","category":"RuntimeCorrectness","severity":"Critical","summary":"shell displayed stale thinking text","evidence":["shell://chat-bubble"],"details":{"source_artifact":"shell://chat-bubble"}}"#,
    )
    .unwrap();
    write_required_sentinel_evidence(&root);
    let (report, verdict, exit) = build_report(&root, &["--strict=1".to_string()]);
    assert_eq!(exit, 0);
    assert_eq!(report["evidence_ingestion"]["normalized_record_count"], 15);
    let shell_record = report["evidence_ingestion"]["normalized_records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["source"] == "shell_telemetry")
        .expect("shell telemetry record should be normalized");
    assert_eq!(shell_record["may_write_verdict"], false);
    assert_eq!(verdict["finding_count"], 0);
    assert_eq!(report["findings"].as_array().unwrap().len(), 0);
    assert_eq!(report["release_gate"]["pass"], true);
}

#[test]
fn missing_shell_telemetry_does_not_make_complete_required_evidence_partial() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-required-coverage-{}",
        crate::deterministic_receipt_hash(&json!({"test": "required-coverage"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    for (file_name, subject, category) in [
        ("kernel_receipts.jsonl", "receipt-1", "ReceiptIntegrity"),
        ("runtime_observations.jsonl", "runtime-1", "RuntimeCorrectness"),
        ("state_mutations.jsonl", "mutation-1", "StateTransition"),
        ("scheduler_admission.jsonl", "admission-1", "CapabilityEnforcement"),
        ("live_recovery.jsonl", "recovery-1", "RuntimeCorrectness"),
        ("boundedness_observations.jsonl", "boundedness-1", "Boundedness"),
        ("release_proof_packs.jsonl", "proof-pack-1", "ReleaseEvidence"),
        ("release_repairs.jsonl", "repair-1", "ReleaseEvidence"),
        ("gateway_health.jsonl", "gateway-1", "GatewayIsolation"),
        ("gateway_quarantine.jsonl", "quarantine-1", "GatewayIsolation"),
        ("gateway_recovery.jsonl", "gateway-recovery-1", "GatewayIsolation"),
        ("gateway_isolation.jsonl", "gateway-isolation-1", "GatewayIsolation"),
        ("queue_backpressure.jsonl", "queue-1", "QueueBackpressure"),
        ("control_plane_eval.jsonl", "eval-1", "RuntimeCorrectness"),
    ] {
        fs::write(
            evidence_dir.join(file_name),
            format!(
                "{{\"id\":\"{subject}\",\"ok\":true,\"subject\":\"{subject}\",\"kind\":\"required_stream_regression\",\"category\":\"{category}\",\"evidence\":[\"fixture://{subject}\"],\"details\":{{\"source_artifact\":\"fixture://{subject}\"}}}}"
            ),
        )
        .unwrap();
    }
    let (report, _verdict, exit) = build_report(&root, &[]);
    assert_eq!(exit, 0);
    assert_eq!(report["evidence_ingestion"]["observation_state"], "healthy_observation");
    assert_eq!(
        report["evidence_ingestion"]["coverage"]["missing_required_source_count"],
        0
    );
    assert_eq!(
        report["evidence_ingestion"]["coverage"]["missing_optional_source_count"],
        1
    );
    assert_eq!(
        report["operator_summary"]["missing_required_source_count"],
        0
    );
}

#[test]
fn valid_human_waiver_unblocks_strict_report() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-test-{}",
        crate::deterministic_receipt_hash(&json!({"test": "waiver-unblocks"}))
    ));
    let dir = root.join("local/state/kernel_sentinel");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("findings.jsonl"),
        r#"{"schema_version":1,"id":"ks-critical","severity":"critical","category":"receipt_integrity","fingerprint":"receipt:missing:mutation","evidence":["receipt://missing"],"summary":"mutation missing receipt","recommended_action":"restore receipt","status":"open"}"#,
    )
    .unwrap();
    fs::write(
        dir.join("waivers.jsonl"),
        r#"{"id":"w1","fingerprint":"receipt:missing:mutation","approved_by":"human:jay","expires_at_epoch":4102444800,"evidence":["review://w1"],"rollback_plan":"restore previous build","mitigation_plan":"monitor receipts","receipt":"waiver_receipt://w1"}"#,
    )
    .unwrap();
    write_required_sentinel_evidence(&root);
    let args = vec!["--strict=1".to_string()];
    let (report, verdict, exit) = build_report(&root, &args);
    assert_eq!(exit, 0);
    assert_eq!(verdict["verdict"], "allow");
    assert_eq!(report["waivers"]["applied_count"], 1);
    assert_eq!(report["release_gate"]["pass"], true);
    assert_eq!(report["findings"][0]["status"], "waived");
}

#[test]
fn explicit_state_root_controls_kernel_sentinel_output_location() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-state-root-{}",
        crate::deterministic_receipt_hash(&json!({"test": "state-root"}))
    ));
    let state_root = root.join("operator_state");
    let args = vec![
        format!("--state-root={}", state_root.display()),
        "--watch-refresh=1".to_string(),
    ];
    let (report, _verdict, exit) = build_report(&root, &args);
    assert_eq!(exit, 0);
    assert_eq!(
        report["state_dir"],
        Value::from(state_root.join("kernel_sentinel").display().to_string())
    );
}

#[test]
fn nested_evidence_details_drive_rsi_handoff_gate() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-rsi-nested-{}",
        crate::deterministic_receipt_hash(&json!({"test": "rsi-nested"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(
        evidence_dir.join("runtime_observations.jsonl"),
        r#"{"id":"rsi-1","subject":"patch-loop-1","kind":"self_modification_proposal","evidence":["proposal://patch-loop-1"],"details":{"advance_requested":true,"sentinel_verdict":"allow"}}"#,
    )
    .unwrap();
    let (report, verdict, exit) = build_report(&root, &["--strict=1".to_string()]);
    assert_eq!(exit, 2);
    assert_eq!(verdict["verdict"], "release_fail");
    assert_eq!(report["governance_preflight"]["rsi_handoff_blocking_count"], 1);
    assert_eq!(
        report["findings"][0]["fingerprint"],
        "rsi_handoff:missing_contract:patch-loop-1"
    );
}

#[test]
fn evidence_category_aliases_accept_pascal_case_operator_input() {
    let root = std::env::temp_dir().join(format!(
        "kernel-sentinel-category-alias-{}",
        crate::deterministic_receipt_hash(&json!({"test": "category-alias"}))
    ));
    let evidence_dir = root.join("local/state/kernel_sentinel/evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(
        evidence_dir.join("kernel_receipts.jsonl"),
        r#"{"id":"receipt-1","ok":false,"subject":"mutation-1","kind":"receipt_gap","category":"ReceiptIntegrity","severity":"Critical","summary":"receipt missing","evidence":["receipt://missing"],"recommended_action":"restore receipt"}"#,
    )
    .unwrap();
    let (report, verdict, exit) = build_report(&root, &["--strict=1".to_string()]);
    assert_eq!(exit, 2);
    assert_eq!(verdict["verdict"], "release_fail");
    assert_eq!(report["malformed_findings"].as_array().unwrap().len(), 0);
    assert_eq!(report["findings"][0]["category"], "receipt_integrity");
}
