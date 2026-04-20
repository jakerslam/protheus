
pub fn is_default_agent_name_for_agent(name: &str, agent_id: &str) -> bool {
    let normalized_name = clean_text(name, 120).to_ascii_lowercase();
    if normalized_name.is_empty() {
        return true;
    }
    let cleaned_id = clean_agent_id(agent_id);
    let default_name = default_agent_name(agent_id);
    let candidate_keys = [
        normalized_name_key(&cleaned_id),
        normalized_name_key(&cleaned_id.replace('_', "-")),
        normalized_name_key(&cleaned_id.replace('-', "_")),
        normalized_name_key(&default_name),
    ];
    candidate_keys
        .iter()
        .any(|candidate| !candidate.is_empty() && &normalized_name == candidate)
}

fn role_name_stem(role: &str) -> Vec<&'static str> {
    let role_key = clean_text(role, 80).to_ascii_lowercase();
    if role_key.contains("planner")
        || role_key.contains("plan")
        || role_key.contains("strategy")
        || role_key.contains("roadmap")
    {
        return vec![
            "Plan", "Route", "Compass", "Vector", "Blueprint", "Navigator",
        ];
    }
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
    if role_key.contains("support") || role_key.contains("helpdesk") {
        return vec!["Guide", "Harbor", "Assist", "Relay", "Beacon", "Support"];
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
    let cleaned_id = canonical_agent_id(agent_id);
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
