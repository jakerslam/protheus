
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|v| (*v).to_string()).collect()
    }

    #[test]
    fn register_resume_and_steer_persist_state() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();

        assert_eq!(
            run(
                root,
                &argv(&[
                    "register",
                    "--session-id=session-alpha",
                    "--lineage-id=lineage-alpha",
                    "--task=ship_feature"
                ])
            ),
            0
        );
        assert_eq!(run(root, &argv(&["resume", "session-alpha"])), 0);
        assert_eq!(
            run(
                root,
                &argv(&[
                    "send",
                    "session-alpha",
                    "--message=apply patch and run tests"
                ])
            ),
            0
        );

        let state_file = root.join(DEFAULT_STATE_PATH);
        let registry = load_registry(&state_file).expect("state load");
        let session = registry.sessions.get("session-alpha").expect("session");
        assert_eq!(session.lineage_id, "lineage-alpha");
        assert_eq!(session.attach_count, 1);
        assert_eq!(session.steering_count, 1);
        assert_eq!(session.token_count, 0);
        assert!(session.cost_usd >= 0.0);
        assert!(session.last_attach_epoch_ms.is_some());
        assert!(session.last_steering_hash.is_some());
        assert!(!session.events.is_empty());
    }

    #[test]
    fn cannot_resume_terminated_session() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        assert_eq!(
            run(
                root,
                &argv(&[
                    "register",
                    "--session-id=session-z",
                    "--status=terminated",
                    "--lineage-id=lineage-z"
                ])
            ),
            0
        );
        assert_eq!(run(root, &argv(&["resume", "session-z"])), 4);
    }

    #[test]
    fn lifecycle_kill_tail_and_inspect_work() {
        let tmp = tempdir().expect("tmp");
        let root = tmp.path();
        assert_eq!(
            run(
                root,
                &argv(&[
                    "register",
                    "--session-id=session-b",
                    "--lineage-id=lineage-b",
                    "--task=triage"
                ])
            ),
            0
        );
        assert_eq!(
            run(
                root,
                &argv(&[
                    "send",
                    "session-b",
                    "--message=collect evidence",
                    "--token-delta=42",
                    "--cost-delta=0.12"
                ])
            ),
            0
        );
        assert_eq!(run(root, &argv(&["tail", "session-b", "--lines=5"])), 0);
        assert_eq!(run(root, &argv(&["inspect", "session-b"])), 0);
        assert_eq!(run(root, &argv(&["kill", "session-b"])), 0);
        assert_eq!(
            run(
                root,
                &argv(&["send", "session-b", "--message=should fail after kill"])
            ),
            4
        );

        let state_file = root.join(DEFAULT_STATE_PATH);
        let registry = load_registry(&state_file).expect("state load");
        let session = registry.sessions.get("session-b").expect("session");
        assert_eq!(session.status, "terminated");
        assert_eq!(session.token_count, 42);
        assert!(session.cost_usd >= 0.12);
        assert!(session.terminated_epoch_ms.is_some());
        assert!(session.events.iter().any(|e| e.kind == "kill"));
    }
}
