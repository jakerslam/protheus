fn run_action_family_skills(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "skills.run" => {
            let skill = payload
                .get("skill")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 80))
                .unwrap_or_default();
            if skill.is_empty() {
                return LaneResult {
                    ok: false,
                    status: 2,
                    argv: vec!["skills-plane".to_string(), "run".to_string()],
                    payload: Some(json!({
                        "ok": false,
                        "type": "infring_dashboard_action_error",
                        "error": "skill_required"
                    })),
                };
            }
            let input = payload
                .get("input")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 600))
                .unwrap_or_default();
            let mut args = vec!["run".to_string(), format!("--skill={skill}")];
            if !input.is_empty() {
                args.push(format!("--input={input}"));
            }
            run_lane(root, "skills-plane", &args)
        }
        _ => run_action_family_dashboard_core(root, normalized, payload),
    }
}
