fn now_iso() -> String {
    crate::now_iso()
}

fn clean_text(value: &str, max_len: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn clean_chat_text(value: &str, max_len: usize) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .take(max_len)
        .collect::<String>()
}

fn has_non_whitespace(value: &str) -> bool {
    value.chars().any(|ch| !ch.is_whitespace())
}

fn normalize_agent_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in clean_text(raw, 140).chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
    }
    out
}

fn parse_json_loose(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    for line in text.lines().rev() {
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

fn read_json_file(path: &Path) -> Option<Value> {
    let body = fs::read_to_string(path).ok()?;
    parse_json_loose(&body)
}

fn ensure_dir(path: &Path) {
    let _ = fs::create_dir_all(path);
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        ensure_dir(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{body}\n"));
    }
}

fn as_array_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    if !root.get(key).map(Value::is_array).unwrap_or(false) {
        root[key] = Value::Array(Vec::new());
    }
    root.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array shape")
}

fn as_object_mut<'a>(root: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    if !root.get(key).map(Value::is_object).unwrap_or(false) {
        root[key] = json!({});
    }
    root.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("object shape")
}

fn sessions_dir(root: &Path) -> PathBuf {
    root.join(AGENT_SESSIONS_DIR_REL)
}

fn session_path(root: &Path, agent_id: &str) -> PathBuf {
    sessions_dir(root).join(format!("{}.json", normalize_agent_id(agent_id)))
}

fn default_session_state(agent_id: &str) -> Value {
    let now = now_iso();
    json!({
        "type": "infring_dashboard_agent_session",
        "agent_id": agent_id,
        "active_session_id": "default",
        "sessions": [
            {
                "session_id": "default",
                "label": "Session",
                "created_at": now,
                "updated_at": now,
                "messages": []
            }
        ],
        "memory_kv": {}
    })
}

fn load_session_state(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    let mut state =
        read_json_file(&session_path(root, &id)).unwrap_or_else(|| default_session_state(&id));
    if !state.is_object() {
        state = default_session_state(&id);
    }
    state["agent_id"] = Value::String(id.clone());
    if !state
        .get("active_session_id")
        .and_then(Value::as_str)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        state["active_session_id"] = Value::String("default".to_string());
    }
    let sessions = as_array_mut(&mut state, "sessions");
    if sessions.is_empty() {
        sessions.push(json!({
            "session_id": "default",
            "label": "Session",
            "created_at": now_iso(),
            "updated_at": now_iso(),
            "messages": []
        }));
    }
    let _ = as_object_mut(&mut state, "memory_kv");
    state
}

fn save_session_state(root: &Path, agent_id: &str, state: &Value) {
    let id = normalize_agent_id(agent_id);
    ensure_dir(&sessions_dir(root));
    write_json(&session_path(root, &id), state);
}

fn text_from_message(row: &Value) -> String {
    if let Some(text) = row.get("text").and_then(Value::as_str) {
        return clean_text(text, 400);
    }
    if let Some(text) = row.get("content").and_then(Value::as_str) {
        return clean_text(text, 400);
    }
    if let Some(text) = row.as_str() {
        return clean_text(text, 400);
    }
    String::new()
}

fn token_set(value: &str) -> HashSet<String> {
    clean_text(value, 300)
        .to_ascii_lowercase()
        .split(' ')
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect::<HashSet<_>>()
}

fn is_too_similar(left: &str, right: &str) -> bool {
    let a = token_set(left);
    let b = token_set(right);
    if a.is_empty() || b.is_empty() {
        return clean_text(left, 240).eq_ignore_ascii_case(&clean_text(right, 240));
    }
    let overlap = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union <= 0.0 {
        return false;
    }
    (overlap / union) >= 0.8
}

fn is_trailing_query_filler(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "and"
            | "as"
            | "at"
            | "for"
            | "from"
            | "in"
            | "into"
            | "of"
            | "on"
            | "or"
            | "the"
            | "then"
            | "to"
            | "via"
            | "with"
    )
}

fn normalize_suggestion_voice(value: &str) -> String {
    let mut normalized = clean_text(value, 220)
        .trim_end_matches('?')
        .trim()
        .to_string();
    if normalized.is_empty() {
        return String::new();
    }
    let lowered = normalized.to_ascii_lowercase();
    let prefixes = [
        "should i ",
        "want me to ",
        "do you want me to ",
        "would you like me to ",
        "can you ",
        "could you ",
    ];
    for prefix in prefixes {
        if lowered.starts_with(prefix) {
            normalized = normalized.chars().skip(prefix.len()).collect::<String>();
            break;
        }
    }
    clean_text(&normalized, 220)
}

fn sanitize_suggestion(value: &str) -> String {
    let cleaned = normalize_suggestion_voice(value)
        .replace('"', "")
        .replace('\'', "");
    if cleaned.is_empty() {
        return String::new();
    }
    let mut words = cleaned
        .split(' ')
        .filter(|row| !row.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if words.len() > PROMPT_SUGGESTION_MAX_WORDS {
        words.truncate(PROMPT_SUGGESTION_MAX_WORDS);
    }
    while words.len() > 1 {
        let last = words
            .last()
            .map(|row| {
                row.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                    .to_ascii_lowercase()
            })
            .unwrap_or_default();
        if last.is_empty() || is_trailing_query_filler(&last) {
            let _ = words.pop();
            continue;
        }
        break;
    }
    words.join(" ")
}

fn is_focus_stop_word(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "after"
            | "all"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "before"
            | "can"
            | "confirm"
            | "confirmed"
            | "could"
            | "do"
            | "for"
            | "from"
            | "how"
            | "i"
            | "in"
            | "into"
            | "is"
            | "it"
            | "me"
            | "my"
            | "now"
            | "of"
            | "ok"
            | "okay"
            | "on"
            | "or"
            | "please"
            | "should"
            | "sure"
            | "that"
            | "the"
            | "then"
            | "this"
            | "to"
            | "we"
            | "what"
            | "when"
            | "where"
            | "why"
            | "with"
            | "would"
            | "yeah"
            | "yep"
            | "yes"
            | "you"
            | "your"
    )
}

fn extract_focus_tokens(value: &str, max_tokens: usize) -> Vec<String> {
    let cap = max_tokens.clamp(1, PROMPT_SUGGESTION_MAX_WORDS);
    let mut out = Vec::<String>::new();
    for raw in clean_text(value, 320)
        .to_ascii_lowercase()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
    {
        let token = raw.trim();
        if token.len() < 3 || is_focus_stop_word(token) {
            continue;
        }
        out.push(token.to_string());
        if out.len() >= cap {
            break;
        }
    }
    out
}

fn compact_topic_phrase(thread: &[(String, String)], keywords: &[String]) -> String {
    for (role, text) in thread.iter().rev() {
        if role != "user" {
            continue;
        }
        let tokens = extract_focus_tokens(text, 3);
        if !tokens.is_empty() {
            return tokens.join(" ");
        }
    }
    if !keywords.is_empty() {
        return keywords
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
    }
    for (_, text) in thread.iter().rev() {
        let tokens = extract_focus_tokens(text, 3);
        if !tokens.is_empty() {
            return tokens.join(" ");
        }
    }
    String::new()
}

fn normalize_message_role(row: &Value) -> String {
    let raw = clean_text(
        row.get("role")
            .or_else(|| row.get("type"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        32,
    )
    .to_ascii_lowercase();
    if raw.contains("user") {
        return "user".to_string();
    }
    if raw.contains("agent") || raw.contains("assistant") {
        return "assistant".to_string();
    }
    if raw.contains("system") {
        return "system".to_string();
    }
    "assistant".to_string()
}

fn is_suggestion_noise(text: &str) -> bool {
    let lowered = clean_text(text, 320).to_ascii_lowercase();
    lowered.is_empty()
        || lowered == "heartbeat_ok"
        || lowered.starts_with("[runtime-task]")
        || lowered
            .contains("task accepted. report findings in this thread with receipt-backed evidence")
        || lowered.contains("the user wants exactly 3 actionable next user prompts")
}

fn collect_recent_thread_context(messages: &[Value], limit: usize) -> Vec<(String, String)> {
    let mut out = Vec::<(String, String)>::new();
    for row in messages.iter().rev() {
        let role = normalize_message_role(row);
        if role == "system" {
            continue;
        }
        let text = text_from_message(row);
        if is_suggestion_noise(&text) {
            continue;
        }
        out.push((role, text));
        if out.len() >= limit {
            break;
        }
    }
    out.reverse();
    out
}

#[derive(Default)]
struct SuggestionStyle {
    prefer_can_you: bool,
    prefer_question_mark: bool,
    prefer_lowercase: bool,
}

fn derive_suggestion_style(thread: &[(String, String)]) -> SuggestionStyle {
    let mut user_rows = Vec::<String>::new();
    for (role, text) in thread.iter().rev() {
        if role == "user" {
            user_rows.push(clean_text(text, 240));
        }
    }
    if user_rows.is_empty() {
        return SuggestionStyle {
            prefer_can_you: true,
            prefer_question_mark: true,
            prefer_lowercase: true,
        };
    }
    let mut can_you = 0usize;
    let mut question = 0usize;
    let mut lowercase = 0usize;
    for row in user_rows.iter() {
        let lowered = row.to_ascii_lowercase();
        if lowered.starts_with("can you")
            || lowered.starts_with("could you")
            || lowered.starts_with("would you")
        {
            can_you += 1;
        }
        if lowered.ends_with('?') {
            question += 1;
        }
        if row
            .chars()
            .next()
            .map(|ch| ch.is_ascii_lowercase())
            .unwrap_or(false)
        {
            lowercase += 1;
        }
    }
    let n = user_rows.len().max(1);
    SuggestionStyle {
        prefer_can_you: (can_you * 2) >= n,
        prefer_question_mark: (question * 2) >= n || can_you > 0,
        prefer_lowercase: (lowercase * 2) >= n,
    }
}
