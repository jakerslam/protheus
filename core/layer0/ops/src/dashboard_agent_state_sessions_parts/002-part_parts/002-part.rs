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
