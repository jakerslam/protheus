#[derive(Clone, Debug)]
struct MediaGenerateProviderListRow {
    id: String,
    default_model: Option<String>,
    models: Vec<String>,
    modes: Vec<String>,
    auth_env_vars: Vec<String>,
    capabilities: Value,
    capability_summary: String,
}

fn create_media_generate_provider_list_action_result(
    providers: &[MediaGenerateProviderListRow],
    empty_text: &str,
) -> Value {
    if providers.is_empty() {
        return json!({
            "content": [{"type": "text", "text": clean_text(empty_text, 240)}],
            "details": { "providers": [] }
        });
    }
    let lines = providers
        .iter()
        .map(|provider| {
            let mut parts = vec![format!(
                "{}: default={}",
                provider.id,
                provider
                    .default_model
                    .clone()
                    .unwrap_or_else(|| "none".to_string())
            )];
            if !provider.models.is_empty() {
                parts.push(format!("models={}", provider.models.join(", ")));
            }
            if !provider.capability_summary.is_empty() {
                parts.push(provider.capability_summary.clone());
            }
            if !provider.auth_env_vars.is_empty() {
                parts.push(format!("auth={}", provider.auth_env_vars.join(" / ")));
            }
            parts.join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    json!({
        "content": [{"type": "text", "text": lines}],
        "details": {
            "providers": providers.iter().map(|provider| json!({
                "id": provider.id,
                "defaultModel": provider.default_model,
                "models": provider.models,
                "modes": provider.modes,
                "authEnvVars": provider.auth_env_vars,
                "capabilities": provider.capabilities
            })).collect::<Vec<_>>()
        }
    })
}

fn create_media_generate_status_action_result(
    inactive_text: &str,
    active_text: Option<String>,
    active_details: Option<Value>,
) -> Value {
    match active_details {
        Some(details) => json!({
            "content": [{"type": "text", "text": active_text.unwrap_or_default()}],
            "details": {
                "action": "status",
                "active": true,
                "existingTask": true,
                "duplicateGuard": false,
                "status": details.get("status").cloned().unwrap_or(Value::Null),
                "taskKind": details.get("taskKind").cloned().unwrap_or(Value::Null),
                "provider": details.get("provider").cloned().unwrap_or(Value::Null),
                "progressSummary": details.get("progressSummary").cloned().unwrap_or(Value::Null),
                "task": details.pointer("/task").cloned().unwrap_or(Value::Null)
            }
        }),
        None => json!({
            "content": [{"type": "text", "text": clean_text(inactive_text, 240)}],
            "details": {
                "action": "status",
                "active": false
            }
        }),
    }
}

fn create_media_generate_duplicate_guard_result(
    active_text: Option<String>,
    active_details: Option<Value>,
) -> Option<Value> {
    active_details.map(|details| {
        json!({
            "content": [{"type": "text", "text": active_text.unwrap_or_default()}],
            "details": {
                "action": "status",
                "duplicateGuard": true,
                "active": true,
                "existingTask": true,
                "status": details.get("status").cloned().unwrap_or(Value::Null),
                "taskKind": details.get("taskKind").cloned().unwrap_or(Value::Null),
                "provider": details.get("provider").cloned().unwrap_or(Value::Null),
                "progressSummary": details.get("progressSummary").cloned().unwrap_or(Value::Null),
                "task": details.pointer("/task").cloned().unwrap_or(Value::Null)
            }
        })
    })
}

fn video_generate_capability_summary_fields() -> Vec<&'static str> {
    vec![
        "modes",
        "maxVideos",
        "maxInputImages",
        "maxInputVideos",
        "maxInputAudios",
        "maxDurationSeconds",
        "supportedDurationSeconds",
        "resolution",
        "aspectRatio",
        "size",
        "audio",
        "watermark",
        "providerOptions",
    ]
}

fn music_generate_capability_summary_fields() -> Vec<&'static str> {
    vec![
        "modes",
        "maxTracks",
        "maxInputImages",
        "maxDurationSeconds",
        "lyrics",
        "instrumental",
        "duration",
        "format",
        "supportedFormats",
        "supportedFormatsByModel",
    ]
}

fn video_generate_action_contract() -> Value {
    let active_status = json!({
        "status": "queued",
        "taskKind": "video_generate",
        "provider": "google",
        "progressSummary": "Queued video generation",
        "task": build_media_task_run_details("task-active", "tool:video_generate:active")["task"].clone()
    });
    let duplicate_status = json!({
        "status": "running",
        "taskKind": "video_generate",
        "provider": "openai",
        "progressSummary": "Generating video",
        "task": build_media_task_run_details("task-active", "tool:video_generate:active")["task"].clone()
    });
    json!({
        "actions": ["generate", "list", "status"],
        "inactive_text": "No active video generation task is currently running for this session.",
        "capability_summary_fields": video_generate_capability_summary_fields(),
        "provider_list_example": create_media_generate_provider_list_action_result(
            &[MediaGenerateProviderListRow {
                id: "openai".to_string(),
                default_model: Some("sora-mini".to_string()),
                models: vec!["sora-mini".to_string(), "sora".to_string()],
                modes: vec!["generate".to_string(), "imageToVideo".to_string()],
                auth_env_vars: vec!["OPENAI_API_KEY".to_string()],
                capabilities: json!({"generate": {"supportsAudio": true}, "imageToVideo": {"maxInputImages": 1}}),
                capability_summary: "modes=generate/imageToVideo, audio, maxInputImages=1".to_string(),
            }],
            "No video-generation providers are registered."
        ),
        "status_example": create_media_generate_status_action_result(
            "No active video generation task is currently running for this session.",
            Some("Video generation task task-active is already queued with google.".to_string()),
            Some(active_status)
        ),
        "duplicate_guard_example": create_media_generate_duplicate_guard_result(
            Some("Video generation task task-active is already running with openai. Do not call video_generate again for this request.".to_string()),
            Some(duplicate_status)
        )
    })
}

fn music_generate_action_contract() -> Value {
    let active_status = json!({
        "status": "queued",
        "taskKind": "music_generate",
        "provider": "minimax",
        "progressSummary": "Queued music generation",
        "task": build_media_task_run_details("task-active", "tool:music_generate:active")["task"].clone()
    });
    let duplicate_status = json!({
        "status": "running",
        "taskKind": "music_generate",
        "provider": "google",
        "progressSummary": "Generating music",
        "task": build_media_task_run_details("task-active", "tool:music_generate:active")["task"].clone()
    });
    json!({
        "actions": ["generate", "list", "status"],
        "inactive_text": "No active music generation task is currently running for this session.",
        "capability_summary_fields": music_generate_capability_summary_fields(),
        "provider_list_example": create_media_generate_provider_list_action_result(
            &[MediaGenerateProviderListRow {
                id: "google".to_string(),
                default_model: Some("lyria-2".to_string()),
                models: vec!["lyria-2".to_string()],
                modes: vec!["generate".to_string(), "edit".to_string()],
                auth_env_vars: vec!["GOOGLE_API_KEY".to_string()],
                capabilities: json!({"generate": {"supportsLyrics": true, "supportsDuration": true}}),
                capability_summary: "modes=generate/edit, lyrics, duration".to_string(),
            }],
            "No music-generation providers are registered."
        ),
        "status_example": create_media_generate_status_action_result(
            "No active music generation task is currently running for this session.",
            Some("Music generation task task-active is already queued with minimax.".to_string()),
            Some(active_status)
        ),
        "duplicate_guard_example": create_media_generate_duplicate_guard_result(
            Some("Music generation task task-active is already running with google. Do not call music_generate again for this request.".to_string()),
            Some(duplicate_status)
        )
    })
}

fn media_generate_action_contracts() -> Value {
    json!({
        "shared": {
            "provider_list_details_fields": ["id", "defaultModel", "models", "modes", "authEnvVars", "capabilities"],
            "status_details_fields": ["action", "active", "existingTask", "status", "taskKind", "provider", "progressSummary", "task.taskId", "task.runId"],
            "duplicate_guard_field": "duplicateGuard"
        },
        "video_generation": video_generate_action_contract(),
        "music_generation": music_generate_action_contract()
    })
}
