#[test]
fn v8_skill_002_enforces_backward_compatibility_and_forced_migration_receipts() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let skills_root = root
        .join("client")
        .join("runtime")
        .join("systems")
        .join("skills")
        .join("packages");
    fs::create_dir_all(&skills_root).expect("mkdir skills root");

    let create_exit = skills_plane::run(
        root,
        &[
            "create".to_string(),
            "--strict=1".to_string(),
            "--name=compat-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);

    let skill_dir = skills_root.join("compat-skill");
    let skill_yaml = skill_dir.join("skill.yaml");

    // Initial install at v1 establishes previous_version for compat checks.
    let install_v1_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v1_exit, 0);

    // Upgrade to v2 without force migration must fail-closed.
    let yaml_v2 = fs::read_to_string(&skill_yaml)
        .expect("read skill yaml")
        .replace("version: v1", "version: v2");
    fs::write(&skill_yaml, yaml_v2).expect("write upgraded yaml");

    let install_v2_without_force = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v2_without_force, 1);
    let blocked_latest = read_json(&latest_path(root));
    assert_eq!(
        blocked_latest.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert!(blocked_latest
        .get("errors")
        .and_then(Value::as_array)
        .map(|rows| rows
            .iter()
            .any(|row| row.as_str() == Some("backward_compat_break_requires_force_migration")))
        .unwrap_or(false));
    assert_eq!(
        blocked_latest
            .get("compatibility")
            .and_then(|v| v.get("migration_required"))
            .and_then(Value::as_bool),
        Some(true)
    );

    // Forced migration with explicit reason and ticket must pass and emit receipts.
    let install_v2_forced = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            "--force-migration=1".to_string(),
            "--deprecation-policy=enforce".to_string(),
            "--deprecation-ticket=SEC-123".to_string(),
            "--migration-reason=major_api_upgrade".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v2_forced, 0);
    let forced_latest = read_json(&latest_path(root));
    assert_eq!(forced_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_claim(&forced_latest, "V8-SKILL-002");
    assert_eq!(
        forced_latest
            .get("compatibility")
            .and_then(|v| v.get("migration_required"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        forced_latest
            .get("compatibility")
            .and_then(|v| v.get("forced_migration"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        forced_latest
            .get("compatibility")
            .and_then(|v| v.get("migration_receipt_emitted"))
            .and_then(Value::as_bool),
        Some(true)
    );

    let migration_latest_path = forced_latest
        .get("compatibility")
        .and_then(|v| v.get("migration_latest_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .expect("migration_latest_path");
    let migration_latest = read_json(&migration_latest_path);
    assert_eq!(
        migration_latest.get("type").and_then(Value::as_str),
        Some("skills_plane_migration_receipt")
    );
    assert_eq!(
        migration_latest.get("skill_id").and_then(Value::as_str),
        Some("compat-skill")
    );
}

#[test]
fn v8_skill_002_run_gate_fails_closed_on_registry_version_drift() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let skills_root = root
        .join("client")
        .join("runtime")
        .join("systems")
        .join("skills")
        .join("packages");
    fs::create_dir_all(&skills_root).expect("mkdir skills root");

    let create_exit = skills_plane::run(
        root,
        &[
            "create".to_string(),
            "--strict=1".to_string(),
            "--name=run-gate-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);

    let skill_dir = skills_root.join("run-gate-skill");
    let install_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_exit, 0);

    let registry_path = root
        .join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("skills_plane")
        .join("registry.json");
    let mut registry = read_json(&registry_path);
    registry["installed"]["run-gate-skill"]["version"] = Value::String("v0".to_string());
    fs::write(
        &registry_path,
        serde_json::to_string_pretty(&registry).expect("encode registry"),
    )
    .expect("write registry");

    let run_exit = skills_plane::run(
        root,
        &[
            "run".to_string(),
            "--strict=1".to_string(),
            "--skill=run-gate-skill".to_string(),
            "--input=smoke".to_string(),
        ],
    );
    assert_eq!(run_exit, 1, "strict run should fail on compat gate drift");
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("skills_plane_run")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert!(
        latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|row| row
                .as_str()
                .unwrap_or_default()
                .starts_with("backward_compat_gate_failed:")))
            .unwrap_or(false),
        "run should fail with backward compatibility gate error"
    );
    assert_claim(&latest, "V8-SKILL-002");
}

#[test]
fn v8_skill_002_rejects_downgrade_without_allow_flag_and_accepts_when_allowed() {
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let skills_root = root
        .join("client")
        .join("runtime")
        .join("systems")
        .join("skills")
        .join("packages");
    fs::create_dir_all(&skills_root).expect("mkdir skills root");

    let create_exit = skills_plane::run(
        root,
        &[
            "create".to_string(),
            "--strict=1".to_string(),
            "--name=downgrade-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);

    let skill_dir = skills_root.join("downgrade-skill");
    let skill_yaml = skill_dir.join("skill.yaml");
    let yaml_v2 = fs::read_to_string(&skill_yaml)
        .expect("read skill yaml")
        .replace("version: v1", "version: v2");
    fs::write(&skill_yaml, yaml_v2).expect("write v2 yaml");

    let install_v2_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v2_exit, 0);

    let yaml_v1 = fs::read_to_string(&skill_yaml)
        .expect("read skill yaml")
        .replace("version: v2", "version: v1");
    fs::write(&skill_yaml, yaml_v1).expect("write v1 yaml");

    let blocked_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(
        blocked_exit, 1,
        "strict downgrade should fail without --allow-downgrade"
    );
    let blocked_latest = read_json(&latest_path(root));
    assert_eq!(
        blocked_latest.get("ok").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        blocked_latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("version_downgrade_requires_allow_downgrade")))
            .unwrap_or(false),
        "expected explicit downgrade gate error"
    );

    let allow_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            "--allow-downgrade=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(allow_exit, 0);
    let allow_latest = read_json(&latest_path(root));
    assert_eq!(allow_latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(
        allow_latest
            .pointer("/compatibility/downgrade_detected")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        allow_latest
            .pointer("/compatibility/allow_downgrade")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_claim(&allow_latest, "V8-SKILL-002");
}

