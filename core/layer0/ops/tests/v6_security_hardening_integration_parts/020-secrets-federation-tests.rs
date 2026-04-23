
#[test]
fn v6_sec_016_secrets_federation_issues_scoped_handles_and_revokes_them() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::env::set_var(
        "INFRING_SECRET_VAULT_APP_DB_PASSWORD",
        "super-secret-password",
    );

    assert_eq!(
        security_plane::run(
            root,
            &[
                "secrets-federation".to_string(),
                "fetch".to_string(),
                "--provider=vault".to_string(),
                "--path=app/db/password".to_string(),
                "--scope=billing".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    let latest = read_json(&latest_path(root));
    let handle_id = latest
        .get("handle_id")
        .and_then(Value::as_str)
        .expect("handle id")
        .to_string();
    assert_claim(&latest, "V6-SEC-016");

    assert_eq!(
        security_plane::run(
            root,
            &[
                "secrets-federation".to_string(),
                "rotate".to_string(),
                format!("--handle-id={handle_id}"),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        security_plane::run(
            root,
            &[
                "secrets-federation".to_string(),
                "revoke".to_string(),
                format!("--handle-id={handle_id}"),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        security_plane::run(
            root,
            &[
                "secrets-federation".to_string(),
                "status".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    let status_latest = read_json(&latest_path(root));
    assert_eq!(
        status_latest.get("active_handles").and_then(Value::as_u64),
        Some(0)
    );

    std::env::remove_var("INFRING_SECRET_VAULT_APP_DB_PASSWORD");
}

#[test]
fn v6_sec_016_rejects_unsupported_provider_in_strict_mode() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let exit = security_plane::run(
        root,
        &[
            "secrets-federation".to_string(),
            "fetch".to_string(),
            "--provider=unsupported-cloud".to_string(),
            "--path=app/db/password".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        exit, 2,
        "strict mode must fail-closed on unsupported provider"
    );
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("security_plane_secrets_federation")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("unsupported_provider:unsupported-cloud")
    );
    assert_claim(&latest, "V6-SEC-016");
}

#[test]
fn v6_sec_014_audit_logs_handles_empty_event_history_without_false_blocks() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let exit = security_plane::run(
        root,
        &[
            "audit-logs".to_string(),
            "--max-events=100".to_string(),
            "--max-failures=0".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(exit, 0, "empty history should not trigger strict blocking");
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("security_plane_audit_logs")
    );
    assert_eq!(
        latest
            .pointer("/summary/security_events_considered")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        latest
            .pointer("/summary/failed_events")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        latest
            .pointer("/summary/audit_blocked")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_claim(&latest, "V6-SEC-014");
}

#[test]
fn v6_sec_contract_and_skill_path_lanes_fail_closed_in_strict_mode() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let contract_exit = security_plane::run(
        root,
        &[
            "required-checks-policy-guard".to_string(),
            "--codeql=optional".to_string(),
            "--dependabot=required".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        contract_exit, 2,
        "strict contract lane should fail when required checks mismatch"
    );
    let contract_latest = read_json(&latest_path(root));
    assert_eq!(
        contract_latest.get("type").and_then(Value::as_str),
        Some("security_plane_contract_lane")
    );
    assert_eq!(
        contract_latest.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        contract_latest
            .get("mismatch_flags")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("codeql:required")))
            .unwrap_or(false),
        "expected mismatch receipt for codeql required check"
    );

    let missing_path_exit = security_plane::run(
        root,
        &[
            "skill-install-path-enforcer".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        missing_path_exit, 2,
        "strict skill path enforcer should fail when skill-path is missing"
    );
    let missing_path_latest = read_json(&latest_path(root));
    assert_eq!(
        missing_path_latest.get("type").and_then(Value::as_str),
        Some("security_plane_skill_install_path_enforcer")
    );
    assert_eq!(
        missing_path_latest.get("error").and_then(Value::as_str),
        Some("skill_path_required")
    );

    let quarantine_exit = security_plane::run(
        root,
        &[
            "skill-quarantine".to_string(),
            "quarantine".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        quarantine_exit, 2,
        "strict quarantine command should fail when skill id is omitted"
    );
    let quarantine_latest = read_json(&latest_path(root));
    assert_eq!(
        quarantine_latest.get("type").and_then(Value::as_str),
        Some("security_plane_skill_quarantine")
    );
    assert_eq!(
        quarantine_latest.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        quarantine_latest.get("error").and_then(Value::as_str),
        Some("skill_id_required")
    );
}

#[test]
fn v6_sec_011_remediation_requires_scan_state_in_strict_mode() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let exit = security_plane::run(root, &["remediate".to_string(), "--strict=1".to_string()]);
    assert_eq!(
        exit, 2,
        "strict remediation must fail without scan artifacts"
    );
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("security_plane_auto_remediation")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("scan_missing")
    );
    assert_claim(&latest, "V6-SEC-011");
}

#[test]
fn v6_sec_015_threat_model_medium_boundary_is_receipted_and_thresholded() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let allow_exit = security_plane::run(
        root,
        &[
            "threat-model".to_string(),
            "--scenario=prompt_injection_review".to_string(),
            "--surface=runtime".to_string(),
            "--vector=prompt injection".to_string(),
            "--block-threshold=60".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        allow_exit, 0,
        "risk score at medium band should pass when threshold is higher"
    );
    let allow_latest = read_json(&latest_path(root));
    assert_eq!(
        allow_latest
            .pointer("/event/severity")
            .and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        allow_latest
            .pointer("/event/risk_score")
            .and_then(Value::as_u64),
        Some(50)
    );
    assert_eq!(
        allow_latest
            .pointer("/event/blocked")
            .and_then(Value::as_bool),
        Some(false)
    );

    let block_exit = security_plane::run(
        root,
        &[
            "threat-model".to_string(),
            "--scenario=prompt_injection_review".to_string(),
            "--surface=runtime".to_string(),
            "--vector=prompt injection".to_string(),
            "--block-threshold=50".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(
        block_exit, 2,
        "same medium-risk scenario should fail when threshold matches boundary"
    );
    let block_latest = read_json(&latest_path(root));
    assert_eq!(
        block_latest
            .pointer("/event/severity")
            .and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        block_latest
            .pointer("/event/risk_score")
            .and_then(Value::as_u64),
        Some(50)
    );
    assert_eq!(
        block_latest
            .pointer("/event/blocked")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_claim(&block_latest, "V6-SEC-015");
}

#[test]
fn v6_sec_stub_contracts_are_now_authoritative_security_lanes() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let cases: [(&str, &[&str], &str); 13] = [
        (
            "supply-chain-reproducible-build-plane",
            &[
                "--sbom-path=docs/client/reports/benchmark_matrix_run_2026-03-06.json",
                "--release-tag=v0.3.0-alpha",
                "--strict=1",
            ],
            "V6-SEC-001",
        ),
        (
            "ip-posture-review",
            &["--public-url=https://example.com/security", "--strict=1"],
            "V6-SEC-002",
        ),
        (
            "required-checks-policy-guard",
            &["--codeql=required", "--dependabot=required", "--strict=1"],
            "V6-SEC-003",
        ),
        (
            "repository-access-auditor",
            &["--report-path=docs/workspace/SRS.md", "--strict=1"],
            "V6-SEC-004",
        ),
        (
            "formal-invariant-engine",
            &["--proof-pack=proofs/layer0", "--strict=1"],
            "V6-SEC-005",
        ),
        (
            "request-ingress",
            &[
                "--policy-version=2026-03",
                "--contact=security@infring.ai",
                "--strict=1",
            ],
            "V6-SEC-006",
        ),
        (
            "external-security-cycle",
            &["--deployment-id=fleet-alpha", "--strict=1"],
            "V6-SEC-007",
        ),
        (
            "model-vaccine-sandbox",
            &["--suite=nightly-fuzz-chaos", "--strict=1"],
            "V6-SEC-008",
        ),
        (
            "enterprise-access-gate",
            &["--profile=gov-high-assurance", "--strict=1"],
            "V6-SEC-009",
        ),
        (
            "governance-hardening-lane",
            &[
                "--scoreboard-path=core/local/state/ops/security_plane/contracts/V6-SEC-013.json",
                "--window-days=30",
                "--strict=1",
            ],
            "V6-SEC-013",
        ),
        (
            "copy-hardening-pack",
            &[
                "--pack-uri=security://zeroleaks-hardened",
                "--version=2026.03",
                "--strict=1",
            ],
            "V6-SEC-014",
        ),
        (
            "mcp-a2a-venom-contract-gate",
            &["--boundary=conduit_only", "--strict=1"],
            "V6-SEC-015",
        ),
        (
            "signed-plugin-trust-marketplace",
            &[
                "--advisory-id=CVE-2026-0001",
                "--sbom-digest=sha256:abc123",
                "--strict=1",
            ],
            "V6-SEC-017",
        ),
    ];

    for (command, args, claim_id) in cases {
        let mut argv = vec![command.to_string()];
        argv.extend(args.iter().map(|v| v.to_string()));
        let exit = security_plane::run(root, &argv);
        assert_eq!(exit, 0, "expected command to pass: {command}");
        let latest = read_json(&latest_path(root));
        assert_eq!(
            latest.get("contract_id").and_then(Value::as_str),
            Some(claim_id),
            "expected contract id match for command {command}"
        );
        assert_claim(&latest, claim_id);
    }

    let fail = security_plane::run(
        root,
        &[
            "mcp-a2a-venom-contract-gate".to_string(),
            "--boundary=any".to_string(),
            "--strict=1".to_string(),
        ],
    );
    assert_eq!(fail, 2, "strict conduit boundary mismatch must fail closed");
}

