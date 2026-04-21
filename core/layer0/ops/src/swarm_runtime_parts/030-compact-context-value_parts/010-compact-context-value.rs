fn compact_context_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(clean_text(text, 160)),
        Value::Array(rows) => Value::Array(
            rows.iter()
                .take(8)
                .map(compact_context_value)
                .collect::<Vec<_>>(),
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys.into_iter().take(12) {
                if let Some(value) = map.get(&key) {
                    out.insert(clean_text(&key, 64), compact_context_value(value));
                }
            }
            Value::Object(out)
        }
        _ => value.clone(),
    }
}
fn normalize_context_map(input: Value) -> Map<String, Value> {
    match input {
        Value::Object(map) => map,
        other => {
            let mut out = Map::new();
            out.insert("value".to_string(), other);
            out
        }
    }
}

fn safe_registry_slug(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if out.len() >= max_len {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.' | ':') {
            out.push(ch);
        } else if ch.is_whitespace() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn state_path(root: &Path, argv: &[String]) -> PathBuf {
    parse_flag(argv, "state-path")
        .filter(|v| !v.trim().is_empty())
        .map(|v| {
            let candidate = PathBuf::from(v.trim());
            if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            }
        })
        .unwrap_or_else(|| root.join(DEFAULT_STATE_PATH))
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))
}

#[derive(Debug, Clone)]
struct StateCacheEntry {
    modified_ms: u128,
    byte_len: u64,
    state: SwarmState,
}

fn state_cache() -> &'static Mutex<BTreeMap<String, StateCacheEntry>> {
    static STATE_CACHE: OnceLock<Mutex<BTreeMap<String, StateCacheEntry>>> = OnceLock::new();
    STATE_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn state_file_fingerprint(path: &Path) -> Option<(u128, u64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some((modified_ms, metadata.len()))
}

fn load_cached_state(path: &Path, modified_ms: u128, byte_len: u64) -> Option<SwarmState> {
    let key = path.to_string_lossy().to_string();
    let guard = state_cache().lock().ok()?;
    guard.get(&key).and_then(|entry| {
        (entry.modified_ms == modified_ms && entry.byte_len == byte_len)
            .then(|| entry.state.clone())
    })
}

fn store_cached_state(path: &Path, modified_ms: u128, byte_len: u64, state: &SwarmState) {
    let key = path.to_string_lossy().to_string();
    if let Ok(mut guard) = state_cache().lock() {
        guard.insert(
            key,
            StateCacheEntry {
                modified_ms,
                byte_len,
                state: state.clone(),
            },
        );
    }
}

fn clear_cached_state(path: &Path) {
    let key = path.to_string_lossy().to_string();
    if let Ok(mut guard) = state_cache().lock() {
        guard.remove(&key);
    }
}

fn total_mailbox_message_count(state: &SwarmState) -> usize {
    state.mailboxes.values().fold(0usize, |acc, mailbox| {
        acc.saturating_add(mailbox.unread.len().saturating_add(mailbox.read.len()))
    })
}

fn should_pretty_encode_state(state: &SwarmState) -> bool {
    state.sessions.len() <= STATE_PRETTY_MAX_SESSIONS
        && total_mailbox_message_count(state) <= STATE_PRETTY_MAX_MAILBOX_MESSAGES
        && state.events.len() <= STATE_PRETTY_MAX_EVENT_ROWS
        && state.dead_letters.len() <= STATE_PRETTY_MAX_DEAD_LETTERS
}

fn load_state(path: &Path) -> Result<SwarmState, String> {
    if !path.exists() {
        clear_cached_state(path);
        return Ok(SwarmState::default());
    }
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        if let Some(state) = load_cached_state(path, modified_ms, byte_len) {
            return Ok(state);
        }
    }
    let raw = fs::read_to_string(path).map_err(|err| format!("state_read_failed:{err}"))?;
    if raw.trim().is_empty() {
        if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
            store_cached_state(path, modified_ms, byte_len, &SwarmState::default());
        }
        return Ok(SwarmState::default());
    }
    let parsed = serde_json::from_str::<SwarmState>(&raw)
        .map_err(|err| format!("state_parse_failed:{err}"))?;
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        store_cached_state(path, modified_ms, byte_len, &parsed);
    }
    Ok(parsed)
}

fn save_state(path: &Path, state: &SwarmState) -> Result<(), String> {
    ensure_parent(path)?;
    let encoded = if should_pretty_encode_state(state) {
        serde_json::to_string_pretty(state).map_err(|err| format!("state_encode_failed:{err}"))?
    } else {
        serde_json::to_string(state).map_err(|err| format!("state_encode_failed:{err}"))?
    };
    fs::write(path, encoded).map_err(|err| format!("state_write_failed:{err}"))?;
    if let Some((modified_ms, byte_len)) = state_file_fingerprint(path) {
        store_cached_state(path, modified_ms, byte_len, state);
    } else {
        clear_cached_state(path);
    }
    Ok(())
}

fn effective_spawn_max_depth(state: &SwarmState, requested_max_depth: u8) -> u8 {
    requested_max_depth
        .max(1)
        .min(state.scale_policy.max_depth_hard.max(1))
}

fn recommended_manager_fanout_for_target(target_agents: usize) -> usize {
    if target_agents >= 100_000 {
        32
    } else if target_agents >= 10_000 {
        24
    } else if target_agents >= 1_000 {
        12
    } else if target_agents >= 500 {
        10
    } else if target_agents >= 100 {
        8
    } else {
        5
    }
}

fn compute_hierarchy_topology(target_agents: usize, fanout: usize) -> Value {
    let target_agents = target_agents.max(1);
    let fanout = fanout.max(2);

    let mut remaining = target_agents;
    let mut level_capacity = 1usize;
    let mut level = 0usize;
    let mut level_counts: Vec<usize> = Vec::new();
    let mut levels = Vec::new();

    while remaining > 0 {
        let count = remaining.min(level_capacity);
        level_counts.push(count);
        levels.push(json!({
            "level": level,
            "agents": count,
            "capacity": level_capacity,
        }));
        remaining = remaining.saturating_sub(count);
        if remaining == 0 {
            break;
        }
        level = level.saturating_add(1);
        level_capacity = level_capacity.saturating_mul(fanout);
        if level > 512 {
            break;
        }
    }

    let mut managers_by_level = Vec::new();
    let mut manager_count = 0usize;
    for idx in 1..level_counts.len() {
        let children_at_level = level_counts[idx];
        let manager_agents = (children_at_level.saturating_add(fanout).saturating_sub(1)) / fanout;
        manager_count = manager_count.saturating_add(manager_agents);
        managers_by_level.push(json!({
            "level": idx - 1,
            "manager_agents": manager_agents,
        }));
    }
    let leaf_count = target_agents.saturating_sub(manager_count);
    let required_depth = level_counts.len().saturating_sub(1);

    json!({
        "target_agents": target_agents,
        "fanout": fanout,
        "required_depth": required_depth,
        "levels": levels,
        "managers_by_level": managers_by_level,
        "manager_count": manager_count,
        "leaf_count": leaf_count,
        "manager_ratio": if target_agents == 0 { 0.0 } else { manager_count as f64 / target_agents as f64 },
    })
}
