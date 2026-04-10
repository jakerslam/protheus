fn run_security(root: &Path, args: &[&str]) -> i32 {
    let argv = args.iter().map(|row| row.to_string()).collect::<Vec<_>>();
    security_plane::run(root, &argv)
}

fn assert_latest_type_claim(root: &Path, expected_type: &str, claim: &str) -> Value {
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some(expected_type)
    );
    assert_claim(&latest, claim);
    latest
}

#[test]
fn v6_sec_connected_skill_and_hygiene_guards_fail_closed() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let invalid_path = "../../etc/passwd";
    assert_eq!(
        run_security(
            root,
            &[
                "skill-install-path-enforcer",
                &format!("--skill-path={invalid_path}"),
                "--strict=1",
            ],
        ),
        2
    );
    let invalid_latest = assert_latest_type_claim(
        root,
        "security_plane_skill_install_path_enforcer",
        "V6-SEC-SKILL-PATH-001",
    );
    assert_eq!(
        invalid_latest.get("allowed").and_then(Value::as_bool),
        Some(false)
    );

    assert_eq!(
        run_security(
            root,
            &[
                "skill-install-path-enforcer",
                "--skill-path=client/runtime/systems/skills/packages/demo",
                "--strict=1",
            ],
        ),
        0
    );

    assert_eq!(
        run_security(
            root,
            &[
                "skill-quarantine",
                "quarantine",
                "--skill-id=demo-skill",
                "--reason=suspicious-network",
                "--strict=1",
            ],
        ),
        0
    );
    let quarantine_latest = assert_latest_type_claim(
        root,
        "security_plane_skill_quarantine",
        "V6-SEC-SKILL-QUARANTINE-001",
    );
    assert_eq!(
        quarantine_latest
            .get("quarantined_count")
            .and_then(Value::as_u64),
        Some(1)
    );

    assert_eq!(
        run_security(
            root,
            &[
                "skill-quarantine",
                "release",
                "--skill-id=demo-skill",
                "--strict=1",
            ],
        ),
        0
    );

    write_file(
        &root
            .join("core")
            .join("local")
            .join("state")
            .join("ops")
            .join("skills_plane")
            .join("registry.json"),
        r#"{"installed":{"demo-a":{},"demo-b":{},"demo-c":{}}}"#,
    );
    assert_eq!(
        run_security(
            root,
            &[
                "autonomous-skill-necessity-audit",
                "--required-skills=demo-a",
                "--max-installed=1",
                "--strict=1",
            ],
        ),
        2
    );
    let audit_latest = assert_latest_type_claim(
        root,
        "security_plane_autonomous_skill_necessity_audit",
        "V6-SEC-SKILL-AUDIT-001",
    );
    assert_eq!(
        audit_latest.get("overloaded").and_then(Value::as_bool),
        Some(true)
    );
}

#[test]
fn v6_sec_remediate_fails_closed_when_scan_is_missing_in_strict_mode() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let exit = run_security(root, &["remediate", "--strict=1"]);
    assert_eq!(exit, 2, "strict remediation must fail without a prior scan");

    let latest = assert_latest_type_claim(root, "security_plane_auto_remediation", "V6-SEC-011");
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("scan_missing")
    );
}

#[test]
fn v6_sec_skill_quarantine_requires_skill_id_in_strict_mode() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let exit = run_security(root, &["skill-quarantine", "quarantine", "--strict=1"]);
    assert_eq!(exit, 2, "strict quarantine must require --skill-id");

    let latest = assert_latest_type_claim(
        root,
        "security_plane_skill_quarantine",
        "V6-SEC-SKILL-QUARANTINE-001",
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
    assert_eq!(
        latest.get("error").and_then(Value::as_str),
        Some("skill_id_required")
    );
}

#[test]
fn v6_sec_connected_runtime_guards_detect_risk_markers() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let scan_root = root.join("scan");
    write_file(
        &scan_root.join("conflict.rs"),
        "<<<<<<< HEAD\nlet x = 1;\n=======\nlet x = 2;\n>>>>>>> main\n",
    );
    assert_eq!(
        security_plane::run(
            root,
            &[
                "repo-hygiene-guard".to_string(),
                format!("--scan-root={}", scan_root.display()),
                "--strict=1".to_string(),
            ],
        ),
        2
    );
    let hygiene_latest = read_json(&latest_path(root));
    assert_eq!(
        hygiene_latest.get("type").and_then(Value::as_str),
        Some("security_plane_repo_hygiene_guard")
    );
    assert!(
        hygiene_latest
            .get("hit_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
    assert_claim(&hygiene_latest, "V6-SEC-REPO-HYGIENE-001");

    assert_eq!(
        security_plane::run(
            root,
            &[
                "log-redaction-guard".to_string(),
                "--text=token sk-123456".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        2
    );
    let redaction_latest = read_json(&latest_path(root));
    assert_eq!(
        redaction_latest.get("type").and_then(Value::as_str),
        Some("security_plane_log_redaction_guard")
    );
    assert_claim(&redaction_latest, "V6-SEC-LOG-REDACTION-001");

    let secret_path = root.join("secrets").join(".env");
    write_file(&secret_path, "TOKEN=abcd");
    assert_eq!(
        security_plane::run(
            root,
            &[
                "workspace-dump-guard".to_string(),
                "--path=secrets/.env".to_string(),
                "--max-bytes=100000".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        2
    );
    let dump_latest = read_json(&latest_path(root));
    assert_eq!(
        dump_latest.get("type").and_then(Value::as_str),
        Some("security_plane_workspace_dump_guard")
    );
    assert_eq!(
        dump_latest.get("blocked").and_then(Value::as_bool),
        Some(true)
    );
    assert_claim(&dump_latest, "V6-SEC-WORKSPACE-DUMP-001");

    assert_eq!(
        security_plane::run(
            root,
            &[
                "llm-gateway-guard".to_string(),
                "--provider=openai".to_string(),
                "--model=gpt-5.4".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
    assert_eq!(
        security_plane::run(
            root,
            &[
                "llm-gateway-guard".to_string(),
                "--provider=unknown".to_string(),
                "--model=rogue-model".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        2
    );
    let gateway_latest = read_json(&latest_path(root));
    assert_eq!(
        gateway_latest.get("type").and_then(Value::as_str),
        Some("security_plane_llm_gateway_guard")
    );
    assert_claim(&gateway_latest, "V6-SEC-LLM-GATEWAY-001");
}

#[test]
fn v6_sec_rsi_self_mod_gate_requires_approval_for_sensitive_paths() {
    let _guard = env_guard();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let init = Command::new("git")
        .arg("init")
        .arg(root)
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init should succeed");

    write_file(
        &root
            .join("core")
            .join("layer0")
            .join("ops")
            .join("src")
            .join("placeholder.rs"),
        "pub fn placeholder() {}\n",
    );
    let add = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("add")
        .arg("core/layer0/ops/src/placeholder.rs")
        .output()
        .expect("git add");
    assert!(add.status.success(), "git add should succeed");
    assert_eq!(
        security_plane::run(
            root,
            &[
                "rsi-git-patch-self-mod-gate".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        2
    );
    let blocked = read_json(&latest_path(root));
    assert_eq!(
        blocked.get("type").and_then(Value::as_str),
        Some("security_plane_rsi_git_patch_self_mod_gate")
    );
    assert!(
        blocked
            .get("sensitive_change_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
    assert_claim(&blocked, "V6-SEC-RSI-SELFMOD-001");

    assert_eq!(
        security_plane::run(
            root,
            &[
                "rsi-git-patch-self-mod-gate".to_string(),
                "--self-mod-approved=1".to_string(),
                "--strict=1".to_string(),
            ],
        ),
        0
    );
}
