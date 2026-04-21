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

fn context_role_label(row: &Value) -> String {
    clean_text(
        row.get("role")
            .or_else(|| row.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("assistant"),
        24,
    )
    .to_ascii_lowercase()
}

fn prompt_role_label(row: &Value) -> Option<&'static str> {
    let role = context_role_label(row);
    if role == "system" {
        None
    } else if role.contains("user") {
        Some("User")
    } else {
        Some("Agent")
    }
}

fn role_snippet_key(role: &str, snippet: &str) -> String {
    format!(
        "{}|{}",
        role.to_ascii_lowercase(),
        snippet.to_ascii_lowercase()
    )
}

fn looks_like_image_heavy_tool_context(row: &Value) -> bool {
    let text = clean_text(&message_text(row), 16_000);
    if text.len() < 256 {
        return false;
    }
    let lowered = text.to_ascii_lowercase();
    [
        "content_base64",
        "contentbase64",
        "image_base64",
        "imagebase64",
        "\"image\":\"data:image/",
        "\"image_url\":\"data:image/",
        "\"imageurl\":\"data:image/",
        "\"imagedata\":\"data:image/",
        "\"image_data\":\"data:image/",
        "\"base64_image\"",
        "\"image_base64\"",
        "\"screenshot_base64\"",
        "screenshotbase64",
        "data:image/",
        "\"input_image\"",
        "\"computer_call_output\"",
        "\"inlinedata\"",
        "\"mime_type\":\"image/",
        "\"mimetype\":\"image/",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_verbose_tool_context(row: &Value) -> bool {
    if looks_like_image_heavy_tool_context(row) {
        return true;
    }
    let role = context_role_label(row);
    let text = clean_text(&message_text(row), 8_000);
    if text.len() < 900 {
        return false;
    }
    if role == "tool" {
        return true;
    }
    let lowered = text.to_ascii_lowercase();
    [
        "from web retrieval:",
        "web benchmark synthesis:",
        "tool call",
        "tool result",
        "terminal output",
        "\"tool_calls\"",
        "\"nexus_connection\"",
        "\"turn_transaction\"",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn write_message_text(row: &mut Value, text: &str) {
    if let Some(obj) = row.as_object_mut() {
        let normalized = clean_text(text, 1_200);
        if obj.contains_key("text") {
            obj.insert("text".to_string(), Value::String(normalized));
        } else if obj.contains_key("content") {
            obj.insert("content".to_string(), Value::String(normalized));
        } else {
            obj.insert("text".to_string(), Value::String(normalized));
        }
        obj.insert("compacted_tool_context".to_string(), json!(true));
    }
}

fn compact_old_tool_context_messages(messages: &[Value], keep_recent: usize) -> Vec<Value> {
    let mut out = messages.to_vec();
    let candidate_indices = out
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            if looks_like_verbose_tool_context(row) {
                Some(idx)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if candidate_indices.len() <= keep_recent.max(1) {
        return out;
    }
    let compact_count = candidate_indices.len().saturating_sub(keep_recent.max(1));
    for idx in candidate_indices.into_iter().take(compact_count) {
        let original = message_text(&out[idx]);
        let compacted = if looks_like_image_heavy_tool_context(&out[idx]) {
            "Screenshot taken [earlier image context compacted]".to_string()
        } else {
            let snippet = first_sentence(&original, 180);
            if snippet.is_empty() {
                "Earlier verbose tool output compacted for context continuity.".to_string()
            } else {
                format!("{snippet} [earlier verbose tool output compacted]")
            }
        };
        if compacted.len() < original.len() {
            write_message_text(&mut out[idx], &compacted);
        }
    }
    out
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

fn recent_context_floor_target_count(history_messages: &[Value], min_recent: usize) -> usize {
    if history_messages.is_empty() {
        return 0;
    }
    min_recent
        .clamp(1, 256)
        .min(history_messages.len())
}

fn recent_context_floor_missing_count(
    history_messages: &[Value],
    pooled_messages: &[Value],
    min_recent: usize,
) -> usize {
    let target = recent_context_floor_target_count(history_messages, min_recent);
    if target == 0 {
        return 0;
    }
    let mut required_tail = history_messages
        .iter()
        .rev()
        .take(target)
        .cloned()
        .collect::<Vec<_>>();
    required_tail.reverse();
    let seen = pooled_messages
        .iter()
        .map(context_message_fingerprint)
        .collect::<HashSet<_>>();
    required_tail
        .iter()
        .filter(|row| !seen.contains(&context_message_fingerprint(row)))
        .count()
}

fn recent_context_floor_satisfied(
    history_messages: &[Value],
    pooled_messages: &[Value],
    min_recent: usize,
) -> bool {
    recent_context_floor_missing_count(history_messages, pooled_messages, min_recent) == 0
}

fn recent_context_floor_coverage_ratio(
    history_messages: &[Value],
    pooled_messages: &[Value],
    min_recent: usize,
) -> f64 {
    let target = recent_context_floor_target_count(history_messages, min_recent);
    if target == 0 {
        return 1.0;
    }
    let missing = recent_context_floor_missing_count(history_messages, pooled_messages, min_recent);
    let covered = target.saturating_sub(missing);
    (covered as f64 / target as f64).clamp(0.0, 1.0)
}

fn trim_context_pool(messages: &[Value], limit_tokens: i64) -> Vec<Value> {
    let cap = limit_tokens.max(2_048);
    let mut out = compact_old_tool_context_messages(messages, 2);
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
