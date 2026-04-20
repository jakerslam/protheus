    #[test]
    fn speculative_overlay_start_isolated_until_verified_merge() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "overlay"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "alpha,beta"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let base_semantic_snapshot_id = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let started = context_stacks_speculative_start(
            tmp.path(),
            &parsed(
                &["speculative-start"],
                &[
                    ("stack-id", "overlay"),
                    (
                        "patch-json",
                        "{\"stable_nodes_add\":[\"candidate-gamma\"],\"volatile_metadata_patch\":{\"phase\":\"speculative\"}}",
                    ),
                ],
            ),
        );
        assert_true_key(&started, "ok");
        let overlay_id = started
            .pointer("/overlay/overlay_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(
            !overlay_id.is_empty(),
            "speculative-start should emit an overlay id"
        );
        let proposed_semantic_snapshot_id = started
            .pointer("/overlay/proposed_semantic_snapshot/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(
            proposed_semantic_snapshot_id, base_semantic_snapshot_id,
            "speculative proposal should diverge from base semantic snapshot"
        );
        let state = load_context_stacks_state(tmp.path());
        let manifest = state
            .manifests
            .iter()
            .find(|row| row.stack_id == "overlay")
            .expect("manifest");
        assert_eq!(
            manifest.semantic_snapshot_id, base_semantic_snapshot_id,
            "manifest must stay canonical until merge approval"
        );
        let overlay = state
            .speculative_overlays
            .iter()
            .find(|row| row.overlay_id == overlay_id)
            .expect("overlay");
        assert_eq!(overlay.status, "active");

        let status = context_stacks_speculative_status(
            tmp.path(),
            &parsed(
                &["speculative-status"],
                &[("stack-id", "overlay"), ("overlay-id", overlay_id.as_str())],
            ),
        );
        assert_true_key(&status, "ok");
        assert_eq!(status.get("overlay_count").and_then(Value::as_u64), Some(1));
        assert!(
            status
                .get("receipt_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1,
            "speculative status should expose disposable receipt stream"
        );
    }

    #[test]
    fn speculative_overlay_merge_gate_and_single_step_rollback_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let create = create_context_stack(
            tmp.path(),
            &parsed(
                &["create"],
                &[
                    ("stack-id", "rollback"),
                    ("system-prompt", "system"),
                    ("stable-nodes", "seed"),
                ],
            ),
        );
        assert_true_key(&create, "ok");
        let base_semantic_snapshot_id = create
            .pointer("/stack/semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let started = context_stacks_speculative_start(
            tmp.path(),
            &parsed(
                &["speculative-start"],
                &[
                    ("stack-id", "rollback"),
                    ("overlay-id", "rb1"),
                    ("patch-json", "{\"stable_nodes_add\":[\"candidate\"]}"),
                ],
            ),
        );
        assert_true_key(&started, "ok");

        let blocked_merge = context_stacks_speculative_merge(
            tmp.path(),
            &parsed(&["speculative-merge"], &[("overlay-id", "rb1")]),
        );
        assert_eq!(
            blocked_merge.get("ok").and_then(Value::as_bool),
            Some(false),
            "merge must fail closed without verity approval"
        );
        assert_eq!(
            blocked_merge.get("error").and_then(Value::as_str),
            Some("speculative_merge_approval_required")
        );

        let merged = context_stacks_speculative_merge(
            tmp.path(),
            &parsed(
                &["speculative-merge"],
                &[
                    ("overlay-id", "rb1"),
                    ("verify-merge", "true"),
                    ("approval-note", "approved by verity gate"),
                ],
            ),
        );
        assert_true_key(&merged, "ok");
        let merged_semantic_snapshot_id = merged
            .get("merged_semantic_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_ne!(
            merged_semantic_snapshot_id, base_semantic_snapshot_id,
            "approved merge should promote speculative snapshot"
        );

        let state_after_merge = load_context_stacks_state(tmp.path());
        let manifest_after_merge = state_after_merge
            .manifests
            .iter()
            .find(|row| row.stack_id == "rollback")
            .expect("manifest");
        assert_eq!(
            manifest_after_merge.semantic_snapshot_id, merged_semantic_snapshot_id,
            "manifest should move to merged speculative snapshot"
        );

