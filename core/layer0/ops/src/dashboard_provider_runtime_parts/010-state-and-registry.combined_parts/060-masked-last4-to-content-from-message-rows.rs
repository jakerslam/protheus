
fn masked_last4(key: &str) -> String {
    let chars = key.chars().collect::<Vec<_>>();
    if chars.len() <= 4 {
        clean_text(key, 8)
    } else {
        chars[chars.len() - 4..].iter().collect::<String>()
    }
}

fn guess_provider_from_key(raw: &str) -> String {
    let key = clean_text(raw, 512);
    if key.starts_with("sk-ant-") {
        return "frontier_provider".to_string();
    }
    if key.starts_with("gsk_") || key.starts_with("gsk-") {
        return "groq".to_string();
    }
    if key.starts_with("AIza") {
        return "google".to_string();
    }
    if key.starts_with("sk-or-v1-") {
        return "openrouter".to_string();
    }
    if key.starts_with("xai-") {
        return "xai".to_string();
    }
    if key.starts_with("sk-") {
        return "openai".to_string();
    }
    "openai".to_string()
}

fn content_from_message_rows(rows: &[Value]) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|row| {
            let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 40)
                .to_ascii_lowercase();
            let text = clean_text(
                row.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| row.get("content").and_then(Value::as_str))
                    .unwrap_or(""),
                16_000,
            );
            if role.is_empty() || text.is_empty() {
                None
            } else {
                Some((role, text))
            }
        })
        .collect()
}
