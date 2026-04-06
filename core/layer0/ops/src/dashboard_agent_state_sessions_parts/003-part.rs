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
        let status = clean_text(
            contract.get("status").and_then(Value::as_str).unwrap_or(""),
            40,
        )
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
        let state = clean_text(
            profile.get("state").and_then(Value::as_str).unwrap_or(""),
            40,
        )
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
            // Keep session-backed agents discoverable in fresh state roots where
            // profile/contract records have not been materialized yet.
            if !file_agent_id.is_empty()
                && !allowed_ids.is_empty()
                && !allowed_ids.contains(&file_agent_id)
            {
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
            "the llm menu still misses kimi and qwen models",
            "I'll inspect model discovery and runtime provider wiring.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "prompt suggestions still look repetitive and generic",
            "Understood, I'll tighten suggestion quality and context grounding.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "remove template-like suggestions from this thread",
            "I'll switch to model-generated suggestions with stricter filtering.",
        );
        let _ = append_turn(
            root,
            agent_id,
            "also gate suggestions so weak models skip this feature",
            "I'll add a param threshold gate and validate behavior.",
        );
    }

    fn seed_profile(root: &Path, agent_id: &str, provider: &str, runtime_model: &str) {
        let _ = crate::dashboard_agent_state::upsert_profile(
            root,
            agent_id,
            &json!({
                "name": agent_id,
                "role": "analyst",
                "state": "Running",
                "model_provider": provider,
                "runtime_model": runtime_model
            }),
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
    fn suggestions_require_seven_recent_messages() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_profile(
            root.path(),
            "agent-min-context",
            "ollama",
            "deepseek-v3.1:671b-cloud",
        );
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
    fn sanitize_suggestion_strips_agent_offer_voice_prefixes() {
        assert_eq!(
            sanitize_suggestion("Do you want me to run install doctor?"),
            "run install doctor"
        );
        assert_eq!(
            sanitize_suggestion("Should I check gateway health now?"),
            "check gateway health now"
        );
        assert_eq!(
            sanitize_suggestion("Would you like me to compare model routing?"),
            "compare model routing"
        );
    }

    #[test]
    fn suggestions_skip_models_below_param_threshold() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_profile(root.path(), "agent-small-model", "ollama", "llama3.3:70b");
        seed_suggestion_context(root.path(), "agent-small-model");

        let value = suggestions(root.path(), "agent-small-model", "");
        let rows = value
            .get("suggestions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(rows.is_empty());
    }

    #[test]
    fn suggestions_reject_template_like_rows_even_with_large_model() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_profile(
            root.path(),
            "agent-template-filter",
            "ollama",
            "deepseek-v3.1:671b-cloud",
        );
        seed_suggestion_context(root.path(), "agent-template-filter");

        // Test-only override hook to simulate model output.
        std::env::set_var(
            "INFRING_PROMPT_SUGGESTION_TEST_RESPONSE",
            r#"{"suggestions":["Can you continue with compare other","Show the exact root cause path","Which verification should we run next"]}"#,
        );
        let value = suggestions(root.path(), "agent-template-filter", "");
        std::env::remove_var("INFRING_PROMPT_SUGGESTION_TEST_RESPONSE");

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
        assert!(!joined.contains("continue with"));
        assert!(!joined.contains("can you continue"));
        assert!(rows
            .iter()
            .filter_map(Value::as_str)
            .all(|row| !row.trim_end().ends_with('?')));
        assert!(rows.len() <= PROMPT_SUGGESTION_MAX_COUNT);
    }

    #[test]
    fn suggestions_fall_back_to_non_template_rows_when_model_output_is_invalid_json() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_profile(
            root.path(),
            "agent-invalid-response",
            "ollama",
            "deepseek-v3.1:671b-cloud",
        );
        seed_suggestion_context(root.path(), "agent-invalid-response");

        let value = suggestions(root.path(), "agent-invalid-response", "");
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
        assert!(!joined.contains("continue with"));
        assert!(!joined.contains("can you continue"));
        assert!(rows
            .iter()
            .filter_map(Value::as_str)
            .all(|row| !row.trim_end().ends_with('?')));
        assert!(rows.len() <= PROMPT_SUGGESTION_MAX_COUNT);
    }

    #[test]
    fn suggestions_include_analytics_grounded_follow_ups_for_command_like_context() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_profile(
            root.path(),
            "agent-analytics-suggestions",
            "ollama",
            "deepseek-v3.1:671b-cloud",
        );
        let _ = append_turn(
            root.path(),
            "agent-analytics-suggestions",
            "git status",
            "I can check that.",
        );
        let _ = append_turn(
            root.path(),
            "agent-analytics-suggestions",
            "cargo test --workspace",
            "Running tests.",
        );
        let _ = append_turn(
            root.path(),
            "agent-analytics-suggestions",
            "docker logs api",
            "Collecting logs.",
        );
        let _ = append_turn(
            root.path(),
            "agent-analytics-suggestions",
            "infring gateway status",
            "Checking gateway status now.",
        );

        let value = suggestions(root.path(), "agent-analytics-suggestions", "");
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
        assert!(!joined.contains("can you continue"));
        assert!(rows
            .iter()
            .filter_map(Value::as_str)
            .all(|row| !row.trim_end().ends_with('?')));
        assert!(joined.contains("infring") || joined.contains("command flow"));
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
