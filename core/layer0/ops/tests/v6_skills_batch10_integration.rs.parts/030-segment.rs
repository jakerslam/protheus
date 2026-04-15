#[test]
fn v8_skill_002_forced_migration_requires_reason_in_strict_mode() {
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
            "--name=migration-reason-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);

    let skill_dir = skills_root.join("migration-reason-skill");
    let install_v1_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v1_exit, 0);

    let skill_yaml = skill_dir.join("skill.yaml");
    let yaml_v2 = fs::read_to_string(&skill_yaml)
        .expect("read skill yaml")
        .replace("version: v1", "version: v2");
    fs::write(&skill_yaml, yaml_v2).expect("write v2 yaml");

    let blocked_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            "--force-migration=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(blocked_exit, 1);
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
                .any(|row| row.as_str() == Some("migration_reason_required_for_forced_migration")))
            .unwrap_or(false),
        "expected missing migration reason to fail closed in strict mode"
    );
}

#[test]
fn v8_skill_002_enforced_deprecation_requires_ticket_in_strict_mode() {
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
            "--name=deprecation-ticket-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);

    let skill_dir = skills_root.join("deprecation-ticket-skill");
    let install_v1_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_v1_exit, 0);

    let skill_yaml = skill_dir.join("skill.yaml");
    let yaml_v2 = fs::read_to_string(&skill_yaml)
        .expect("read skill yaml")
        .replace("version: v1", "version: v2");
    fs::write(&skill_yaml, yaml_v2).expect("write v2 yaml");

    let blocked_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            "--force-migration=1".to_string(),
            "--deprecation-policy=enforce".to_string(),
            "--migration-reason=breaking_contract_change".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(blocked_exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert!(
        latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str()
                    == Some("deprecation_ticket_required_for_enforced_migration")))
            .unwrap_or(false),
        "expected deprecation ticket enforcement error in strict mode"
    );
}

#[test]
fn v8_skill_002_chain_validate_strict_rejects_version_and_missing_smoke() {
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
            "--name=chain-skill".to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(create_exit, 0);
    let skill_dir = skills_root.join("chain-skill");

    // Remove smoke script to trigger strict chain skill smoke requirement.
    fs::remove_file(skill_dir.join("tests").join("smoke.sh")).expect("remove smoke script");

    let install_exit = skills_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            format!("--skill-path={}", skill_dir.display()),
        ],
    );
    assert_eq!(install_exit, 0);

    let chain_exit = skills_plane::run(
        root,
        &[
            "chain-validate".to_string(),
            "--strict=1".to_string(),
            "--chain-json={\"version\":\"v2\",\"skills\":[{\"id\":\"chain-skill\",\"version\":\"v1\"}]}"
                .to_string(),
            format!("--skills-root={}", skills_root.display()),
        ],
    );
    assert_eq!(chain_exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("skills_plane_chain_validate")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert!(
        latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("chain_version_invalid")))
            .unwrap_or(false),
        "expected chain version mismatch failure"
    );
    assert!(
        latest
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("chain_skill_smoke_missing:chain-skill")))
            .unwrap_or(false),
        "expected missing smoke script strict failure"
    );
}
