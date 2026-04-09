#[cfg(test)]
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
            | "extra"
            | "message"
            | "messages"
    )
}

#[cfg(test)]
fn is_action_focus_token(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "archive"
            | "build"
            | "check"
            | "clean"
            | "cleanup"
            | "compare"
            | "continue"
            | "create"
            | "debug"
            | "delete"
            | "deploy"
            | "disable"
            | "drop"
            | "enable"
            | "finish"
            | "fix"
            | "implement"
            | "inspect"
            | "kill"
            | "list"
            | "make"
            | "patch"
            | "remove"
            | "revive"
            | "run"
            | "ship"
            | "show"
            | "test"
            | "validate"
            | "verify"
    )
}

#[cfg(test)]
fn is_topic_fragment_noise_token(word: &str) -> bool {
    if is_low_signal_focus_token(word) || is_action_focus_token(word) {
        return true;
    }
    matches!(
        word,
        "again"
            | "already"
            | "after"
            | "before"
            | "confirm"
            | "confirmed"
            | "does"
            | "did"
            | "done"
            | "doing"
            | "going"
            | "keep"
            | "maybe"
            | "more"
            | "next"
            | "now"
            | "ok"
            | "okay"
            | "same"
            | "still"
            | "some"
            | "think"
            | "root"
            | "cause"
            | "sure"
            | "works"
            | "working"
            | "extra"
            | "current"
            | "status"
            | "blocker"
            | "blockers"
            | "yeah"
            | "yep"
            | "yes"
    )
}

fn model_id_is_placeholder(model_id: &str) -> bool {
    matches!(
        clean_text(model_id, 240).to_ascii_lowercase().as_str(),
        "model" | "<model>" | "(model)" | "auto"
    )
}

fn parse_provider_model_ref(raw: &str) -> Option<(String, String)> {
    let cleaned = clean_text(raw, 240);
    if cleaned.is_empty() {
        return None;
    }
    let (provider, model) = cleaned.split_once('/')?;
    let provider_clean = clean_text(provider, 80).to_ascii_lowercase();
    let model_clean = clean_text(model, 240);
    if provider_clean.is_empty() || model_clean.is_empty() || model_id_is_placeholder(&model_clean)
    {
        return None;
    }
    Some((provider_clean, model_clean))
}

fn parse_i64_value(value: Option<&Value>) -> i64 {
    value
        .and_then(|row| {
            row.as_i64().or_else(|| {
                row.as_u64()
                    .and_then(|number| i64::try_from(number).ok())
                    .or_else(|| {
                        row.as_f64().map(|number| {
                            if number.is_finite() {
                                number.round() as i64
                            } else {
                                0
                            }
                        })
                    })
                    .or_else(|| {
                        row.as_str()
                            .and_then(|text| clean_text(text, 40).parse::<i64>().ok())
                    })
            })
        })
        .unwrap_or(0)
}

fn parse_param_billion_hint(model_id: &str) -> i64 {
    let lower = clean_text(model_id, 240).to_ascii_lowercase();
    let chars = lower.chars().collect::<Vec<_>>();
    let mut best = 0i64;
    let mut index = 0usize;
    while index < chars.len() {
        if !chars[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let mut cursor = index;
        while cursor < chars.len() && chars[cursor].is_ascii_digit() {
            cursor += 1;
        }
        let number = chars[index..cursor].iter().collect::<String>();
        let mut end = cursor;
        if cursor < chars.len() && chars[cursor] == '.' {
            end += 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
        }
        let has_billion_suffix = end < chars.len() && chars[end] == 'b';
        if has_billion_suffix {
            if let Ok(parsed) = number.parse::<i64>() {
                best = best.max(parsed);
            }
        }
        index = end.saturating_add(1);
    }
    best.max(0)
}

fn read_agent_profile(root: &Path, agent_id: &str) -> Value {
    read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/agent_profiles.json"),
    )
    .and_then(|value| value.get("agents").and_then(Value::as_object).cloned())
    .and_then(|agents| agents.get(agent_id).cloned())
    .unwrap_or_else(|| json!({}))
}

fn resolve_prompt_suggestion_model(root: &Path, agent_id: &str) -> Option<(String, String, i64)> {
    let profile = read_agent_profile(root, agent_id);
    let model_override = clean_text(
        profile
            .get("model_override")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    let mut provider = clean_text(
        profile
            .get("model_provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    )
    .to_ascii_lowercase();
    let mut model = clean_text(
        profile
            .get("runtime_model")
            .or_else(|| profile.get("model_name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if let Some((override_provider, override_model)) = parse_provider_model_ref(&model_override) {
        if provider.is_empty() {
            provider = override_provider;
        }
        if model.is_empty() || model_id_is_placeholder(&model) {
            model = override_model;
        }
    }
    if model_id_is_placeholder(&model) {
        model.clear();
    }
    if provider.is_empty() {
        provider = "auto".to_string();
    }
    if model.is_empty() {
        model = "auto".to_string();
    }

    let snapshot = read_json_file(
        &root.join("client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json"),
    )
    .unwrap_or_else(|| json!({}));
    let route_request = json!({
        "agent_id": agent_id,
        "task_type": "prompt_suggestions",
        "complexity": "general",
        "budget_mode": "balanced"
    });
    let (resolved_provider, resolved_model, _) =
        crate::dashboard_model_catalog::resolve_model_selection(
            root,
            &snapshot,
            &provider,
            &model,
            &route_request,
        );
    if resolved_provider.is_empty()
        || resolved_model.is_empty()
        || model_id_is_placeholder(&resolved_model)
    {
        return None;
    }

    let catalog = crate::dashboard_model_catalog::catalog_payload(root, &snapshot);
    let mut params_billion = catalog
        .get("models")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter().find(|row| {
                clean_text(
                    row.get("provider").and_then(Value::as_str).unwrap_or(""),
                    80,
                )
                .eq_ignore_ascii_case(&resolved_provider)
                    && clean_text(row.get("model").and_then(Value::as_str).unwrap_or(""), 240)
                        == resolved_model
            })
        })
        .map(|row| parse_i64_value(row.get("params_billion")))
        .unwrap_or(0);
    if params_billion <= 0 {
        params_billion = parse_param_billion_hint(&resolved_model)
            .max(parse_param_billion_hint(&model_override))
            .max(parse_i64_value(profile.get("param_count_billion")));
    }

    Some((resolved_provider, resolved_model, params_billion.max(0)))
}

fn is_template_like_suggestion(text: &str) -> bool {
    let lowered = clean_text(text, 240).to_ascii_lowercase();
    if lowered.is_empty() {
        return true;
    }
    lowered.contains("continue with")
        || lowered.contains("what should we")
        || lowered.contains("what should i")
        || lowered.contains("can you continue")
        || lowered.contains("can you verify")
        || lowered.contains("can you test")
        || lowered.contains("does compare other")
}

fn parse_model_suggestion_rows(raw: &str) -> Vec<String> {
    let value = parse_json_loose(raw).unwrap_or_else(|| Value::String(String::new()));
    let rows = if let Some(array) = value.as_array() {
        array.clone()
    } else if let Some(array) = value.get("suggestions").and_then(Value::as_array) {
        array.clone()
    } else if let Some(array) = value.get("rows").and_then(Value::as_array) {
        array.clone()
    } else {
        Vec::new()
    };
    rows.into_iter()
        .filter_map(|row| row.as_str().map(|text| sanitize_suggestion(text)))
        .filter(|row| !row.is_empty())
        .filter(|row| !is_template_like_suggestion(row))
        .collect::<Vec<_>>()
}

fn bool_env(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => matches!(
            clean_text(&raw, 40).to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => fallback,
    }
}

fn looks_like_shell_command_line(line: &str) -> bool {
    let first = clean_text(line, 200)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "git"
            | "gh"
            | "cargo"
            | "npm"
            | "npx"
            | "pnpm"
            | "node"
            | "python"
            | "pytest"
            | "ls"
            | "cat"
            | "rg"
            | "grep"
            | "find"
            | "tree"
            | "curl"
            | "wget"
            | "docker"
            | "kubectl"
            | "infring"
            | "infringctl"
            | "protheus-ops"
    )
}

fn collect_recent_command_candidates(
    recent_thread: &[(String, String)],
    max_rows: usize,
) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for (role, text) in recent_thread.iter().rev() {
        if role != "user" {
            continue;
        }
        for line in text.lines() {
            let normalized = clean_text(line.trim_start_matches("$ "), 220);
            if normalized.is_empty() || !looks_like_shell_command_line(&normalized) {
                continue;
            }
            if out.iter().any(|existing| existing == &normalized) {
                continue;
            }
            out.push(normalized);
            if out.len() >= max_rows.max(1) {
                return out;
            }
        }
    }
    out
}

fn analytics_prompt_suggestions(
    root: &Path,
    agent_id: &str,
    recent_thread: &[(String, String)],
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

    let (provider, model, params_billion) = match resolve_prompt_suggestion_model(root, &id) {
        Some(row) => row,
        None => {
            return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
        }
    };
    if params_billion < PROMPT_SUGGESTION_MIN_PARAMS_BILLION {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }

    let base_style = derive_suggestion_style(&recent_thread);
    let style = SuggestionStyle {
        prefer_can_you: false,
        prefer_question_mark: false,
        prefer_lowercase: base_style.prefer_lowercase,
    };
    let mut candidates = analytics_prompt_suggestions(root, &id, &recent_thread);
    let model_candidates =
        model_generated_prompt_suggestions(root, &provider, &model, &recent_thread);
    for row in model_candidates {
        if candidates.len() >= PROMPT_SUGGESTION_MAX_COUNT.saturating_mul(2) {
            break;
        }
        candidates.push(row);
    }
    if candidates.is_empty() {
        return json!({"ok": true, "type": "dashboard_agent_suggestions", "agent_id": id, "suggestions": []});
    }

    let recent_set = recent_user
        .iter()
        .map(|row| sanitize_suggestion(row).to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let tuning = load_prompt_suggestion_tuning(root);
    let mut out = Vec::<String>::new();
    for raw in candidates {
        let row = apply_suggestion_style(&style, &raw);
        if row.is_empty() {
            continue;
        }
        if is_template_like_suggestion(&row) {
            continue;
        }
        if suggestion_matches_tuned_blocklist(&row, &tuning) {
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
