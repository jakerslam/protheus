#[test]
fn v6_flow_batch10_rejects_bypass_when_strict() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = flow_plane::run(
        root,
        &[
            "compile".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
            "--canvas-json={\"version\":\"v1\",\"kind\":\"flow_canvas_graph\",\"nodes\":[{\"id\":\"a\",\"type\":\"source\"}],\"edges\":[]}"
                .to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_conduit_gate")
    );
    assert!(latest
        .get("conduit_enforcement")
        .and_then(|v| v.get("claim_evidence"))
        .and_then(Value::as_array)
        .map(|rows| rows
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some("V6-FLOW-001.6")))
        .unwrap_or(false));
}

#[test]
fn v6_flow_batch10_default_component_manifest_passes_in_strict_mode() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    std::env::set_var(
        "FLOW_COMPONENT_SIGNING_KEY",
        "flow-component-default-signing-key",
    );
    let exit = flow_plane::run(root, &["components".to_string(), "--strict=1".to_string()]);
    assert_eq!(exit, 0);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_component_marketplace")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        latest
            .get("result")
            .and_then(|v| v.get("validated_components"))
            .and_then(Value::as_array)
            .map(|rows| rows.len() >= 1)
            .unwrap_or(false),
        "default flow component marketplace manifest should validate at least one signed component in strict mode"
    );
}

#[test]
fn v6_flow_batch10_default_template_manifest_passes_in_strict_mode() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    std::env::set_var(
        "FLOW_TEMPLATE_SIGNING_KEY",
        "flow-template-default-signing-key",
    );
    let exit = flow_plane::run(root, &["templates".to_string(), "--strict=1".to_string()]);
    assert_eq!(exit, 0);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_template_governance")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        latest
            .get("result")
            .and_then(|v| v.get("validated_templates"))
            .and_then(Value::as_array)
            .map(|rows| rows.len() >= 1)
            .unwrap_or(false),
        "default flow template governance manifest should validate at least one signed template in strict mode"
    );
}

#[test]
fn v6_flow_batch10_run_alias_executes_through_playground() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = flow_plane::run(
        root,
        &[
            "run".to_string(),
            "--strict=1".to_string(),
            "--run-id=batch29-flow".to_string(),
        ],
    );
    assert_eq!(exit, 0);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_playground")
    );
    assert_eq!(latest.get("op").and_then(Value::as_str), Some("play"));
    assert_claim(&latest, "V6-FLOW-001.6");
}

#[test]
fn v6_flow_batch10_install_alias_executes_template_governance() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    std::env::set_var(
        "FLOW_TEMPLATE_SIGNING_KEY",
        "flow-template-default-signing-key",
    );
    let exit = flow_plane::run(root, &["install".to_string(), "--strict=1".to_string()]);
    assert_eq!(exit, 0);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_template_governance")
    );
    assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(true));
    assert_claim(&latest, "V6-FLOW-001.6");
}

#[test]
fn v6_flow_batch10_rejects_bypass_for_run_alias_when_strict() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = flow_plane::run(
        root,
        &[
            "run".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
            "--run-id=batch29-flow".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_conduit_gate")
    );
}

#[test]
fn v6_flow_batch10_rejects_bypass_for_install_alias_when_strict() {
    let _guard = flow_test_lock();
    let fixture = stage_fixture_root();
    let root = fixture.path();
    let exit = flow_plane::run(
        root,
        &[
            "install".to_string(),
            "--strict=1".to_string(),
            "--bypass=1".to_string(),
        ],
    );
    assert_eq!(exit, 1);
    let latest = read_json(&latest_path(root));
    assert_eq!(
        latest.get("type").and_then(Value::as_str),
        Some("flow_plane_conduit_gate")
    );
}
