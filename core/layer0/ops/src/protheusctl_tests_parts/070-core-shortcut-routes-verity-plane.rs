
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
