pub fn session_summaries(root: &Path, limit: usize) -> Value {
    let profiles = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/agent_profiles.json"),
    )
    .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let contracts = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/agent_contracts.json"),
    )
    .and_then(|value| value.get("contracts").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let archived = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/archived_agents.json"),
    )
    .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
    .unwrap_or_default();
    let mut allowed_ids = HashSet::<String>::new();
    for id in profiles.keys() {
        let normalized = normalize_agent_id(id);
        if !normalized.is_empty() {
            allowed_ids.insert(normalized);
        }
    }
    for (id, contract) in &contracts {
        let normalized = normalize_agent_id(id);
        if normalized.is_empty() {
            continue;
        }
        let status = clean_text(contract.get("status").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if status != "terminated" {
            allowed_ids.insert(normalized);
        }
    }
    for id in archived.keys() {
        let normalized = normalize_agent_id(id);
        if !normalized.is_empty() {
            allowed_ids.remove(&normalized);
        }
    }
    for (id, profile) in &profiles {
        let state = clean_text(profile.get("state").and_then(Value::as_str).unwrap_or(""), 40)
            .to_ascii_lowercase();
        if state == "archived" {
            let normalized = normalize_agent_id(id);
            if !normalized.is_empty() {
                allowed_ids.remove(&normalized);
            }
        }
    }

    let mut rows = Vec::<Value>::new();
    let dir = sessions_dir(root);
    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            let file_agent_id = normalize_agent_id(
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or(""),
            );
            if !file_agent_id.is_empty() && !allowed_ids.contains(&file_agent_id) {
                continue;
            }
            if let Some(state) = read_json_file(&path) {
                let mut agent_id = clean_text(
                    state.get("agent_id").and_then(Value::as_str).unwrap_or(""),
                    140,
                );
                if agent_id.is_empty() {
                    agent_id = file_agent_id.clone();
                }
                if agent_id.is_empty() {
                    continue;
                }
                let active = clean_text(
                    state
                        .get("active_session_id")
                        .and_then(Value::as_str)
                        .unwrap_or("default"),
                    120,
                );
                let sessions = state
                    .get("sessions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let current = sessions
                    .iter()
                    .find(|row| {
                        row.get("session_id")
                            .and_then(Value::as_str)
                            .map(|v| v == active)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .unwrap_or_else(|| json!({"messages": []}));
                let messages = current
                    .get("messages")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let updated_at = clean_text(
                    current
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    80,
                );
                rows.push(json!({
                    "agent_id": agent_id,
                    "active_session_id": active,
                    "message_count": messages.len(),
                    "updated_at": updated_at
                }));
            }
        }
    }
    rows.sort_by_key(|row| {
        std::cmp::Reverse(clean_text(
            row.get("updated_at").and_then(Value::as_str).unwrap_or(""),
            80,
        ))
    });
    rows.truncate(limit.clamp(1, 500));
    json!({"type": "dashboard_agent_session_summaries", "rows": rows})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_suggestion_context(root: &Path, agent_id: &str) {
        let _ = append_turn(
            root,
            agent_id,
            "chat scroll thrashes near bottom after long replies",
            "I can inspect the bottom lock and viewport anchoring logic.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "the bounce still appears when I manually drag down",
            "I'll patch the drag edge clamp and rerun the scroll test.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "prompt suggestions still look generic in this thread",
            "I'll tighten suggestions to recent context and remove generic phrasing.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "make sure suggestions read like real user followups",
            "Understood, I'll constrain wording to human-readable followups.",
        );
    }

    #[test]
    fn append_turn_preserves_multiline_markdown_layout() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = append_turn(
            root.path(),
            "agent-layout",
            "Please return:\n1. alpha\n2. beta",
            "Sure.\n1. one\n2. two\n   - nested",
        );
        let state = load_session_state(root.path(), "agent-layout");
        let messages = state
            .pointer("/sessions/0/messages")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(messages.len(), 2);
        let user = messages
            .first()
            .and_then(|row| row.get("text").and_then(Value::as_str))
            .unwrap_or("");
        let assistant = messages
            .get(1)
            .and_then(|row| row.get("text").and_then(Value::as_str))
            .unwrap_or("");
        assert!(user.contains("\n1. alpha\n2. beta"));
        assert!(assistant.contains("\n1. one\n2. two\n   - nested"));
    }

    #[test]
    fn suggestions_are_deduped_and_never_quoted() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_suggestion_context(root.path(), "agent-a");
        let value = suggestions(
            root.path(),
            "agent-a",
            "\"Can you reduce queue depth before spikes?\"",
        );
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.len() <= 3);
        for row in rows {
            let text = row.as_str().unwrap_or("");
            assert!(!text.contains('"'));
            assert!(!text.contains('\''));
        }
    }

    #[test]
    fn suggestions_follow_recent_thread_context_window() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = append_turn(
            root.path(),
            "agent-b",
            "neon trail still drifts while scrolling",
            "I can inspect pointer math and scrolling anchors.",
        );
        let _ = append_turn(
            root.path(),
            "agent-b",
            "fix neon trail anchor now",
            "I patched the anchor but we should verify it.",
        );
        let _ = append_turn(
            root.path(),
            "agent-b",
            "the neon trail still jitters at chat bottom",
            "I see jitter around scroll bounds and bottom padding.",
        );
        let _ = append_turn(
            root.path(),
            "agent-b",
            "make neon trail stay pinned to cursor while scrolling",
            "I'll run one more pass and verify smoothness.",
        );

        let value = suggestions(root.path(), "agent-b", "");
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        assert!(rows.len() <= 3);
        let mut joined = String::new();
        for row in rows {
            let text = row.as_str().unwrap_or("");
            assert!(!text.is_empty());
            assert!(text.split_whitespace().count() <= PROMPT_SUGGESTION_MAX_WORDS);
            assert!(text.ends_with('?'));
            if let Some(first) = text.chars().next() {
                assert!(!first.is_ascii_uppercase());
            }
            joined.push_str(&text.to_ascii_lowercase());
            joined.push(' ');
        }
        assert!(
            joined.contains("neon")
                || joined.contains("trail")
                || joined.contains("scroll")
                || joined.contains("cursor")
        );
    }

    #[test]
    fn suggestions_ignore_hint_and_use_recent_messages_only() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_suggestion_context(root.path(), "agent-c");
        let value = suggestions(root.path(), "agent-c", "run system diagnostic full scan");
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        let joined = rows
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        assert!(!joined.trim().is_empty());
        assert!(!joined.contains("diagnostic"));
        assert!(!joined.contains("scan"));
    }

    #[test]
    fn suggestions_are_human_readable_and_under_word_budget() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_suggestion_context(root.path(), "agent-d");
        let value = suggestions(root.path(), "agent-d", "");
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        for row in rows {
            let text = row.as_str().unwrap_or("");
            assert!(text.ends_with('?'));
            assert!(text.split_whitespace().count() <= PROMPT_SUGGESTION_MAX_WORDS);
            assert!(!text.contains("  "));
            assert!(!text.ends_with(" and?"));
            assert!(!text.ends_with(" to?"));
        }
    }

    #[test]
    fn suggestions_require_seven_recent_messages() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = append_turn(
            root.path(),
            "agent-min-context",
            "fix chat bounce",
            "I'll inspect the bottom clamp.",
        );
        let _ = append_turn(
            root.path(),
            "agent-min-context",
            "still bouncing",
            "I'll patch and re-test.",
        );
        let _ = append_turn(
            root.path(),
            "agent-min-context",
            "retry now",
            "Applying update.",
        );

        let value = suggestions(root.path(), "agent-min-context", "");
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.is_empty());
    }

    #[test]
    fn session_summaries_skip_orphaned_session_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = crate::dashboard_agent_state::upsert_profile(
            root.path(),
            "agent-known",
            &json!({"name": "Known", "role": "operator", "state": "Running"}),
        );
        let _ = append_turn(root.path(), "agent-known", "hello", "world");
        let _ = append_turn(root.path(), "agent-zombie", "stale", "session");

        let summaries = session_summaries(root.path(), 100);
        let rows = summaries
            .get("rows")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids = rows
            .iter()
            .filter_map(|row| row.get("agent_id").and_then(Value::as_str))
            .map(|row| clean_text(row, 140))
            .collect::<Vec<_>>();

        assert!(ids.iter().any(|id| id == "agent-known"));
        assert!(!ids.iter().any(|id| id == "agent-zombie"));
    }
}

