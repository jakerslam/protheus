    #[test]
    fn helper_primitives_batch13_match_contract() {
        let temp_root = std::env::temp_dir().join("inv_batch13");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let state_dir = temp_root.join("state");
        let _ = compute_ensure_dir(&EnsureDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(state_dir.exists());

        let json_path = state_dir.join("x.json");
        let _ = compute_write_json_atomic(&WriteJsonAtomicInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            value: Some(json!({"a": 1})),
        });
        let read_json = compute_read_json(&ReadJsonInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some(json!({})),
        });
        assert_eq!(read_json.value, json!({"a": 1}));

        let jsonl_path = state_dir.join("x.jsonl");
        let _ = compute_append_jsonl(&AppendJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
            row: Some(json!({"k": "v"})),
        });
        let read_jsonl = compute_read_jsonl(&ReadJsonlInput {
            file_path: Some(jsonl_path.to_string_lossy().to_string()),
        });
        assert_eq!(read_jsonl.rows.len(), 1);

        let read_text = compute_read_text(&ReadTextInput {
            file_path: Some(json_path.to_string_lossy().to_string()),
            fallback: Some("fallback".to_string()),
        });
        assert!(read_text.text.contains("\"a\""));

        let latest = compute_latest_json_file_in_dir(&LatestJsonFileInDirInput {
            dir_path: Some(state_dir.to_string_lossy().to_string()),
        });
        assert!(latest.file_path.is_some());

        let out_channel = compute_normalize_output_channel(&NormalizeOutputChannelInput {
            base_out: Some(json!({"enabled": false, "test_enabled": true})),
            src_out: Some(json!({"enabled": true})),
        });
        assert!(out_channel.enabled);
        assert!(out_channel.test_enabled);

        let normalized_repo = compute_normalize_repo_path(&NormalizeRepoPathInput {
            value: Some("client/runtime/config/x.json".to_string()),
            fallback: Some("/tmp/fallback.json".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert!(normalized_repo.path.contains("/tmp/root"));

        let paths = compute_runtime_paths(&RuntimePathsInput {
            policy_path: Some("/tmp/policy.json".to_string()),
            inversion_state_dir_env: Some("/tmp/state-root".to_string()),
            dual_brain_policy_path_env: Some("/tmp/dual.json".to_string()),
            default_state_dir: Some("/tmp/default-state".to_string()),
            root: Some("/tmp/root".to_string()),
        });
        assert_eq!(
            paths
                .paths
                .as_object()
                .and_then(|m| m.get("state_dir"))
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "/tmp/state-root"
        );
    }

    #[test]
    fn helper_primitives_batch14_match_contract() {
        let out_axioms = compute_normalize_axiom_list(&NormalizeAxiomListInput {
            raw_axioms: Some(json!([
                {
                    "id": " A1 ",
                    "patterns": [" Do no harm ", ""],
                    "regex": ["^never\\s+harm"],
                    "intent_tags": [" safety ", "guard"],
                    "signals": {
                        "action_terms": ["harm"],
                        "subject_terms": ["operator"],
                        "object_terms": ["user"]
                    },
                    "min_signal_groups": 2,
                    "semantic_requirements": {
                        "actions": ["protect"],
                        "subjects": ["human"],
                        "objects": ["safety"]
                    }
                }
            ])),
            base_axioms: Some(json!([])),
        });
        assert_eq!(out_axioms.axioms.len(), 1);
        let axiom = out_axioms.axioms[0].as_object().expect("axiom object");
        assert_eq!(axiom.get("id").and_then(|v| v.as_str()).unwrap_or(""), "a1");

        let out_suite = compute_normalize_harness_suite(&NormalizeHarnessSuiteInput {
            raw_suite: Some(json!([
                {
                    "id": " HX-1 ",
                    "objective": " validate lane ",
                    "impact": "critical",
                    "target": "directive",
                    "difficulty": "hard"
                }
            ])),
            base_suite: Some(json!([])),
        });
        assert_eq!(out_suite.suite.len(), 1);
        let row = out_suite.suite[0].as_object().expect("suite row");
        assert_eq!(row.get("id").and_then(|v| v.as_str()).unwrap_or(""), "hx-1");
        assert_eq!(
            row.get("target").and_then(|v| v.as_str()).unwrap_or(""),
            "directive"
        );

        let temp_root = std::env::temp_dir().join("inv_batch14");
        let _ = fs::remove_dir_all(&temp_root);
        let _ = fs::create_dir_all(&temp_root);
        let harness_path = temp_root.join("harness.json");
        let first_principles_path = temp_root.join("lock_state.json");
        let approvals_path = temp_root.join("observer_approvals.jsonl");
        let correspondence_path = temp_root.join("correspondence.md");

        let saved_harness = compute_save_harness_state(&SaveHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            state: Some(json!({"last_run_ts":"2026-03-04T00:00:00.000Z","cursor":7})),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            saved_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );
        let loaded_harness = compute_load_harness_state(&LoadHarnessStateInput {
            file_path: Some(harness_path.to_string_lossy().to_string()),
            now_iso: Some("2026-03-04T12:00:00.000Z".to_string()),
        });
        assert_eq!(
            loaded_harness
                .state
                .as_object()
                .and_then(|m| m.get("cursor"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            7
        );

