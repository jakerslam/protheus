fn provider_available_for_metadata_research(root: &Path, provider_id: &str) -> bool {
    let provider = normalize_provider_id(provider_id);
    if provider.is_empty() {
        return false;
    }
    if provider_is_local(&provider) {
        let providers = providers_payload(root, &json!({}));
        return providers
            .get("providers")
            .and_then(Value::as_array)
            .and_then(|rows| {
                rows.iter()
                    .find(|row| {
                        normalize_provider_id(
                            row.get("id").and_then(Value::as_str).unwrap_or(""),
                        ) == provider
                    })
                    .and_then(|row| row.get("reachable").and_then(Value::as_bool))
            })
            .unwrap_or(false);
    }
    provider_key(root, &provider).is_some()
}

fn profile_needs_metadata_research(profile: &Value) -> bool {
    let power = profile
        .get("power_rating")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let cost = profile
        .get("cost_rating")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let param = profile
        .get("param_count_billion")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let context = profile
        .get("context_window")
        .or_else(|| profile.get("context_window_tokens"))
        .or_else(|| profile.get("context_size"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let max_output = profile
        .get("max_output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let specialty = clean_text(
        profile
            .get("specialty")
            .and_then(Value::as_str)
            .unwrap_or("general"),
        40,
    )
    .to_ascii_lowercase();
    let tags_general_only =
        profile_tags_are_general_only(profile.get("specialty_tags").unwrap_or(&Value::Null));
    context == 0
        || max_output == 0
        || (power <= 3 && cost <= 3 && param == 0 && specialty == "general" && tags_general_only)
}

fn parse_first_json_object(raw: &str) -> Option<Value> {
    let text = clean_text(raw, 24_000);
    if text.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(&text) {
        if value.is_object() {
            return Some(value);
        }
    }
    let bytes = text.as_bytes();
    let mut depth = 0i64;
    let mut start = None::<usize>;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, byte) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if *byte == b'\\' {
                escaped = true;
                continue;
            }
            if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        if *byte == b'"' {
            in_string = true;
            continue;
        }
        if *byte == b'{' {
            if depth == 0 {
                start = Some(idx);
            }
            depth += 1;
            continue;
        }
        if *byte == b'}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                if let Some(begin) = start {
                    if let Ok(value) = serde_json::from_str::<Value>(&text[begin..=idx]) {
                        if value.is_object() {
                            return Some(value);
                        }
                    }
                }
                start = None;
            }
        }
    }
    None
}

fn normalize_researched_profile(
    base: &Value,
    researched: &Value,
    force_local: bool,
) -> Option<Value> {
    let mut merged = base.as_object().cloned().unwrap_or_default();
    if merged.is_empty() {
        return None;
    }
    let value_to_i64 = |value: Option<&Value>| -> Option<i64> {
        if let Some(raw) = value.and_then(Value::as_i64) {
            return Some(raw);
        }
        if let Some(raw) = value.and_then(Value::as_f64) {
            if raw.is_finite() {
                return Some(raw.round() as i64);
            }
        }
        value
            .and_then(Value::as_str)
            .and_then(|raw| clean_text(raw, 40).parse::<i64>().ok())
    };
    let mut touched = false;
    if let Some(power) = value_to_i64(researched.get("power_rating")) {
        let clamped = power.clamp(1, 5);
        if merged.get("power_rating").and_then(Value::as_i64).unwrap_or(0) != clamped {
            merged.insert("power_rating".to_string(), json!(clamped));
            touched = true;
        }
    }
    if let Some(cost) = value_to_i64(researched.get("cost_rating")) {
        let clamped = cost.clamp(1, 5);
        if merged.get("cost_rating").and_then(Value::as_i64).unwrap_or(0) != clamped {
            merged.insert("cost_rating".to_string(), json!(clamped));
            touched = true;
        }
    }
    if let Some(param) = value_to_i64(researched.get("param_count_billion")) {
        let clamped = param.clamp(0, 4_000);
        if clamped > 0
            && merged
                .get("param_count_billion")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                != clamped
        {
            merged.insert("param_count_billion".to_string(), json!(clamped));
            touched = true;
        }
    }
    if let Some(context) = value_to_i64(
        researched
            .get("context_window")
            .or_else(|| researched.get("context_window_tokens"))
            .or_else(|| researched.get("context_size")),
    ) {
        let clamped = context.clamp(1024, 4_194_304);
        if merged
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            != clamped
        {
            merged.insert("context_window".to_string(), json!(clamped));
            touched = true;
        }
    }
    if let Some(max_output) = value_to_i64(researched.get("max_output_tokens")) {
        let clamped = max_output.clamp(1, 1_048_576);
        if merged
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            != clamped
        {
            merged.insert("max_output_tokens".to_string(), json!(clamped));
            touched = true;
        }
    }
    let specialty = clean_text(
        researched
            .get("specialty")
            .and_then(Value::as_str)
            .unwrap_or(""),
        40,
    )
    .to_ascii_lowercase();
    if !specialty.is_empty() {
        let allowed = [
            "general",
            "reasoning",
            "coding",
            "vision",
            "speed",
            "audio",
            "multimodal",
            "search",
        ];
        if allowed.iter().any(|row| *row == specialty) {
            if merged
                .get("specialty")
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 40).to_ascii_lowercase())
                .unwrap_or_default()
                != specialty
            {
                merged.insert("specialty".to_string(), json!(specialty));
                touched = true;
            }
        }
    }
    if let Some(tags) = researched.get("specialty_tags").and_then(Value::as_array) {
        let normalized = tags
            .iter()
            .filter_map(Value::as_str)
            .map(|raw| clean_text(raw, 40).to_ascii_lowercase())
            .filter(|raw| !raw.is_empty())
            .take(8)
            .collect::<Vec<_>>();
        if !normalized.is_empty() {
            merged.insert("specialty_tags".to_string(), json!(normalized));
            touched = true;
        }
    }
    if force_local {
        merged.insert("deployment_kind".to_string(), json!("local"));
        touched = true;
    }
    if !touched {
        return None;
    }
    merged.insert("metadata_researched".to_string(), json!(true));
    merged.insert("metadata_researched_at".to_string(), json!(crate::now_iso()));
    Some(Value::Object(merged))
}

fn research_model_profile_overlay(
    root: &Path,
    provider_id: &str,
    model_id: &str,
    base_profile: &Value,
) -> Option<Value> {
    let route = crate::dashboard_model_catalog::route_decision_payload(
        root,
        &json!({}),
        &json!({
            "task_type": "model metadata research",
            "complexity": "high",
            "budget_mode": "balanced",
            "prefer_local": false
        }),
    );
    let research_provider = clean_text(
        route
            .pointer("/route/provider")
            .and_then(Value::as_str)
            .unwrap_or(""),
        80,
    );
    let research_model = clean_text(
        route
            .pointer("/route/model")
            .and_then(Value::as_str)
            .unwrap_or(""),
        240,
    );
    if research_provider.is_empty() || research_model.is_empty() {
        return None;
    }
    if !provider_available_for_metadata_research(root, &research_provider) {
        return None;
    }
    let system_prompt = "You are a model metadata researcher. Return only JSON object with keys: power_rating (1-5), cost_rating (1-5), context_window, max_output_tokens, param_count_billion, specialty, specialty_tags (array), confidence (0-1). If unknown, omit that key.";
    let user_prompt = format!(
        "Research metadata for provider `{}` model `{}`. Current profile: {}. Return JSON only.",
        clean_text(provider_id, 80),
        clean_text(model_id, 240),
        serde_json::to_string(base_profile).unwrap_or_else(|_| "{}".to_string())
    );
    let response = invoke_chat(
        root,
        &research_provider,
        &research_model,
        system_prompt,
        &[],
        &user_prompt,
    )
    .ok()?;
    let raw = response.get("response").and_then(Value::as_str).unwrap_or("");
    let researched = parse_first_json_object(raw)?;
    normalize_researched_profile(
        base_profile,
        &researched,
        provider_is_local(&normalize_provider_id(provider_id)),
    )
}

pub fn ensure_model_profile(root: &Path, provider_id: &str, model_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut model = clean_text(model_id, 240);
    if provider.is_empty() || model.is_empty() {
        return json!({"ok": false, "error": "model_profile_ref_invalid"});
    }
    if model.contains('/') && provider != "openrouter" {
        let mut parts = model.splitn(2, '/');
        let maybe_provider = normalize_provider_id(parts.next().unwrap_or(""));
        let maybe_model = clean_text(parts.next().unwrap_or(""), 200);
        if !maybe_provider.is_empty() && !maybe_model.is_empty() {
            model = maybe_model;
        }
    }

    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    if !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
        row["model_profiles"] = json!({});
    }
    let current = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&model))
        .cloned()
        .unwrap_or(Value::Null);
    let mut next =
        enrich_single_model_profile(&provider, &model, &current, provider_is_local(&provider));
    let mut metadata_researched = false;
    if profile_needs_metadata_research(&next) {
        if let Some(researched) = research_model_profile_overlay(root, &provider, &model, &next) {
            next = researched;
            metadata_researched = true;
        }
    }
    if let Some(obj) = next.as_object_mut() {
        if obj
            .get("max_output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            <= 0
        {
            let context = obj
                .get("context_window")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            if context > 0 {
                obj.insert(
                    "max_output_tokens".to_string(),
                    json!((context / 8).clamp(1024, 131_072)),
                );
            }
        }
        obj.insert("updated_at".to_string(), json!(crate::now_iso()));
    }

    let changed = current != next;
    row["model_profiles"][model.clone()] = next.clone();
    if !row.get("detected_models").map(Value::is_array).unwrap_or(false) {
        row["detected_models"] = json!([]);
    }
    let detected = row
        .get_mut("detected_models")
        .and_then(Value::as_array_mut)
        .expect("detected_models");
    if !detected
        .iter()
        .filter_map(Value::as_str)
        .any(|entry| clean_text(entry, 240) == model)
    {
        detected.push(json!(model.clone()));
    }
    if changed {
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
    }
    json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "changed": changed,
        "metadata_researched": metadata_researched,
        "profile": next
    })
}
