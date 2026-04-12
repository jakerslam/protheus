pub(crate) const DEFAULT_IMAGE_TOOL_PROMPT: &str = "Describe the image.";
pub(crate) const DEFAULT_IMAGE_TOOL_MAX_IMAGES: u64 = 20;
pub(crate) const DEFAULT_IMAGE_TOOL_MAX_BYTES: u64 = 10 * 1024 * 1024;
pub(crate) const DEFAULT_IMAGE_TOOL_TIMEOUT_SECONDS: u64 = 60;
pub(crate) const DEFAULT_IMAGE_TOOL_OUTPUT_MAX_BUFFER_BYTES: u64 = 5 * 1024 * 1024;
pub(crate) const DEFAULT_IMAGE_TOOL_MEDIA_CONCURRENCY: u64 = 2;

fn image_tool_provider_execution_supported(provider: &str) -> bool {
    matches!(
        normalize_image_tool_provider_id(provider).as_str(),
        "openai"
            | "frontier_provider"
            | "google"
            | "groq"
            | "moonshot"
            | "xai"
            | "openrouter"
            | "deepseek"
            | "together"
            | "fireworks"
            | "mistral"
            | "ollama"
    )
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
        "execution_mode": "direct_multimodal_provider",
        "execution_gap": "provider_subset_only",
        "auto_provider_order": [],
        "ready_provider_order": [],
        "provider_catalog": [],
        "diagnostics": []
    })
}

fn web_image_tool_supported_provider_ids(root: &Path) -> Vec<String> {
    image_tool_provider_catalog_rows(root)
        .into_iter()
        .filter(|row| {
            row.get("execution_supported")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|row| row.get("provider").and_then(Value::as_str).map(str::to_string))
        .collect()
}

pub(crate) fn web_image_tool_contract(root: &Path, policy: &Value) -> Value {
    let configured_prompt = image_tool_config_string(policy, "default_prompt", 4000);
    let supported_provider_ids = web_image_tool_supported_provider_ids(root);
    let unsupported_provider_examples = image_tool_provider_catalog_rows(root)
        .into_iter()
        .filter(|row| {
            !row.get("execution_supported")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|row| row.get("provider").and_then(Value::as_str).map(str::to_string))
        .take(6)
        .collect::<Vec<_>>();
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
        "returns": ["analysis", "provider", "model", "image_count", "attempts", "provider_resolution"],
        "execution_contract": {
            "mode": "direct_multimodal_provider",
            "gap": "provider_subset_only",
            "supported_provider_ids": supported_provider_ids,
            "unsupported_provider_examples": unsupported_provider_examples
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
