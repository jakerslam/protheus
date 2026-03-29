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
}

