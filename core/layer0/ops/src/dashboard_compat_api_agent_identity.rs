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

pub fn default_agent_name(agent_id: &str) -> String {
    let cleaned = clean_agent_id(agent_id);
    if cleaned.is_empty() {
        return "agent".to_string();
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.starts_with("agent-") || lowered.starts_with("agent_") || lowered == "agent" {
        cleaned
    } else {
        format!("agent-{cleaned}")
    }
}

pub fn is_default_agent_name_for_agent(name: &str, agent_id: &str) -> bool {
    let normalized_name = clean_text(name, 120).to_ascii_lowercase();
    if normalized_name.is_empty() {
        return true;
    }
    let cleaned_id = clean_agent_id(agent_id).to_ascii_lowercase();
    let default_name = default_agent_name(agent_id).to_ascii_lowercase();
    normalized_name == cleaned_id || normalized_name == default_name
}

fn role_name_stem(role: &str) -> Vec<&'static str> {
    let role_key = clean_text(role, 80).to_ascii_lowercase();
    if role_key.contains("code")
        || role_key.contains("coder")
        || role_key.contains("engineer")
        || role_key.contains("developer")
    {
        return vec!["Kernel", "Patch", "Vector", "Stack", "Circuit", "Byte"];
    }
    if role_key.contains("devops") || role_key.contains("infra") || role_key.contains("sre") {
        return vec!["Forge", "Pipeline", "Harbor", "Sentry", "Cluster", "Atlas"];
    }
    if role_key.contains("research")
        || role_key.contains("analyst")
        || role_key.contains("investig")
    {
        return vec!["Insight", "Signal", "Prism", "Lens", "Probe", "Delta"];
    }
    if role_key.contains("writer") || role_key.contains("editor") || role_key.contains("content") {
        return vec!["Quill", "Draft", "Verse", "Script", "Narrative", "Ink"];
    }
    if role_key.contains("teacher")
        || role_key.contains("tutor")
        || role_key.contains("mentor")
        || role_key.contains("coach")
        || role_key.contains("instructor")
    {
        return vec!["Mentor", "Guide", "Beacon", "Scholar", "Tutor", "Coach"];
    }
    vec!["Nova", "Pulse", "Axis", "Echo", "Comet", "Astra"]
}

pub fn resolve_post_init_agent_name(root: &Path, agent_id: &str, role: &str) -> String {
    let (mut used_names, _) = collect_reserved_name_and_emoji_keys(root);
    let cleaned_id = clean_agent_id(agent_id);
    let default_name = default_agent_name(&cleaned_id);
    let role_key = clean_text(role, 80).to_ascii_lowercase();
    let stems = role_name_stem(&role_key);
    let tails = [
        "Arc", "Prime", "Flow", "Node", "Spark", "Pilot", "Shift", "Works", "Lab", "Core",
    ];
    let seed_hex = crate::deterministic_receipt_hash(&json!({
        "agent_id": cleaned_id,
        "role": role_key
    }));
    let mut seed_a = 0usize;
    let mut seed_b = 0usize;
    if seed_hex.len() >= 16 {
        seed_a = usize::from_str_radix(&seed_hex[0..8], 16).unwrap_or(0);
        seed_b = usize::from_str_radix(&seed_hex[8..16], 16).unwrap_or(0);
    }
    for attempt in 0..96usize {
        let stem = stems[(seed_a + attempt) % stems.len()];
        let tail = tails[(seed_b + attempt.saturating_mul(3)) % tails.len()];
        let candidate = format!("{stem} {tail}");
        let key = normalized_name_key(&candidate);
        if key.is_empty() {
            continue;
        }
        if key == normalized_name_key(&default_name) {
            continue;
        }
        if used_names.insert(key) {
            return candidate;
        }
    }
    let short_id = cleaned_id
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let fallback_role = title_case(&role_key);
    let fallback = if fallback_role.is_empty() {
        if short_id.is_empty() {
            "Agent Prime".to_string()
        } else {
            format!("Agent {short_id}")
        }
    } else if short_id.is_empty() {
        fallback_role
    } else {
        format!("{fallback_role} {short_id}")
    };
    if fallback.eq_ignore_ascii_case(&default_name) || clean_text(&fallback, 120).is_empty() {
        return humanize_agent_name(&cleaned_id);
    }
    fallback
}

pub fn resolve_agent_name(root: &Path, requested_name: &str, _role: &str) -> String {
    let (mut used_names, _) = collect_reserved_name_and_emoji_keys(root);
    let manual = clean_text(requested_name, 120);
    if manual.is_empty() {
        return String::new();
    }
    let manual_key = normalized_name_key(&manual);
    if !manual_key.is_empty() && used_names.insert(manual_key) {
        return manual;
    }
    for idx in 2..5000 {
        let candidate = format!("{manual}{idx}");
        let key = normalized_name_key(&candidate);
        if !key.is_empty() && used_names.insert(key) {
            return candidate;
        }
    }
    manual
}

pub fn resolve_agent_identity(_root: &Path, request: &Value, role: &str) -> Value {
    let mut identity_map = request
        .get("identity")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let allow_reserved_system_emoji = request
        .get("is_system_thread")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || clean_text(role, 80).eq_ignore_ascii_case("system");
    let mut explicit_emoji = normalized_emoji_key(
        request
            .pointer("/identity/emoji")
            .and_then(Value::as_str)
            .or_else(|| request.get("emoji").and_then(Value::as_str))
            .unwrap_or(""),
    );
    if !allow_reserved_system_emoji && is_reserved_system_emoji_key(&explicit_emoji) {
        explicit_emoji.clear();
    }
    let emoji = if !explicit_emoji.is_empty() {
        explicit_emoji
    } else if allow_reserved_system_emoji {
        DEFAULT_SYSTEM_EMOJI.to_string()
    } else {
        DEFAULT_AGENT_EMOJI.to_string()
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
