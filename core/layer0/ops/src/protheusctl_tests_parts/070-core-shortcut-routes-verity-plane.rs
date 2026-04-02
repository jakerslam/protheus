
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
fn core_shortcut_routes_top_level_kairos_to_autonomy_controller() {
    let route = resolve_core_shortcuts("kairos", &["cycle".to_string(), "--auto=1".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["kairos", "cycle", "--auto=1"]);
}

#[test]
fn core_shortcut_routes_top_level_speculate_to_autonomy_controller() {
    let route = resolve_core_shortcuts("speculate", &["status".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://autonomy-controller");
    assert_eq!(route.args, vec!["speculate", "status"]);
}
