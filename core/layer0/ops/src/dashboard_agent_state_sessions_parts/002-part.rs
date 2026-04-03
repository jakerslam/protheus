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
            text = text
                .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | ';' | ':'))
                .trim()
                .to_string();
            if !text.is_empty() {
                text.push('?');
            }
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
        let user = clean_chat_text(user_text, 2000);
        let assistant = clean_chat_text(assistant_text, 4000);
        if has_non_whitespace(&user) {
            messages.push(json!({"role": "user", "text": user, "ts": now_iso()}));
        }
        if has_non_whitespace(&assistant) {
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
    if role_key.contains("analyst")
        || role_key.contains("analysis")
        || role_key.contains("data")
    {
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

pub fn seed_intro_message(root: &Path, agent_id: &str, role: &str, display_name: &str) -> Value {
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
    let mut appended = false;
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
        let has_meaningful_content = messages.iter().any(|row| {
            let row_role = normalize_message_role(row);
            if row_role == "system" {
                return false;
            }
            !text_from_message(row).is_empty()
        });
        if !has_meaningful_content {
            let intro = intro_text_for_role(role, display_name);
            if !intro.is_empty() {
                messages.push(
                    json!({"role": "assistant", "text": intro, "ts": now_iso(), "intro": true}),
                );
                appended = true;
            }
        }
        if messages.len() > MAX_MESSAGES {
            let drain = messages.len() - MAX_MESSAGES;
            messages.drain(0..drain);
        }
        message_count = messages.len();
        session["updated_at"] = Value::String(now_iso());
    }
    if appended {
        save_session_state(root, &id, &state);
    }
    json!({
        "ok": true,
        "type": "dashboard_agent_intro_seed",
        "agent_id": id,
        "appended": appended,
        "message_count": message_count
    })
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
    if recent_thread.len() < PROMPT_SUGGESTION_CONTEXT_WINDOW {
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

    let base_style = derive_suggestion_style(&recent_thread);
    let style = SuggestionStyle {
        prefer_can_you: true,
        prefer_question_mark: true,
        prefer_lowercase: base_style.prefer_lowercase,
    };
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
    candidates.push(format!("continue with {topic}"));
    candidates.push(format!("verify {topic} works"));
    candidates.push(format!("test {topic} end to end"));
    candidates.push(format!("finish {topic}"));
    if keywords.len() >= 2 {
        candidates.push(format!("compare {} and {}", keywords[0], keywords[1]));
    }
    if !last_user.is_empty() {
        candidates.push(format!("continue with {last_user}"));
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

