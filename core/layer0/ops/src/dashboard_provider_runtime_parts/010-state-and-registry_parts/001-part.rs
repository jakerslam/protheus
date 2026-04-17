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

fn enrich_single_model_profile(
    provider_id: &str,
    model_id: &str,
    profile: &Value,
    force_local: bool,
) -> Value {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_id, 240).to_ascii_lowercase();
    let enforce_context_floor =
        provider == "moonshot" || model.contains("moonshot") || model.contains("kimi");
    let inferred = inferred_model_profile(provider_id, model_id, force_local);
    let Some(mut merged) = profile.as_object().cloned() else {
        return inferred;
    };
    let inferred_obj = inferred.as_object().cloned().unwrap_or_default();
    let inferred_power = inferred_obj
        .get("power_rating")
        .and_then(Value::as_i64)
        .unwrap_or(3);
    let inferred_cost = inferred_obj
        .get("cost_rating")
        .and_then(Value::as_i64)
        .unwrap_or(3);
    let inferred_param = inferred_obj
        .get("param_count_billion")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let inferred_context = inferred_obj
        .get("context_window")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let inferred_specialty = inferred_obj
        .get("specialty")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 40).to_ascii_lowercase())
        .unwrap_or_else(|| "general".to_string());
    let inferred_tags = inferred_obj
        .get("specialty_tags")
        .cloned()
        .unwrap_or_else(|| json!(["general"]));

    let current_power = merged
        .get("power_rating")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let current_cost = merged
        .get("cost_rating")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let current_param = merged
        .get("param_count_billion")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let current_context = merged
        .get("context_window")
        .or_else(|| merged.get("context_window_tokens"))
        .or_else(|| merged.get("context_size"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let current_specialty = merged
        .get("specialty")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 40).to_ascii_lowercase())
        .unwrap_or_else(|| "general".to_string());
    let current_tags_general_only =
        profile_tags_are_general_only(merged.get("specialty_tags").unwrap_or(&Value::Null));
    let generic_profile = current_power == 3
        && current_specialty == "general"
        && current_tags_general_only
        && current_param == 0;

    if current_power == 0 || (generic_profile && inferred_power > current_power) {
        merged.insert("power_rating".to_string(), json!(inferred_power.max(1)));
    }
    if current_cost == 0 || (generic_profile && inferred_cost != current_cost) {
        merged.insert("cost_rating".to_string(), json!(inferred_cost.max(1)));
    }
    if current_param == 0 && inferred_param > 0 {
        merged.insert("param_count_billion".to_string(), json!(inferred_param));
    }
    if inferred_context > 0
        && (current_context == 0 || (enforce_context_floor && current_context < inferred_context))
    {
        merged.insert("context_window".to_string(), json!(inferred_context));
    }
    if (current_specialty.is_empty() || current_specialty == "general")
        && inferred_specialty != "general"
    {
        merged.insert("specialty".to_string(), json!(inferred_specialty));
    }
    if current_tags_general_only {
        merged.insert("specialty_tags".to_string(), inferred_tags);
    }
    if merged
        .get("deployment_kind")
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 30).is_empty())
        .unwrap_or(true)
    {
        let inferred_deployment = inferred_obj
            .get("deployment_kind")
            .cloned()
            .unwrap_or_else(|| json!(if force_local { "local" } else { "api" }));
        merged.insert("deployment_kind".to_string(), inferred_deployment);
    }

    Value::Object(merged)
}

fn enrich_model_profiles_for_provider(
    provider_id: &str,
    profiles: &mut Map<String, Value>,
) -> bool {
    let mut changed = false;
    let force_local = provider_is_local(provider_id);
    let model_ids = profiles.keys().cloned().collect::<Vec<_>>();
    for model_id in model_ids {
        let current = profiles.get(&model_id).cloned().unwrap_or(Value::Null);
        let next = enrich_single_model_profile(provider_id, &model_id, &current, force_local);
        if next != current {
            profiles.insert(model_id, next);
            changed = true;
        }
    }
    changed
}

fn ensure_provider_row_mut<'a>(registry: &'a mut Value, provider_id: &str) -> &'a mut Value {
    if !registry.is_object() {
        *registry = json!({});
    }
    if registry.get("providers").is_none()
        || !registry
            .get("providers")
            .map(Value::is_object)
            .unwrap_or(false)
    {
        registry["providers"] = json!({});
    }
    let providers = registry
        .get_mut("providers")
        .and_then(Value::as_object_mut)
        .expect("providers");
    providers.entry(provider_id.to_string()).or_insert_with(|| {
        json!({
            "id": provider_id,
            "display_name": provider_display_name(provider_id),
            "is_local": provider_is_local(provider_id),
            "needs_key": provider_needs_key(provider_id),
            "auth_status": if provider_is_local(provider_id) { "configured" } else { "not_set" },
            "base_url": provider_base_url_default(provider_id),
            "api_key_env": provider_api_key_env(provider_id),
            "key_prefix": "",
            "key_last4": "",
            "key_hash": "",
            "key_set_at": "",
            "reachable": provider_is_local(provider_id),
            "detected_models": [],
            "local_model_root": "",
            "local_model_paths": [],
            "model_profiles": model_profiles_for_provider(provider_id),
            "updated_at": crate::now_iso()
        })
    })
}

fn provider_row(root: &Path, provider_id: &str) -> Value {
    let registry = load_registry(root);
    let id = normalize_provider_id(provider_id);
    let mut row = registry
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&id))
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "id": id,
                "display_name": provider_display_name(provider_id),
                "is_local": provider_is_local(provider_id),
                "needs_key": provider_needs_key(provider_id),
                "auth_status": if provider_is_local(provider_id) { "configured" } else { "not_set" },
                "base_url": provider_base_url_default(provider_id),
                "api_key_env": provider_api_key_env(provider_id),
                "reachable": provider_is_local(provider_id),
                "detected_models": [],
                "model_profiles": model_profiles_for_provider(provider_id),
                "updated_at": crate::now_iso()
            })
        });
    let mut profiles = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if profiles.is_empty() {
        profiles = model_profiles_for_provider(&id);
    }
    if !profiles.is_empty() {
        let _ = enrich_model_profiles_for_provider(&id, &mut profiles);
        row["model_profiles"] = Value::Object(profiles);
    }
    row
}

fn masked_prefix(key: &str) -> String {
    clean_text(&key.chars().take(6).collect::<String>(), 8)
}

fn masked_last4(key: &str) -> String {
    let chars = key.chars().collect::<Vec<_>>();
    if chars.len() <= 4 {
        clean_text(key, 8)
    } else {
        chars[chars.len() - 4..].iter().collect::<String>()
