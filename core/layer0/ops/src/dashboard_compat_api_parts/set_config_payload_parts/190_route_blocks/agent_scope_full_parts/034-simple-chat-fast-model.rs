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
