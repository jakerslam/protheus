fn message_token_cost(row: &Value) -> i64 {
    estimate_tokens(&message_text(row))
}

fn total_message_tokens(rows: &[Value]) -> i64 {
    rows.iter().map(message_token_cost).sum::<i64>().max(0)
}

fn context_pressure_label(ratio: f64) -> &'static str {
    if !ratio.is_finite() || ratio <= 0.0 {
        "low"
    } else if ratio >= 0.96 {
        "critical"
    } else if ratio >= 0.82 {
        "high"
    } else if ratio >= 0.55 {
        "medium"
    } else {
        "low"
    }
}

fn context_message_fingerprint(row: &Value) -> String {
    let id = clean_text(
        row.get("id")
            .or_else(|| row.get("message_id"))
            .map(|value| match value {
                Value::String(text) => text.to_string(),
                Value::Number(num) => num.to_string(),
                _ => String::new(),
            })
            .as_deref()
            .unwrap_or(""),
        120,
    );
    if !id.is_empty() {
        return format!("id:{id}");
    }
    let role = clean_text(
        row.get("role")
            .or_else(|| row.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("assistant"),
        24,
    );
    let ts = clean_text(
        row.get("ts")
            .or_else(|| row.get("timestamp"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let text = clean_text(&message_text(row), 420);
    format!(
        "{}|{}|{}",
        role.to_ascii_lowercase(),
        ts.to_ascii_lowercase(),
        text.to_ascii_lowercase()
    )
}

fn enforce_recent_context_floor(
    history_messages: &[Value],
    pooled_messages: &[Value],
    min_recent: usize,
) -> (Vec<Value>, usize) {
    let floor = min_recent.clamp(1, 256);
    if history_messages.is_empty() {
        return (pooled_messages.to_vec(), 0);
    }
    let mut required_tail = history_messages
        .iter()
        .rev()
        .take(floor.min(history_messages.len()))
        .cloned()
        .collect::<Vec<_>>();
    if required_tail.is_empty() {
        return (pooled_messages.to_vec(), 0);
    }
    required_tail.reverse();
    let mut out = pooled_messages.to_vec();
    let mut seen = out
        .iter()
        .map(context_message_fingerprint)
        .collect::<HashSet<_>>();
    let mut injected = 0usize;
    for row in required_tail {
        let key = context_message_fingerprint(&row);
        if seen.insert(key) {
            out.push(row);
            injected += 1;
        }
    }
    (out, injected)
}

fn trim_context_pool(messages: &[Value], limit_tokens: i64) -> Vec<Value> {
    let cap = limit_tokens.max(2_048);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > 1 && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

fn select_active_context_window(
    messages: &[Value],
    target_tokens: i64,
    min_recent: usize,
) -> Vec<Value> {
    let cap = target_tokens.max(1_024);
    let floor = min_recent.clamp(1, 256);
    let mut out = messages.to_vec();
    let mut total = total_message_tokens(&out);
    while out.len() > floor && total > cap {
        let removed = message_token_cost(&out[0]);
        out.remove(0);
        total = (total - removed).max(0);
    }
    out
}

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
        let role = clean_text(
            row.get("role")
                .or_else(|| row.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("assistant"),
            24,
        )
        .to_ascii_lowercase();
        if role == "system" {
            continue;
        }
        let snippet = first_sentence(&message_text(row), 220);
        if snippet.is_empty() {
            continue;
        }
        let role_label = if role.contains("user") {
            "User".to_string()
        } else {
            "Agent".to_string()
        };
        candidates.push((role_label, snippet));
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
        let key = format!(
            "{}|{}",
            role.to_ascii_lowercase(),
            snippet.to_ascii_lowercase()
        );
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
        let role = clean_text(
            row.get("role")
                .or_else(|| row.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("assistant"),
            24,
        )
        .to_ascii_lowercase();
        if role == "system" {
            continue;
        }
        let snippet = clean_text(&message_text(row), 360);
        if snippet.is_empty() {
            continue;
        }
        let role_label = if role.contains("user") {
            "User".to_string()
        } else {
            "Agent".to_string()
        };
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
        let key = format!(
            "{}|{}",
            role.to_ascii_lowercase(),
            snippet.to_ascii_lowercase()
        );
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

fn enforce_tool_completion_contract(response_text: String, response_tools: &[Value]) -> (String, Value) {
    let raw_actionable_reason = has_actionable_tool_reason(&response_text);
    let mut tools_present = 0usize;
    let mut successful_tools = 0usize;
    let mut error_tools = 0usize;
    for tool in response_tools {
        let name = clean_text(tool.get("name").and_then(Value::as_str).unwrap_or(""), 80)
            .to_ascii_lowercase();
        if name.is_empty() || name == "thought_process" {
            continue;
        }
        tools_present += 1;
        if tool
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            error_tools += 1;
        } else {
            successful_tools += 1;
        }
    }
    let findings = {
        let candidate = response_tools_summary_for_user(response_tools, 4);
        let cleaned = clean_text(&candidate, 24_000);
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    };
    let findings_available = findings.is_some();
    let (mut finalized, mut outcome, initial_ack_only) =
        finalize_user_facing_response_with_outcome(response_text, findings.clone());
    let mut applied = outcome != "unchanged";

    if tools_present > 0 {
        let finalized_cleaned = clean_text(&finalized, 32_000);
        let actionable_reason = raw_actionable_reason || has_actionable_tool_reason(&finalized_cleaned);
        if actionable_reason && !findings_available {
            finalized = clean_text(&finalized_cleaned, 32_000);
            if response_is_no_findings_placeholder(&finalized) {
                finalized = clean_text(
                    "I need your confirmation before running this command. Reply `yes` to continue.",
                    32_000,
                );
            }
            outcome = append_tool_completion_outcome(&outcome, "tool_completion_preserved_reason");
            applied = true;
        }
        if findings_available
            && (finalized_cleaned.is_empty()
                || response_looks_like_tool_ack_without_findings(&finalized_cleaned)
                || response_is_no_findings_placeholder(&finalized_cleaned))
        {
            finalized = findings.unwrap_or_else(no_findings_user_facing_response);
            outcome = append_tool_completion_outcome(&outcome, "tool_completion_replaced_with_findings");
            applied = true;
        } else if !findings_available
            && !actionable_reason
            && (finalized_cleaned.is_empty()
                || response_looks_like_tool_ack_without_findings(&finalized_cleaned)
                || response_is_no_findings_placeholder(&finalized_cleaned))
        {
            finalized = no_findings_user_facing_response();
            outcome =
                append_tool_completion_outcome(&outcome, "tool_completion_replaced_with_no_findings");
            applied = true;
        }
        if response_looks_like_tool_ack_without_findings(&finalized)
            && !has_actionable_tool_reason(&finalized)
        {
            finalized = no_findings_user_facing_response();
            outcome = append_tool_completion_outcome(&outcome, "tool_completion_forced_no_findings");
            applied = true;
        }
    }

    let final_ack_only = response_looks_like_tool_ack_without_findings(&finalized);
    let final_no_findings = response_is_no_findings_placeholder(&finalized);
    let completion_state = if tools_present == 0 {
        "not_applicable"
    } else if findings_available {
        "reported_findings"
    } else if final_no_findings {
        "reported_no_findings"
    } else {
        "reported_reason"
    };

    (
        finalized,
        json!({
            "applied": applied,
            "outcome": clean_text(&outcome, 200),
            "tools_present": tools_present > 0,
            "tool_count": tools_present,
            "successful_tools": successful_tools,
            "error_tools": error_tools,
            "findings_available": findings_available,
            "initial_ack_only": initial_ack_only,
            "final_ack_only": final_ack_only,
            "final_no_findings": final_no_findings,
            "completion_state": completion_state
        }),
    )
}
