
#[test]
fn core_shortcut_routes_verity_default_to_status_on_verity_plane() {
    let route = resolve_core_shortcuts("verity", &[]).expect("route");
    assert_eq!(route.script_rel, "core://verity-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_verity_drift_to_verity_plane_drift_status() {
    let route = resolve_core_shortcuts("verity", &["drift".to_string(), "--limit=5".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://verity-plane");
    assert_eq!(route.args, vec!["drift-status", "--limit=5"]);
}

#[test]
fn core_shortcut_routes_top_level_dream_to_autonomy_controller() {
    let route = resolve_core_shortcuts("dream", &["--hand-id=agent-1".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["dream", "--hand-id=agent-1"]);
}

#[test]
fn core_shortcut_routes_top_level_compact_to_autonomy_controller() {
    let route = resolve_core_shortcuts("compact", &["reactive".to_string(), "--strict=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["compact", "reactive", "--strict=1"]);
}

#[test]
fn core_shortcut_routes_top_level_proactive_daemon_to_autonomy_controller() {
    let route = resolve_core_shortcuts("proactive_daemon", &["cycle".to_string(), "--auto=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["proactive_daemon", "cycle", "--auto=1"]);
}

#[test]
fn core_shortcut_routes_top_level_speculate_to_autonomy_controller() {
    let route = resolve_core_shortcuts("speculate", &["status".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["speculate", "status"]);
}

#[test]
fn core_shortcut_routes_dashboard_ui_serve_to_daemon_control_start_with_flag_normalization() {
    let route = resolve_core_shortcuts(
        "dashboard-ui",
        &[
            "serve".to_string(),
            "--host=0.0.0.0".to_string(),
            "--port=4310".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(
        route.args,
        vec![
            "start",
            "--dashboard-host=0.0.0.0",
            "--dashboard-port=4310",
            "--dashboard-open=0"
        ]
    );
}

#[test]
fn core_shortcut_routes_dashboard_alias_to_daemon_control_start() {
    let route = resolve_core_shortcuts("dashboard", &[]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--dashboard-open=1"]);
}

#[test]
fn core_shortcut_routes_doctor_to_install_doctor_domain() {
    let route = resolve_core_shortcuts("doctor", &[]).expect("route");
    assert_eq!(route.script_rel, "core://install-doctor");
    assert_eq!(route.args, vec!["doctor"]);
}

#[test]
fn core_shortcut_routes_verify_install_to_install_doctor_domain() {
    let route = resolve_core_shortcuts("verify-install", &["--json=1".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://install-doctor");
    assert_eq!(route.args, vec!["verify-install", "--json=1"]);
}
