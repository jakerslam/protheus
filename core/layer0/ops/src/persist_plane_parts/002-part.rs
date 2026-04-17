            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(schedules_path(root.path()).exists());
    }

    #[test]
    fn continuity_checkpoint_and_reconstruct_roundtrip() {
        let root = tempfile::tempdir().expect("tempdir");
        let checkpoint = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=checkpoint".to_string(),
                "--session-id=s1".to_string(),
                "--context-json={\"context\":[\"a\"],\"user_model\":{\"style\":\"direct\"},\"active_tasks\":[\"t\"]}".to_string(),
            ]),
            true,
        );
        assert_eq!(checkpoint.get("ok").and_then(Value::as_bool), Some(true));
        let reconstruct = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=reconstruct".to_string(),
                "--session-id=s1".to_string(),
            ]),
            true,
        );
        assert_eq!(reconstruct.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn continuity_validate_passes_with_matching_hashes() {
        let root = tempfile::tempdir().expect("tempdir");
        let checkpoint = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=checkpoint".to_string(),
                "--session-id=s-validate".to_string(),
                "--context-json={\"context\":[\"a\"],\"user_model\":{\"style\":\"direct\"},\"active_tasks\":[\"t\"]}".to_string(),
            ]),
            true,
        );
        assert_eq!(checkpoint.get("ok").and_then(Value::as_bool), Some(true));
        let reconstruct = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=reconstruct".to_string(),
                "--session-id=s-validate".to_string(),
            ]),
            true,
        );
        assert_eq!(reconstruct.get("ok").and_then(Value::as_bool), Some(true));
        let validate = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=validate".to_string(),
                "--session-id=s-validate".to_string(),
            ]),
            true,
        );
        assert_eq!(validate.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn continuity_validate_fails_when_snapshot_hash_is_tampered() {
        let root = tempfile::tempdir().expect("tempdir");
        let checkpoint = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=checkpoint".to_string(),
                "--session-id=s-tamper".to_string(),
                "--context-json={\"context\":[\"a\"],\"user_model\":{\"style\":\"direct\"},\"active_tasks\":[\"t\"]}".to_string(),
            ]),
            true,
        );
        assert_eq!(checkpoint.get("ok").and_then(Value::as_bool), Some(true));

        let path = continuity_snapshot_path(root.path(), "s-tamper");
        let mut snapshot = read_json(&path).expect("snapshot");
        snapshot["context_payload"] = json!({
            "context": ["tampered"],
            "user_model": {"style": "direct"},
            "active_tasks": ["t"]
        });
        write_json(&path, &snapshot).expect("write tampered snapshot");

        let validate = run_continuity(
            root.path(),
            &crate::parse_args(&[
                "continuity".to_string(),
                "--op=validate".to_string(),
                "--session-id=s-tamper".to_string(),
            ]),
            true,
        );
        assert_eq!(validate.get("ok").and_then(Value::as_bool), Some(false));
        let errors = validate
            .get("errors")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(str::to_string))
            .collect::<Vec<String>>();
        assert!(errors
            .iter()
            .any(|row| row.contains("snapshot_context_hash_match")));
    }

    #[test]
    fn connector_add_creates_registry_row() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_connector(
            root.path(),
            &crate::parse_args(&[
                "connector".to_string(),
                "--op=add".to_string(),
                "--provider=slack".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(connectors_path(root.path()).exists());
    }

    #[test]
    fn cowork_delegate_creates_parent_child_chain() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = run_cowork(
            root.path(),
            &crate::parse_args(&[
                "cowork".to_string(),
                "--op=delegate".to_string(),
                "--task=ship batch".to_string(),
                "--parent=lead".to_string(),
                "--child=worker".to_string(),
                "--mode=sub-agent".to_string(),
                "--budget-ms=1000".to_string(),
            ]),
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(cowork_path(root.path()).exists());
    }
}
