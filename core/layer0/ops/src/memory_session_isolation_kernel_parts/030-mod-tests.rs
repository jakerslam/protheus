
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_blocks_cross_session_leakage() {
        let root =
            std::env::temp_dir().join(format!("memory-session-kernel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let state_path = root.join("state/session.json");
        let allow = validate_value(
            &root,
            payload_obj(&json!({
                "args": ["query-index", "--session-id=session-a", "--resource-id=node-1"],
                "options": { "statePath": state_path.to_string_lossy().to_string() }
            })),
        );
        assert_eq!(allow.get("ok").and_then(Value::as_bool), Some(true));
        let blocked = validate_value(
            &root,
            payload_obj(&json!({
                "args": ["query-index", "--session-id=session-b", "--resource-id=node-1"],
                "options": { "statePath": state_path.to_string_lossy().to_string() }
            })),
        );
        assert_eq!(blocked.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            blocked.get("reason_code").and_then(Value::as_str),
            Some("cross_session_leak_blocked")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_fails_closed_when_state_persist_write_fails() {
        let root = tempfile::tempdir().expect("tempdir");
        let state_dir = root.path().join("state-dir");
        fs::create_dir_all(&state_dir).expect("create state dir");
        let result = validate_value(
            root.path(),
            payload_obj(&json!({
                "args": ["query-index", "--session-id=session-a", "--resource-id=node-1"],
                "options": { "statePath": state_dir.to_string_lossy().to_string() }
            })),
        );
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            result.get("reason_code").and_then(Value::as_str),
            Some("state_persist_failed")
        );
    }
}
