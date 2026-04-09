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

fn local_provider_reachable(provider_id: &str, row: &Value) -> bool {
    match provider_id {
        "ollama" => {
            let base_url = clean_text(
                row.get("base_url")
                    .and_then(Value::as_str)
                    .unwrap_or(&provider_base_url_default("ollama")),
                400,
            );
            probe_ollama_runtime_online(&base_url)
        }
        "claude-code" => command_exists("claude"),
        "local" => row
            .get("local_model_root")
            .and_then(Value::as_str)
            .map(|value| {
                let cleaned = clean_text(value, 4000);
                !cleaned.is_empty() && Path::new(&cleaned).exists()
            })
            .unwrap_or(false),
        _ => row
            .get("reachable")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

pub fn provider_supports_chat(provider_id: &str, base_url: &str) -> bool {
    let cleaned = clean_text(base_url, 400);
    match provider_id {
        "openai" | "frontier_provider" | "google" | "groq" | "moonshot" | "xai" | "openrouter"
        | "deepseek" | "together" | "fireworks" | "perplexity" | "mistral" | "ollama"
        | "llama.cpp" => !cleaned.is_empty(),
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
        "frontier_provider" => &[
            "ANTHROPIC_API_KEY",
            "FRONTIER_PROVIDER_API_KEY",
            "CLAUDE_API_KEY",
        ],
        "google" => &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        "groq" => &["GROQ_API_KEY"],
        "moonshot" => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
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
        "frontier_provider" => "https://api.anthropic.com".to_string(),
        "google" => "https://generativelanguage.googleapis.com".to_string(),
        "groq" => "https://api.groq.com/openai/v1".to_string(),
        "moonshot" => "https://api.moonshot.ai/v1".to_string(),
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
        "frontier_provider" => serde_json::from_value(json!({
            "claude-sonnet-4-20250514": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api"},
            "claude-opus-4-20250514": {"power_rating": 5, "cost_rating": 5, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api"},
            "claude-3-7-sonnet-latest": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general", "coding"], "deployment_kind": "api"},
            "claude-3-5-haiku-latest": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "api"}
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
        "moonshot" => serde_json::from_value(json!({
            "kimi-k2.5": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144},
            "kimi-k2-thinking": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144},
            "kimi-k2": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 1000, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "api", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "deepseek" => serde_json::from_value(json!({
            "deepseek-chat": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general", "coding"], "deployment_kind": "api", "context_window": 65536},
            "deepseek-reasoner": {"power_rating": 5, "cost_rating": 3, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 65536}
        }))
        .unwrap_or_default(),
        "openrouter" => serde_json::from_value(json!({
            "google/gemini-2.5-flash": {"power_rating": 3, "cost_rating": 2, "param_count_billion": 0, "specialty": "vision", "specialty_tags": ["vision", "speed", "general"], "deployment_kind": "api", "context_window": 1048576},
            "frontier_provider/claude-sonnet-4": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 200000},
            "moonshotai/kimi-k2": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 1000, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "api", "context_window": 262144},
            "moonshotai/kimi-k2.5": {"power_rating": 5, "cost_rating": 4, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "api", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "xai" => serde_json::from_value(json!({
            "grok-2": {"power_rating": 4, "cost_rating": 4, "param_count_billion": 0, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "api", "context_window": 131072}
        }))
        .unwrap_or_default(),
        "ollama" => serde_json::from_value(json!({
            "qwen2.5-coder:7b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 7, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen2.5-coder:latest": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 7, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen2.5:3b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:8b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 8, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "qwen3:14b": {"power_rating": 3, "cost_rating": 1, "param_count_billion": 14, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "smallthinker:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "ollama", "context_window": 131072},
            "tinyllama:latest": {"power_rating": 1, "cost_rating": 1, "param_count_billion": 1, "specialty": "speed", "specialty_tags": ["speed", "general"], "deployment_kind": "ollama", "context_window": 32768},
            "gemma3:4b": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 4, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "phi:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 32768},
            "llama3.2:latest": {"power_rating": 2, "cost_rating": 1, "param_count_billion": 3, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 128000},
            "llama3.3:70b": {"power_rating": 4, "cost_rating": 1, "param_count_billion": 70, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "ollama", "context_window": 131072},
            "kimi-k2.5:cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "kimi-k2.5:thinking": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "reasoning", "specialty_tags": ["reasoning", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "deepseek-v3.2:cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 0, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "deepseek-v3.1:671b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 671, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "qwen3-coder:480b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 480, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "qwen3-vl:235b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 235, "specialty": "vision", "specialty_tags": ["vision", "general"], "deployment_kind": "cloud", "context_window": 262144},
            "gpt-oss:120b-cloud": {"power_rating": 5, "cost_rating": 2, "param_count_billion": 120, "specialty": "general", "specialty_tags": ["general"], "deployment_kind": "cloud", "context_window": 262144}
        }))
        .unwrap_or_default(),
        "claude-code" => serde_json::from_value(json!({
            "sonnet": {"power_rating": 4, "cost_rating": 2, "param_count_billion": 0, "specialty": "coding", "specialty_tags": ["coding", "general"], "deployment_kind": "local", "context_window": 200000}
        }))
        .unwrap_or_default(),
        _ => Map::new(),
    }
}

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
    if current_context == 0 && inferred_context > 0 {
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
