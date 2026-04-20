
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, argv);
    let result = match command.as_str() {
        "evaluate" => evaluate_command(root, argv, &policy),
        "monitor" => monitor_command(argv, &policy),
        "commit" => commit_command(argv, &policy),
        "rollback" => rollback_command(argv, &policy),
        "status" => status_receipt(&policy),
        _ => Err("unknown_command".to_string()),
    };

    match result {
        Ok(receipt) => {
            print_json_line(&receipt);
            if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(error) => {
            print_json_line(&cli_error(&command, &error));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_root(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "protheus_autophagy_auto_approval_{name}_{}",
            now_epoch_ms()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join("client/runtime/config")).expect("config dir");
        path
    }

    fn write_policy(root: &Path) {
        let path = root.join("client/runtime/config/autophagy_auto_approval_policy.json");
        let policy = json!({
            "enabled": true,
            "auto_approval": {
                "enabled": true,
                "min_confidence": 0.85,
                "min_historical_success_rate": 0.90,
                "max_impact_score": 50,
                "excluded_types": ["safety_critical", "budget_hold"],
                "auto_rollback_on_degradation": true,
                "rollback_window_minutes": 1,
                "regret_issue_label": "auto_approval_regret",
                "degradation_threshold": {
                    "max_drift_delta": 0.01,
                    "max_yield_drop": 0.05
                }
            }
        });
        write_json(&path, &policy).expect("write policy");
    }

    #[test]
    fn evaluate_apply_creates_pending_commit_record() {
        let root = temp_root("evaluate");
        write_policy(&root);
        let args = vec![
            "evaluate".to_string(),
            "--apply=1".to_string(),
            "--proposal-json={\"id\":\"p1\",\"title\":\"Fix drift\",\"type\":\"ops_remediation\",\"confidence\":0.91,\"historical_success_rate\":0.94,\"impact_score\":18}".to_string(),
        ];
        assert_eq!(run(&root, &args), 0);
        let state = load_state(&root.join(DEFAULT_STATE_PATH));
        assert_eq!(
            state["pending_commit"].as_array().map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn excluded_type_requires_human_review() {
        let root = temp_root("excluded");
        write_policy(&root);
        let args = vec![
            "evaluate".to_string(),
            "--proposal-json={\"id\":\"p2\",\"title\":\"Touch safety\",\"type\":\"safety_critical\",\"confidence\":0.99,\"historical_success_rate\":0.99,\"impact_score\":1}".to_string(),
        ];
        let exit = run(&root, &args);
        assert_eq!(exit, 0);
        let latest = read_json(&root.join(DEFAULT_LATEST_PATH)).expect("latest");
        assert_eq!(
            latest.get("decision").and_then(Value::as_str),
            Some("human_review_required")
        );
    }

    #[test]
    fn monitor_can_auto_rollback_on_degradation() {
        let root = temp_root("monitor");
        write_policy(&root);
        let eval_args = vec![
            "evaluate".to_string(),
            "--apply=1".to_string(),
            "--proposal-json={\"id\":\"p3\",\"title\":\"Optimize batch\",\"type\":\"ops_remediation\",\"confidence\":0.92,\"historical_success_rate\":0.95,\"impact_score\":14}".to_string(),
        ];
        assert_eq!(run(&root, &eval_args), 0);
        let monitor_args = vec![
            "monitor".to_string(),
            "--proposal-id=p3".to_string(),
            "--drift=0.02".to_string(),
            "--yield-drop=0.00".to_string(),
            "--apply=1".to_string(),
        ];
        assert_eq!(run(&root, &monitor_args), 0);
        let state = load_state(&root.join(DEFAULT_STATE_PATH));
        assert_eq!(
            state["pending_commit"].as_array().map(|rows| rows.len()),
            Some(0)
        );
        assert_eq!(
            state["rolled_back"].as_array().map(|rows| rows.len()),
            Some(1)
        );
    }

    #[test]
    fn commit_moves_pending_into_committed() {
        let root = temp_root("commit");
        write_policy(&root);
        let eval_args = vec![
            "evaluate".to_string(),
            "--apply=1".to_string(),
            "--proposal-json={\"id\":\"p4\",\"title\":\"Refresh docs\",\"type\":\"documentation\",\"confidence\":0.95,\"historical_success_rate\":0.97,\"impact_score\":7}".to_string(),
        ];
        assert_eq!(run(&root, &eval_args), 0);
        std::thread::sleep(Duration::from_millis(5));
        let commit_args = vec![
            "commit".to_string(),
            "--proposal-id=p4".to_string(),
            "--reason=operator_confirmed".to_string(),
        ];
        assert_eq!(run(&root, &commit_args), 0);
        let state = load_state(&root.join(DEFAULT_STATE_PATH));
        assert_eq!(
            state["pending_commit"].as_array().map(|rows| rows.len()),
            Some(0)
        );
        assert_eq!(
            state["committed"].as_array().map(|rows| rows.len()),
            Some(1)
        );
    }
}
