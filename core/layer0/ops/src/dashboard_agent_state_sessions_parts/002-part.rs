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
        "other",
        "others",
        "thing",
        "things",
        "stuff",
        "issue",
        "issues",
        "problem",
        "problems",
        "work",
        "works",
        "working",
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

fn is_low_signal_focus_token(word: &str) -> bool {
    matches!(
        word,
        "other"
            | "others"
            | "thing"
            | "things"
            | "stuff"
            | "issue"
            | "issues"
            | "problem"
            | "problems"
            | "work"
            | "works"
            | "working"
            | "item"
            | "items"
            | "part"
            | "parts"
            | "step"
            | "steps"
            | "task"
            | "tasks"
            | "chat"
            | "message"
            | "messages"
    )
}

fn is_action_focus_token(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "build"
            | "check"
            | "compare"
            | "continue"
            | "create"
            | "debug"
            | "deploy"
            | "finish"
            | "fix"
            | "implement"
            | "inspect"
            | "make"
            | "patch"
            | "run"
            | "ship"
            | "test"
            | "validate"
            | "verify"
    )
}

fn canonical_action_verb(raw: &str) -> Option<&'static str> {
    match raw {
        "fix" | "debug" | "repair" | "resolve" => Some("fix"),
        "implement" | "build" | "create" | "ship" => Some("implement"),
        "verify" | "validate" | "check" | "test" => Some("verify"),
        "compare" | "analyze" | "evaluate" | "assess" => Some("compare"),
        "continue" | "finish" | "complete" => Some("continue"),
        "show" | "explain" | "summarize" => Some("show"),
        _ => None,
    }
}

fn dominant_user_action_verbs(recent_thread: &[(String, String)], limit: usize) -> Vec<String> {
    let mut counts = HashMap::<String, usize>::new();
    for (role, text) in recent_thread {
        if role != "user" {
            continue;
        }
        let first = clean_text(text, 120)
            .to_ascii_lowercase()
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .find(|token| !token.trim().is_empty())
            .map(|token| token.trim().to_string())
            .unwrap_or_default();
        if first.is_empty() {
            continue;
        }
        if let Some(verb) = canonical_action_verb(&first) {
            *counts.entry(verb.to_string()).or_insert(0) += 1;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(limit.max(1))
        .map(|(verb, _)| verb)
        .collect::<Vec<_>>()
}

fn compose_topic_phrase(recent_thread: &[(String, String)], keywords: &[String]) -> String {
    let mut tokens = Vec::<String>::new();
    if let Some(last_user) = recent_thread
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, text)| text)
    {
        let candidate = extract_focus_tokens(last_user, 4)
            .into_iter()
            .filter(|token| !is_low_signal_focus_token(token))
            .collect::<Vec<_>>();
        let has_domain_signal = candidate
            .iter()
            .any(|token| !is_action_focus_token(token));
        if has_domain_signal {
            tokens = candidate;
        }
    }
    if tokens.len() < 2 {
        for (role, text) in recent_thread.iter().rev() {
            if role != "user" {
                continue;
            }
            let candidate = extract_focus_tokens(text, 4)
                .into_iter()
                .filter(|token| !is_low_signal_focus_token(token))
                .collect::<Vec<_>>();
            if candidate.is_empty() {
                continue;
            }
            let has_domain_signal = candidate
                .iter()
                .any(|token| !is_action_focus_token(token));
            if !has_domain_signal {
                continue;
            }
            for token in candidate {
                if tokens.iter().any(|existing| existing == &token) {
                    continue;
                }
                tokens.push(token);
                if tokens.len() >= 4 {
                    break;
                }
            }
            if tokens.len() >= 2 {
                break;
            }
        }
    }
    if tokens.len() < 2 {
        for keyword in keywords {
            if is_low_signal_focus_token(keyword) {
                continue;
            }
            if tokens.iter().any(|existing| existing == keyword) {
                continue;
            }
            tokens.push(keyword.clone());
            if tokens.len() >= 4 {
                break;
            }
        }
    }
    if tokens.is_empty() {
        let compact = compact_topic_phrase(recent_thread, keywords);
        return extract_focus_tokens(&compact, 4)
            .into_iter()
            .filter(|token| !is_low_signal_focus_token(token))
            .collect::<Vec<_>>()
            .join(" ");
    }
    tokens.join(" ")
}

fn thread_contains_terms(recent_thread: &[(String, String)], terms: &[&str]) -> bool {
    let mut haystack = String::new();
    for (_, text) in recent_thread {
        haystack.push_str(&clean_text(text, 320).to_ascii_lowercase());
        haystack.push(' ');
    }
    terms.iter().any(|term| haystack.contains(term))
}

fn build_suggestion_candidates(recent_thread: &[(String, String)], topic: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let action_verbs = dominant_user_action_verbs(recent_thread, 3);
    let lead_verb = action_verbs
        .first()
        .cloned()
        .unwrap_or_else(|| "show".to_string());
    let has_fix_intent = thread_contains_terms(
        recent_thread,
        &[
            "fix",
            "bug",
            "broken",
            "regression",
            "error",
            "fail",
            "issue",
            "not working",
        ],
    );
    let has_build_intent = thread_contains_terms(
        recent_thread,
        &[
            "build",
            "implement",
            "add",
            "create",
            "make",
            "ship",
            "patch",
        ],
    );
    let has_analysis_intent = thread_contains_terms(
        recent_thread,
        &[
            "compare",
            "research",
            "analyze",
            "tradeoff",
            "option",
            "strategy",
            "plan",
        ],
    );
    let has_validation_intent = thread_contains_terms(
        recent_thread,
        &[
            "verify",
            "test",
            "validate",
            "check",
            "proof",
            "benchmark",
            "smoke",
        ],
    );

    if has_fix_intent {
        out.push(format!("show the smallest patch to fix {topic}"));
        out.push(format!(
            "what caused {topic} and how does the fix prevent regressions"
        ));
    }
    if has_build_intent {
        out.push(format!("implement {topic} now and show the diff"));
        out.push(format!("what is the fastest next step to finish {topic}"));
    }
    if has_analysis_intent {
        out.push(format!("compare the strongest options for {topic}"));
        out.push(format!("what tradeoffs matter most for {topic}"));
    }
    if has_validation_intent {
        out.push(format!(
            "run a quick validation for {topic} and share the result"
        ));
        out.push(format!("add one regression test for {topic}"));
    }

    out.push(format!("{lead_verb} current status and blockers for {topic}"));
    out.push(format!("what should we tackle next for {topic}"));
    out.push(format!(
        "give me one concrete next action to move {topic} forward"
    ));
    out
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
    let mut out = sanitize_suggestion(&text);
    if out.is_empty() {
        return out;
    }
    if style.prefer_question_mark {
        out = out
            .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | ';' | ':'))
            .trim()
            .to_string();
        if !out.is_empty() && !out.ends_with('?') {
            out.push('?');
        }
    } else {
        out = out.trim_end_matches('?').trim().to_string();
    }
    out
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
        prefer_can_you: base_style.prefer_can_you,
        prefer_question_mark: true,
        prefer_lowercase: base_style.prefer_lowercase,
    };
    let keywords = extract_thread_keywords(&recent_thread, 6);
    let topic = compose_topic_phrase(&recent_thread, &keywords);
    if topic.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }
    let candidates = build_suggestion_candidates(&recent_thread, &topic);

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
