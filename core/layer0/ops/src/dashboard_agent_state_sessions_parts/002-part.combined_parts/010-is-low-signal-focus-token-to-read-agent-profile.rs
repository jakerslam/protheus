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
