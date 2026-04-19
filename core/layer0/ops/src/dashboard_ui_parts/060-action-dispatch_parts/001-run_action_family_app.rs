fn run_action_family_app(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "app.switchProvider" => run_action_family_app_arm_001(root, normalized, payload),
        "app.chat" => run_action_family_app_arm_002(root, normalized, payload),
        _ => run_action_family_collab(root, normalized, payload),
    }
}

include!("001-run_action_family_app_arm_parts/001-run_action_family_app_arm_001.rs");
include!("001-run_action_family_app_arm_parts/002-run_action_family_app_arm_002.rs");
