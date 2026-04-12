const DEFAULT_IMAGE_TOOL_PROMPT: &str = "Describe the image.";
const DEFAULT_IMAGE_TOOL_MAX_IMAGES: u64 = 20;
const DEFAULT_IMAGE_TOOL_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS: u64 = 60;
const DEFAULT_IMAGE_TOOL_OUTPUT_MAX_BUFFER_BYTES: u64 = 5 * 1024 * 1024;
const DEFAULT_IMAGE_TOOL_MEDIA_CONCURRENCY: u64 = 2;

fn normalize_image_tool_provider_id(raw: &str) -> String {
    match clean_text(raw, 80).to_ascii_lowercase().as_str() {
        "anthropic" | "claude" | "frontier-provider" => "frontier_provider".to_string(),
        "gemini" | "google-ai" => "google".to_string(),
        "moonshot-ai" | "kimi" => "moonshot".to_string(),
        "openai-codex" | "openai-codex-responses" => "openai".to_string(),
        other => other.to_string(),
    }
}

fn image_tool_provider_priority(provider: &str) -> Option<u64> {
    match normalize_image_tool_provider_id(provider).as_str() {
        "openai" => Some(10),
        "frontier_provider" => Some(20),
        "google" => Some(30),
        "minimax" => Some(40),
        "minimax-portal" => Some(50),
        "zai" => Some(60),
        _ => None,
    }
}

fn image_tool_preferred_models(provider: &str) -> &'static [&'static str] {
    match normalize_image_tool_provider_id(provider).as_str() {
        "openai" => &["gpt-4o", "gpt-5.4-mini", "gpt-5-mini", "gpt-5"],
        "frontier_provider" => &[
            "claude-opus-4-20250514",
            "claude-3-7-sonnet-latest",
            "claude-sonnet-4-20250514",
        ],
        "google" => &["gemini-2.5-flash", "gemini-2.5-pro"],
        "openrouter" => &["google/gemini-2.5-flash", "google/gemini-2.5-pro", "auto"],
        "ollama" => &["qwen3-vl:235b-cloud"],
        _ => &[],
    }
}

fn image_tool_model_has_vision(model_id: &str, profile: &Value) -> bool {
    if profile
        .get("specialty")
        .and_then(Value::as_str)
        .map(|row| clean_text(row, 40).eq_ignore_ascii_case("vision"))
        .unwrap_or(false)
    {
        return true;
    }
    if profile
        .get("specialty_tags")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter().any(|row| {
                row.as_str()
                    .map(|text| clean_text(text, 40).eq_ignore_ascii_case("vision"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
    {
        return true;
    }
    let normalized = clean_text(model_id, 240).to_ascii_lowercase();
    normalized.contains("vision")
        || normalized.contains("-vl")
        || normalized.contains("multimodal")
        || normalized.contains("image")
}

fn image_tool_model_rows(row: &Value) -> Vec<Value> {
    let mut rows = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(|(model_id, profile)| {
                    if !image_tool_model_has_vision(model_id, profile) {
                        return None;
                    }
                    Some(json!({
                        "id": clean_text(model_id, 240),
                        "specialty": clean_text(profile.get("specialty").and_then(Value::as_str).unwrap_or(""), 40),
                        "specialty_tags": profile.get("specialty_tags").cloned().unwrap_or_else(|| json!([])),
                        "power_rating": profile.get("power_rating").and_then(Value::as_i64).unwrap_or(0),
                        "cost_rating": profile.get("cost_rating").and_then(Value::as_i64).unwrap_or(0),
                        "context_window": profile
                            .get("context_window")
                            .or_else(|| profile.get("context_window_tokens"))
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        "deployment_kind": clean_text(
                            profile.get("deployment_kind").and_then(Value::as_str).unwrap_or(""),
                            40
                        )
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.sort_by(|left, right| {
        let left_id = clean_text(left.get("id").and_then(Value::as_str).unwrap_or(""), 240);
        let right_id = clean_text(right.get("id").and_then(Value::as_str).unwrap_or(""), 240);
        let left_power = left
            .get("power_rating")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let right_power = right
            .get("power_rating")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let left_context = left
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let right_context = right
            .get("context_window")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        right_power
            .cmp(&left_power)
            .then_with(|| right_context.cmp(&left_context))
            .then_with(|| left_id.cmp(&right_id))
    });
    rows
}

fn pick_default_image_tool_model(provider: &str, models: &[Value]) -> Option<String> {
    if models.is_empty() {
        return None;
    }
    for preferred in image_tool_preferred_models(provider) {
        if models.iter().any(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .map(|id| id == *preferred)
                .unwrap_or(false)
        }) {
            return Some(preferred.to_string());
        }
    }
    models
        .first()
        .and_then(|row| row.get("id").and_then(Value::as_str))
        .map(|row| row.to_string())
}

fn image_tool_provider_ready(row: &Value) -> bool {
    row.get("is_local")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || crate::dashboard_provider_runtime::auth_status_configured(
            row.get("auth_status").and_then(Value::as_str).unwrap_or(""),
        )
}

fn image_tool_provider_catalog_rows(root: &Path) -> Vec<Value> {
    let provider_rows = crate::dashboard_provider_runtime::provider_rows(root, &json!({}));
    let mut rows = provider_rows
        .into_iter()
        .filter_map(|row| {
            let provider = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 80);
            if provider.is_empty() {
                return None;
            }
            let image_models = image_tool_model_rows(&row);
            if image_models.is_empty() {
                return None;
            }
            let auto_priority = image_tool_provider_priority(&provider);
            let ready = image_tool_provider_ready(&row);
            Some(json!({
                "provider": provider,
                "display_name": clean_text(row.get("display_name").and_then(Value::as_str).unwrap_or(""), 120),
                "aliases": row.get("aliases").cloned().unwrap_or_else(|| json!([])),
                "is_local": row.get("is_local").and_then(Value::as_bool).unwrap_or(false),
                "auth_configured": crate::dashboard_provider_runtime::auth_status_configured(
                    row.get("auth_status").and_then(Value::as_str).unwrap_or("")
                ),
                "ready": ready,
                "api_key_env": clean_text(row.get("api_key_env").and_then(Value::as_str).unwrap_or(""), 160),
                "auto_priority": auto_priority,
                "auto_priority_source": if auto_priority.is_some() { "bundled" } else { "runtime_registry" },
                "default_model": pick_default_image_tool_model(&provider, &image_models),
                "image_models": image_models,
                "supports_image_description": true
            }))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_priority = left
            .get("auto_priority")
            .and_then(Value::as_u64)
            .unwrap_or(1000);
        let right_priority = right
            .get("auto_priority")
            .and_then(Value::as_u64)
            .unwrap_or(1000);
        let left_ready = left.get("ready").and_then(Value::as_bool).unwrap_or(false);
        let right_ready = right.get("ready").and_then(Value::as_bool).unwrap_or(false);
        let left_provider = clean_text(
            left.get("provider").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        let right_provider = clean_text(
            right.get("provider").and_then(Value::as_str).unwrap_or(""),
            80,
        );
        left_priority
            .cmp(&right_priority)
            .then_with(|| right_ready.cmp(&left_ready))
            .then_with(|| left_provider.cmp(&right_provider))
    });
    rows
}

fn image_tool_config_string(policy: &Value, key: &str, max_len: usize) -> String {
    policy
        .pointer(&format!("/web_conduit/image_tool/{key}"))
        .and_then(Value::as_str)
        .map(|row| clean_text(row, max_len))
        .unwrap_or_default()
}

fn image_tool_request_string(request: &Value, key: &str, max_len: usize) -> String {
    request
        .get(key)
        .and_then(Value::as_str)
        .map(|row| clean_text(row, max_len))
        .unwrap_or_default()
}

fn image_tool_config_u64(policy: &Value, key: &str, fallback: u64, min: u64, max: u64) -> u64 {
    policy
        .pointer(&format!("/web_conduit/image_tool/{key}"))
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn split_image_tool_model_ref(raw: &str) -> (String, String) {
    let cleaned = clean_text(raw, 240);
    if let Some((provider, model)) = cleaned.split_once('/') {
        return (
            normalize_image_tool_provider_id(provider),
            clean_text(model, 240),
        );
    }
    (String::new(), cleaned)
}

fn image_tool_provider_entry<'a>(catalog: &'a [Value], provider: &str) -> Option<&'a Value> {
    let normalized = normalize_image_tool_provider_id(provider);
    catalog.iter().find(|row| {
        row.get("provider")
            .and_then(Value::as_str)
            .map(|value| value == normalized)
            .unwrap_or(false)
    })
}

fn default_image_tool_runtime_metadata() -> Value {
    json!({
        "enabled": true,
        "configured_provider_input": Value::Null,
        "configured_model_input": Value::Null,
        "provider_source": "auto-detect",
        "selection_scope": "auto-detect",
        "allow_fallback": true,
        "selected_provider": Value::Null,
        "selected_model": Value::Null,
        "selection_ready": false,
        "selection_fallback_reason": Value::Null,
        "default_prompt": DEFAULT_IMAGE_TOOL_PROMPT,
        "max_images": DEFAULT_IMAGE_TOOL_MAX_IMAGES,
        "max_bytes": DEFAULT_IMAGE_TOOL_MAX_BYTES,
        "timeout_seconds": DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS,
        "output_max_buffer_bytes": DEFAULT_IMAGE_TOOL_OUTPUT_MAX_BUFFER_BYTES,
        "media_concurrency": DEFAULT_IMAGE_TOOL_MEDIA_CONCURRENCY,
        "execution_mode": "selection_only",
        "execution_gap": "multimodal_transport_not_enabled",
        "auto_provider_order": [],
        "ready_provider_order": [],
        "provider_catalog": [],
        "diagnostics": []
    })
}

pub(crate) fn web_image_tool_contract(root: &Path, policy: &Value) -> Value {
    let configured_prompt = image_tool_config_string(policy, "default_prompt", 4000);
    json!({
        "input_fields": ["prompt", "provider", "model", "path", "url", "image", "images"],
        "default_prompt": if configured_prompt.is_empty() {
            DEFAULT_IMAGE_TOOL_PROMPT.to_string()
        } else {
            configured_prompt
        },
        "max_images": image_tool_config_u64(policy, "max_images", DEFAULT_IMAGE_TOOL_MAX_IMAGES, 1, 64),
        "default_max_bytes": image_tool_config_u64(policy, "max_bytes", DEFAULT_IMAGE_TOOL_MAX_BYTES, 1024, 50 * 1024 * 1024),
        "timeout_seconds": image_tool_config_u64(policy, "timeout_seconds", DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS, 1, 600),
        "output_max_buffer_bytes": image_tool_config_u64(
            policy,
            "output_max_buffer_bytes",
            DEFAULT_IMAGE_TOOL_OUTPUT_MAX_BUFFER_BYTES,
            1024,
            20 * 1024 * 1024
        ),
        "media_concurrency": image_tool_config_u64(policy, "media_concurrency", DEFAULT_IMAGE_TOOL_MEDIA_CONCURRENCY, 1, 16),
        "source_contract": "same_as_web_media",
        "execution_contract": {
            "mode": "selection_only",
            "gap": "multimodal_transport_not_enabled"
        },
        "provider_resolution_contract": {
            "supports_provider_override": true,
            "supports_model_override": true,
            "request_provider_scope": "no_fallback",
            "configured_provider_scope": "fallback_allowed",
            "auto_provider_priority_contract": "bundled_defaults_then_runtime_registry_image_models"
        },
        "provider_catalog_contract": {
            "image_model_required": true,
            "ready_rule": "local_provider_or_configured_auth",
            "supported_provider_count": image_tool_provider_catalog_rows(root).len()
        }
    })
}

pub(crate) fn image_tool_runtime_resolution_snapshot(
    root: &Path,
    policy: &Value,
    request: &Value,
) -> Value {
    let catalog = image_tool_provider_catalog_rows(root);
    let auto_provider_order = catalog
        .iter()
        .filter_map(|row| row.get("provider").and_then(Value::as_str))
        .map(|row| row.to_string())
        .collect::<Vec<_>>();
    let ready_provider_order = catalog
        .iter()
        .filter(|row| row.get("ready").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|row| row.get("provider").and_then(Value::as_str))
        .map(|row| row.to_string())
        .collect::<Vec<_>>();

    let request_provider = image_tool_request_string(request, "provider", 80);
    let request_model = image_tool_request_string(request, "model", 240);
    let configured_provider = image_tool_config_string(policy, "provider", 80);
    let configured_model = image_tool_config_string(policy, "model", 240);
    let requested_provider_input = if !request_provider.is_empty() {
        request_provider.clone()
    } else {
        configured_provider.clone()
    };
    let requested_model_input = if !request_model.is_empty() {
        request_model.clone()
    } else {
        configured_model.clone()
    };

    let (model_provider, model_id_only) = split_image_tool_model_ref(&requested_model_input);
    let explicit_provider = if !model_provider.is_empty() {
        model_provider
    } else if !requested_provider_input.is_empty() {
        normalize_image_tool_provider_id(&requested_provider_input)
    } else {
        String::new()
    };
    let selection_scope = if !request_model.is_empty() {
        "request_model"
    } else if !request_provider.is_empty() {
        "request_provider"
    } else if !configured_model.is_empty() {
        "configured_model"
    } else if !configured_provider.is_empty() {
        "configured_provider"
    } else {
        "auto-detect"
    };
    let allow_fallback = !matches!(selection_scope, "request_model" | "request_provider");
    let mut diagnostics = Vec::<Value>::new();
    let mut selected_provider = String::new();
    let mut selected_model = String::new();
    let mut selection_fallback_reason = Value::Null;

    if !explicit_provider.is_empty() {
        if let Some(provider_row) = image_tool_provider_entry(&catalog, &explicit_provider) {
            selected_provider = explicit_provider.clone();
            let available_models = provider_row
                .get("image_models")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !model_id_only.is_empty() {
                let explicit_model_exists = available_models.iter().any(|row| {
                    row.get("id")
                        .and_then(Value::as_str)
                        .map(|id| id == model_id_only)
                        .unwrap_or(false)
                });
                if explicit_model_exists {
                    selected_model = model_id_only.clone();
                } else if allow_fallback {
                    selected_model = provider_row
                        .get("default_model")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    selection_fallback_reason = json!("invalid_configured_model");
                    diagnostics.push(json!({
                        "code": "WEB_IMAGE_TOOL_MODEL_INVALID_FALLBACK_USED",
                        "message": format!(
                            "Configured image tool model \"{}\" is not image-capable for provider \"{}\"; falling back to \"{}\".",
                            model_id_only,
                            explicit_provider,
                            selected_model
                        )
                    }));
                } else {
                    diagnostics.push(json!({
                        "code": "WEB_IMAGE_TOOL_MODEL_INVALID",
                        "message": format!(
                            "Requested image tool model \"{}\" is not image-capable for provider \"{}\".",
                            model_id_only,
                            explicit_provider
                        )
                    }));
                }
            } else {
                selected_model = provider_row
                    .get("default_model")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
            }
        } else if allow_fallback {
            selection_fallback_reason = json!("invalid_configured_provider");
            diagnostics.push(json!({
                "code": "WEB_IMAGE_TOOL_PROVIDER_INVALID_FALLBACK_USED",
                "message": format!(
                    "Configured image tool provider \"{}\" is unavailable or has no image-capable models; falling back to auto-detect.",
                    explicit_provider
                )
            }));
        } else {
            diagnostics.push(json!({
                "code": "WEB_IMAGE_TOOL_PROVIDER_INVALID",
                "message": format!(
                    "Requested image tool provider \"{}\" is unavailable or has no image-capable models.",
                    explicit_provider
                )
            }));
        }
    }

    if selected_provider.is_empty() && allow_fallback {
        if let Some(provider) = ready_provider_order.first() {
            selected_provider = provider.clone();
        } else if let Some(provider) = auto_provider_order.first() {
            selected_provider = provider.clone();
        }
        if !selected_provider.is_empty() {
            selected_model = image_tool_provider_entry(&catalog, &selected_provider)
                .and_then(|row| row.get("default_model").and_then(Value::as_str))
                .unwrap_or("")
                .to_string();
        }
    }

    let selection_ready = image_tool_provider_entry(&catalog, &selected_provider)
        .and_then(|row| row.get("ready").and_then(Value::as_bool))
        .unwrap_or(false)
        && !selected_model.is_empty();
    let provider_source = match selection_scope {
        "request_model" | "request_provider" => "request",
        "configured_model" | "configured_provider" => {
            if selection_fallback_reason.is_null() {
                "configured"
            } else {
                "auto-detect"
            }
        }
        _ => "auto-detect",
    };
    let configured_prompt = image_tool_config_string(policy, "default_prompt", 4000);

    json!({
        "enabled": policy.pointer("/web_conduit/enabled").and_then(Value::as_bool).unwrap_or(true),
        "configured_provider_input": if requested_provider_input.is_empty() { Value::Null } else { json!(requested_provider_input) },
        "configured_model_input": if requested_model_input.is_empty() { Value::Null } else { json!(requested_model_input) },
        "provider_source": provider_source,
        "selection_scope": selection_scope,
        "allow_fallback": allow_fallback,
        "selected_provider": if selected_provider.is_empty() { Value::Null } else { json!(selected_provider) },
        "selected_model": if selected_model.is_empty() { Value::Null } else { json!(selected_model) },
        "selection_ready": selection_ready,
        "selection_fallback_reason": selection_fallback_reason,
        "default_prompt": if configured_prompt.is_empty() {
            DEFAULT_IMAGE_TOOL_PROMPT.to_string()
        } else {
            configured_prompt
        },
        "max_images": image_tool_config_u64(policy, "max_images", DEFAULT_IMAGE_TOOL_MAX_IMAGES, 1, 64),
        "max_bytes": image_tool_config_u64(policy, "max_bytes", DEFAULT_IMAGE_TOOL_MAX_BYTES, 1024, 50 * 1024 * 1024),
        "timeout_seconds": image_tool_config_u64(policy, "timeout_seconds", DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS, 1, 600),
        "output_max_buffer_bytes": image_tool_config_u64(
            policy,
            "output_max_buffer_bytes",
            DEFAULT_IMAGE_TOOL_OUTPUT_MAX_BUFFER_BYTES,
            1024,
            20 * 1024 * 1024
        ),
        "media_concurrency": image_tool_config_u64(policy, "media_concurrency", DEFAULT_IMAGE_TOOL_MEDIA_CONCURRENCY, 1, 16),
        "execution_mode": "selection_only",
        "execution_gap": "multimodal_transport_not_enabled",
        "auto_provider_order": auto_provider_order,
        "ready_provider_order": ready_provider_order,
        "provider_catalog": catalog,
        "diagnostics": diagnostics
    })
}
