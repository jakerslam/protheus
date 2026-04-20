
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
        && !lowered.starts_with("what ")
        && !lowered.starts_with("which ")
        && !lowered.starts_with("why ")
        && !lowered.starts_with("how ")
        && !lowered.starts_with("is ")
        && !lowered.starts_with("are ")
        && !lowered.starts_with("should ")
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
    let _ = style.prefer_question_mark;
    text = text
        .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | ';' | ':'))
        .trim()
        .to_string();
    text = strip_trailing_suggestion_question_marks(&text);
    let mut out = sanitize_suggestion(&text);
    if out.is_empty() {
        return out;
    }
    out = out
        .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | ';' | ':'))
        .trim()
        .to_string();
    strip_trailing_suggestion_question_marks(&out)
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
        // Preserve durable chat history with higher per-message ceilings so
        // long threads don't silently lose semantic detail.
        let user = clean_chat_text(user_text, 16_000);
        let assistant = clean_chat_text(assistant_text, 64_000);
        if has_non_whitespace(&user) {
            messages.push(json!({"role": "user", "text": user, "ts": now_iso()}));
        }
        if has_non_whitespace(&assistant) {
            messages.push(json!({"role": "assistant", "text": assistant, "ts": now_iso()}));
        }
        message_count = messages.len();
        session["updated_at"] = Value::String(now_iso());
    }
    save_session_state(root, &id, &state);
    json!({"ok": true, "type": "dashboard_agent_turn_append", "agent_id": id, "message_count": message_count})
}

fn intro_text_for_role(role: &str, display_name: &str) -> String {
    let role_key = clean_text(role, 80).to_ascii_lowercase();
    let mut name = clean_text(display_name, 120);
    if name.is_empty() {
        name = "your agent".to_string();
    }
    if role_key.contains("teacher")
        || role_key.contains("tutor")
        || role_key.contains("mentor")
        || role_key.contains("coach")
        || role_key.contains("instructor")
    {
        return format!("Hi, I'm {name}. What do you want to learn today?");
    }
    if role_key.contains("code")
        || role_key.contains("coder")
        || role_key.contains("engineer")
        || role_key.contains("developer")
        || role_key.contains("devops")
    {
        return format!("Hi, I'm {name}. What are we coding today?");
    }
    if role_key.contains("research") || role_key.contains("investig") {
        return format!("Hi, I'm {name}. What should we research first?");
    }
    if role_key.contains("analyst") || role_key.contains("analysis") || role_key.contains("data") {
        return format!("Hi, I'm {name}. What should we analyze first?");
    }
    if role_key.contains("writer") || role_key.contains("editor") || role_key.contains("content") {
        return format!("Hi, I'm {name}. What are we writing today?");
    }
    if role_key.contains("design") || role_key.contains("ui") || role_key.contains("ux") {
        return format!("Hi, I'm {name}. What should we design first?");
    }
    format!("Hi, I'm {name}. What are we working on today?")
}
