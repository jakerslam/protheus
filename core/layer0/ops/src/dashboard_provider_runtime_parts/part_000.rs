// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const PROVIDER_REGISTRY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_registry.json";
const PROVIDER_SECRETS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_secrets.json";
const DEFAULT_PROVIDER_IDS: &[&str] = &[
    "openai",
    "anthropic",
    "google",
    "groq",
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

fn registry_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_REGISTRY_REL)
}

fn secrets_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_SECRETS_REL)
}

fn load_registry(root: &Path) -> Value {
    read_json(&registry_path(root)).unwrap_or_else(|| json!({
        "type": "infring_dashboard_provider_registry",
        "updated_at": crate::now_iso(),
        "providers": {}
    }))
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
    read_json(&secrets_path(root)).unwrap_or_else(|| json!({
        "type": "infring_dashboard_provider_secrets",
        "updated_at": crate::now_iso(),
        "providers": {}
    }))
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
        other => other.to_string(),
    }
}

fn provider_display_name(provider_id: &str) -> String {
    match provider_id {
        "auto" => "Auto Router".to_string(),
        "openai" => "OpenAI".to_string(),
        "anthropic" => "Anthropic".to_string(),
        "google" => "Gemini".to_string(),
        "groq" => "Groq".to_string(),
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
    matches!(provider_id, "ollama" | "local" | "llama.cpp" | "claude-code")
}

fn provider_needs_key(provider_id: &str) -> bool {
    !matches!(provider_id, "auto" | "ollama" | "local" | "llama.cpp" | "claude-code")
}

fn provider_has_builtin_defaults(provider_id: &str) -> bool {
    provider_id == "auto" || DEFAULT_PROVIDER_IDS.iter().any(|value| *value == provider_id)
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn local_provider_reachable(provider_id: &str, row: &Value) -> bool {
    match provider_id {
        "claude-code" => command_exists("claude"),
        "local" => row
            .get("local_model_root")
            .and_then(Value::as_str)
            .map(|value| {
                let cleaned = clean_text(value, 4000);
                !cleaned.is_empty() && Path::new(&cleaned).exists()
            })
            .unwrap_or(false),
        _ => row.get("reachable").and_then(Value::as_bool).unwrap_or(false),
    }
}

pub fn provider_supports_chat(provider_id: &str, base_url: &str) -> bool {
    let cleaned = clean_text(base_url, 400);
    match provider_id {
        "openai" | "anthropic" | "google" | "groq" | "xai" | "openrouter" | "deepseek"
        | "together" | "fireworks" | "perplexity" | "mistral" | "ollama" | "llama.cpp" => {
            !cleaned.is_empty()
        }
        "claude-code" | "cohere" | "auto" => false,
        _ => !cleaned.is_empty(),
    }
}

pub fn auth_status_configured(raw: &str) -> bool {
    matches!(
        clean_text(raw, 40).to_ascii_lowercase().as_str(),
        "configured" | "set" | "ok"
    )
}

fn provider_api_key_env(provider_id: &str) -> String {
    provider_key_env_names(provider_id)
        .first()
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn provider_key_env_names(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "openai" => &["OPENAI_API_KEY"],
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        "groq" => &["GROQ_API_KEY"],
        "xai" => &["XAI_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "together" => &["TOGETHER_API_KEY"],
        "fireworks" => &["FIREWORKS_API_KEY"],
        "perplexity" => &["PERPLEXITY_API_KEY"],
        "mistral" => &["MISTRAL_API_KEY"],
        "cohere" => &["COHERE_API_KEY"],
        "minimax" => &["MINIMAX_API_KEY"],
        _ => &[],
    }
}

fn provider_key_from_env(provider_id: &str) -> Option<String> {
    for key in provider_key_env_names(provider_id) {
        if let Ok(value) = std::env::var(key) {
            let cleaned = clean_text(&value, 4096);
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn provider_key_from_store(root: &Path, provider_id: &str) -> Option<String> {
    load_secrets(root)
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(provider_id))
        .and_then(Value::as_object)
        .and_then(|row| row.get("key").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 4096))
        .filter(|value| !value.is_empty())
}

fn provider_key(root: &Path, provider_id: &str) -> Option<String> {
    provider_key_from_env(provider_id).or_else(|| provider_key_from_store(root, provider_id))
}

fn provider_base_url_default(provider_id: &str) -> String {
    match provider_id {
        "openai" => "https://api.openai.com/v1".to_string(),
        "anthropic" => "https://api.anthropic.com".to_string(),
        "google" => "https://generativelanguage.googleapis.com".to_string(),
        "groq" => "https://api.groq.com/openai/v1".to_string(),
        "xai" => "https://api.x.ai/v1".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1".to_string(),
        "deepseek" => "https://api.deepseek.com/v1".to_string(),
        "together" => "https://api.together.xyz/v1".to_string(),
        "fireworks" => "https://api.fireworks.ai/inference/v1".to_string(),
        "perplexity" => "https://api.perplexity.ai".to_string(),
        "mistral" => "https://api.mistral.ai/v1".to_string(),
        "ollama" => "http://127.0.0.1:11434".to_string(),
        "llama.cpp" => "http://127.0.0.1:8080".to_string(),
        _ => String::new(),
    }
}

fn model_profiles_for_provider(provider_id: &str) -> Map<String, Value> {
    match provider_id {
        "openai" => serde_json::from_value(json!({
            "gpt-5": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api"},
            "gpt-5-mini": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 0, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api"},
            "gpt-4o": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "api"}
        }))
        .unwrap_or_default(),
        "anthropic" => serde_json::from_value(json!({
            "claude-sonnet-4-20250514": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api"},
            "claude-opus-4-20250514": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api"}
        }))
        .unwrap_or_default(),
        "google" => serde_json::from_value(json!({
            "gemini-2.5-pro": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "api", "context_window": 1048576},
            "gemini-2.5-flash": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "speed", "general"], "deployment_kind": "api", "context_window": 1048576}
        }))
        .unwrap_or_default(),
        "groq" => serde_json::from_value(json!({
            "llama-3.3-70b-versatile": {"power_rating": 4, "cost_rating": 3, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 131072},
            "llama-3.1-8b-instant": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 8, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api", "context_window": 131072}
        }))
        .unwrap_or_default(),
        "deepseek" => serde_json::from_value(json!({
            "deepseek-chat": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general", "coding"], "deployment_kind": "api", "context_window": 65536},
            "deepseek-reasoner": {"power_rating": 5, "cost_rating": 3, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 65536}
        }))
        .unwrap_or_default(),
        "openrouter" => serde_json::from_value(json!({
            "google/gemini-2.5-flash": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "speed", "general"], "deployment_kind": "api", "context_window": 1048576},
            "anthropic/claude-sonnet-4": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 200000}
        }))
        .unwrap_or_default(),
        "xai" => serde_json::from_value(json!({
            "grok-2": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 131072}
        }))
        .unwrap_or_default(),
        "ollama" => serde_json::from_value(json!({
            "qwen2.5-coder:7b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 7, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "llama3.2:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 128000}
        }))
        .unwrap_or_default(),
        "claude-code" => serde_json::from_value(json!({
            "sonnet": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "local", "context_window": 200000}
        }))
        .unwrap_or_default(),
        _ => Map::new(),
    }
}

fn ensure_provider_row_mut<'a>(registry: &'a mut Value, provider_id: &str) -> &'a mut Value {
    if !registry.is_object() {
        *registry = json!({});
    }
    if registry.get("providers").is_none() || !registry.get("providers").map(Value::is_object).unwrap_or(false)
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
    registry
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
        })
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
    }
}

fn guess_provider_from_key(raw: &str) -> String {
    let key = clean_text(raw, 512);
    if key.starts_with("sk-ant-") {
        return "anthropic".to_string();
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

