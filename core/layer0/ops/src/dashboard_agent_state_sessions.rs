// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const MAX_MESSAGES: usize = 4000;
const PROMPT_SUGGESTION_CONTEXT_WINDOW: usize = 7;
const PROMPT_SUGGESTION_MAX_WORDS: usize = 5;
const PROMPT_SUGGESTION_MAX_COUNT: usize = 3;

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

fn sanitize_suggestion(value: &str) -> String {
    let cleaned = clean_text(value, 160).replace('"', "").replace('\'', "");
    if cleaned.is_empty() {
        return String::new();
    }
    cleaned
        .split(' ')
        .filter(|row| !row.is_empty())
        .take(PROMPT_SUGGESTION_MAX_WORDS)
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_focus_stop_word(word: &str) -> bool {
    matches!(
        word,
        "a"
            | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "can"
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
            | "on"
            | "or"
            | "please"
            | "should"
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
        if user_rows.len() >= 5 {
            break;
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
    for row in user_rows {
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
    let n = 5usize.min(
        thread
            .iter()
            .rev()
            .filter(|(role, _)| role == "user")
            .count()
            .max(1),
    );
    SuggestionStyle {
        prefer_can_you: (can_you * 2) >= n,
        prefer_question_mark: (question * 2) >= n || can_you > 0,
        prefer_lowercase: (lowercase * 2) >= n,
    }
}

fn extract_thread_keywords(thread: &[(String, String)], limit: usize) -> Vec<String> {
    let stop = [
        "this",
        "that",
        "with",
        "from",
        "your",
        "have",
        "will",
        "into",
        "about",
        "after",
        "before",
        "where",
        "when",
        "which",
        "what",
        "please",
        "could",
        "would",
        "should",
        "there",
        "their",
        "them",
        "just",
        "also",
        "same",
        "thread",
        "message",
        "messages",
        "agent",
        "assistant",
        "system",
        "chat",
        "next",
        "step",
        "report",
        "runtime",
        "update",
    ];
    let stop_set = stop.into_iter().collect::<HashSet<_>>();
    let mut counts = HashMap::<String, usize>::new();
    for (_, text) in thread {
        let lowered = clean_text(text, 320).to_ascii_lowercase();
        for token in
            lowered.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
        {
            let word = token.trim();
            if word.len() < 4 || stop_set.contains(word) {
                continue;
            }
            *counts.entry(word.to_string()).or_insert(0) += 1;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(limit.max(1));
    ranked.into_iter().map(|(word, _)| word).collect()
}

fn apply_suggestion_style(style: &SuggestionStyle, body: &str) -> String {
    let mut text = clean_text(body, 240);
    if text.is_empty() {
        return String::new();
    }
    let lowered = text.to_ascii_lowercase();
    if style.prefer_can_you
        && !lowered.starts_with("can you")
        && !lowered.starts_with("could you")
        && !lowered.starts_with("would you")
    {
        text = format!("can you {text}");
    }
    text = clean_text(&text, 240);
    if style.prefer_lowercase {
        if let Some(first) = text.chars().next() {
            if first.is_ascii_uppercase() {
                let mut chars = text.chars();
                let _ = chars.next();
                text = format!(
                    "{}{}",
                    first.to_ascii_lowercase(),
                    chars.collect::<String>()
                );
            }
        }
    } else if let Some(first) = text.chars().next() {
        if first.is_ascii_lowercase() {
            let mut chars = text.chars();
            let _ = chars.next();
            text = format!(
                "{}{}",
                first.to_ascii_uppercase(),
                chars.collect::<String>()
            );
        }
    }
    if style.prefer_question_mark {
        if !text.ends_with('?') {
            text.push('?');
        }
    } else {
        text = text.trim_end_matches('?').trim().to_string();
    }
    sanitize_suggestion(&text)
}

pub fn append_turn(root: &Path, agent_id: &str, user_text: &str, assistant_text: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let mut state = load_session_state(root, &id);
    let active_id = clean_text(
        state
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("default"),
        120,
    );
    let message_count;
    {
        let sessions = as_array_mut(&mut state, "sessions");
        let mut target_idx = sessions
            .iter()
            .position(|row| {
                row.get("session_id")
                    .and_then(Value::as_str)
                    .map(|v| v == active_id)
                    .unwrap_or(false)
            })
            .unwrap_or(0);
        if sessions.is_empty() {
            sessions.push(json!({
                "session_id": "default",
                "label": "Session",
                "created_at": now_iso(),
                "updated_at": now_iso(),
                "messages": []
            }));
            target_idx = 0;
        }
        let session = &mut sessions[target_idx];
        if !session
            .get("messages")
            .map(Value::is_array)
            .unwrap_or(false)
        {
            session["messages"] = Value::Array(Vec::new());
        }
        let messages = session
            .get_mut("messages")
            .and_then(Value::as_array_mut)
            .expect("messages");
        let user = clean_text(user_text, 2000);
        let assistant = clean_text(assistant_text, 4000);
        if !user.is_empty() {
            messages.push(json!({"role": "user", "text": user, "ts": now_iso()}));
        }
        if !assistant.is_empty() {
            messages.push(json!({"role": "assistant", "text": assistant, "ts": now_iso()}));
        }
        if messages.len() > MAX_MESSAGES {
            let drain = messages.len() - MAX_MESSAGES;
            messages.drain(0..drain);
        }
        message_count = messages.len();
        session["updated_at"] = Value::String(now_iso());
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_turn_append", "agent_id": id, "message_count": message_count})
}

pub fn load_session(root: &Path, agent_id: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required"});
    }
    let state = load_session_state(root, &id);
    json!({"ok": true, "type": "dashboard_agent_session", "agent_id": id, "session": state})
}

pub fn suggestions(root: &Path, agent_id: &str, _user_hint: &str) -> Value {
    let id = normalize_agent_id(agent_id);
    if id.is_empty() {
        return json!({"ok": false, "error": "agent_id_required", "suggestions": []});
    }
    let state = load_session_state(root, &id);
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
                .map(|v| v == active_id)
                .unwrap_or(false)
        })
        .cloned()
        .unwrap_or_else(|| json!({"messages": []}));
    let messages = active
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let recent_thread = collect_recent_thread_context(&messages, PROMPT_SUGGESTION_CONTEXT_WINDOW);
    if recent_thread.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }

    let recent_user = recent_thread
        .iter()
        .filter(|(role, _)| role == "user")
        .map(|(_, text)| text.clone())
        .collect::<Vec<_>>();
    if recent_user.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }

    let style = derive_suggestion_style(&recent_thread);
    let keywords = extract_thread_keywords(&recent_thread, 6);
    let topic = compact_topic_phrase(&recent_thread, &keywords);
    if topic.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }
    let last_user = recent_thread
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, text)| sanitize_suggestion(text))
        .unwrap_or_default();

    let mut candidates = Vec::<String>::new();
    candidates.push(format!("finish {topic}"));
    candidates.push(format!("verify {topic}"));
    candidates.push(format!("test {topic}"));
    candidates.push(format!("continue {topic}"));
    if keywords.len() >= 2 {
        candidates.push(format!("compare {} {}", keywords[0], keywords[1]));
    }
    if !last_user.is_empty() {
        candidates.push(format!("finish {last_user}"));
    }

    let recent_set = recent_user
        .iter()
        .map(|row| sanitize_suggestion(row).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut out = Vec::<String>::new();
    for raw in candidates {
        let row = apply_suggestion_style(&style, &raw);
        if row.is_empty() {
            continue;
        }
        let row_lc = row.to_ascii_lowercase();
        if recent_set.contains(&row_lc) {
            continue;
        }
        if out.iter().any(|existing| is_too_similar(existing, &row)) {
            continue;
        }
        out.push(row);
        if out.len() >= PROMPT_SUGGESTION_MAX_COUNT {
            break;
        }
    }

    json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": out})
}

pub fn session_summaries(root: &Path, limit: usize) -> Value {
    let mut rows = Vec::<Value>::new();
    let dir = sessions_dir(root);
    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            if let Some(state) = read_json_file(&path) {
                let agent_id = clean_text(
                    state.get("agent_id").and_then(Value::as_str).unwrap_or(""),
                    140,
                );
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

    #[test]
    fn suggestions_are_deduped_and_never_quoted() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = append_turn(
            root.path(),
            "agent-a",
            "Can you reduce queue depth before spikes?",
            "On it.",
        );
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
        let _ = append_turn(
            root.path(),
            "agent-c",
            "fix chat scroll bounce jitter",
            "I will patch bottom lock logic.",
        );
        let value = suggestions(
            root.path(),
            "agent-c",
            "run system diagnostic full scan",
        );
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
        assert!(joined.contains("scroll") || joined.contains("bounce") || joined.contains("jitter"));
        assert!(!joined.contains("diagnostic"));
        assert!(!joined.contains("scan"));
    }
}
