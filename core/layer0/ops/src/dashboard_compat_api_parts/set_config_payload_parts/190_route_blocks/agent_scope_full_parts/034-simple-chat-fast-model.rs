fn split_fast_chat_model_ref(raw: &str) -> Option<(String, String)> {
    let cleaned = clean_text(raw, 260);
    let (provider, model) = cleaned.split_once('/')?;
    let provider = clean_text(provider, 80);
    let model = clean_text(model, 240);
    if provider.is_empty() || model.is_empty() {
        None
    } else {
        Some((provider, model))
    }
}

fn simple_direct_chat_fast_model_candidates() -> Vec<String> {
    std::env::var("INFRING_SIMPLE_CHAT_FAST_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .split(',')
                .map(|row| clean_text(row, 260))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| {
            vec![
                "ollama/qwen2.5:3b-instruct-q4_K_M".to_string(),
                "ollama/qwen2.5:3b".to_string(),
                "ollama/llama3.2:latest".to_string(),
                "ollama/phi:latest".to_string(),
                "ollama/tinyllama:latest".to_string(),
                "openai/gpt-5-mini".to_string(),
            ]
        })
}

fn simple_direct_chat_model_allows_visible_chat(model_ref: &str) -> bool {
    let lowered = model_ref.to_ascii_lowercase();
    ![
        "think",
        "reason",
        "qwq",
        "deepseek-r1",
        "r1:",
        "r1-",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn visible_response_fast_model_route(
    root: &Path,
    snapshot: &Value,
    current_provider: &str,
    current_model: &str,
    reason: &str,
) -> Option<(String, String, Value)> {
    let current_provider = clean_text(current_provider, 80);
    let current_model = clean_text(current_model, 240);
    let reason = clean_text(reason, 120);
    for candidate in simple_direct_chat_fast_model_candidates() {
        let Some((provider, model)) = split_fast_chat_model_ref(&candidate) else {
            continue;
        };
        if !simple_direct_chat_model_allows_visible_chat(&format!("{provider}/{model}")) {
            continue;
        }
        if provider.eq_ignore_ascii_case(&current_provider) && model == current_model {
            return None;
        }
        if !crate::dashboard_model_catalog::model_ref_available(root, snapshot, &provider, &model)
        {
            continue;
        }
        let route = json!({
            "route": {
                "provider": provider,
                "model": model,
                "reason": reason,
                "source": "dashboard_agent_message_route",
                "previous_provider": current_provider,
                "previous_model": current_model
            }
        });
        return Some((provider, model, route));
    }
    None
}

fn simple_direct_chat_fast_model_route(
    root: &Path,
    snapshot: &Value,
    message: &str,
    inline_tools_allowed: bool,
    current_provider: &str,
    current_model: &str,
) -> Option<(String, String, Value)> {
    if !simple_direct_chat_suppresses_passive_context(message, inline_tools_allowed)
        || !workflow_turn_is_simple_conversation_without_tool_intent(message)
    {
        return None;
    }

    visible_response_fast_model_route(
        root,
        snapshot,
        current_provider,
        current_model,
        "simple_direct_chat_fast_model",
    )
}
