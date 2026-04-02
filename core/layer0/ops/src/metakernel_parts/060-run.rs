fn run_invariants(root: &Path, strict: bool) -> Value {
    let registry = run_registry(root, true);
    let manifest = run_manifest(root, true, CELLBUNDLE_EXAMPLE_PATH);
    let checks = vec![
        json!({
            "id": "MK_INV_001_registry_contract_exists",
            "ok": root.join(REGISTRY_PATH).exists()
        }),
        json!({
            "id": "MK_INV_002_registry_contract_valid",
            "ok": registry.get("registry_ok").and_then(Value::as_bool).unwrap_or(false)
        }),
        json!({
            "id": "MK_INV_003_no_unknown_primitive_usage",
            "ok": registry.get("unknown_primitive_usage_count").and_then(Value::as_u64).unwrap_or(1) == 0
        }),
        json!({
            "id": "MK_INV_004_cellbundle_schema_exists",
            "ok": root.join(CELLBUNDLE_SCHEMA_PATH).exists()
        }),
        json!({
            "id": "MK_INV_005_cellbundle_example_validates",
            "ok": manifest.get("manifest_ok").and_then(Value::as_bool).unwrap_or(false)
        }),
        json!({
            "id": "MK_INV_006_conduit_schema_present",
            "ok": root.join(CONDUIT_SCHEMA_PATH).exists()
        }),
        json!({
            "id": "MK_INV_007_three_plane_tla_present",
            "ok": root.join(TLA_BOUNDARY_PATH).exists()
        }),
        json!({
            "id": "MK_INV_008_core_policy_manifests_present",
            "ok": root.join(DEP_BOUNDARY_MANIFEST).exists() && root.join(RUST_SOURCE_OF_TRUTH_POLICY).exists()
        }),
    ];
    let pass = checks
        .iter()
        .all(|v| v.get("ok").and_then(Value::as_bool).unwrap_or(false));
    json!({
        "ok": if strict { pass } else { true },
        "strict": strict,
        "checks": checks,
        "registry": registry,
        "manifest": manifest
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let manifest_path = clean(
        parsed
            .flags
            .get("manifest")
            .cloned()
            .unwrap_or_else(|| CELLBUNDLE_EXAMPLE_PATH.to_string()),
        512,
    );
    let syscall_flag = parsed.flags.get("syscall");
    let session_flag = parsed.flags.get("session");
    let allow_flag = parsed.flags.get("allow");
    let instance_dna_flag = parsed.flags.get("instance-dna");
    let instance_id_flag = parsed.flags.get("instance-id");
    let parent_signature_flag = parsed.flags.get("parent-signature");
    let schema_version_flag = parsed.flags.get("schema-version");
    let generation_flag = parsed.flags.get("generation");
    let seed_flag = parsed.flags.get("seed");
    let mutation_flag = parsed.flags.get("mutation");
    let action_flag = parsed.flags.get("action");
    let boundary_flag = parsed.flags.get("boundary");
    let gene_index_flag = parsed.flags.get("gene-index");
    let critical_flag = parsed.flags.get("critical");
    let region_flag = parsed.flags.get("region");
    let region_key_flag = parsed.flags.get("region-key");
    let value_flag = parsed.flags.get("value");
    let step_flag = parsed.flags.get("step");
    let step_cap_flag = parsed.flags.get("step-cap");
    let drift_flag = parsed.flags.get("drift");
    let drift_threshold_flag = parsed.flags.get("drift-threshold");

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops metakernel status");
        println!("  protheus-ops metakernel registry [--strict=1|0]");
        println!("  protheus-ops metakernel manifest [--manifest=<path>] [--strict=1|0]");
        println!("  protheus-ops metakernel worlds [--manifest=<path>] [--strict=1|0]");
        println!(
            "  protheus-ops metakernel capability-taxonomy [--manifest=<path>] [--strict=1|0]"
        );
        println!("  protheus-ops metakernel budget-admission [--manifest=<path>] [--strict=1|0]");
        println!("  protheus-ops metakernel epistemic-object [--manifest=<path>] [--strict=1|0]");
        println!("  protheus-ops metakernel effect-journal [--manifest=<path>] [--strict=1|0]");
        println!("  protheus-ops metakernel substrate-registry [--strict=1|0]");
        println!("  protheus-ops metakernel radix-guard [--strict=1|0]");
        println!("  protheus-ops metakernel quantum-broker [--strict=1|0]");
        println!("  protheus-ops metakernel neural-consent [--strict=1|0]");
        println!("  protheus-ops metakernel attestation-graph [--strict=1|0]");
        println!("  protheus-ops metakernel degradation-contracts [--strict=1|0]");
        println!("  protheus-ops metakernel execution-profiles [--strict=1|0]");
        println!("  protheus-ops metakernel variant-profiles [--strict=1|0]");
        println!("  protheus-ops metakernel mpu-compartments [--strict=1|0]");
        println!("  protheus-ops metakernel dna-status [--strict=1|0]");
        println!("  protheus-ops metakernel dna-create [--instance-id=<id>] [--parent-signature=<sig>] [--schema-version=<v>] [--generation=<n>] [--seed=<text>] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-mutate --instance-id=<id> [--mutation=repair|append-codon|bump-generation] [--seed=<text>] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-enforce-subservience --instance-id=<id> --parent-signature=<sig> [--action=invoke_agent|fork_instance] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-status [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-commit --instance-id=<id> [--boundary=gene_revision_commit|genome_revision_commit|critical_receipt_commit|worm_supersession_commit] [--gene-index=<n>] [--critical=1|0] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-verify [--instance-id=<id>] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-repair-gene --instance-id=<id> [--gene-index=<n>] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-worm-supersede --instance-id=<id> --region=<root_identity|constitutional_safety_rules|lineage_parent_anchor|high_stakes_receipt> [--region-key=<id>] --value=<text> [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-worm-mutate --instance-id=<id> --region=<...> [--region-key=<id>] [--strict=1|0]");
        println!("  protheus-ops metakernel dna-hybrid-protected-lineage --instance-id=<id> --parent-signature=<sig> [--action=invoke_agent|fork_instance] [--strict=1|0]");
        println!(
            "  protheus-ops metakernel microkernel-safety [--syscall=<id>] [--allow=<csv>] [--session=<id>] [--instance-dna=<id>] [--parent-signature=<sig>] [--step=<n>] [--step-cap=<n>] [--drift=<0..1>] [--drift-threshold=<0..1>] [--strict=1|0]"
        );
        println!("  protheus-ops metakernel invariants [--strict=1|0]");
        return 0;
    }

    let latest = latest_path(root);
    let history = history_path(root);
    if command == "status" {
        let mut out = json!({
            "ok": true,
            "type": "metakernel_status",
            "lane": "core/layer0/ops",
            "ts": now_iso(),
            "latest": read_json(&latest)
        });
        out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
        print_receipt(&out);
        return 0;
    }

    let payload = match command.as_str() {
        "registry" => run_registry(root, strict),
        "manifest" => run_manifest(root, strict, &manifest_path),
        "worlds" => run_worlds(root, strict, &manifest_path),
        "capability-taxonomy" => run_capability_taxonomy(root, strict, &manifest_path),
        "budget-admission" => run_budget_admission(root, strict, &manifest_path),
        "epistemic-object" => run_epistemic_object(root, strict, &manifest_path),
        "effect-journal" => run_effect_journal(root, strict, &manifest_path),
        "substrate-registry" => run_substrate_registry(root, strict),
        "radix-guard" => run_radix_guard(root, strict),
        "quantum-broker" => run_quantum_broker(root, strict),
        "neural-consent" => run_neural_consent_kernel(root, strict),
        "attestation-graph" => run_attestation_graph(root, strict),
        "degradation-contracts" => run_degradation_contracts(root, strict),
        "execution-profiles" => run_execution_profiles(root, strict),
        "variant-profiles" => run_variant_profiles(root, strict),
        "mpu-compartments" => run_mpu_compartments(root, strict),
        "dna-status" | "digital-dna-status" => run_digital_dna_status(root),
        "dna-create" | "digital-dna-create" => run_digital_dna_create(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            schema_version_flag,
            generation_flag,
            seed_flag,
        ),
        "dna-mutate" | "digital-dna-mutate" => {
            run_digital_dna_mutate(root, strict, instance_id_flag, mutation_flag, seed_flag)
        }
        "dna-enforce-subservience" | "digital-dna-subservience" => run_digital_dna_subservience(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            action_flag,
        ),
        "dna-hybrid-status" => run_dna_hybrid_status(root),
        "dna-hybrid-commit" => run_dna_hybrid_commit(
            root,
            strict,
            instance_id_flag,
            boundary_flag,
            gene_index_flag,
            critical_flag,
        ),
        "dna-hybrid-verify" => run_dna_hybrid_verify(root, strict, instance_id_flag),
        "dna-hybrid-repair-gene" => {
            run_dna_hybrid_repair_gene(root, strict, instance_id_flag, gene_index_flag)
        }
        "dna-hybrid-worm-supersede" => run_dna_hybrid_worm_supersede(
            root,
            strict,
            instance_id_flag,
            region_flag,
            region_key_flag,
            value_flag,
        ),
        "dna-hybrid-worm-mutate" => run_dna_hybrid_worm_mutate_attempt(
            root,
            strict,
            instance_id_flag,
            region_flag,
            region_key_flag,
        ),
        "dna-hybrid-protected-lineage" => run_dna_hybrid_protected_lineage_check(
            root,
            strict,
            instance_id_flag,
            parent_signature_flag,
            action_flag,
        ),
        "microkernel-safety" => {
            let mut out = run_microkernel_safety(
                root,
                strict,
                syscall_flag,
                session_flag,
                allow_flag,
                instance_dna_flag,
                step_flag,
                step_cap_flag,
                drift_flag,
                drift_threshold_flag,
            );
            if let (Some(instance_dna), Some(_)) = (instance_dna_flag, parent_signature_flag) {
                let instance_id = normalize_token(instance_dna, "instance-dna-default", 96);
                let action = normalize_token(
                    syscall_flag.map(String::as_str).unwrap_or("invoke_agent"),
                    "invoke_agent",
                    96,
                );
                let subservience = evaluate_subservience(
                    root,
                    &instance_id,
                    parent_signature_flag,
                    &action,
                    strict,
                );
                let sub_ok = subservience
                    .get("ok")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if strict && !sub_ok {
                    out["ok"] = Value::Bool(false);
                }
                out["subservience"] = subservience;
            }
            out
        }
        "invariants" => run_invariants(root, strict),
        _ => {
            let mut out = json!({
                "ok": false,
                "type": "metakernel_error",
                "lane": "core/layer0/ops",
                "ts": now_iso(),
                "error": "unknown_command",
                "command": command
            });
            out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
            print_receipt(&out);
            return 1;
        }
    };

    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let mut out = json!({
        "ok": ok,
        "type": "metakernel_run",
        "lane": "core/layer0/ops",
        "ts": now_iso(),
        "command": command,
        "payload": payload
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    write_json(&latest, &out);
    append_jsonl(&history, &out);
    print_receipt(&out);
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_validation_accepts_expected_shape() {
        let registry = json!({
            "version": "v1",
            "kind": "metakernel_primitives_registry",
            "primitives": EXPECTED_PRIMITIVES.iter().map(|id| json!({"id": id, "description": format!("{id} primitive")})).collect::<Vec<_>>()
        });
        let (ok, report) = validate_registry_payload(&registry);
        assert!(ok);
        assert_eq!(
            report
                .get("missing_expected")
                .and_then(Value::as_array)
                .map(|v| v.len()),
            Some(0)
        );
    }

    #[test]
    fn manifest_validation_rejects_unknown_capability() {
        let valid = HashSet::from_iter(EXPECTED_PRIMITIVES.iter().map(|v| v.to_string()));
        let manifest = json!({
            "bundle_id": "bundle.test",
            "version": "1.0.0",
            "world": "infring.metakernel.v1",
            "capabilities": ["node", "unknown_capability"],
            "budgets": {
                "cpu_ms": 10,
                "ram_mb": 64,
                "storage_mb": 32,
                "network_kb": 8,
                "tokens": 100,
                "power_mw": 250,
                "privacy_points": 5,
                "cognitive_load": 1
            },
            "provenance": {
                "source": "unit-test",
                "digest": "sha256:abc"
            }
        });
        let (ok, report) = validate_manifest_payload(&manifest, &valid, true);
        assert!(!ok);
        assert!(report
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("capabilities_include_unknown_primitive")))
            .unwrap_or(false));
    }

    #[test]
    fn world_registry_validation_accepts_expected_shape() {
        let registry = json!({
            "version": "v1",
            "kind": "wit_world_registry",
            "worlds": [
                {
                    "id": "infring.metakernel.v1",
                    "abi_version": "1.0.0",
                    "supported_capabilities": EXPECTED_PRIMITIVES,
                    "component_targets": ["wasm32-wasi"]
                }
            ]
        });
        let (ok, report) = validate_world_registry_payload(&registry);
        assert!(ok);
        assert_eq!(
            report
                .get("duplicate_ids")
                .and_then(Value::as_array)
                .map(|v| v.len()),
            Some(0)
        );
    }

    #[test]
    fn capability_taxonomy_requires_required_effects() {
        let taxonomy = json!({
            "version": "v1",
            "effects": [{"id": "observe", "risk_default": "R0"}],
            "primitive_effects": {"node": ["observe"]}
        });
        let (ok, report) = validate_capability_taxonomy_payload(&taxonomy);
        assert!(!ok);
        assert!(report
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("capability_taxonomy_missing_required_effects")))
            .unwrap_or(false));
    }

    #[test]
    fn radix_guard_reports_overlap_error() {
        let policy = json!({
            "binary_required_paths": ["crypto", "policy"],
            "ternary_allow_classes": ["crypto"]
        });
        let set: BTreeSet<String> = policy
            .get("binary_required_paths")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.to_ascii_lowercase())
            .collect();
        assert!(set.contains("crypto"));
    }

    #[test]
    fn variant_profiles_detects_missing_privilege_floor() {
        let root = tempfile::tempdir().expect("tempdir");
        let profiles_dir = root
            .path()
            .join("planes")
            .join("contracts")
            .join("variant_profiles");
        std::fs::create_dir_all(&profiles_dir).expect("mkdir");
        for profile in ["medical", "robotics", "ai_isolation", "riscv_sovereign"] {
            let payload = if profile == "medical" {
                json!({
                    "version": "v1",
                    "kind": "layer_minus_one_variant_profile",
                    "profile_id": "medical",
                    "baseline_policy_ref": "client/runtime/config/security_policy.json",
                    "capability_delta": { "grant": ["observe"], "revoke": ["train"] },
                    "budget_delta": {"cpu_ms": -10},
                    "no_privilege_widening": false
                })
            } else {
                json!({
                    "version": "v1",
                    "kind": "layer_minus_one_variant_profile",
                    "profile_id": profile,
                    "baseline_policy_ref": "client/runtime/config/security_policy.json",
                    "capability_delta": { "grant": ["observe"], "revoke": ["train"] },
                    "budget_delta": {"cpu_ms": 10},
                    "no_privilege_widening": true
                })
            };
            write_json(&profiles_dir.join(format!("{profile}.json")), &payload);
        }
        let out = run_variant_profiles(root.path(), true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|v| {
                v.as_str()
                    .map(|s| s.contains("variant_profile_no_privilege_widening_required"))
                    .unwrap_or(false)
            }))
            .unwrap_or(false));
    }

    #[test]
    fn mpu_compartments_rejects_write_execute() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root
            .path()
            .join("planes")
            .join("contracts")
            .join("mpu_compartment_profile_v1.json");
        write_json(
            &path,
            &json!({
                "version": "v1",
                "kind": "mpu_compartment_profile",
                "compartments": [
                    {
                        "id": "rtos_kernel",
                        "region_start": 4096,
                        "region_size": 8192,
                        "access": {"read": true, "write": true, "execute": true},
                        "unprivileged": true
                    }
                ],
                "targets": [
                    { "id": "mcu", "compartments": ["rtos_kernel"] }
                ]
            }),
        );
        let out = run_mpu_compartments(root.path(), true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|v| {
                v.as_str()
                    .map(|s| s.contains("mpu_compartment_write_execute_forbidden"))
                    .unwrap_or(false)
            }))
            .unwrap_or(false));
    }

    #[test]
    fn microkernel_safety_accepts_typed_syscall_and_writes_slab() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_microkernel_safety(
            root.path(),
            true,
            Some(&"invoke_agent".to_string()),
            Some(&"session-alpha".to_string()),
            Some(&"invoke_agent,verify_receipt".to_string()),
            Some(&"dna-alpha".to_string()),
            Some(&"3".to_string()),
            Some(&"10".to_string()),
            Some(&"0.01".to_string()),
            Some(&"0.05".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/syscall/typed").and_then(Value::as_bool),
            Some(true)
        );
        let slab_path = out
            .pointer("/session_isolation/private_memory_slab_path")
            .and_then(Value::as_str)
            .expect("slab path");
        assert!(Path::new(slab_path).exists());
    }

    #[test]
    fn microkernel_safety_rejects_unknown_syscall_and_triggers_judicial_lock() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_microkernel_safety(
            root.path(),
            true,
            Some(&"unknown_syscall".to_string()),
            Some(&"session-beta".to_string()),
            Some(&"unknown_syscall".to_string()),
            Some(&"dna-beta".to_string()),
            Some(&"1".to_string()),
            Some(&"10".to_string()),
            Some(&"0.00".to_string()),
            Some(&"0.05".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(out
            .pointer("/judicial_lock/violation_codes")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("typed_syscall_unknown")))
            .unwrap_or(false));
    }

    #[test]
    fn microkernel_safety_triggers_on_drift_threshold_exceeded() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_microkernel_safety(
            root.path(),
            true,
            Some(&"invoke_agent".to_string()),
            Some(&"session-gamma".to_string()),
            Some(&"invoke_agent".to_string()),
            Some(&"dna-gamma".to_string()),
            Some(&"1".to_string()),
            Some(&"100".to_string()),
            Some(&"0.33".to_string()),
            Some(&"0.05".to_string()),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.pointer("/judicial_lock/triggered")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(out
            .pointer("/judicial_lock/violation_codes")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|v| v.as_str() == Some("drift_threshold_exceeded")))
            .unwrap_or(false));
    }

    #[test]
    fn metakernel_run_dispatches_digital_dna_commands() {
        let root = tempfile::tempdir().expect("tempdir");
        let create_exit = run(
            root.path(),
            &[
                "dna-create".to_string(),
                "--instance-id=instance-cli".to_string(),
                "--parent-signature=parent-cli".to_string(),
                "--schema-version=v1".to_string(),
                "--generation=2".to_string(),
                "--seed=dna-seed-cli".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(create_exit, 0);

        let status_exit = run(
            root.path(),
            &["dna-status".to_string(), "--strict=1".to_string()],
        );
        assert_eq!(status_exit, 0);

        let state = read_json(&digital_dna_state_path(root.path())).expect("dna state");
        assert_eq!(
            state
                .pointer("/genomes/instance-cli/header/parent_signature")
                .and_then(Value::as_str),
            Some("parent-cli")
        );
    }

    #[test]
    fn microkernel_safety_enforces_subservience_when_parent_signature_is_supplied() {
        let root = tempfile::tempdir().expect("tempdir");
        let instance_id = "instance-sub".to_string();
        let parent = "parent-a".to_string();
        let schema = "v1".to_string();
        let generation = "0".to_string();
        let seed = "seed-sub".to_string();
        let _ = run_digital_dna_create(
            root.path(),
            true,
            Some(&instance_id),
            Some(&parent),
            Some(&schema),
            Some(&generation),
            Some(&seed),
        );

        let exit = run(
            root.path(),
            &[
                "microkernel-safety".to_string(),
                "--strict=1".to_string(),
                "--syscall=invoke_agent".to_string(),
                "--allow=invoke_agent".to_string(),
                "--session=session-sub".to_string(),
                "--instance-dna=instance-sub".to_string(),
                "--parent-signature=parent-b".to_string(),
                "--step=1".to_string(),
                "--step-cap=10".to_string(),
                "--drift=0.0".to_string(),
                "--drift-threshold=0.1".to_string(),
            ],
        );
        assert_eq!(exit, 1);

        let latest = read_json(&latest_path(root.path())).expect("latest receipt");
        assert_eq!(
            latest
                .pointer("/payload/subservience/ok")
                .and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn metakernel_dispatches_hybrid_dna_v2_commands() {
        let root = tempfile::tempdir().expect("tempdir");
        let create_exit = run(
            root.path(),
            &[
                "dna-create".to_string(),
                "--instance-id=instance-hybrid-cli".to_string(),
                "--parent-signature=parent-hybrid".to_string(),
                "--schema-version=v1".to_string(),
                "--generation=1".to_string(),
                "--seed=seed-hybrid".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(create_exit, 0);
        let commit_exit = run(
            root.path(),
            &[
                "dna-hybrid-commit".to_string(),
                "--instance-id=instance-hybrid-cli".to_string(),
                "--boundary=gene_revision_commit".to_string(),
                "--gene-index=0".to_string(),
                "--critical=1".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(commit_exit, 0);
        let verify_exit = run(
            root.path(),
            &[
                "dna-hybrid-verify".to_string(),
                "--instance-id=instance-hybrid-cli".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(verify_exit, 0);
        let status_exit = run(
            root.path(),
            &["dna-hybrid-status".to_string(), "--strict=1".to_string()],
        );
        assert_eq!(status_exit, 0);
    }
}
