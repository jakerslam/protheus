        let rollback = context_stacks_speculative_rollback(
            tmp.path(),
            &parsed(
                &["speculative-rollback"],
                &[("overlay-id", "rb1"), ("reason", "destructive_failure_simulation")],
            ),
        );
        assert_true_key(&rollback, "ok");
        assert_eq!(
            rollback.get("rollback_semantic_snapshot_id").and_then(Value::as_str),
            Some(base_semantic_snapshot_id.as_str())
        );

        let state_after_rollback = load_context_stacks_state(tmp.path());
        let manifest_after_rollback = state_after_rollback
            .manifests
            .iter()
            .find(|row| row.stack_id == "rollback")
            .expect("manifest");
        assert_eq!(
            manifest_after_rollback.semantic_snapshot_id, base_semantic_snapshot_id,
            "rollback should restore pre-merge semantic snapshot in one step"
        );
        let overlay = state_after_rollback
            .speculative_overlays
            .iter()
            .find(|row| row.overlay_id == "rb1")
            .expect("overlay");
        assert_eq!(overlay.status, "rolled_back");
        assert!(
            state_after_rollback.speculative_overlay_receipts.len() >= 3,
            "start + merge + rollback should emit auditable receipts"
        );
    }
}
