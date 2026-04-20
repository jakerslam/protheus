    #[test]
    fn tail_merge_and_promote_updates_snapshot_and_clears_tail() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "demo"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "a,b"),
                    ("objective", "ship reliable context stacks"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let before = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let merge = merge_context_stack_tail(
            tmp.path(),
            &parsed(
                &["tail-merge"],
                &[
                    ("stack-id", "demo"),
                    ("merge-type", "append_working_note"),
                    ("value", "Capture provider cache counters and merge outcome."),
                ],
            ),
        );
        assert_true_key(&merge, "ok");
        let promote = promote_context_stack_tail(
            tmp.path(),
            &parsed(&["tail-promote"], &[("stack-id", "demo")]),
        );
        assert_true_key(&promote, "ok");
        let after = promote
            .get("semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(before, after);
        let state = load_context_stacks_state(tmp.path());
        let tail = state
            .delta_tails
            .iter()
            .find(|row| row.stack_id == "demo")
            .expect("tail");
        assert!(tail.entries.is_empty());
    }

    #[test]
    fn nexus_authorization_succeeds_for_context_stacks_route() {
        let out = authorize_context_stacks_command_with_nexus_inner("list", false)
            .expect("nexus auth");
        assert_true_key(&out, "enabled");
        assert!(out
            .get("lease_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn nexus_authorization_fails_closed_when_context_route_blocked() {
        let err = authorize_context_stacks_command_with_nexus_inner("list", true)
            .err()
            .unwrap_or_else(|| "missing_error".to_string());
        assert!(err.contains("lease_denied") || err.contains("delivery_denied"));
    }

    #[test]
    fn taste_tune_updates_family_weight_with_guardrails() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = context_stacks_taste_tune(
            tmp.path(),
            &parsed(
                &["taste-tune"],
                &[("family", "memory_compaction"), ("merge-lift", "0.8")],
            ),
        );
        assert_true_key(&out, "ok");
        let next = out.get("next").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(next >= 0.2 && next <= 2.0, "next taste weight out of bounds");
        let state = load_context_stacks_state(tmp.path());
        let stored = state
            .taste_vectors
            .get("memory_compaction")
            .copied()
            .unwrap_or(0.0);
        assert!((stored - next).abs() < 1e-9);
    }

    #[test]
    fn partial_merge_applies_only_changed_slices() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "partial"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "alpha,beta"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let out = context_stacks_partial_merge(
            tmp.path(),
            &parsed(
                &["partial-merge"],
                &[
                    ("stack-id", "partial"),
                    (
                        "patch-json",
                        "{\"stable_nodes_add\":[\"gamma\"],\"volatile_metadata_patch\":{\"stage\":\"partial\"}}",
                    ),
                ],
            ),
        );
        assert_true_key(&out, "ok");
        assert_eq!(
            out.get("mode").and_then(Value::as_str),
            Some("diff_scoped_partial")
        );
        let changed = out
            .get("changed_slices")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(changed.iter().any(|row| row.as_str() == Some("stable_head")));
        let state = load_context_stacks_state(tmp.path());
        assert!(
            !state.partial_merge_events.is_empty(),
            "partial merge events should be recorded"
        );
    }

