mod tests {
    use super::*;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(
            path,
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string()),
        );
    }

    #[test]
    fn search_returns_ranked_rows_with_snippet() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(AGENT_PROFILES_REL),
            &json!({
                "agents": {
                    "agent-alpha": { "name": "Lucas", "identity": { "emoji": "🔬" } }
                }
            }),
        );
        write_json(
            &root
                .path()
                .join(AGENT_SESSIONS_DIR_REL)
                .join("agent-alpha.json"),
            &json!({
                "agent_id": "agent-alpha",
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "updated_at": "2026-04-01T01:02:03Z",
                    "messages": [
                        {"role": "user", "text": "Fix websocket reconnect stability"},
                        {"role": "agent", "text": "I patched reconnect jitter and retry cadence"}
                    ]
                }]
            }),
        );
        let out = search_conversations(root.path(), "reconnect jitter", 10);
        let rows = out
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        let first = rows.first().cloned().unwrap_or_else(|| json!({}));
        assert_eq!(
            first.get("agent_id").and_then(Value::as_str),
            Some("agent-alpha")
        );
        let snippet = first
            .get("snippet")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(snippet.starts_with("...["));
        assert!(snippet.ends_with("]..."));
    }

    #[test]
    fn archived_agents_are_included() {
        let root = tempfile::tempdir().expect("tempdir");
        write_json(
            &root.path().join(AGENT_PROFILES_REL),
            &json!({
                "agents": {
                    "agent-zed": { "name": "Zed" }
                }
            }),
        );
        write_json(
            &root.path().join(AGENT_CONTRACTS_REL),
            &json!({
                "contracts": {
                    "agent-zed": { "status": "terminated" }
                }
            }),
        );
        write_json(
            &root
                .path()
                .join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
            &json!({
                "agents": { "agent-zed": { "reason": "user_archive" } }
            }),
        );
        write_json(
            &root
                .path()
                .join(AGENT_SESSIONS_DIR_REL)
                .join("agent-zed.json"),
            &json!({
                "agent_id": "agent-zed",
                "active_session_id": "default",
                "sessions": [{
                    "session_id": "default",
                    "updated_at": "2026-03-30T00:00:00Z",
                    "messages": [
                        {"role": "user", "text": "Review archived onboarding plan"}
                    ]
                }]
            }),
        );
        let out = search_conversations(root.path(), "onboarding", 8);
        let row = out
            .get("results")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            row.get("agent_id").and_then(Value::as_str),
            Some("agent-zed")
        );
        assert_eq!(row.get("archived").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn snippet_prefers_query_terms_over_connectors() {
        let snippet = snippet_for_line(
            "and with the reconnect jitter fix now reduces retries",
            &["reconnect".to_string(), "jitter".to_string()],
        );
        assert!(snippet.contains("[reconnect jitter"));
    }
}
