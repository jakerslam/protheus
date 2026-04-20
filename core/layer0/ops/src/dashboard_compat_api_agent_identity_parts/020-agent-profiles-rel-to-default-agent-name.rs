
const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const ARCHIVED_AGENTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";

const DEFAULT_AGENT_EMOJI: &str = "∞";
const DEFAULT_SYSTEM_EMOJI: &str = "⚙️";

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn clean_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(raw: &str) -> Option<Value> {
    if raw.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        return Some(value);
    }
    for line in raw.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Some(value);
        }
    }
    None
}

fn read_json_loose(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| parse_json_loose(&raw))
}

fn state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn profiles_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, AGENT_PROFILES_REL))
        .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn archived_map(root: &Path) -> Map<String, Value> {
    read_json_loose(&state_path(root, ARCHIVED_AGENTS_REL))
        .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
        .unwrap_or_default()
}

fn normalized_name_key(raw: &str) -> String {
    clean_text(raw, 120).to_ascii_lowercase()
}

fn normalized_emoji_key(raw: &str) -> String {
    clean_text(raw, 24)
        .replace('\u{FE0F}', "")
        .replace('\u{FE0E}', "")
}

fn is_reserved_system_emoji_key(raw: &str) -> bool {
    let normalized = normalized_emoji_key(raw);
    normalized == "⚙"
}

fn identity_emoji(row: &Value) -> String {
    let from_identity = row
        .pointer("/identity/emoji")
        .and_then(Value::as_str)
        .unwrap_or("");
    let from_flat = row.get("emoji").and_then(Value::as_str).unwrap_or("");
    let emoji = if from_identity.is_empty() {
        from_flat
    } else {
        from_identity
    };
    normalized_emoji_key(emoji)
}

fn profile_name_or_humanized(agent_id: &str, row: &Value) -> String {
    let profile_name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120);
    if !profile_name.is_empty() {
        return profile_name;
    }
    humanize_agent_name(agent_id)
}

fn collect_reserved_name_and_emoji_keys(root: &Path) -> (HashSet<String>, HashSet<String>) {
    let profiles = profiles_map(root);
    let archived = archived_map(root);
    let mut used_names = HashSet::<String>::new();
    let mut used_emojis = HashSet::<String>::new();

    for (agent_id, row) in &profiles {
        let name_key = normalized_name_key(&profile_name_or_humanized(agent_id, row));
        if !name_key.is_empty() {
            used_names.insert(name_key);
        }
        let emoji_key = identity_emoji(row);
        if !emoji_key.is_empty() {
            used_emojis.insert(emoji_key);
        }
    }

    for (agent_id, row) in &archived {
        let archived_name = clean_text(row.get("name").and_then(Value::as_str).unwrap_or(""), 120);
        let fallback_profile = profiles.get(agent_id).cloned().unwrap_or_else(|| json!({}));
        let fallback_name = profile_name_or_humanized(agent_id, &fallback_profile);
        let name_key = normalized_name_key(if archived_name.is_empty() {
            &fallback_name
        } else {
            &archived_name
        });
        if !name_key.is_empty() {
            used_names.insert(name_key);
        }
        let archived_emoji = normalized_emoji_key(
            row.pointer("/identity/emoji")
                .and_then(Value::as_str)
                .or_else(|| row.get("emoji").and_then(Value::as_str))
                .unwrap_or(""),
        );
        let emoji_key = if archived_emoji.is_empty() {
            identity_emoji(&fallback_profile)
        } else {
            archived_emoji
        };
        if !emoji_key.is_empty() {
            used_emojis.insert(emoji_key);
        }
    }
    (used_names, used_emojis)
}

fn title_case(raw: &str) -> String {
    let mut out = Vec::<String>::new();
    for word in clean_text(raw, 120).split_whitespace() {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            let mut built = String::new();
            built.push(first.to_ascii_uppercase());
            built.push_str(chars.as_str());
            out.push(built);
        }
    }
    out.join(" ")
}

fn humanize_agent_name(agent_id: &str) -> String {
    let cleaned = clean_agent_id(agent_id).replace('-', " ").replace('_', " ");
    let out = title_case(&cleaned);
    if out.is_empty() {
        "Agent".to_string()
    } else {
        out
    }
}

fn canonical_agent_id(raw: &str) -> String {
    let mut canonical = clean_agent_id(raw)
        .to_ascii_lowercase()
        .replace('_', "-")
        .trim_matches('-')
        .to_string();
    while canonical.contains("--") {
        canonical = canonical.replace("--", "-");
    }
    canonical
}

pub fn default_agent_name(agent_id: &str) -> String {
    let canonical = canonical_agent_id(agent_id);
    if canonical.is_empty() {
        return "agent".to_string();
    }
    if canonical == "agent" {
        return "agent".to_string();
    }
    if let Some(rest) = canonical.strip_prefix("agent-") {
        if rest.is_empty() {
            "agent".to_string()
        } else {
            format!("agent-{rest}")
        }
    } else {
        format!("agent-{canonical}")
    }
}
