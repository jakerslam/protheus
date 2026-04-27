
fn historical_context_keyframes_prompt_context(
    history_messages: &[Value],
    active_messages: &[Value],
    max_keyframes: usize,
    max_chars: usize,
) -> String {
    let target = max_keyframes.clamp(1, 24);
    let dropped = history_messages.len().saturating_sub(active_messages.len());
    if dropped == 0 {
        return String::new();
    }
    let mut candidates = Vec::<(String, String)>::new();
    for row in history_messages.iter().take(dropped) {
        let Some(role_label) = prompt_role_label(row) else {
            continue;
        };
        let snippet = first_sentence(&message_text(row), 220);
        if snippet.is_empty() {
            continue;
        }
        if text_contains_external_framework_identity_bleed_for_host(&snippet) {
            continue;
        }
        candidates.push((role_label.to_string(), snippet));
    }
    if candidates.is_empty() {
        return String::new();
    }
    let mut selected = Vec::<(String, String)>::new();
    if candidates.len() <= target {
        selected = candidates;
    } else {
        selected.push(candidates[0].clone());
        if target > 2 {
            let remaining_slots = target.saturating_sub(2);
            let last_idx = candidates.len().saturating_sub(1);
            for slot in 0..remaining_slots {
                let idx = 1 + ((slot + 1) * last_idx.saturating_sub(1)) / (remaining_slots + 1);
                if idx < last_idx {
                    selected.push(candidates[idx].clone());
                }
            }
        }
        selected.push(candidates[candidates.len().saturating_sub(1)].clone());
    }
    let mut dedup = HashSet::<String>::new();
    let mut lines = Vec::<String>::new();
    for (role, snippet) in selected {
        let key = role_snippet_key(&role, &snippet);
        if !dedup.insert(key) {
            continue;
        }
        lines.push(format!("- [{role}] {snippet}"));
        if lines.len() >= target {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!(
            "Long-thread keyframes outside the active window (retain for continuity):\n{}",
            lines.join("\n")
        ),
        max_chars.max(400),
    )
}

fn historical_relevant_recall_prompt_context(
    history_messages: &[Value],
    active_messages: &[Value],
    user_message: &str,
    max_rows: usize,
    max_chars: usize,
) -> String {
    let target = max_rows.clamp(2, 20);
    let dropped = history_messages.len().saturating_sub(active_messages.len());
    if dropped == 0 {
        return String::new();
    }
    let user_terms = important_memory_terms(user_message, 24)
        .into_iter()
        .collect::<HashSet<_>>();
    let recall_intent = memory_recall_requested(user_message);
    if user_terms.is_empty() && !recall_intent {
        return String::new();
    }
    let mut scored = Vec::<(i64, String, String)>::new();
    for (idx, row) in history_messages.iter().take(dropped).enumerate() {
        let Some(role_label) = prompt_role_label(row) else {
            continue;
        };
        let snippet = clean_text(&message_text(row), 360);
        if snippet.is_empty() {
            continue;
        }
        if response_contains_cross_project_assimilation_bleed(user_message, &snippet) {
            continue;
        }
        let role_label = role_label.to_string();
        let snippet_terms = important_memory_terms(&snippet, 24)
            .into_iter()
            .collect::<HashSet<_>>();
        let overlap = if user_terms.is_empty() {
            0
        } else {
            user_terms.intersection(&snippet_terms).count() as i64
        };
        if overlap == 0 && !recall_intent {
            continue;
        }
        let recency_score = (idx as i64).min(60);
        let score = overlap.saturating_mul(8) + recency_score;
        scored.push((score, role_label, first_sentence(&snippet, 260)));
    }
    if scored.is_empty() {
        return String::new();
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let mut dedup = HashSet::<String>::new();
    let mut lines = Vec::<String>::new();
    for (_, role, snippet) in scored.into_iter().take(target.saturating_mul(2)) {
        if snippet.is_empty() {
            continue;
        }
        let key = role_snippet_key(&role, &snippet);
        if !dedup.insert(key) {
            continue;
        }
        lines.push(format!("- [{role}] {snippet}"));
        if lines.len() >= target {
            break;
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    trim_text(
        &format!(
            "Relevant long-thread recall outside the active window (use for continuity):\n{}",
            lines.join("\n")
        ),
        max_chars.max(500),
    )
}

fn append_tool_completion_outcome(current: &str, event: &str) -> String {
    let cleaned_current = clean_text(current, 200);
    let cleaned_event = clean_text(event, 120);
    if cleaned_event.is_empty() {
        return if cleaned_current.is_empty() {
            "unchanged".to_string()
        } else {
            cleaned_current
        };
    }
    if cleaned_current.is_empty() || cleaned_current == "unchanged" {
        return cleaned_event;
    }
    format!("{cleaned_current}+{cleaned_event}")
}

fn has_actionable_tool_reason(text: &str) -> bool {
    let lowered = clean_text(text, 1200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let confirmation_reason = lowered.contains("need your confirmation")
        || lowered.contains("requires confirmation")
        || lowered.contains("reply `yes`")
        || lowered.contains("reply yes")
        || lowered.contains("permission");
    let precondition_reason = lowered.contains("before running")
        || lowered.contains("before i can run")
        || lowered.contains("to execute it now")
        || lowered.contains("confirm this step");
    confirmation_reason && precondition_reason
}
