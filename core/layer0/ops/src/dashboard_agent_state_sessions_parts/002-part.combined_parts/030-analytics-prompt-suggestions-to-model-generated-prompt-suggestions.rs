
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
