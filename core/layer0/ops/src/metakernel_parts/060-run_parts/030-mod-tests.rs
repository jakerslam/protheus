
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
