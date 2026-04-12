fn media_tool_key_alias(key: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in key.chars() {
        if ch == '_' || ch == '-' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn media_tool_param_value<'a>(params: &'a Value, key: &str) -> Option<&'a Value> {
    let alias = media_tool_key_alias(key);
    params.get(key).or_else(|| params.get(alias.as_str()))
}

fn media_tool_read_boolean_param(params: &Value, key: &str) -> Option<bool> {
    let raw = media_tool_param_value(params, key)?;
    if let Some(value) = raw.as_bool() {
        return Some(value);
    }
    raw.as_str().and_then(|text| match clean_text(text, 16).to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    })
}

fn media_tool_request_string_list(params: &Value, key: &str, max_len: usize) -> Vec<String> {
    match media_tool_param_value(params, key) {
        Some(Value::Array(rows)) => rows
            .iter()
            .filter_map(Value::as_str)
            .map(|row| clean_text(row, max_len))
            .filter(|row| !row.is_empty())
            .collect(),
        Some(Value::String(row)) => {
            let clean = clean_text(row, max_len);
            if clean.is_empty() {
                Vec::new()
            } else {
                vec![clean]
            }
        }
        _ => Vec::new(),
    }
}

fn normalize_media_reference_candidates(
    candidates: Vec<String>,
    max_count: usize,
    label: &str,
) -> Result<Vec<String>, Value> {
    let mut deduped = Vec::<String>::new();
    let mut seen = std::collections::BTreeSet::<String>::new();
    for candidate in candidates {
        let trimmed = candidate.trim().to_string();
        let dedupe_key = trimmed.trim_start_matches('@').trim().to_string();
        if dedupe_key.is_empty() || seen.contains(&dedupe_key) {
            continue;
        }
        seen.insert(dedupe_key);
        deduped.push(trimmed);
    }
    if deduped.len() > max_count {
        return Err(json!({
            "ok": false,
            "error": "too_many_media_references",
            "label": clean_text(label, 80),
            "count": deduped.len(),
            "max": max_count
        }));
    }
    Ok(deduped)
}

fn normalize_media_reference_inputs(
    params: &Value,
    singular_key: &str,
    plural_key: &str,
    max_count: usize,
    label: &str,
) -> Result<Vec<String>, Value> {
    let mut combined = Vec::<String>::new();
    if let Some(single) = media_tool_param_value(params, singular_key).and_then(Value::as_str) {
        let clean = clean_text(single, 4000);
        if !clean.is_empty() {
            combined.push(clean);
        }
    }
    combined.extend(media_tool_request_string_list(params, plural_key, 4000));
    normalize_media_reference_candidates(combined, max_count, label)
}

fn resolve_media_tool_local_root_patterns(
    root: &Path,
    workspace_dir_raw: Option<&str>,
    workspace_only: bool,
    _media_sources: &[String],
) -> Vec<String> {
    let workspace_dir = workspace_dir_raw
        .map(|row| clean_text(row, 2200))
        .filter(|row| !row.is_empty())
        .map(|row| {
            let expanded = media_expand_user_path(&row);
            if expanded.is_absolute() {
                expanded
            } else {
                root.join(expanded)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    if workspace_only {
        return vec![workspace_dir.display().to_string()];
    }
    media_default_local_root_patterns(root, &workspace_dir, &json!({}))
}

fn resolve_media_tool_prompt_and_model_override(
    params: &Value,
    default_prompt: &str,
) -> (String, Option<String>) {
    let prompt = normalize_request_string(params, "prompt", &[], 4000);
    let model = normalize_request_string(params, "model", &[], 240);
    (
        if prompt.is_empty() {
            default_prompt.to_string()
        } else {
            prompt
        },
        if model.is_empty() { None } else { Some(model) },
    )
}

fn build_media_task_run_details(task_id: &str, run_id: &str) -> Value {
    json!({
        "task": {
            "taskId": clean_text(task_id, 120),
            "runId": clean_text(run_id, 120)
        }
    })
}

fn web_media_tool_shared_contract() -> Value {
    json!({
        "boolean_param_contract": {
            "accepts_boolean_literals": true,
            "accepts_string_literals": ["true", "false"],
            "snake_case_reads_camel_case_alias": true
        },
        "media_reference_contract": {
            "dedupe_rule": "trim_and_dedupe_with_leading_at_ignored_for_identity",
            "max_count_error": "too_many_media_references",
            "supported_single_fields": ["image", "video", "audio_ref", "pdf", "path", "url"],
            "supported_plural_fields": ["images", "videos", "audio_refs", "pdfs", "sources"]
        },
        "local_root_resolution_contract": {
            "workspace_only_supported": true,
            "media_sources_expand_roots": false,
            "default_local_root_suffixes": media_default_local_root_suffixes()
        },
        "prompt_model_override_contract": {
            "prompt_field": "prompt",
            "model_override_field": "model",
            "default_prompt_strategy": "caller_defined_default_when_prompt_empty"
        },
        "task_run_details_contract": {
            "shape": ["task.taskId", "task.runId"]
        },
        "default_model_config_contract": {
            "keys": [
                "agents.defaults.imageModel",
                "agents.defaults.imageGenerationModel",
                "agents.defaults.videoGenerationModel",
                "agents.defaults.musicGenerationModel"
            ]
        }
    })
}
