fn compact_active_session(root: &Path, agent_id: &str, request: &Value) -> Value {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let target_window = request
        .get("target_context_window")
        .and_then(Value::as_i64)
        .unwrap_or(8192)
        .clamp(512, 2_000_000);
    let target_ratio = request
        .get("target_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.8)
        .clamp(0.2, 0.95);
    let min_recent_messages = request
        .get("min_recent_messages")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(2, 200) as usize;
    let max_messages = request
        .get("max_messages")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(20, 800) as usize;
    let persist_compaction_to_session = request
        .get("persist_compaction_to_session")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let mut before_tokens = 0i64;
    let mut after_tokens = 0i64;
    let mut before_messages = 0usize;
    let mut after_messages = 0usize;
    let mut removed_messages = Vec::<Value>::new();
    let mut emitted_keyframes = Vec::<Value>::new();
    if let Some(rows) = state.get_mut("sessions").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            let sid = clean_text(
                row.get("session_id").and_then(Value::as_str).unwrap_or(""),
                120,
            );
            if sid != active_id {
                continue;
            }
            if !row.get("messages").map(Value::is_array).unwrap_or(false) {
                row["messages"] = Value::Array(Vec::new());
            }
            let messages = row
                .get_mut("messages")
                .and_then(Value::as_array_mut)
                .expect("messages");
            before_messages = messages.len();
            before_tokens = messages
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            let mut compacted = messages.clone();
            let target_tokens = ((target_window as f64) * target_ratio).round() as i64;
            if compacted.len() > max_messages {
                let drain = compacted.len().saturating_sub(max_messages);
                removed_messages.extend(compacted.drain(0..drain));
            }
            while compacted.len() > min_recent_messages {
                let current_tokens = compacted
                    .iter()
                    .map(|item| {
                        let text = item
                            .get("text")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("content").and_then(Value::as_str))
                            .unwrap_or("");
                        estimate_tokens(text)
                    })
                    .sum::<i64>();
                if current_tokens <= target_tokens {
                    break;
                }
                if !compacted.is_empty() {
                    removed_messages.push(compacted.remove(0));
                }
            }
            after_messages = compacted.len();
            after_tokens = compacted
                .iter()
                .map(|item| {
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("content").and_then(Value::as_str))
                        .unwrap_or("");
                    estimate_tokens(text)
                })
                .sum::<i64>();
            if persist_compaction_to_session {
                *messages = compacted;
            }
            emitted_keyframes = build_context_keyframes_from_removed(&removed_messages, 8);
            if !row
                .get("context_keyframes")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["context_keyframes"] = Value::Array(Vec::new());
            }
            if let Some(keyframes) = row
                .get_mut("context_keyframes")
                .and_then(Value::as_array_mut)
            {
                keyframes.extend(emitted_keyframes.clone());
                if keyframes.len() > 48 {
                    let trim = keyframes.len().saturating_sub(48);
                    keyframes.drain(0..trim);
                }
            }
            if !row
                .get("compaction_archives")
                .map(Value::is_array)
                .unwrap_or(false)
            {
                row["compaction_archives"] = Value::Array(Vec::new());
            }
            let archive_messages = removed_messages
                .iter()
                .take(240)
                .map(|item| {
                    json!({
                        "role": clean_text(item.get("role").and_then(Value::as_str).unwrap_or(""), 24),
                        "text": clean_text(&compaction_message_text(item), 1200),
                        "ts": item.get("ts").cloned().unwrap_or(Value::Null),
                        "created_at": item.get("created_at").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>();
            let archive = json!({
                "archive_id": format!("cmp-{}", &crate::deterministic_receipt_hash(&json!({
                    "agent_id": id,
                    "removed_count": removed_messages.len(),
                    "before_tokens": before_tokens,
                    "after_tokens": after_tokens,
                    "captured_at": crate::now_iso()
                }))[..12]),
                "captured_at": crate::now_iso(),
                "removed_count": removed_messages.len(),
                "persisted_to_session": persist_compaction_to_session,
                "removed_excerpt_count": archive_messages.len(),
                "removed_messages": archive_messages,
                "keyframes": emitted_keyframes
            });
            if let Some(archives) = row
                .get_mut("compaction_archives")
                .and_then(Value::as_array_mut)
            {
                archives.push(archive);
                if archives.len() > 12 {
                    let trim = archives.len().saturating_sub(12);
                    archives.drain(0..trim);
                }
            }
            row["updated_at"] = Value::String(crate::now_iso());
            break;
        }
    }
    save_session_state(root, &id, &state);
    json!({
        "ok": true,
        "type": "dashboard_agent_session_compact",
        "agent_id": id,
        "before_tokens": before_tokens,
        "after_tokens": after_tokens,
        "before_messages": before_messages,
        "after_messages": after_messages,
        "removed_messages": removed_messages.len(),
        "persisted_to_session": persist_compaction_to_session,
        "keyframes_emitted": emitted_keyframes.len(),
        "keyframes": emitted_keyframes,
        "message": format!("Compaction complete: {} -> {} tokens", before_tokens, after_tokens)
    })
}

fn parse_agent_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|v| clean_text(v, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn request_mode_is_cua(request: &Value) -> bool {
    let mode = clean_text(
        request.get("mode").and_then(Value::as_str).unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    mode == "cua" || request.get("cua").and_then(Value::as_bool).unwrap_or(false)
}

fn request_has_nonempty_array(request: &Value, key: &str) -> bool {
    request
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
}

fn request_has_nonempty_object(request: &Value, key: &str) -> bool {
    request
        .get(key)
        .and_then(Value::as_object)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
}

fn cua_unsupported_features(request: &Value) -> Vec<&'static str> {
    let mut features = Vec::<&'static str>::new();
    if request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        features.push("streaming");
    }
    if request
        .get("signal")
        .map(|row| !row.is_null())
        .unwrap_or(false)
    {
        features.push("abort signal");
    }
    if request
        .get("messages")
        .map(|row| !row.is_null())
        .unwrap_or(false)
    {
        features.push("message continuation");
    }
    if request_has_nonempty_array(request, "excludeTools")
        || request_has_nonempty_array(request, "exclude_tools")
    {
        features.push("excludeTools");
    }
    if request
        .get("output")
        .map(|row| !row.is_null())
        .unwrap_or(false)
        || request
            .get("output_schema")
            .map(|row| !row.is_null())
            .unwrap_or(false)
    {
        features.push("output schema");
    }
    if request_has_nonempty_object(request, "variables") {
        features.push("variables");
    }
    features
}

fn resolve_agent_id_alias(root: &Path, requested: &str) -> String {
    let normalized = clean_agent_id(requested);
    if normalized.is_empty() {
        return String::new();
    }
    let profiles = profiles_map(root);
    if profiles.contains_key(&normalized) {
        return normalized;
    }
    let contracts = contracts_map(root);
    if contracts.contains_key(&normalized) {
        return normalized;
    }
    let requested_name = clean_text(requested, 120).to_ascii_lowercase();
    if requested_name.is_empty() {
        return normalized;
    }
    for (id, profile) in &profiles {
        let profile_name = clean_text(
            profile.get("name").and_then(Value::as_str).unwrap_or(""),
            120,
        )
        .to_ascii_lowercase();
        if !profile_name.is_empty() && profile_name == requested_name {
            let resolved = clean_agent_id(id);
            if !resolved.is_empty() {
                return resolved;
            }
        }
    }
    normalized
}

fn parse_provider_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/providers/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let provider_id = decode_path_segment(&parts.remove(0));
    if provider_id.is_empty() {
        return None;
    }
    Some((provider_id, parts))
}

fn parse_virtual_key_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/virtual-keys/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail
        .split('/')
        .map(|value| clean_text(value, 180))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    let key_id = decode_path_segment(&parts.remove(0));
    if key_id.is_empty() {
        return None;
    }
    Some((key_id, parts))
}

fn parse_memory_route(path_only: &str) -> Option<(String, Vec<String>)> {
    let prefix = "/api/memory/agents/";
    if !path_only.starts_with(prefix) {
        return None;
    }
    let tail = path_only.trim_start_matches(prefix).trim_matches('/');
    if tail.is_empty() {
        return None;
    }
    let mut parts = tail.split('/').map(decode_path_segment).collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let agent_id = clean_agent_id(&parts.remove(0));
    if agent_id.is_empty() {
        return None;
    }
    Some((agent_id, parts))
}

fn decode_path_segment(raw: &str) -> String {
    let decoded = urlencoding::decode(raw)
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_else(|| raw.to_string());
    clean_text(&decoded, 300)
}

fn workspace_base_for_agent(root: &Path, row: Option<&Value>) -> PathBuf {
    let raw = clean_text(
        row.and_then(|v| v.get("workspace_dir").and_then(Value::as_str))
            .unwrap_or(""),
        4000,
    );
    let base = if raw.is_empty() {
        root.to_path_buf()
    } else {
        let as_path = PathBuf::from(raw);
        if as_path.is_absolute() {
            as_path
        } else {
            root.join(as_path)
        }
    };
    normalize_lexical(&base)
}

fn resolve_workspace_path(base: &Path, requested_path: &str) -> Option<PathBuf> {
    let cleaned = requested_path.trim();
    if cleaned.is_empty() {
        return None;
    }
    let requested = PathBuf::from(cleaned);
    let candidate = if requested.is_absolute() {
        requested
    } else {
        base.join(requested)
    };
    let base_norm = normalize_lexical(base);
    let candidate_norm = normalize_lexical(&candidate);
    if !candidate_norm.starts_with(&base_norm) {
        return None;
    }
    Some(candidate_norm)
}

