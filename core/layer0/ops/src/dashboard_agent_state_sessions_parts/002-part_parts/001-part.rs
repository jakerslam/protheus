) -> Vec<String> {
    if !bool_env("INFRING_SESSION_ANALYTICS_SUGGESTIONS_ENABLED", true) {
        return Vec::new();
    }
    let mut commands = collect_recent_command_candidates(recent_thread, 8);
    if let Ok(summary) = crate::session_command_tracking_kernel::summary_for_kernel(
        root,
        &json!({"session_id": agent_id, "since_days": 14}),
    ) {
        if let Some(top) = summary.get("top_segments").and_then(Value::as_array) {
            for row in top {
                let segment = clean_text(
                    row.get("segment").and_then(Value::as_str).unwrap_or(""),
                    220,
                );
                if segment.is_empty() {
                    continue;
                }
                if commands.iter().any(|existing| existing == &segment) {
                    continue;
                }
                commands.push(segment);
                if commands.len() >= 12 {
                    break;
                }
            }
        }
    }
    if commands.is_empty() {
        return Vec::new();
    }
    let suggestions =
        crate::session_command_session_analytics_kernel::follow_up_suggestions_for_kernel(
            &json!({
                "session_id": agent_id,
                "commands": commands
            }),
            PROMPT_SUGGESTION_MAX_COUNT,
        );
    suggestions
        .into_iter()
        .map(|row| sanitize_suggestion(&row))
        .filter(|row| !row.is_empty() && !is_template_like_suggestion(row))
        .collect::<Vec<_>>()
}

fn load_prompt_suggestion_tuning(root: &Path) -> Value {
    read_json_file(&root.join("local/state/ops/session_command_tracking/nightly_tuning.json"))
        .unwrap_or_else(|| json!({}))
}

fn suggestion_matches_tuned_blocklist(text: &str, tuning: &Value) -> bool {
    let lowered = clean_text(text, 240).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let blocked_phrases = tuning
        .pointer("/suggestions/blocked_phrases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in blocked_phrases {
        let phrase = clean_text(row.as_str().unwrap_or(""), 120).to_ascii_lowercase();
        if phrase.is_empty() {
            continue;
        }
        if lowered.contains(&phrase) {
            return true;
        }
    }
    let blocked_stems = tuning
        .pointer("/suggestions/blocked_stems")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in blocked_stems {
        let stem = clean_text(row.as_str().unwrap_or(""), 80).to_ascii_lowercase();
        if stem.is_empty() {
            continue;
        }
        if lowered.starts_with(&stem) {
            return true;
        }
    }
    false
}

fn model_generated_prompt_suggestions(
    _root: &Path,
    _provider: &str,
    _model: &str,
    recent_thread: &[(String, String)],
) -> Vec<String> {
    #[cfg(test)]
    {
        if let Ok(mock_raw) = std::env::var("INFRING_PROMPT_SUGGESTION_TEST_RESPONSE") {
            let parsed = parse_model_suggestion_rows(&mock_raw);
            if !parsed.is_empty() {
                return parsed;
            }
        }
        let mut synthesized = Vec::<String>::new();
        for (role, text) in recent_thread.iter().rev() {
            if role != "user" {
                continue;
            }
            let focus = extract_focus_tokens(text, PROMPT_SUGGESTION_MAX_WORDS)
                .into_iter()
                .filter(|token| !is_topic_fragment_noise_token(token))
                .collect::<Vec<_>>();
            if focus.len() < 3 {
                continue;
            }
            let mut row = sanitize_suggestion(&focus.join(" "));
            if row.is_empty() {
                continue;
            }
            row = row
                .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | ';' | ':' | '?'))
                .trim()
                .to_string();
            if row.is_empty() || is_template_like_suggestion(&row) {
                continue;
            }
            synthesized.push(strip_trailing_suggestion_question_marks(&row));
            if synthesized.len() >= PROMPT_SUGGESTION_MAX_COUNT {
                break;
            }
        }
        return synthesized;
    }

    #[cfg(not(test))]
    {
        let root = _root;
        let provider = _provider;
        let model = _model;
        let mut transcript_rows = Vec::<String>::new();
        for (role, text) in recent_thread {
            let role_name = if role == "user" { "user" } else { "assistant" };
            let cleaned = clean_text(text, 320);
            if cleaned.is_empty() {
                continue;
            }
            transcript_rows.push(format!("{role_name}: {cleaned}"));
        }
        if transcript_rows.is_empty() {
            return Vec::new();
        }

        let system_prompt = "You generate exactly 3 realistic next-user follow-up prompts for an active chat. Output ONLY JSON: {\"suggestions\":[\"...\",\"...\",\"...\"]}. Rules: no templates, no repetitive stems, no copied long phrases from transcript, each suggestion <= 10 words, each suggestion should sound like a human follow-up that advances the current task.";
        let user_prompt = format!(
            "Generate 3 next user prompts from this transcript.\n{}\nReturn JSON only.",
            transcript_rows.join("\n")
        );

        match crate::dashboard_provider_runtime::invoke_chat(
            root,
            provider,
            model,
            system_prompt,
            &[],
            &user_prompt,
        ) {
            Ok(response) => {
                let raw = clean_chat_text(
                    response
                        .get("response")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    16_000,
                );
                parse_model_suggestion_rows(&raw)
            }
            Err(_) => Vec::new(),
        }
    }
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
    if role_key.contains("planner")
        || role_key.contains("plan")
        || role_key.contains("strategy")
        || role_key.contains("roadmap")
    {
        return format!("Hi, I'm {name}. What should we plan first?");
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
    if role_key.contains("support") || role_key.contains("helpdesk") {
        return format!("Hi, I'm {name}. How can I help first?");
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
