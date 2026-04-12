fn resolve_tool_name_fallback(normalized: &str, input: &Value) -> String {
    if normalized.is_empty() {
        return normalized.to_string();
    }
    let looks_like_batch = input.is_array()
        || input
            .get("paths")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false);
    let has_queryish_text = !clean_text(
        input.as_str().unwrap_or_else(|| {
            input.get("query")
                .or_else(|| input.get("message"))
                .or_else(|| input.get("prompt"))
                .or_else(|| input.get("objective"))
                .and_then(Value::as_str)
                .unwrap_or("")
        }),
        400,
    )
    .is_empty();
    if normalized.contains("batch") && normalized.contains("query") {
        return "batch_query".to_string();
    }
    if normalized.contains("search") || normalized.contains("web_query") {
        return "batch_query".to_string();
    }
    if (normalized.contains("compare")
        || normalized.contains("ranking")
        || normalized.contains("rank")
        || normalized.contains("peer")
        || normalized.contains("framework"))
        && (has_queryish_text
            || input
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("")
                .eq_ignore_ascii_case("web"))
    {
        return "batch_query".to_string();
    }
    if normalized.contains("browse")
        || normalized.contains("web_fetch")
        || normalized.contains("fetch_url")
        || normalized == "fetch"
        || normalized.contains("open_url")
        || normalized.contains("read_url")
    {
        return "web_fetch".to_string();
    }
    if normalized.contains("file") && (normalized.contains("read") || normalized.contains("open")) {
        return if looks_like_batch {
            "file_read_many".to_string()
        } else {
            "file_read".to_string()
        };
    }
    if normalized.contains("folder") && (normalized.contains("list") || normalized.contains("tree"))
    {
        return "folder_export".to_string();
    }
    if normalized == "workspace_analyze"
        || (normalized.contains("workspace")
            && (normalized.contains("analy")
                || normalized.contains("metric")
                || normalized.contains("stat")
                || normalized.contains("loc")))
    {
        return "terminal_exec".to_string();
    }
    if normalized.contains("terminal")
        || normalized.contains("shell")
        || normalized.contains("command_exec")
        || normalized.contains("run_command")
    {
        return "terminal_exec".to_string();
    }
    if normalized.contains("spawn") && normalized.contains("agent") {
        return "spawn_subagents".to_string();
    }
    normalized.to_string()
}

fn is_terminal_tool_name(normalized: &str) -> bool {
    matches!(
        normalized,
        "terminal_exec" | "run_terminal" | "terminal" | "shell_exec"
    )
}

fn input_text_hint_for_terminal_alias(input: &Value) -> String {
    clean_text(
        input
            .get("query")
            .or_else(|| input.get("objective"))
            .or_else(|| input.get("message"))
            .or_else(|| input.get("prompt"))
            .or_else(|| input.get("task"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    )
}

fn terminal_alias_command_for_tool(normalized_tool: &str, input: &Value) -> Option<String> {
    if normalized_tool == "workspace_analyze"
        || (normalized_tool.contains("workspace")
            && (normalized_tool.contains("analy")
                || normalized_tool.contains("metric")
                || normalized_tool.contains("stat")
                || normalized_tool.contains("loc")))
    {
        let hint = input_text_hint_for_terminal_alias(input).to_ascii_lowercase();
        if hint.contains("loc")
            || hint.contains("line count")
            || hint.contains("linecount")
            || hint.contains("lines of code")
            || hint.contains("effective loc")
            || hint.contains("effective lines")
        {
            return Some("git ls-files | xargs wc -l | tail -n 1".to_string());
        }
        return Some("infring workspace-search status --workspace=. --json".to_string());
    }
    None
}

#[cfg(test)]
mod tool_name_fallback_tests {
    use super::*;

    #[test]
    fn resolves_search_like_names_to_batch_query() {
        assert_eq!(
            resolve_tool_name_fallback("internet_search_now", &json!({"query": "status"})),
            "batch_query"
        );
    }

    #[test]
    fn resolves_compare_like_names_to_batch_query() {
        assert_eq!(
            resolve_tool_name_fallback(
                "framework_compare",
                &json!({"query": "top ai agent frameworks", "source": "web"})
            ),
            "batch_query"
        );
    }

    #[test]
    fn resolves_file_read_batch_from_paths_payload() {
        assert_eq!(
            resolve_tool_name_fallback("open_file_reader", &json!({"paths": ["README.md"]})),
            "file_read_many"
        );
    }

    #[test]
    fn resolves_workspace_analyze_names_to_terminal_exec() {
        assert_eq!(
            resolve_tool_name_fallback("workspace_analyze", &json!({"query":"effective loc"})),
            "terminal_exec"
        );
    }

    #[test]
    fn terminal_alias_prefers_loc_command_for_line_count_prompts() {
        let cmd =
            terminal_alias_command_for_tool("workspace_analyze", &json!({"query":"effective loc"}))
                .unwrap_or_default();
        assert!(cmd.contains("git ls-files"));
    }

    #[test]
    fn leaves_unmapped_names_unchanged() {
        assert_eq!(
            resolve_tool_name_fallback("memory_semantic_query", &json!({})),
            "memory_semantic_query"
        );
    }
}

fn find_json_object_span(raw: &str, from_index: usize) -> Option<(usize, usize)> {
    let mut start = None;
    for (idx, ch) in raw.char_indices().skip_while(|(idx, _)| *idx < from_index) {
        if ch == '{' {
            start = Some(idx);
            break;
        }
    }
    let start_idx = start?;
    let mut depth = 0i64;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in raw.char_indices().skip_while(|(idx, _)| *idx < start_idx) {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some((start_idx, idx + ch.len_utf8()));
            }
        }
    }
    None
}

fn extract_inline_tool_calls(
    text: &str,
    max_calls: usize,
) -> (String, Vec<(String, Value, String)>) {
    let mut calls = Vec::<(String, Value, String)>::new();
    let mut spans = Vec::<(usize, usize)>::new();
    let mut cursor = 0usize;
    let cap = max_calls.clamp(1, 12);

    while cursor < text.len() && calls.len() < cap {
        let next_open = text[cursor..].find("<function=").map(|idx| cursor + idx);
        let next_close = text[cursor..].find("</function>").map(|idx| cursor + idx);
        let next = match (next_open, next_close) {
            (Some(open), Some(close)) => Some(if open <= close {
                ("open", open)
            } else {
                ("close", close)
            }),
            (Some(open), None) => Some(("open", open)),
            (None, Some(close)) => Some(("close", close)),
            (None, None) => None,
        };
        let Some((kind, idx)) = next else {
            break;
        };
        if kind == "open" {
            let name_start = idx + "<function=".len();
            let Some(gt_rel) = text[name_start..].find('>') else {
                break;
            };
            let name_end = name_start + gt_rel;
            let raw_name = &text[name_start..name_end];
            let name = raw_name
                .chars()
                .take_while(|ch| tool_name_char(*ch))
                .collect::<String>();
            if name.is_empty() {
                cursor = name_end.saturating_add(1);
                continue;
            }
            let payload_start = name_end + 1;
            let Some((json_start, json_end)) = find_json_object_span(text, payload_start) else {
                cursor = payload_start;
                continue;
            };
            let parsed = serde_json::from_str::<Value>(&text[json_start..json_end]).ok();
            let Some(input) = parsed else {
                cursor = json_end;
                continue;
            };
            let tail = &text[json_end..];
            let full_end = tail
                .find("</function>")
                .map(|end| json_end + end + "</function>".len())
                .unwrap_or(json_end);
            let raw = text[idx..full_end].to_string();
            calls.push((name, input, raw));
            spans.push((idx, full_end));
            cursor = full_end;
            continue;
        }

        let close_idx = idx;
        let close_end = close_idx + "</function>".len();
        let prefix = &text[..close_idx];
        let mut back = prefix.len();
        while back > 0 {
            let ch = prefix[..back].chars().next_back().unwrap_or(' ');
            if tool_name_char(ch) {
                back -= ch.len_utf8();
            } else {
                break;
            }
        }
        let name = prefix[back..close_idx]
            .chars()
            .filter(|ch| tool_name_char(*ch))
            .collect::<String>();
        if name.is_empty() {
            cursor = close_end;
            continue;
        }
        let Some((json_start, json_end)) = find_json_object_span(text, close_end) else {
            cursor = close_end;
            continue;
        };
        let parsed = serde_json::from_str::<Value>(&text[json_start..json_end]).ok();
        let Some(input) = parsed else {
            cursor = json_end;
            continue;
        };
        let raw = text[back..json_end].to_string();
        calls.push((name, input, raw));
        spans.push((back, json_end));
        cursor = json_end;
    }

    if spans.is_empty() {
        return (text.to_string(), Vec::new());
    }
    spans.sort_by_key(|(start, _)| *start);
    let mut cleaned = String::new();
    let mut last = 0usize;
    for (start, end) in spans {
        if start > last {
            cleaned.push_str(&text[last..start]);
        }
        last = last.max(end);
    }
    if last < text.len() {
        cleaned.push_str(&text[last..]);
    }
    (cleaned.trim().to_string(), calls)
}

fn trim_text(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars.max(1)).collect::<String>()
}

fn tool_governance_policy(root: &Path) -> Value {
    let path = root.join("client/runtime/config/tool_governance_policy.json");
    let default = json!({
        "enabled": true,
        "tiers": {
            "green": {"confirm_required": false, "approval_note_min": 0},
            "yellow": {"confirm_required": true, "approval_note_min": 0},
            "red": {"confirm_required": true, "approval_note_min": 8}
        }
    });
    let mut merged = default.clone();
    if let Some(custom) = read_json_loose(&path) {
        if let Some(enabled) = custom.get("enabled").and_then(Value::as_bool) {
            merged["enabled"] = json!(enabled);
        }
        for tier in ["green", "yellow", "red"] {
            if let Some(confirm_required) = custom
                .pointer(&format!("/tiers/{tier}/confirm_required"))
                .and_then(Value::as_bool)
            {
                merged["tiers"][tier]["confirm_required"] = json!(confirm_required);
            }
            if let Some(min_note) = custom
                .pointer(&format!("/tiers/{tier}/approval_note_min"))
                .and_then(Value::as_i64)
            {
                merged["tiers"][tier]["approval_note_min"] = json!(min_note.max(0));
            }
        }
    }
    merged
}

fn input_has_confirmation(input: &Value) -> bool {
    input
        .get("confirm")
        .or_else(|| input.get("confirmed"))
        .or_else(|| input.get("approved"))
        .or_else(|| input.get("user_confirmed"))
        .or_else(|| input.get("signoff"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn input_approval_note(input: &Value) -> String {
    clean_text(
        input
            .get("approval_note")
            .or_else(|| input.get("note"))
            .or_else(|| input.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        400,
    )
}

fn tool_error_requires_confirmation(payload: &Value) -> bool {
    matches!(
        tool_error_text(payload).to_ascii_lowercase().as_str(),
        "tool_explicit_signoff_required" | "tool_confirmation_required"
    )
}

fn message_is_affirmative_confirmation(message: &str) -> bool {
    let lowered = clean_text(message, 200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let normalized = lowered
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    let collapsed = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if collapsed.is_empty() {
        return false;
    }
    let token_count = collapsed.split_whitespace().count();
    if token_count > 12 {
        return false;
    }
    matches!(
        collapsed.as_str(),
        "y" | "yes"
            | "yeah"
            | "yep"
            | "ok"
            | "okay"
            | "confirm"
            | "confirmed"
            | "do it"
            | "go ahead"
            | "proceed"
            | "run it"
            | "execute"
            | "execute it"
            | "please do"
            | "please proceed"
            | "yes please"
            | "yes do it"
    ) || collapsed.starts_with("yes ")
        || collapsed.starts_with("confirm ")
}

fn message_is_negative_confirmation(message: &str) -> bool {
    let lowered = clean_text(message, 200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let normalized = lowered
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    let collapsed = normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    matches!(
        collapsed.as_str(),
        "n" | "no"
            | "cancel"
            | "stop"
            | "skip"
            | "dont"
            | "do not"
            | "no thanks"
            | "never mind"
            | "nevermind"
            | "abort"
    ) || collapsed.starts_with("cancel ")
        || collapsed.starts_with("no ")
}

fn pending_tool_confirmation_payload(root: &Path, agent_id: &str) -> Option<Value> {
    let id = clean_agent_id(agent_id);
    if id.is_empty() {
        return None;
    }
    profiles_map(root)
        .get(&id)
        .and_then(|row| row.get("pending_tool_confirmation"))
        .and_then(|value| {
            if value.is_object() {
                Some(value.clone())
            } else {
                None
            }
        })
}

fn pending_tool_confirmation_call(root: &Path, agent_id: &str) -> Option<(String, Value)> {
    let payload = pending_tool_confirmation_payload(root, agent_id)?;
    let tool_name = normalize_tool_name(&clean_text(
        payload
            .get("tool")
            .or_else(|| payload.get("tool_name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        120,
    ));
    if tool_name.is_empty() {
        return None;
    }
    let input = payload.get("input").cloned().unwrap_or_else(|| json!({}));
    Some((tool_name, input))
}
