    use super::*;

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
