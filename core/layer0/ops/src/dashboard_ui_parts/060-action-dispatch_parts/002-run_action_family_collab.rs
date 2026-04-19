fn run_action_family_collab(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "collab.launchRole" => {
            let team = payload
                .get("team")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| DEFAULT_TEAM.to_string());
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 60))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "analyst".to_string());
            let shadow = payload
                .get("shadow")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| format!("{team}-{role}-shadow"));
            run_lane(
                root,
                "collab-plane",
                &[
                    "launch-role".to_string(),
                    format!("--team={team}"),
                    format!("--role={role}"),
                    format!("--shadow={shadow}"),
                ],
            )
        }
        _ => run_action_family_skills(root, normalized, payload),
    }
}
