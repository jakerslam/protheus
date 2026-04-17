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
