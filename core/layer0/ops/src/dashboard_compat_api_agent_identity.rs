// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const ARCHIVED_AGENTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";

const HUMAN_NAME_POOL: [&str; 26] = [
    "Avery", "Kai", "Maya", "Noah", "Iris", "Leo", "Nora", "Theo", "Aria", "Milo", "Sage",
    "Luna", "Jules", "Nia", "Ezra", "Zara", "Rhea", "Owen", "Dylan", "Skye", "Ruby", "Hugo",
    "Mira", "Finn", "Elio", "Wren",
];

const ROLE_EMOJI_ANALYST: [&str; 10] =
    ["🔎", "📊", "🧠", "📈", "🧭", "🧪", "🧩", "🛰️", "📡", "📝"];
const ROLE_EMOJI_ENGINEER: [&str; 10] =
    ["🛠️", "💻", "⚙️", "🧰", "🔧", "🧪", "📐", "🧬", "🛰️", "🔌"];
const ROLE_EMOJI_RUNTIME: [&str; 10] =
    ["⚙️", "🛰️", "📡", "🔧", "🛡️", "🧯", "🧰", "📊", "🧭", "🧠"];
const ROLE_EMOJI_WRITER: [&str; 8] = ["✍️", "📝", "📚", "🧾", "🗂️", "🧠", "📖", "🧩"];
const ROLE_EMOJI_GENERIC: [&str; 44] = [
    "🤖", "🧑‍💻", "🧠", "🧭", "🛰️", "⚡", "🔮", "🧪", "🛡️", "📡", "📈", "📊", "🧩", "🛠️",
    "🔧", "🧰", "📐", "🗺️", "🗂️", "📎", "📦", "📌", "🧱", "🧿", "🌐", "🕹️", "🎛️", "🎯",
    "🪐", "🌟", "✨", "🔥", "🌀", "🪄", "🧲", "🧬", "🔬", "🔭", "📘", "📙", "📗", "📓",
    "📒", "🗃️",
];

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

fn role_title(role: &str) -> String {
    let normalized = clean_text(role, 80);
    if normalized.is_empty() {
        return "Analyst".to_string();
    }
    let built = title_case(&normalized.replace('_', " ").replace('-', " "));
    if built.is_empty() {
        "Analyst".to_string()
    } else {
        built
    }
}

fn role_emoji_candidates(role: &str) -> Vec<&'static str> {
    let lowered = clean_text(role, 80).to_ascii_lowercase();
    let mut rows = Vec::<&'static str>::new();
    if lowered.contains("analyst") || lowered.contains("research") {
        rows.extend(ROLE_EMOJI_ANALYST);
    }
    if lowered.contains("engineer")
        || lowered.contains("coder")
        || lowered.contains("dev")
        || lowered.contains("builder")
    {
        rows.extend(ROLE_EMOJI_ENGINEER);
    }
    if lowered.contains("runtime")
        || lowered.contains("ops")
        || lowered.contains("reliability")
        || lowered.contains("sre")
    {
        rows.extend(ROLE_EMOJI_RUNTIME);
    }
    if lowered.contains("writer")
        || lowered.contains("editor")
        || lowered.contains("content")
        || lowered.contains("copy")
    {
        rows.extend(ROLE_EMOJI_WRITER);
    }
    rows.extend(ROLE_EMOJI_GENERIC);
    let mut seen = HashSet::<&'static str>::new();
    rows.into_iter().filter(|row| seen.insert(*row)).collect()
}

pub fn resolve_agent_name(root: &Path, requested_name: &str, role: &str) -> String {
    let manual = clean_text(requested_name, 120);
    if !manual.is_empty() {
        return manual;
    }
    let (mut used_names, _) = collect_reserved_name_and_emoji_keys(root);
    let role_name = role_title(role);
    let seed = crate::deterministic_receipt_hash(
        &json!({"role": role_name, "ts": crate::now_iso(), "kind": "agent_name"}),
    );
    let parity = seed.as_bytes().first().copied().unwrap_or_default() % 2;
    let human_offset = seed.as_bytes().get(1).copied().unwrap_or_default() as usize;

    let mut candidates = Vec::<String>::new();
    if parity == 0 {
        candidates.push(role_name.clone());
    }
    for idx in 0..HUMAN_NAME_POOL.len() {
        candidates.push(HUMAN_NAME_POOL[(human_offset + idx) % HUMAN_NAME_POOL.len()].to_string());
    }
    if parity != 0 {
        candidates.push(role_name.clone());
    }
    candidates.push(format!("{role_name} Agent"));
    for base in candidates {
        let key = normalized_name_key(&base);
        if key.is_empty() {
            continue;
        }
        if used_names.insert(key) {
            return base;
        }
    }
    for idx in 2..5000 {
        let candidate = format!("{role_name} {idx}");
        let key = normalized_name_key(&candidate);
        if !key.is_empty() && used_names.insert(key) {
            return candidate;
        }
    }
    role_name
}

pub fn resolve_agent_identity(root: &Path, request: &Value, role: &str) -> Value {
    let mut identity_map = request
        .get("identity")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let explicit_emoji = normalized_emoji_key(
        request
            .pointer("/identity/emoji")
            .and_then(Value::as_str)
            .or_else(|| request.get("emoji").and_then(Value::as_str))
            .unwrap_or(""),
    );
    let (_, mut used_emojis) = collect_reserved_name_and_emoji_keys(root);
    let emoji = if !explicit_emoji.is_empty() {
        explicit_emoji
    } else {
        let mut chosen = String::new();
        for candidate in role_emoji_candidates(role) {
            let key = normalized_emoji_key(candidate);
            if key.is_empty() || !used_emojis.insert(key.clone()) {
                continue;
            }
            chosen = key;
            break;
        }
        if chosen.is_empty() {
            let hash = crate::deterministic_receipt_hash(
                &json!({"role": role, "ts": crate::now_iso(), "kind": "agent_emoji"}),
            );
            let idx = (hash.as_bytes().first().copied().unwrap_or_default() as usize) % 10;
            format!("🤖{idx}")
        } else {
            chosen
        }
    };
    let color = clean_text(
        identity_map
            .get("color")
            .and_then(Value::as_str)
            .or_else(|| request.get("color").and_then(Value::as_str))
            .unwrap_or("#2563EB"),
        24,
    );
    let archetype = clean_text(
        identity_map
            .get("archetype")
            .and_then(Value::as_str)
            .or_else(|| request.get("archetype").and_then(Value::as_str))
            .unwrap_or(role),
        80,
    );
    let vibe = clean_text(
        identity_map
            .get("vibe")
            .and_then(Value::as_str)
            .or_else(|| request.get("vibe").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    );
    identity_map.insert("emoji".to_string(), Value::String(emoji));
    identity_map.insert(
        "color".to_string(),
        Value::String(if color.is_empty() {
            "#2563EB".to_string()
        } else {
            color
        }),
    );
    identity_map.insert(
        "archetype".to_string(),
        Value::String(if archetype.is_empty() {
            "assistant".to_string()
        } else {
            archetype
        }),
    );
    identity_map.insert("vibe".to_string(), Value::String(vibe));
    Value::Object(identity_map)
}

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
    fn auto_name_avoids_active_and_archived_collisions() {
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
        let name = resolve_agent_name(tmp.path(), "", "analyst");
        let key = normalized_name_key(&name);
        assert_ne!(key, "analyst");
        assert_ne!(key, "kai");
        assert_ne!(key, "avery");
    }

    #[test]
    fn auto_emoji_avoids_collisions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_json(
            &tmp.path().join(AGENT_PROFILES_REL),
            &json!({
                "type": "infring_dashboard_agent_profiles",
                "agents": {
                    "agent-a": {"name": "Nora", "identity": {"emoji": "🔎"}},
                    "agent-b": {"name": "Theo", "identity": {"emoji": "📊"}}
                }
            }),
        );
        let identity = resolve_agent_identity(tmp.path(), &json!({}), "analyst");
        let emoji = identity
            .get("emoji")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!emoji.is_empty());
        assert_ne!(emoji, "🔎");
        assert_ne!(emoji, "📊");
    }
}
