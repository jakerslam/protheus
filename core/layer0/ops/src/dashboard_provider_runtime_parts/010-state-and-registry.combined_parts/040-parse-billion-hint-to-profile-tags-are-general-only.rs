fn parse_billion_hint(model_id: &str) -> i64 {
    let lower = model_id.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut best = 0i64;
    for idx in 1..bytes.len() {
        let unit = bytes[idx];
        if unit != b'b' && unit != b't' {
            continue;
        }
        let mut start = idx;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start == idx {
            continue;
        }
        if let Ok(raw) = lower[start..idx].parse::<i64>() {
            if raw <= 0 {
                continue;
            }
            let scaled = if unit == b't' {
                raw.saturating_mul(1000)
            } else {
                raw
            };
            if scaled > best {
                best = scaled;
            }
        }
    }
    best
}

fn infer_model_context_window(provider_id: &str, model_id: &str) -> i64 {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_id, 240).to_ascii_lowercase();
    if provider == "google" || model.contains("gemini-2.5") {
        return 1_048_576;
    }
    if provider == "moonshot" || model.contains("kimi") {
        return 262_144;
    }
    if model.contains("claude") {
        return 200_000;
    }
    if model.contains("qwen") || model.contains("llama") || model.contains("mixtral") {
        return 131_072;
    }
    if model.contains("deepseek") {
        return 65_536;
    }
    if provider_is_local(&provider) {
        return 131_072;
    }
    0
}

fn infer_model_specialty_and_tags(model_id: &str) -> (String, Vec<String>) {
    let model = clean_text(model_id, 240).to_ascii_lowercase();
    let mut specialty = "general".to_string();
    let mut tags = vec!["general".to_string()];
    let mut add_tag = |value: &str| {
        if !tags.iter().any(|row| row == value) {
            tags.push(value.to_string());
        }
    };

    if model.contains("thinking")
        || model.contains("reason")
        || model.contains("-r1")
        || model.contains("o1")
        || model.contains("o3")
    {
        specialty = "reasoning".to_string();
        add_tag("reasoning");
    }
    if model.contains("coder") || model.contains("code") {
        if specialty == "general" {
            specialty = "coding".to_string();
        }
        add_tag("coding");
    }
    if model.contains("vision")
        || model.contains("vl")
        || model.contains("multimodal")
        || model.contains("image")
    {
        if specialty == "general" {
            specialty = "vision".to_string();
        }
        add_tag("vision");
    }
    if model.contains("flash")
        || model.contains("instant")
        || model.contains("mini")
        || model.contains("nano")
        || model.contains("small")
        || model.contains("lite")
    {
        if specialty == "general" {
            specialty = "speed".to_string();
        }
        add_tag("speed");
    }

    (specialty, tags)
}

fn inferred_model_profile(provider_id: &str, model_id: &str, force_local: bool) -> Value {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_id, 240).to_ascii_lowercase();
    let is_local = force_local || provider_is_local(&provider);
    let param_count_billion = parse_billion_hint(&model);
    let (specialty, specialty_tags) = infer_model_specialty_and_tags(&model);
    let mut power_rating = 3i64;
    if model.contains("kimi-k2.5")
        || model.contains("kimi-k2-thinking")
        || model.contains("kimi-k2")
        || model.contains("gpt-5")
        || model.contains("claude-opus")
        || model.contains("reasoner")
        || model.contains("-r1")
        || model.contains("thinking")
        || model.contains("deepseek-r1")
    {
        power_rating = 5;
    } else if model.contains("pro")
        || model.contains("sonnet")
        || model.contains("70b")
        || model.contains("72b")
        || model.contains("32b")
        || model.contains("34b")
    {
        power_rating = 4;
    } else if model.contains("flash")
        || model.contains("instant")
        || model.contains("mini")
        || model.contains("haiku")
        || model.contains("8b")
        || model.contains("7b")
        || model.contains("4b")
        || model.contains("3b")
        || model.contains("2b")
        || model.contains("1b")
        || model.contains("small")
        || model.contains("nano")
        || model.contains("tiny")
    {
        power_rating = 2;
    }
    if param_count_billion >= 200 {
        power_rating = power_rating.max(5);
    } else if param_count_billion >= 60 {
        power_rating = power_rating.max(4);
    }

    let mut cost_rating = if is_local { 1 } else { 3 };
    if !is_local {
        if power_rating >= 5 {
            cost_rating = 4;
        } else if model.contains("flash")
            || model.contains("mini")
            || model.contains("instant")
            || model.contains("haiku")
            || model.contains("nano")
        {
            cost_rating = 2;
        }
    }

    let deployment_kind = if model.contains(":cloud") || model.ends_with("-cloud") {
        "cloud"
    } else if is_local {
        "local"
    } else {
        "api"
    };

    json!({
        "power_rating": power_rating,
        "cost_rating": cost_rating,
        "param_count_billion": param_count_billion.max(0),
        "specialty": specialty,
        "specialty_tags": specialty_tags,
        "deployment_kind": deployment_kind,
        "context_window": infer_model_context_window(&provider, &model)
    })
}

fn profile_tags_are_general_only(value: &Value) -> bool {
    let Some(rows) = value.as_array() else {
        return true;
    };
    if rows.is_empty() {
        return true;
    }
    rows.iter()
        .filter_map(Value::as_str)
        .map(|raw| clean_text(raw, 40).to_ascii_lowercase())
        .all(|tag| tag == "general" || tag.is_empty())
}
