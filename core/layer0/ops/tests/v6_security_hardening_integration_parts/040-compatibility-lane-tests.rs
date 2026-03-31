
#[test]
fn v6_sec_additional_compatibility_lanes_now_enforce_contract_flags() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let fail_only: [(&str, &str); 24] = [
        ("capability-envelope-guard", "V6-SEC-ENVELOPE-001"),
        ("execution-sandbox-envelope", "V6-SEC-SANDBOX-ENVELOPE-001"),
        ("formal-threat-modeling-engine", "V6-SEC-THREAT-MODEL-001"),
        ("delegated-authority-branching", "V6-SEC-DELEGATED-AUTH-001"),
        (
            "organ-state-encryption-plane",
            "V6-SEC-ORGAN-ENCRYPTION-001",
        ),
        ("key-lifecycle-governor", "V6-SEC-KEY-LIFECYCLE-001"),
        ("supply-chain-trust-plane", "V6-SEC-SUPPLY-TRUST-001"),
        ("post-quantum-migration-lane", "V6-SEC-POST-QUANTUM-001"),
        ("safety-resilience-guard", "V6-SEC-RESILIENCE-001"),
        ("governance-hardening-pack", "V6-SEC-GOVERNANCE-PACK-001"),
        ("operator-terms-ack", "V6-SEC-OPERATOR-TERMS-001"),
        (
            "critical-runtime-formal-depth-pack",
            "V6-SEC-CRITICAL-RUNTIME-001",
        ),
        (
            "dire-case-emergency-autonomy-protocol",
            "V6-SEC-DIRE-AUTONOMY-001",
        ),
        ("phoenix-protocol-respawn-continuity", "V6-SEC-PHOENIX-001"),
        (
            "multi-mind-isolation-boundary-plane",
            "V6-SEC-MULTI-MIND-001",
        ),
        ("irrevocable-geas-covenant", "V6-SEC-GEAS-001"),
        (
            "insider-threat-split-trust-command-governance",
            "V6-SEC-INSIDER-SPLIT-TRUST-001",
        ),
        (
            "independent-safety-coprocessor-veto-plane",
            "V6-SEC-COPROCESSOR-VETO-001",
        ),
        (
            "hardware-root-of-trust-attestation-mesh",
            "V6-SEC-HARDWARE-ATTESTATION-001",
        ),
        ("alias-verification-vault", "V6-SEC-ALIAS-VAULT-001"),
        ("psycheforge-psycheforge-organ", "V6-SEC-PSYCHE-001"),
        ("psycheforge-profile-synthesizer", "V6-SEC-PSYCHE-001"),
        ("psycheforge-temporal-profile-store", "V6-SEC-PSYCHE-001"),
        ("psycheforge-countermeasure-selector", "V6-SEC-PSYCHE-001"),
    ];

    for (command, contract_id) in fail_only {
        assert_eq!(
            security_plane::run(root, &[command.to_string(), "--strict=1".to_string()]),
            2,
            "expected strict missing-flag failure for {command}"
        );
        let latest = read_json(&latest_path(root));
        assert_eq!(
            latest.get("contract_id").and_then(Value::as_str),
            Some(contract_id)
        );
        assert_eq!(
            latest.get("type").and_then(Value::as_str),
            Some("security_plane_contract_lane")
        );
    }

    let success_cases: [(&str, &[&str], &str); 24] = [
        (
            "capability-envelope-guard",
            &[
                "--capability=tool_exec",
                "--boundary=conduit_only",
                "--strict=1",
            ],
            "V6-SEC-ENVELOPE-001",
        ),
        (
            "execution-sandbox-envelope",
            &["--sandbox=enabled", "--strict=1"],
            "V6-SEC-SANDBOX-ENVELOPE-001",
        ),
        (
            "formal-threat-modeling-engine",
            &[
                "--threat-model-path=docs/security/threat-model.md",
                "--strict=1",
            ],
            "V6-SEC-THREAT-MODEL-001",
        ),
        (
            "delegated-authority-branching",
            &[
                "--authority-branch=ops.secure",
                "--delegation-token=tok_abc123",
                "--strict=1",
            ],
            "V6-SEC-DELEGATED-AUTH-001",
        ),
        (
            "organ-state-encryption-plane",
            &["--algorithm=aes-256-gcm", "--key-id=k1", "--strict=1"],
            "V6-SEC-ORGAN-ENCRYPTION-001",
        ),
        (
            "key-lifecycle-governor",
            &["--key-id=k1", "--action=rotate", "--strict=1"],
            "V6-SEC-KEY-LIFECYCLE-001",
        ),
        (
            "supply-chain-trust-plane",
            &[
                "--sbom-digest=sha256:abc123",
                "--provenance=slsa-level-3",
                "--strict=1",
            ],
            "V6-SEC-SUPPLY-TRUST-001",
        ),
        (
            "post-quantum-migration-lane",
            &["--profile=hybrid", "--phase=pilot", "--strict=1"],
            "V6-SEC-POST-QUANTUM-001",
        ),
        (
            "safety-resilience-guard",
            &[
                "--scenario=region-failover",
                "--rto-seconds=60",
                "--strict=1",
            ],
            "V6-SEC-RESILIENCE-001",
        ),
        (
            "governance-hardening-pack",
            &[
                "--pack-id=gov-hardening-2026q1",
                "--window-days=30",
                "--strict=1",
            ],
            "V6-SEC-GOVERNANCE-PACK-001",
        ),
        (
            "operator-terms-ack",
            &[
                "--operator-id=operator-jh-001",
                "--terms-version=2026-03",
                "--strict=1",
            ],
            "V6-SEC-OPERATOR-TERMS-001",
        ),
        (
            "critical-runtime-formal-depth-pack",
            &[
                "--proof-pack=proofs/layer0",
                "--depth-level=deep",
                "--strict=1",
            ],
            "V6-SEC-CRITICAL-RUNTIME-001",
        ),
        (
            "dire-case-emergency-autonomy-protocol",
            &[
                "--incident-id=inc-2026-0007",
                "--trigger=manual-override",
                "--strict=1",
            ],
            "V6-SEC-DIRE-AUTONOMY-001",
        ),
        (
            "phoenix-protocol-respawn-continuity",
            &[
                "--continuity-id=phoenix-alpha",
                "--checkpoint=cp-42",
                "--strict=1",
            ],
            "V6-SEC-PHOENIX-001",
        ),
        (
            "multi-mind-isolation-boundary-plane",
            &["--boundary=strict", "--mind-id=planner-1", "--strict=1"],
            "V6-SEC-MULTI-MIND-001",
        ),
        (
            "irrevocable-geas-covenant",
            &[
                "--covenant-id=geas-prod",
                "--signer=safety-officer",
                "--strict=1",
            ],
            "V6-SEC-GEAS-001",
        ),
        (
            "insider-threat-split-trust-command-governance",
            &[
                "--approver-a=sec-lead",
                "--approver-b=ops-lead",
                "--strict=1",
            ],
            "V6-SEC-INSIDER-SPLIT-TRUST-001",
        ),
        (
            "independent-safety-coprocessor-veto-plane",
            &[
                "--coprocessor-id=fpga-veto-1",
                "--veto-mode=hard",
                "--strict=1",
            ],
            "V6-SEC-COPROCESSOR-VETO-001",
        ),
        (
            "hardware-root-of-trust-attestation-mesh",
            &[
                "--attestation-doc=proofs/attestation.json",
                "--node-id=node-a1",
                "--strict=1",
            ],
            "V6-SEC-HARDWARE-ATTESTATION-001",
        ),
        (
            "alias-verification-vault",
            &[
                "--alias=prod-deploy-bot",
                "--identity-hash=sha256:deadbeef",
                "--strict=1",
            ],
            "V6-SEC-ALIAS-VAULT-001",
        ),
        (
            "psycheforge-psycheforge-organ",
            &["--profile=probe", "--confidence=0.98", "--strict=1"],
            "V6-SEC-PSYCHE-001",
        ),
        (
            "psycheforge-profile-synthesizer",
            &[
                "--signal-pack=signals/a.json",
                "--profile=exfil",
                "--strict=1",
            ],
            "V6-SEC-PSYCHE-001",
        ),
        (
            "psycheforge-temporal-profile-store",
            &["--profile=drift", "--window-hours=24", "--strict=1"],
            "V6-SEC-PSYCHE-001",
        ),
        (
            "psycheforge-countermeasure-selector",
            &[
                "--profile=escalation",
                "--response-level=high",
                "--strict=1",
            ],
            "V6-SEC-PSYCHE-001",
        ),
    ];

    for (command, args, contract_id) in success_cases {
        let mut argv = vec![command.to_string()];
        argv.extend(args.iter().map(|row| row.to_string()));
        assert_eq!(
            security_plane::run(root, &argv),
            0,
            "expected success for {command}"
        );
        let latest = read_json(&latest_path(root));
        assert_eq!(
            latest.get("contract_id").and_then(Value::as_str),
            Some(contract_id)
        );
        assert_eq!(
            latest.get("ok").and_then(Value::as_bool),
            Some(true),
            "expected contract lane ok for {command}"
        );
    }
}


