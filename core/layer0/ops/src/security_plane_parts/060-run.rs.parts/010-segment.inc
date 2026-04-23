pub fn run(root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let rest = if argv.is_empty() { &[][..] } else { &argv[1..] };
    let strict = parse_bool(parse_flag(rest, "strict"), true);

    let (payload, code) = match cmd.as_str() {
        "guard" => crate::infring_layer1_security_bridge::run_guard(root, rest),
        "anti-sabotage-shield" | "anti_sabotage_shield" => {
            crate::infring_layer1_security_bridge::run_anti_sabotage_shield(root, rest)
        }
        "constitution-guardian" | "constitution_guardian" => {
            crate::infring_layer1_security_bridge::run_constitution_guardian(root, rest)
        }
        "remote-emergency-halt" | "remote_emergency_halt" => {
            crate::infring_layer1_security_bridge::run_remote_emergency_halt(root, rest)
        }
        "soul-token-guard" | "soul_token_guard" => {
            crate::infring_layer1_security_bridge::run_soul_token_guard(root, rest)
        }
        "integrity-reseal" | "integrity_reseal" => {
            crate::infring_layer1_security_bridge::run_integrity_reseal(root, rest)
        }
        "integrity-reseal-assistant" | "integrity_reseal_assistant" => {
            crate::infring_layer1_security_bridge::run_integrity_reseal_assistant(root, rest)
        }
        "capability-lease" | "capability_lease" => {
            crate::infring_layer1_security_bridge::run_capability_lease(root, rest)
        }
        "startup-attestation" | "startup_attestation" => {
            crate::infring_layer1_security_bridge::run_startup_attestation(root, rest)
        }
        "directive-hierarchy-controller" | "directive_hierarchy_controller" => {
            crate::infring_layer1_security_bridge::run_directive_hierarchy_controller(root, rest)
        }
        "integrity-kernel" | "integrity_kernel" => {
            crate::infring_layer1_security_bridge::run_integrity_kernel(root, rest)
        }
        "emergency-stop" | "emergency_stop" => {
            crate::infring_layer1_security_bridge::run_emergency_stop(root, rest)
        }
        "capability-switchboard" | "capability_switchboard" => {
            crate::infring_layer1_security_bridge::run_capability_switchboard(root, rest)
        }
        "black-box-ledger" | "black_box_ledger" => {
            crate::infring_layer1_security_bridge::run_black_box_ledger(root, rest)
        }
        "t0-invariants" | "t0_invariants" => crate::t0_invariants_kernel::run(root, rest),
        "thorn-swarm-protocol" | "thorn_swarm_protocol" | "thorn" => {
            crate::swarm_runtime::run_thorn_contract(root, rest)
        }
        "psycheforge" => crate::psycheforge_kernel::run(root, rest),
        "goal-preservation-kernel" | "goal_preservation_kernel" => {
            crate::infring_layer1_security_bridge::run_goal_preservation_kernel(root, rest)
        }
        "dream-warden-guard" | "dream_warden_guard" => {
            crate::infring_layer1_security_bridge::run_dream_warden_guard(root, rest)
        }
        "truth-seeking-gate" | "truth_seeking_gate" | "truth-gate" | "truth_gate" => {
            crate::infring_layer1_security_bridge::run_truth_seeking_gate(root, rest)
        }
        "abac-policy-plane" | "abac_policy_plane" => {
            crate::infring_layer1_security_bridge::run_abac_policy_plane(root, rest)
        }
        "scan" => run_scan_command(root, rest, strict),
        "remediate" | "auto-remediate" | "auto_remediate" => {
            run_remediation_command(root, rest, strict)
        }
        "verify-proofs" | "verify_proofs" => run_verify_proofs_command(root, rest, strict),
        "audit-logs" | "audit_logs" => run_audit_logs_command(root, rest, strict),
        "threat-model" | "threat_model" => run_threat_model_command(root, rest, strict),
        "blast-radius-sentinel" | "blast_radius_sentinel" => {
            run_blast_radius_command(root, rest, strict)
        }
        "secrets-federation" | "secrets_federation" => {
            run_secrets_federation_command(root, rest, strict)
        }
        "copy-hardening-pack" | "copy_hardening_pack" => run_security_contract_command(
            root,
            rest,
            strict,
            "copy-hardening-pack",
            "V6-SEC-014",
            &[("pack-uri", None), ("version", None)],
        ),
        "governance-hardening-pack" | "governance_hardening_pack" => run_security_contract_command(
            root,
            rest,
            strict,
            "governance-hardening-pack",
            "V6-SEC-GOVERNANCE-PACK-001",
            &[("pack-id", None), ("window-days", None)],
        ),
        "repository-access-auditor" | "repository_access_auditor" => run_security_contract_command(
            root,
            rest,
            strict,
            "repository-access-auditor",
            "V6-SEC-004",
            &[("report-path", None)],
        ),
        "operator-terms-ack" | "operator_terms_ack" => run_security_contract_command(
            root,
            rest,
            strict,
            "operator-terms-ack",
            "V6-SEC-OPERATOR-TERMS-001",
            &[("operator-id", None), ("terms-version", None)],
        ),
        "governance-hardening-lane" | "governance_hardening_lane" => run_security_contract_command(
            root,
            rest,
            strict,
            "governance-hardening-lane",
            "V6-SEC-013",
            &[("scoreboard-path", None), ("window-days", None)],
        ),
        "skill-install-path-enforcer" | "skill_install_path_enforcer" => {
            run_skill_install_path_enforcer(root, rest, strict)
        }
        "skill-quarantine" | "skill_quarantine" => run_skill_quarantine_command(root, rest, strict),
        "autonomous-skill-necessity-audit" | "autonomous_skill_necessity_audit" => {
            run_autonomous_skill_necessity_audit(root, rest, strict)
        }
        "formal-invariant-engine" | "formal_invariant_engine" => run_security_contract_command(
            root,
            rest,
            strict,
            "formal-invariant-engine",
            "V6-SEC-005",
            &[("proof-pack", None)],
        ),
        "repo-hygiene-guard" | "repo_hygiene_guard" => {
            run_repo_hygiene_guard(root, rest, strict, "repo-hygiene-guard")
        }
        "capability-envelope-guard" | "capability_envelope_guard" => run_security_contract_command(
            root,
            rest,
            strict,
            "capability-envelope-guard",
            "V6-SEC-ENVELOPE-001",
            &[("capability", None), ("boundary", Some("conduit_only"))],
        ),
        "ip-posture-review" | "ip_posture_review" => run_security_contract_command(
            root,
            rest,
            strict,
            "ip-posture-review",
            "V6-SEC-002",
            &[("public-url", None)],
        ),
        "habit-hygiene-guard" | "habit_hygiene_guard" => {
            run_repo_hygiene_guard(root, rest, strict, "habit-hygiene-guard")
        }
        "enterprise-access-gate" | "enterprise_access_gate" => run_security_contract_command(
            root,
            rest,
            strict,
            "enterprise-access-gate",
            "V6-SEC-009",
            &[("profile", None)],
        ),
        "model-vaccine-sandbox" | "model_vaccine_sandbox" => run_security_contract_command(
            root,
            rest,
            strict,
            "model-vaccine-sandbox",
            "V6-SEC-008",
            &[("suite", None)],
        ),
        "skill-install-enforcer" | "skill_install_enforcer" => {
            run_skill_install_path_enforcer(root, rest, strict)
        }
        "execution-sandbox-envelope" | "execution_sandbox_envelope" => {
            let mut alias_args = Vec::<String>::new();
            if rest.iter().all(|token| token.starts_with("--")) {
                alias_args.push("enforce".to_string());
            }
            alias_args.extend_from_slice(rest);
            run_openshell_runtime_command(root, &alias_args, strict)
        }
        "openshell" | "openshell-runtime" | "openshell_runtime" | "oshell" => {
            run_openshell_runtime_command(root, rest, strict)
        }
        "workspace-dump-guard" | "workspace_dump_guard" => {
            run_workspace_dump_guard(root, rest, strict)
        }
        "external-security-cycle" | "external_security_cycle" => run_security_contract_command(
            root,
            rest,
            strict,
            "external-security-cycle",
            "V6-SEC-007",
            &[("deployment-id", None)],
        ),
        "log-redaction-guard" | "log_redaction_guard" => {
            run_log_redaction_guard(root, rest, strict)
        }
        "rsi-git-patch-self-mod-gate" | "rsi_git_patch_self_mod_gate" => {
            run_rsi_git_patch_self_mod_gate(root, rest, strict)
        }
        "request-ingress" | "request_ingress" => run_security_contract_command(
            root,
            rest,
            strict,
            "request-ingress",
            "V6-SEC-006",
            &[("policy-version", None), ("contact", None)],
        ),
        "startup-attestation-boot-gate" | "startup_attestation_boot_gate" => {
            run_startup_attestation_boot_gate(root, rest)
        }
        "conflict-marker-guard" | "conflict_marker_guard" => run_repo_hygiene_guard(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "conflict-marker-guard",
        ),
        "llm-gateway-guard" | "llm_gateway_guard" => {
            run_llm_gateway_guard(root, rest, parse_bool(parse_flag(rest, "strict"), true))
        }
        "required-checks-policy-guard" | "required_checks_policy_guard" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "required-checks-policy-guard",
                "V6-SEC-003",
                &[
                    ("codeql", Some("required")),
                    ("dependabot", Some("required")),
                ],
            )
        }
        "mcp-a2a-venom-contract-gate" | "mcp_a2a_venom_contract_gate" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "mcp-a2a-venom-contract-gate",
                "V6-SEC-015",
                &[("boundary", Some("conduit_only"))],
            )
        }
        "critical-runtime-formal-depth-pack" | "critical_runtime_formal_depth_pack" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "critical-runtime-formal-depth-pack",
                "V6-SEC-CRITICAL-RUNTIME-001",
                &[("proof-pack", None), ("depth-level", None)],
            )
        }
        "dire-case-emergency-autonomy-protocol" | "dire_case_emergency_autonomy_protocol" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "dire-case-emergency-autonomy-protocol",
                "V6-SEC-DIRE-AUTONOMY-001",
                &[("incident-id", None), ("trigger", None)],
            )
        }
        "supply-chain-reproducible-build-plane" | "supply_chain_reproducible_build_plane" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "supply-chain-reproducible-build-plane",
                "V6-SEC-001",
                &[("sbom-path", None), ("release-tag", None)],
            )
        }
        "signed-plugin-trust-marketplace" | "signed_plugin_trust_marketplace" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "signed-plugin-trust-marketplace",
                "V6-SEC-017",
                &[("advisory-id", None), ("sbom-digest", None)],
            )
        }
        "phoenix-protocol-respawn-continuity" | "phoenix_protocol_respawn_continuity" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "phoenix-protocol-respawn-continuity",
                "V6-SEC-PHOENIX-001",
                &[("continuity-id", None), ("checkpoint", None)],
            )
        }
        "multi-mind-isolation-boundary-plane" | "multi_mind_isolation_boundary_plane" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "multi-mind-isolation-boundary-plane",
                "V6-SEC-MULTI-MIND-001",
                &[("boundary", Some("strict")), ("mind-id", None)],
            )
        }
        "irrevocable-geas-covenant" | "irrevocable_geas_covenant" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "irrevocable-geas-covenant",
            "V6-SEC-GEAS-001",
            &[("covenant-id", None), ("signer", None)],
        ),
        "insider-threat-split-trust-command-governance"
        | "insider_threat_split_trust_command_governance" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "insider-threat-split-trust-command-governance",
            "V6-SEC-INSIDER-SPLIT-TRUST-001",
            &[("approver-a", None), ("approver-b", None)],
        ),
        "independent-safety-coprocessor-veto-plane"
        | "independent_safety_coprocessor_veto_plane" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "independent-safety-coprocessor-veto-plane",
            "V6-SEC-COPROCESSOR-VETO-001",
            &[("coprocessor-id", None), ("veto-mode", None)],
        ),
        "hardware-root-of-trust-attestation-mesh" | "hardware_root_of_trust_attestation_mesh" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "hardware-root-of-trust-attestation-mesh",
                "V6-SEC-HARDWARE-ATTESTATION-001",
                &[("attestation-doc", None), ("node-id", None)],
            )
        }
        "formal-threat-modeling-engine" | "formal_threat_modeling_engine" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "formal-threat-modeling-engine",
                "V6-SEC-THREAT-MODEL-001",
                &[("threat-model-path", None)],
            )
        }
        "formal-mind-sovereignty-verification" | "formal_mind_sovereignty_verification" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "formal-mind-sovereignty-verification",
                "V6-SEC-MIND-SOVEREIGNTY-001",
                &[("proof-pack", None)],
            )
        }
        "alias-verification-vault" | "alias_verification_vault" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "alias-verification-vault",
            "V6-SEC-ALIAS-VAULT-001",
            &[("alias", None), ("identity-hash", None)],
        ),
        "psycheforge-psycheforge-organ" | "psycheforge_psycheforge_organ" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "psycheforge-psycheforge-organ",
                "V6-SEC-PSYCHE-001",
                &[("profile", None), ("confidence", None)],
            )
        }
        "psycheforge-profile-synthesizer" | "psycheforge_profile_synthesizer" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "psycheforge-profile-synthesizer",
                "V6-SEC-PSYCHE-001",
                &[("signal-pack", None), ("profile", None)],
            )
        }
        "psycheforge-temporal-profile-store" | "psycheforge_temporal_profile_store" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "psycheforge-temporal-profile-store",
                "V6-SEC-PSYCHE-001",
                &[("profile", None), ("window-hours", None)],
            )
        }
        "psycheforge-countermeasure-selector" | "psycheforge_countermeasure_selector" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "psycheforge-countermeasure-selector",
                "V6-SEC-PSYCHE-001",
                &[("profile", None), ("response-level", None)],
            )
        }
        "delegated-authority-branching" | "delegated_authority_branching" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "delegated-authority-branching",
                "V6-SEC-DELEGATED-AUTH-001",
                &[("authority-branch", None), ("delegation-token", None)],
            )
        }
        "organ-state-encryption-plane" | "organ_state_encryption_plane" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "organ-state-encryption-plane",
                "V6-SEC-ORGAN-ENCRYPTION-001",
                &[("algorithm", Some("aes-256-gcm")), ("key-id", None)],
            )
        }
        "remote-tamper-heartbeat" | "remote_tamper_heartbeat" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "remote-tamper-heartbeat",
            "V6-SEC-TAMPER-HEARTBEAT-001",
            &[("heartbeat-id", None), ("epoch", None)],
        ),
        "skin-protection-layer" | "skin_protection_layer" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "skin-protection-layer",
            "V6-SEC-SKIN-PROTECTION-001",
            &[("surface", None)],
        ),
        "critical-path-formal-verifier" | "critical_path_formal_verifier" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "critical-path-formal-verifier",
                "V6-SEC-CRITICAL-FORMAL-001",
                &[("proof-pack", None)],
            )
        }
        "key-lifecycle-governor" | "key_lifecycle_governor" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "key-lifecycle-governor",
            "V6-SEC-KEY-LIFECYCLE-001",
            &[("key-id", None), ("action", None)],
        ),
        "supply-chain-trust-plane" | "supply_chain_trust_plane" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "supply-chain-trust-plane",
            "V6-SEC-SUPPLY-TRUST-001",
            &[("sbom-digest", None), ("provenance", None)],
        ),
        "post-quantum-migration-lane" | "post_quantum_migration_lane" => {
            run_security_contract_command(
                root,
                rest,
                parse_bool(parse_flag(rest, "strict"), true),
                "post-quantum-migration-lane",
                "V6-SEC-POST-QUANTUM-001",
                &[("profile", None), ("phase", None)],
            )
        }
        "safety-resilience-guard" | "safety_resilience_guard" => run_security_contract_command(
            root,
            rest,
            parse_bool(parse_flag(rest, "strict"), true),
            "safety-resilience-guard",
            "V6-SEC-RESILIENCE-001",
            &[("scenario", None), ("rto-seconds", None)],
        ),
        "status" => (
            json!({
                "ok": true,
                "type": "security_plane_status",
                "lane": "core/layer1/security",
                "commands": [
                    "guard",
                    "anti-sabotage-shield",
                    "constitution-guardian",
                    "remote-emergency-halt",
                    "soul-token-guard",
                    "integrity-reseal",
                    "integrity-reseal-assistant",
                    "capability-lease",
                    "startup-attestation",
                    "directive-hierarchy-controller",
                    "integrity-kernel",
                    "emergency-stop",
                    "capability-switchboard",
                    "black-box-ledger",
                    "t0-invariants",
                    "thorn-swarm-protocol",
                    "psycheforge",
                    "goal-preservation-kernel",
                    "dream-warden-guard",
                    "abac-policy-plane",
                    "scan",
                    "remediate",
                    "verify-proofs",
                    "audit-logs",
                    "threat-model",
                    "blast-radius-sentinel",
                    "secrets-federation",
                    "delegated-authority-branching",
                    "organ-state-encryption-plane",
                    "remote-tamper-heartbeat",
                    "skin-protection-layer",
                    "critical-path-formal-verifier",
                    "key-lifecycle-governor",
                    "supply-chain-trust-plane",
                    "post-quantum-migration-lane",
                    "safety-resilience-guard",
                    "rsi-git-patch-self-mod-gate",
                    "request-ingress",
                    "startup-attestation-boot-gate",
                    "conflict-marker-guard",
                    "llm-gateway-guard",
                    "required-checks-policy-guard",
                    "openshell",
                    "mcp-a2a-venom-contract-gate",
                    "critical-runtime-formal-depth-pack",
                    "dire-case-emergency-autonomy-protocol",
                    "supply-chain-reproducible-build-plane",
                    "signed-plugin-trust-marketplace",
                    "phoenix-protocol-respawn-continuity",
                    "multi-mind-isolation-boundary-plane",
                    "irrevocable-geas-covenant",
                    "insider-threat-split-trust-command-governance",
                    "independent-safety-coprocessor-veto-plane",
                    "hardware-root-of-trust-attestation-mesh",
                    "formal-threat-modeling-engine",
                    "formal-mind-sovereignty-verification",
                    "alias-verification-vault",
                    "psycheforge-psycheforge-organ",
                    "psycheforge-profile-synthesizer",
                    "psycheforge-temporal-profile-store",
                    "psycheforge-countermeasure-selector"
                ]
            }),
            0,
        ),
        _ => (
            json!({
                "ok": false,
                "type": "security_plane_error",
                "error": format!("unknown_command:{}", clean(&cmd, 120)),
                "usage": [
                    "infring-ops security-plane guard [--files=<a,b,c>] [--strict=1|0]",
                    "infring-ops security-plane anti-sabotage-shield <snapshot|verify|watch|status> [flags]",
                    "infring-ops security-plane constitution-guardian <init-genesis|propose-change|approve-change|veto-change|run-gauntlet|activate-change|enforce-inheritance|emergency-rollback|status> [flags]",
                    "infring-ops security-plane remote-emergency-halt <status|sign-halt|sign-purge|receive|receive-b64> [flags]",
                    "infring-ops security-plane soul-token-guard <issue|stamp-build|verify|status> [flags]",
                    "infring-ops security-plane integrity-reseal <check|apply> [flags]",
                    "infring-ops security-plane integrity-reseal-assistant <run|status> [flags]",
                    "infring-ops security-plane capability-lease <issue|verify|consume> [flags]",
                    "infring-ops security-plane startup-attestation <issue|verify|status> [flags]",
                    "infring-ops security-plane directive-hierarchy-controller <status|decompose> [flags]",
                    "infring-ops security-plane integrity-kernel <run|status|seal> [flags]",
                    "infring-ops security-plane emergency-stop <status|engage|release> [flags]",
                    "infring-ops security-plane capability-switchboard <status|evaluate|set> [flags]",
                    "infring-ops security-plane black-box-ledger <append|export|rollup|verify|verify-offline|status> [flags]",
                    "infring-ops security-plane t0-invariants <status|evaluate|fuzz> [flags]",
                    "infring-ops security-plane thorn-swarm-protocol <status|quarantine|release> [flags]",
                    "infring-ops security-plane psycheforge <status|profile> [flags]",
                    "infring-ops security-plane goal-preservation-kernel <evaluate|status> [flags]",
                    "infring-ops security-plane dream-warden-guard <run|status> [flags]",
                    "infring-ops security-plane truth-seeking-gate <status|ingest-rule|evaluate> [flags]",
                    "infring-ops security-plane abac-policy-plane <status|evaluate> [flags]",
                    "infring-ops security-plane scan [--prompt=<text>] [--tool-input=<text>] [--mcp=<text>] [--pack=<id>] [--critical-threshold=<n>] [--strict=1|0]",
                    "infring-ops security-plane remediate [--strict=1|0]",
                    "infring-ops security-plane verify-proofs [--proof-pack=<path>] [--min-files=<n>] [--extensions=smt2,lean,proof,json,md] [--strict=1|0]",
                    "infring-ops security-plane audit-logs [--max-events=<n>] [--max-failures=<n>] [--strict=1|0]",
                    "infring-ops security-plane threat-model [--scenario=<id>] [--surface=<id>] [--vector=<text>] [--block-threshold=<n>] [--allow=1|0] [--strict=1|0]",
                    "infring-ops security-plane blast-radius-sentinel <record|status> [--action=<id>] [--target=<id>] [--credential=1|0] [--network=1|0] [--allow=1|0] [--strict=1|0]",
                    "infring-ops security-plane secrets-federation <status|fetch|rotate|revoke> [--provider=vault|aws|1password] [--path=<secret/path>] [--scope=<scope>] [--handle-id=<id>] [--lease-seconds=<n>] [--strict=1|0]",
                    "infring-ops security-plane copy-hardening-pack <command> [flags]",
                    "infring-ops security-plane governance-hardening-pack <command> [flags]",
                    "infring-ops security-plane repository-access-auditor <command> [flags]",
                    "infring-ops security-plane operator-terms-ack <command> [flags]",
                    "infring-ops security-plane governance-hardening-lane <command> [flags]",
                    "infring-ops security-plane skill-install-path-enforcer <command> [flags]",
                    "infring-ops security-plane skill-quarantine <command> [flags]",
                    "infring-ops security-plane autonomous-skill-necessity-audit <command> [flags]",
                    "infring-ops security-plane formal-invariant-engine <command> [flags]",
                    "infring-ops security-plane repo-hygiene-guard <command> [flags]",
                    "infring-ops security-plane rsi-git-patch-self-mod-gate <command> [flags]",
                    "infring-ops security-plane request-ingress <command> [flags]",
                    "infring-ops security-plane startup-attestation-boot-gate <command> [flags]",
                    "infring-ops security-plane conflict-marker-guard <command> [flags]",
                    "infring-ops security-plane llm-gateway-guard <command> [flags]",
                    "infring-ops security-plane openshell <status|validate-policy|enforce|privacy-route|run-agent> [--policy=<path>] [--file=<path>] [--host=<host>] [--credential=<id>] [--payload=<text>] [--strict=1|0]",
                    "infring-ops security-plane required-checks-policy-guard <command> [flags]",
                    "infring-ops security-plane mcp-a2a-venom-contract-gate <command> [flags]",
                    "infring-ops security-plane critical-runtime-formal-depth-pack <command> [flags]",
                    "infring-ops security-plane dire-case-emergency-autonomy-protocol <command> [flags]",
                    "infring-ops security-plane supply-chain-reproducible-build-plane <command> [flags]",
                    "infring-ops security-plane signed-plugin-trust-marketplace <command> [flags]",
                    "infring-ops security-plane phoenix-protocol-respawn-continuity <command> [flags]",
                    "infring-ops security-plane multi-mind-isolation-boundary-plane <command> [flags]",
                    "infring-ops security-plane irrevocable-geas-covenant <command> [flags]",
                    "infring-ops security-plane insider-threat-split-trust-command-governance <command> [flags]",
                    "infring-ops security-plane independent-safety-coprocessor-veto-plane <command> [flags]",
                    "infring-ops security-plane hardware-root-of-trust-attestation-mesh <command> [flags]",
                    "infring-ops security-plane formal-threat-modeling-engine <command> [flags]",
                    "infring-ops security-plane formal-mind-sovereignty-verification <command> [flags]",
                    "infring-ops security-plane alias-verification-vault <command> [flags]",
                    "infring-ops security-plane psycheforge-psycheforge-organ <command> [flags]",
                    "infring-ops security-plane psycheforge-profile-synthesizer <command> [flags]",
                    "infring-ops security-plane psycheforge-temporal-profile-store <command> [flags]",
                    "infring-ops security-plane psycheforge-countermeasure-selector <command> [flags]"
                ]
            }),
            2,
        ),
    };

    let wrapped = wrap_capability_event(root, &cmd, rest, payload);
    persist_security_receipt(root, &wrapped);
    print_json(&wrapped);
    code
}

#[cfg(test)]
mod openshell_integration_tests {
    use super::*;

    fn write_policy(root: &Path, body: &str) -> PathBuf {
        let path = root.join("policy.yaml");
        fs::write(&path, body).expect("write policy");
        path
    }

    #[test]
    fn openshell_validate_policy_signs_when_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = write_policy(
            dir.path(),
            r#"
version: v1
kind: openshell_policy
mode: production
sandbox:
  enabled: true
conduit:
  enforce: true
filesystem:
  allow: ["/workspace/"]
network:
  allow_hosts: ["api.openai.com"]
credentials:
  allow: ["openai_api_key"]
privacy:
  redact_patterns: ["sk-"]
  block_patterns: ["ssn"]
"#,
        );
        let args = vec![
            "validate-policy".to_string(),
            format!("--policy={}", policy.display()),
            "--strict=1".to_string(),
        ];
        let (payload, code) = run_openshell_runtime_command(dir.path(), &args, true);
        assert_eq!(code, 0);
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert!(payload.get("signed_policy").is_some());
    }

    #[test]
    fn openshell_enforce_blocks_disallowed_network_and_triggers_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = write_policy(
            dir.path(),
            r#"
version: v1
kind: openshell_policy
mode: production
sandbox:
  enabled: true
conduit:
  enforce: true
filesystem:
  allow: ["/workspace/"]
network:
  allow_hosts: ["api.allowed.local"]
credentials:
  allow: ["openai_api_key"]
privacy:
  redact_patterns: ["token"]
  block_patterns: ["secret_dump"]
"#,
        );
        let args = vec![
            "enforce".to_string(),
            format!("--policy={}", policy.display()),
            "--host=api.blocked.local".to_string(),
            "--strict=1".to_string(),
        ];
        let (payload, code) = run_openshell_runtime_command(dir.path(), &args, true);
        assert_eq!(code, 2);
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(false));
        assert!(payload
            .get("violation_codes")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("network_egress_denied")))
            .unwrap_or(false));
        assert_eq!(
            payload
                .pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
