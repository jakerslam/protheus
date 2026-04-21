
fn collect_documents(root: &Path) -> Vec<ConversationDocument> {
    let profiles = profile_map(root);
    let contracts = contract_map(root);
    let archived_ids = crate::dashboard_agent_state::archived_agent_ids(root);
    let mut out = Vec::<ConversationDocument>::new();
    let mut seen = HashSet::<String>::new();
    let dir = sessions_dir(root);

    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let state = match read_json_file(&path) {
                Some(value) => value,
                None => continue,
            };
            let mut agent_id = normalize_agent_id(
                state
                    .get("agent_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            if agent_id.is_empty() {
                let stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                agent_id = normalize_agent_id(stem);
            }
            if agent_id.is_empty() || seen.contains(&agent_id) {
                continue;
            }
            seen.insert(agent_id.clone());

            let profile = profiles.get(&agent_id);
            let contract = contracts.get(&agent_id);
            let active_id = clean_text(
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
            let active = sessions
                .iter()
                .find(|row| {
                    row.get("session_id")
                        .and_then(Value::as_str)
                        .map(|value| value == active_id)
                        .unwrap_or(false)
                })
                .cloned()
                .unwrap_or_else(|| sessions.first().cloned().unwrap_or_else(|| json!({})));
            let updated_at = clean_text(
                active
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .or_else(|| state.get("updated_at").and_then(Value::as_str))
                    .unwrap_or(""),
                80,
            );
            let messages = active
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut lines = Vec::<String>::new();
            for message in messages {
                let line = text_from_message(&message);
                if line.is_empty() {
                    continue;
                }
                lines.push(line);
                if lines.len() >= MAX_INDEXED_LINES_PER_AGENT {
                    break;
                }
            }
            let name = clean_text(
                profile
                    .and_then(|row| row.get("name").and_then(Value::as_str))
                    .unwrap_or(""),
                140,
            );
            let name = if name.is_empty() {
                humanize_agent_name(&agent_id)
            } else {
                name
            };
            let avatar_url = clean_text(
                profile
                    .and_then(|row| row.get("avatar_url").and_then(Value::as_str))
                    .unwrap_or(""),
                480,
            );
            let emoji = clean_text(
                profile
                    .and_then(|row| row.get("identity"))
                    .and_then(|row| row.get("emoji"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                16,
            );
            let status = clean_text(
                contract
                    .and_then(|row| row.get("status").and_then(Value::as_str))
                    .unwrap_or("active"),
                40,
            )
            .to_ascii_lowercase();
            let archived = archived_ids.contains(&agent_id) || status == "terminated";
            let state_label = if archived {
                "archived".to_string()
            } else {
                clean_text(
                    profile
                        .and_then(|row| row.get("state").and_then(Value::as_str))
                        .unwrap_or("running"),
                    40,
                )
            };
            out.push(ConversationDocument {
                agent_id,
                name,
                archived,
                state: state_label,
                avatar_url,
                emoji,
                updated_at,
                lines,
            });
        }
    }

    for (agent_id, profile) in profiles {
        if seen.contains(&agent_id) {
            continue;
        }
        let name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            140,
        );
        let avatar_url = clean_text(
            profile
                .get("avatar_url")
                .and_then(Value::as_str)
                .unwrap_or(""),
            480,
        );
        let emoji = clean_text(
            profile
                .get("identity")
                .and_then(|row| row.get("emoji"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            16,
        );
        let contract_status = clean_text(
            contracts
                .get(&agent_id)
                .and_then(|row| row.get("status").and_then(Value::as_str))
                .unwrap_or("active"),
            40,
        )
        .to_ascii_lowercase();
        let archived = archived_ids.contains(&agent_id) || contract_status == "terminated";
        out.push(ConversationDocument {
            agent_id: agent_id.clone(),
            name: if name.is_empty() {
                humanize_agent_name(&agent_id)
            } else {
                name
            },
            archived,
            state: if archived {
                "archived".to_string()
            } else {
                clean_text(
                    profile
                        .get("state")
                        .and_then(Value::as_str)
                        .unwrap_or("running"),
                    40,
                )
            },
            avatar_url,
            emoji,
            updated_at: clean_text(
                profile
                    .get("updated_at")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            ),
            lines: Vec::new(),
        });
    }
    out
}
