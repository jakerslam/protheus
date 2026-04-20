
#[cfg(test)]
mod tests {
    use super::*;

    fn write_json(path: &Path, value: &Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, serde_json::to_string_pretty(value).expect("json")).expect("write");
    }

    #[test]
    fn manual_name_avoids_active_and_archived_collisions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json(
            &tmp.path().join(AGENT_PROFILES_REL),
            &json!({
                "type": "infring_dashboard_agent_profiles",
                "agents": {
                    "agent-a": {"name": "Analyst", "identity": {"emoji": "🔎"}},
                    "agent-b": {"name": "Kai", "identity": {"emoji": "🧠"}}
                }
            }),
        );
        write_json(
            &tmp.path().join(ARCHIVED_AGENTS_REL),
            &json!({
                "type": "infring_dashboard_archived_agents",
                "agents": {
                    "agent-c": {"name": "Avery", "emoji": "🛠️"}
                }
            }),
        );
        let name = resolve_agent_name(tmp.path(), "Kai", "analyst");
        let key = normalized_name_key(&name);
        assert!(key.starts_with("kai"));
        assert_ne!(key, "kai");
    }

    #[test]
    fn default_agent_name_uses_agent_id() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let generated = default_agent_name("agent-9df31f");
        assert_eq!(generated, "agent-9df31f");
        let generated_prefixed = default_agent_name("abc123");
        assert_eq!(generated_prefixed, "agent-abc123");
        let generated_legacy = default_agent_name("AGENT_ABC123");
        assert_eq!(generated_legacy, "agent-abc123");
        let unresolved = resolve_agent_name(tmp.path(), "", "analyst");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn post_init_auto_name_replaces_default_agent_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let generated = resolve_post_init_agent_name(tmp.path(), "agent-9df31f", "engineer");
        assert!(!generated.trim().is_empty());
        assert!(!is_default_agent_name_for_agent(&generated, "agent-9df31f"));
    }

    #[test]
    fn reserved_system_emoji_is_rejected_for_non_system_agents() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let identity =
            resolve_agent_identity(tmp.path(), &json!({"identity": {"emoji": "⚙️"}}), "analyst");
        let emoji = identity
            .get("emoji")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!emoji.is_empty());
        assert!(!is_reserved_system_emoji_key(&emoji));
    }

    #[test]
    fn reserved_system_emoji_allowed_for_system_thread() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let identity = resolve_agent_identity(
            tmp.path(),
            &json!({"identity": {"emoji": "⚙️"}, "is_system_thread": true}),
            "system",
        );
        let emoji = identity
            .get("emoji")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(is_reserved_system_emoji_key(&emoji));
    }

    #[test]
    fn default_identity_uses_infring_symbol() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let identity = resolve_agent_identity(tmp.path(), &json!({}), "analyst");
        let emoji = identity
            .get("emoji")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert_eq!(emoji, DEFAULT_AGENT_EMOJI);
    }
}
