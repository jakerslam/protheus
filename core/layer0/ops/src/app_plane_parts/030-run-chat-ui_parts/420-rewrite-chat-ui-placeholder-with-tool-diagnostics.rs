fn rewrite_chat_ui_placeholder_with_tool_diagnostics(
    assistant: &str,
    diagnostics: &Value,
) -> (String, String) {
    let current = clean(assistant, 16_000);
    if current.is_empty() || !crate::tool_output_match_filter::matches_ack_placeholder(&current) {
        return (current, "unchanged".to_string());
    }
    let errors = diagnostics
        .get("error_codes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let has_error = !errors.is_empty();
    let total_calls = diagnostics
        .get("total_calls")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let has_surface_unavailable = errors.contains_key("web_tool_surface_unavailable");
    let has_surface_degraded = errors.contains_key("web_tool_surface_degraded");
    let has_auth_missing = errors.contains_key("web_tool_auth_missing");
    let has_policy_blocked = errors.contains_key("web_tool_policy_blocked");
    let has_invalid_response = errors.contains_key("web_tool_invalid_response");
    let has_not_found = errors.contains_key("web_tool_not_found");
    let has_silent_failure = errors.contains_key("web_tool_silent_failure");

    if has_surface_unavailable {
        return (
            current,
            "placeholder_detected_surface_unavailable".to_string(),
        );
    }
    if has_surface_degraded {
        return (current, "placeholder_detected_surface_degraded".to_string());
    }
    if has_auth_missing {
        return (current, "placeholder_detected_auth".to_string());
    }
    if has_policy_blocked {
        return (current, "placeholder_detected_policy".to_string());
    }
    if has_invalid_response {
        return (current, "placeholder_detected_invalid_response".to_string());
    }
    if has_not_found {
        return (current, "placeholder_detected_not_found".to_string());
    }
    if has_silent_failure {
        return (current, "placeholder_detected_silent_failure".to_string());
    }
    if has_error {
        return (current, "placeholder_detected_error".to_string());
    }
    if total_calls > 0 {
        return (current, "placeholder_detected_low_signal".to_string());
    }
    (current, "unchanged".to_string())
}

fn rewrite_chat_ui_legacy_route_classifier_copy(assistant: &str) -> (String, String) {
    let contract = chat_ui_default_workflow_contract();
    let legacy_markers = workflow_legacy_copy_markers(&contract);
    let mut rewritten = clean(assistant, 16_000);
    let mut stripped = false;

    rewritten = remove_legacy_route_source_tags(&rewritten);
    rewritten = remove_legacy_route_sentence_blocks(&rewritten, &legacy_markers);

    for marker in &legacy_markers {
        if marker.is_empty() {
            continue;
        }
        let before = rewritten.clone();
        rewritten = remove_ascii_case_insensitive_phrase(&rewritten, marker);
        if !stripped && rewritten != before {
            stripped = true;
        }
    }

    rewritten = collapse_whitespace(&rewritten);
    if rewritten.is_empty() {
        return (rewritten, "legacy_route_copy_stripped".to_string());
    }
    if stripped {
        return (rewritten, "legacy_route_copy_stripped".to_string());
    }
    (rewritten, "unchanged".to_string())
}

fn workflow_legacy_copy_markers(contract: &Value) -> Vec<String> {
    let mut markers = Vec::new();
    for marker_key in [
        "/diagnostic_markers/legacy_retry_templates",
        "/diagnostic_markers/gate_choice_prefix_leakage_phrases",
    ] {
        if let Some(values) = contract.pointer(marker_key).and_then(Value::as_array) {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(|raw| clean(raw, 240))
                .filter(|marker| !marker.is_empty())
                .for_each(|marker| markers.push(marker));
        }
    }
    markers
}

fn remove_legacy_route_sentence_blocks(raw: &str, legacy_markers: &[String]) -> String {
    let mut keep_sentences = Vec::new();
    let fallback_markers = [
        "first gate",
        "automated classification",
        "binary classification",
        "conversation bypass mode",
        "task route",
        "info route",
        "workflow_route",
    ];

    for sentence in split_into_sentences(raw) {
        let lowered = sentence.to_ascii_lowercase();
        let should_strip = legacy_markers.iter().any(|marker| {
            if marker.is_empty() {
                return false;
            }
            lowered.contains(&marker.to_ascii_lowercase())
        }) || fallback_markers.iter().any(|marker| {
            let fallback = marker.to_ascii_lowercase();
            !fallback.is_empty() && lowered.contains(&fallback)
        });

        if !should_strip && !sentence.trim().is_empty() {
            keep_sentences.push(sentence.trim().to_string());
        }
    }

    keep_sentences.join(" ")
}

fn remove_legacy_route_source_tags(raw: &str) -> String {
    let mut rewritten = raw.to_string();
    while let Some(start) = rewritten.find("[source:") {
        let marker_end = rewritten[start..].find(']').map(|idx| start + idx + 1);
        let Some(end) = marker_end else {
            break;
        };
        let mut left = start;
        while left > 0 {
            let prev = left.saturating_sub(1);
            if rewritten.as_bytes().get(prev).copied() == Some(b' ') {
                left = prev;
            } else {
                break;
            }
        }
        let mut right = end;
        while right < rewritten.len() {
            if rewritten.as_bytes().get(right).copied() == Some(b' ') {
                right += 1;
            } else {
                break;
            }
        }
        rewritten.replace_range(left..right, "");
    }
    rewritten
}

fn split_into_sentences(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (idx, ch) in raw.char_indices() {
        if matches!(ch, '.' | '?' | '!' | '\n') {
            let end = idx + ch.len_utf8();
            let piece = raw[start..end].trim();
            if !piece.is_empty() {
                parts.push(piece.to_string());
            }
            start = end;
        }
    }
    if start < raw.len() {
        let piece = raw[start..].trim();
        if !piece.is_empty() {
            parts.push(piece.to_string());
        }
    }
    if parts.is_empty() {
        return vec![raw.trim().to_string()];
    }
    parts
}

fn remove_ascii_case_insensitive_phrase(source: &str, marker: &str) -> String {
    let source_lower = source.to_ascii_lowercase();
    let needle = marker.to_ascii_lowercase();

    let mut rewritten = String::new();
    let mut cursor = 0usize;
    while let Some(byte_offset) = source_lower[cursor..].find(&needle) {
        let offset = cursor + byte_offset;
        rewritten.push_str(&source[cursor..offset]);
        cursor = offset + marker.len();
    }
    rewritten.push_str(&source[cursor..]);
    rewritten
}

fn collapse_whitespace(raw: &str) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
            }
            previous_space = true;
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    out.trim().to_string()
}
