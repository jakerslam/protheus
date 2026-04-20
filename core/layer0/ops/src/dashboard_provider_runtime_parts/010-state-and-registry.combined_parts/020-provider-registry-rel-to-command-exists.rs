
const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";
const PROVIDER_SECRETS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_secrets.json";
const PROVIDER_ROUTING_POLICY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_routing_policy.json";
const PROVIDER_VIRTUAL_KEYS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_virtual_keys.json";
const PROVIDER_ROUTING_EVENTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_routing_events.jsonl";
const DEFAULT_PROVIDER_IDS: &[&str] = &[
    "openai",
    "frontier_provider",
    "google",
    "groq",
    "moonshot",
    "deepseek",
    "openrouter",
    "xai",
    "ollama",
    "claude-code",
    "mistral",
    "together",
    "fireworks",
    "perplexity",
    "cohere",
];

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn model_id_is_placeholder(model_id: &str) -> bool {
    matches!(
        clean_text(model_id, 240).to_ascii_lowercase().as_str(),
        "model" | "<model>" | "(model)"
    )
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn write_json_pretty(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn provider_state_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

fn registry_path(root: &Path) -> PathBuf {
    provider_state_path(root, PROVIDER_REGISTRY_REL)
}

fn secrets_path(root: &Path) -> PathBuf {
    provider_state_path(root, PROVIDER_SECRETS_REL)
}

fn routing_policy_path(root: &Path) -> PathBuf {
    provider_state_path(root, PROVIDER_ROUTING_POLICY_REL)
}

fn virtual_keys_path(root: &Path) -> PathBuf {
    provider_state_path(root, PROVIDER_VIRTUAL_KEYS_REL)
}

fn routing_events_path(root: &Path) -> PathBuf {
    provider_state_path(root, PROVIDER_ROUTING_EVENTS_REL)
}

fn load_registry(root: &Path) -> Value {
    read_json(&registry_path(root)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_provider_registry",
            "updated_at": crate::now_iso(),
            "providers": {}
        })
    })
}

fn save_registry(root: &Path, mut value: Value) {
    if !value.is_object() {
        value = json!({});
    }
    value["type"] = json!("infring_dashboard_provider_registry");
    value["updated_at"] = json!(crate::now_iso());
    write_json_pretty(&registry_path(root), &value);
}

fn load_secrets(root: &Path) -> Value {
    read_json(&secrets_path(root)).unwrap_or_else(|| {
        json!({
            "type": "infring_dashboard_provider_secrets",
            "updated_at": crate::now_iso(),
            "providers": {}
        })
    })
}

fn save_secrets(root: &Path, mut value: Value) {
    if !value.is_object() {
        value = json!({});
    }
    value["type"] = json!("infring_dashboard_provider_secrets");
    value["updated_at"] = json!(crate::now_iso());
    let path = secrets_path(root);
    write_json_pretty(&path, &value);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
}

fn normalize_provider_id(raw: &str) -> String {
    let cleaned = clean_text(raw, 120).to_ascii_lowercase();
    match cleaned.as_str() {
        "gemini" => "google".to_string(),
        "google-ai" => "google".to_string(),
        "kimi" => "moonshot".to_string(),
        "moonshot-ai" => "moonshot".to_string(),
        "anthropic" => "frontier_provider".to_string(),
        "claude" => "frontier_provider".to_string(),
        "frontier-provider" => "frontier_provider".to_string(),
        other => other.to_string(),
    }
}

fn provider_display_name(provider_id: &str) -> String {
    match provider_id {
        "auto" => "Auto Router".to_string(),
        "openai" => "OpenAI".to_string(),
        "frontier_provider" => "Claude (Anthropic)".to_string(),
        "google" => "Gemini".to_string(),
        "groq" => "Groq".to_string(),
        "moonshot" => "Moonshot".to_string(),
        "xai" => "xAI".to_string(),
        "openrouter" => "OpenRouter".to_string(),
        "deepseek" => "DeepSeek".to_string(),
        "ollama" => "Ollama".to_string(),
        "claude-code" => "Claude Code".to_string(),
        other => {
            let mut words = Vec::<String>::new();
            for word in other.replace('-', " ").split_whitespace() {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut built = String::new();
                    built.push(first.to_ascii_uppercase());
                    built.push_str(chars.as_str());
                    words.push(built);
                }
            }
            if words.is_empty() {
                "Provider".to_string()
            } else {
                words.join(" ")
            }
        }
    }
}

fn provider_is_local(provider_id: &str) -> bool {
    matches!(
        provider_id,
        "ollama" | "local" | "llama.cpp" | "claude-code"
    )
}

fn provider_needs_key(provider_id: &str) -> bool {
    !matches!(
        provider_id,
        "auto" | "ollama" | "local" | "llama.cpp" | "claude-code"
    )
}

fn provider_has_builtin_defaults(provider_id: &str) -> bool {
    provider_id == "auto"
        || DEFAULT_PROVIDER_IDS
            .iter()
            .any(|value| *value == provider_id)
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
