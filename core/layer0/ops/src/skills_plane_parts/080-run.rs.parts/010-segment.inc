pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);

    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let status_dashboard = parse_bool(parsed.flags.get("dashboard"), false)
        || parse_bool(parsed.flags.get("top"), false);
    let payload = match command.as_str() {
        "status" if status_dashboard => run_dashboard(root, &parsed, strict),
        "status" => status(root),
        "list" => run_list(root, &parsed, strict),
        "dashboard" => run_dashboard(root, &parsed, strict),
        "create" => run_create(root, &parsed, strict),
        "activate" => run_activate(root, &parsed, strict),
        "chain-validate" | "chain_validate" | "chain" => run_chain_validate(root, &parsed, strict),
        "install" => run_install(root, &parsed, strict),
        "rollback" => run_rollback(root, &parsed, strict),
        "quarantine" => run_quarantine(root, &parsed, strict),
        "run" => run_skill(root, &parsed, strict),
        "share" => run_share(root, &parsed, strict),
        "gallery" => run_gallery(root, &parsed, strict),
        "load" => {
            let mut alias = parsed.clone();
            alias.flags.insert("op".to_string(), "load".to_string());
            if !alias.flags.contains_key("skill") {
                if let Some(skill) = parsed.positional.get(1) {
                    alias.flags.insert("skill".to_string(), clean(skill, 120));
                }
            }
            run_gallery(root, &alias, strict)
        }
        "react-minimal" | "react_minimal" => run_react_minimal(root, &parsed, strict),
        "tot-deliberate" | "tot_deliberate" | "tot" => run_tot_deliberate(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "skills_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" && !status_dashboard {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn has_claim(receipt: &Value, claim_id: &str) -> bool {
        receipt
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
    }

    fn write_backward_compat_contract(
        root: &Path,
        min_version: &str,
        migration_lane: &str,
        receipt_required: bool,
    ) {
        let contract_path = root.join("planes/contracts/srs/V8-SKILL-002.json");
        if let Some(parent) = contract_path.parent() {
            fs::create_dir_all(parent).expect("mkdir contract parent");
        }
        write_json(
            &contract_path,
            &json!({
                "id": "V8-SKILL-002",
                "backward_compat": {
                    "policy": "semver_major",
                    "min_version": min_version,
                    "migration_lane": migration_lane,
                    "receipt_required": receipt_required
                }
            }),
        )
        .expect("write compat contract");
    }

    fn write_skill_yaml(skill_dir: &Path, skill_name: &str, version: &str) {
        fs::create_dir_all(skill_dir).expect("mkdir skill");
        fs::write(
            skill_dir.join("skill.yaml"),
            format!("name: {skill_name}\nversion: {version}\nentrypoint: scripts/run.sh\n"),
        )
        .expect("write yaml");
    }

    fn run_install_for(root: &Path, skill_dir: &Path, extra_flags: &[&str]) -> Value {
        let mut args = vec![
            "install".to_string(),
            format!("--skill-path={}", skill_dir.display()),
            "--strict=1".to_string(),
        ];
        args.extend(extra_flags.iter().map(|flag| (*flag).to_string()));
        run_install(root, &crate::parse_args(&args), true)
    }

    fn assert_error_contains(payload: &Value, expected: &str) {
        assert!(
            payload
                .get("errors")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .any(|row| row.as_str() == Some(expected)),
            "expected errors to contain: {expected}"
        );
    }

    fn write_installed_registry(root: &Path, skill: &str, version: &str) {
        let registry_path = state_root(root).join("registry.json");
        fs::create_dir_all(registry_path.parent().unwrap_or_else(|| Path::new(".")))
            .expect("mkdir registry");
        let mut installed = serde_json::Map::new();
        installed.insert(
            skill.to_string(),
            json!({
                "path": format!("skills/{skill}"),
                "version": version
            }),
        );
        write_json(
            &registry_path,
            &json!({
                "installed": Value::Object(installed)
            }),
        )
        .expect("write registry");
    }

    #[test]
    fn create_requires_name() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["create".to_string()]);
        let out = run_create(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn create_mints_deterministic_skill_id_and_cognition_claim() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "create".to_string(),
            "--name=Weekly Growth Report".to_string(),
        ]);
        let out_a = run_create(root.path(), &parsed, true);
        let out_b = run_create(root.path(), &parsed, true);
        let id_a = out_a
            .get("skill")
            .and_then(|v| v.get("deterministic_id"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let id_b = out_b
            .get("skill")
            .and_then(|v| v.get("deterministic_id"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!id_a.is_empty());
        assert!(id_a.starts_with("skill_"));
        assert_eq!(id_a, id_b);
        assert!(has_claim(&out_a, "V6-COGNITION-012.2"));
    }

    #[test]
    fn dashboard_includes_cognition_ledger_view_and_claim() {
        let root = tempfile::tempdir().expect("tempdir");
        let cognition_dir = root.path().join("local/state/ops/assimilation_controller");
        fs::create_dir_all(&cognition_dir).expect("mkdir cognition dir");
        fs::write(
            cognition_dir.join("latest.json"),
            r#"{"ok":true,"type":"assimilation_controller_skill_create","skill_id":"skill_abc123"}"#,
        )
        .expect("write cognition latest");
        fs::write(
            cognition_dir.join("history.jsonl"),
            r#"{"ok":true,"type":"assimilation_controller_skill_create"}"#,
        )
        .expect("write cognition history");

        let parsed = crate::parse_args(&["dashboard".to_string()]);
        let out = run_dashboard(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("cognition")
                .and_then(|v| v.get("history_events"))
                .and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            out.get("cognition")
                .and_then(|v| v.get("latest"))
                .and_then(|v| v.get("type"))
                .and_then(Value::as_str),
            Some("assimilation_controller_skill_create")
        );
        assert!(has_claim(&out, "V6-COGNITION-012.5"));
    }

    #[test]
    fn conduit_rejects_bypass_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["run".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "run");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn install_rejects_major_upgrade_without_force_migration_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        let skill_dir = root.path().join("skills").join("compat-skill");
        write_skill_yaml(&skill_dir, "compat-skill", "1.0.0");
        let first_out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(first_out.get("ok").and_then(Value::as_bool), Some(true));

        write_skill_yaml(&skill_dir, "compat-skill", "2.0.0");
        let second_out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(second_out.get("ok").and_then(Value::as_bool), Some(false));
        assert_error_contains(
            &second_out,
            "backward_compat_break_requires_force_migration",
        );
    }

    #[test]
    fn install_rejects_requested_version_below_contract_min_version() {
        let root = tempfile::tempdir().expect("tempdir");
        write_backward_compat_contract(root.path(), "v2", "skill_forced_migration", true);

        let skill_dir = root.path().join("skills").join("compat-min-version");
        write_skill_yaml(&skill_dir, "compat-min-version", "1.0.0");
        let out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_error_contains(&out, "requested_skill_version_below_minimum");
    }

    #[test]
    fn install_forced_migration_emits_v8_skill_002_receipt() {
        let root = tempfile::tempdir().expect("tempdir");
        let skill_dir = root.path().join("skills").join("compat-skill");
        write_skill_yaml(&skill_dir, "compat-skill", "1.0.0");
        let baseline_out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(baseline_out.get("ok").and_then(Value::as_bool), Some(true));

        write_skill_yaml(&skill_dir, "compat-skill", "2.0.0");
        let forced_out = run_install_for(
            root.path(),
            &skill_dir,
            &[
                "--force-migration=1",
                "--migration-reason=major_api_refresh",
            ],
        );
        assert_eq!(forced_out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&forced_out, "V8-SKILL-002"));
        assert_eq!(
            forced_out
                .get("compatibility")
                .and_then(|v| v.get("migration_required"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            forced_out
                .get("compatibility")
                .and_then(|v| v.get("migration_receipt_emitted"))
                .and_then(Value::as_bool),
            Some(true)
        );
        let latest = state_root(root.path())
            .join("migrations")
            .join("latest.json");
        assert!(latest.exists());
    }

    #[test]
    fn install_forced_migration_writes_rollback_checkpoint() {
        let root = tempfile::tempdir().expect("tempdir");
        let skill_dir = root.path().join("skills").join("compat-checkpoint");
        write_skill_yaml(&skill_dir, "compat-checkpoint", "1.0.0");
        assert_eq!(
            run_install_for(root.path(), &skill_dir, &[])
                .get("ok")
                .and_then(Value::as_bool),
            Some(true)
        );
        write_skill_yaml(&skill_dir, "compat-checkpoint", "2.0.0");
        let out = run_install_for(
            root.path(),
            &skill_dir,
            &["--force-migration=1", "--migration-reason=breakfix"],
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.pointer("/compatibility/rollback_checkpoint_written")
                .and_then(Value::as_bool),
            Some(true)
        );
        let checkpoint_path = out
            .pointer("/compatibility/rollback_checkpoint_path")
            .and_then(Value::as_str)
            .expect("checkpoint path");
        assert!(Path::new(checkpoint_path).exists());
    }

    #[test]
    fn rollback_restores_previous_version_from_checkpoint() {
        let root = tempfile::tempdir().expect("tempdir");
        let skill_dir = root.path().join("skills").join("compat-rollback");
        write_skill_yaml(&skill_dir, "compat-rollback", "1.0.0");
        assert_eq!(
            run_install_for(root.path(), &skill_dir, &[])
                .get("ok")
                .and_then(Value::as_bool),
            Some(true)
        );

        write_skill_yaml(&skill_dir, "compat-rollback", "2.0.0");
        assert_eq!(
            run_install_for(
                root.path(),
                &skill_dir,
                &["--force-migration=1", "--migration-reason=major_break"],
            )
            .get("ok")
            .and_then(Value::as_bool),
            Some(true)
        );

        let rollback = crate::parse_args(&[
            "rollback".to_string(),
            "--skill=compat-rollback".to_string(),
            "--strict=1".to_string(),
        ]);
        let rollback_out = run_rollback(root.path(), &rollback, true);
        assert_eq!(rollback_out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&rollback_out, "V8-SKILL-002"));

        let registry =
            read_json(&state_root(root.path()).join("registry.json")).expect("registry readable");
        assert_eq!(
            registry
                .pointer("/installed/compat-rollback/version")
                .and_then(Value::as_str),
            Some("1.0.0")
        );
    }

    #[test]
    fn install_forced_migration_creates_default_migration_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        write_backward_compat_contract(root.path(), "v1", "custom_lane_policy", true);
        let skill_dir = root.path().join("skills").join("compat-lane");
        write_skill_yaml(&skill_dir, "compat-lane", "1.2.0");
        let baseline_out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(baseline_out.get("ok").and_then(Value::as_bool), Some(true));

        write_skill_yaml(&skill_dir, "compat-lane", "2.0.0");
        let forced_out = run_install_for(
            root.path(),
            &skill_dir,
            &[
                "--force-migration=1",
                "--migration-reason=api_contract_rollup",
            ],
        );
        assert_eq!(forced_out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            forced_out
                .pointer("/compatibility/migration_lane_exists")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            forced_out
                .pointer("/compatibility/migration_lane_created")
                .and_then(Value::as_bool),
            Some(true)
        );
        let lane_path = forced_out
            .pointer("/compatibility/migration_lane_path")
            .and_then(Value::as_str)
            .expect("lane path");
        assert!(Path::new(lane_path).exists());
        assert!(
            lane_path.contains("custom-lane-policy"),
            "lane path should include policy lane namespace"
        );
    }

    #[test]
    fn install_enforced_deprecation_policy_requires_ticket() {
        let root = tempfile::tempdir().expect("tempdir");
        let skill_dir = root.path().join("skills").join("compat-policy");
        write_skill_yaml(&skill_dir, "compat-policy", "1.0.0");
        let baseline_out = run_install_for(root.path(), &skill_dir, &[]);
        assert_eq!(baseline_out.get("ok").and_then(Value::as_bool), Some(true));

        write_skill_yaml(&skill_dir, "compat-policy", "2.0.0");
        let no_ticket_out = run_install_for(
            root.path(),
            &skill_dir,
            &[
                "--force-migration=1",
                "--migration-reason=major_break",
                "--deprecation-policy=enforce",
            ],
        );
        assert_eq!(
            no_ticket_out.get("ok").and_then(Value::as_bool),
            Some(false)
        );
        assert_error_contains(
            &no_ticket_out,
            "deprecation_ticket_required_for_enforced_migration",
        );

        let with_ticket_out = run_install_for(
            root.path(),
            &skill_dir,
            &[
                "--force-migration=1",
                "--migration-reason=major_break",
                "--deprecation-policy=enforce",
                "--deprecation-ticket=CHG-2026-0319",
            ],
        );
        assert_eq!(
            with_ticket_out.get("ok").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            with_ticket_out
                .pointer("/compatibility/deprecation_policy")
                .and_then(Value::as_str),
            Some("enforce")
        );
    }

    #[test]
    fn run_strict_requires_backward_compat_gate() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--skill=unknown_skill".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_skill(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row
                .as_str()
                .unwrap_or_default()
                .starts_with("backward_compat_gate_failed:")));
    }

    #[test]
    fn run_strict_allows_installed_skill_with_supported_version() {
        let root = tempfile::tempdir().expect("tempdir");
        write_installed_registry(root.path(), "compat_skill", "1.2.0");
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--skill=compat_skill".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_skill(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&out, "V8-SKILL-002"));
    }

    #[test]
    fn run_non_strict_requires_backward_compat_gate() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["run".to_string(), "--skill=unknown_skill".to_string()]);
        let out = run_skill(root.path(), &parsed, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row
                .as_str()
                .unwrap_or_default()
                .starts_with("backward_compat_gate_failed:")));
    }

    #[test]
    fn run_non_strict_allows_installed_skill_with_supported_version() {
        let root = tempfile::tempdir().expect("tempdir");
        write_installed_registry(root.path(), "compat_skill", "1.2.0");
        let parsed = crate::parse_args(&["run".to_string(), "--skill=compat_skill".to_string()]);
        let out = run_skill(root.path(), &parsed, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&out, "V8-SKILL-002"));
    }

    #[test]
    fn quarantine_blocks_run_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &quarantine_path(root.path()),
            &json!({
                "compat_skill": {
                    "reason": "incident_triage",
                    "ts": crate::now_iso()
                }
            }),
        )
        .expect("write quarantine");
        let parsed = crate::parse_args(&[
            "run".to_string(),
            "--skill=compat_skill".to_string(),
            "--strict=1".to_string(),
        ]);
        let out = run_skill(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(has_claim(&out, "V8-SKILL-007"));
        assert!(has_claim(&out, "V8-SKILL-009"));
    }

    #[test]
    fn chain_validate_rejects_version_mismatch_and_missing_smoke_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        let skills_dir = root.path().join("client/runtime/systems/skills/packages");
        let skill_dir = skills_dir.join("chain-skill");
        fs::create_dir_all(&skill_dir).expect("mkdir chain skill");
        fs::write(
            skill_dir.join("skill.yaml"),
            "name: chain-skill\nversion: 1.0.0\nentrypoint: scripts/run.sh\n",
        )
        .expect("write yaml");

        let chain_doc = root.path().join("chain.json");
        write_json(
            &chain_doc,
            &json!({
                "version": "v2",
                "skills": [
                    {"id": "chain-skill", "version": "1.0.0"}
                ]
            }),
        )
        .expect("write chain doc");

        let parsed = crate::parse_args(&[
            "chain-validate".to_string(),
            format!("--chain-path={}", chain_doc.display()),
            "--strict=1".to_string(),
        ]);
        let out = run_chain_validate(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        let errors = out
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(|value| value.to_string()))
            .collect::<Vec<_>>();
        assert!(
            errors.iter().any(|row| row == "chain_version_invalid"),
            "missing chain_version_invalid: {errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|row| row == "chain_skill_smoke_missing:chain-skill"),
            "missing chain_skill_smoke_missing: {errors:?}"
        );
    }

    #[test]
    fn gallery_load_non_strict_allows_missing_signing_key_with_signature() {
        let root = tempfile::tempdir().expect("tempdir");
        let package_dir = root.path().join("skills").join("gallery-demo");
        fs::create_dir_all(&package_dir).expect("mkdir package");
        fs::write(
            package_dir.join("skill.yaml"),
            "name: gallery-demo\nversion: 1.0.0\nentrypoint: scripts/run.sh\n",
        )
        .expect("write yaml");

        let manifest_path = root.path().join("gallery_manifest.json");
        write_json(
            &manifest_path,
            &json!({
                "version": "v1",
                "kind": "skill_gallery_manifest",
                "templates": [
                    {
                        "id": "gallery-demo",
                        "version": "v1",
                        "human_reviewed": true,
                        "package_rel": package_dir.display().to_string()
                    }
                ],
                "signature": "sig:placeholder"
            }),
        )
        .expect("write manifest");

        std::env::remove_var("SKILLS_GALLERY_SIGNING_KEY");
        let parsed = crate::parse_args(&[
            "gallery".to_string(),
            "ingest".to_string(),
            format!("--manifest={}", manifest_path.display()),
            "--strict=0".to_string(),
        ]);
        let out = run_gallery(root.path(), &parsed, false);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("op").and_then(Value::as_str), Some("ingest"));
        assert!(has_claim(&out, "V6-SKILLS-001.6"));
    }
}
