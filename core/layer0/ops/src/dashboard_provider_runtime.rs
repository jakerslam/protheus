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

fn curl_json(
    url: &str,
    method: &str,
    headers: &[String],
    body: Option<&Value>,
    timeout_secs: u64,
) -> Result<(u16, Value), String> {
    let mut cmd = Command::new("curl");
    cmd.arg("-sS")
        .arg("-L")
        .arg("-X")
        .arg(method)
        .arg("--connect-timeout")
        .arg("8")
        .arg("--max-time")
        .arg(timeout_secs.to_string());
    for header in headers {
        cmd.arg("-H").arg(header);
    }
    if body.is_some() {
        cmd.arg("--data-binary").arg("@-");
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.arg("-w").arg("\n__HTTP_STATUS__:%{http_code}").arg(url);
    let mut child = cmd.spawn().map_err(|err| format!("curl_spawn_failed:{err}"))?;
    if let Some(payload) = body {
        let encoded =
            serde_json::to_vec(payload).map_err(|err| format!("http_body_encode_failed:{err}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&encoded)
                .map_err(|err| format!("curl_stdin_write_failed:{err}"))?;
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("curl_wait_failed:{err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = clean_text(&String::from_utf8_lossy(&output.stderr), 600);
    let marker = "\n__HTTP_STATUS__:";
    let Some(index) = stdout.rfind(marker) else {
        return Err(if stderr.is_empty() {
            "curl_http_status_missing".to_string()
        } else {
            stderr
        });
    };
    let body_raw = stdout[..index].trim();
    let status_raw = stdout[index + marker.len()..].trim();
    let status = status_raw.parse::<u16>().unwrap_or(0);
    let value = serde_json::from_str::<Value>(body_raw)
        .unwrap_or_else(|_| json!({"raw": clean_text(body_raw, 12_000)}));
    if !output.status.success() && status == 0 {
        return Err(if stderr.is_empty() {
            "curl_failed".to_string()
        } else {
            stderr
        });
    }
    Ok((status, value))
}

fn error_text_from_value(value: &Value) -> String {
    if let Some(text) = value.get("error").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    if let Some(text) = value
        .get("error")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("message").and_then(Value::as_str))
    {
        return clean_text(text, 280);
    }
    if let Some(text) = value.get("message").and_then(Value::as_str) {
        return clean_text(text, 280);
    }
    clean_text(&value.to_string(), 280)
}

fn extract_openai_text(value: &Value) -> String {
    value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(|text| clean_text(text, 32_000))
        .or_else(|| {
            value.pointer("/choices/0/text")
                .and_then(Value::as_str)
                .map(|text| clean_text(text, 32_000))
        })
        .unwrap_or_default()
}

fn extract_anthropic_text(value: &Value) -> String {
    value.get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("text").and_then(Value::as_str).map(|v| clean_text(v, 12_000)))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_google_text(value: &Value) -> String {
    value
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("text").and_then(Value::as_str).map(|v| clean_text(v, 12_000)))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn model_context_window(root: &Path, provider_id: &str, model_name: &str) -> i64 {
    provider_row(root, provider_id)
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(model_name))
        .and_then(|row| {
            row.get("context_window")
                .or_else(|| row.get("context_size"))
                .or_else(|| row.get("context_tokens"))
                .and_then(Value::as_i64)
        })
        .unwrap_or(0)
}

pub fn provider_rows(root: &Path, _snapshot: &Value) -> Vec<Value> {
    let registry = load_registry(root);
    let mut provider_ids = DEFAULT_PROVIDER_IDS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if let Some(obj) = registry.get("providers").and_then(Value::as_object) {
        for key in obj.keys() {
            let provider_id = normalize_provider_id(key);
            if provider_id.is_empty() || provider_ids.iter().any(|row| row == &provider_id) {
                continue;
            }
            provider_ids.push(provider_id);
        }
    }
    let mut rows = provider_ids
        .into_iter()
        .map(|provider_id| provider_row(root, &provider_id))
        .collect::<Vec<_>>();
    for row in &mut rows {
        let provider_id = normalize_provider_id(row.get("id").and_then(Value::as_str).unwrap_or(""));
        if provider_id.is_empty() {
            continue;
        }
        row["id"] = json!(provider_id.clone());
        if provider_has_builtin_defaults(&provider_id)
            || row
                .get("display_name")
                .and_then(Value::as_str)
                .map(|value| clean_text(value, 120).is_empty())
                .unwrap_or(true)
        {
            row["display_name"] = json!(provider_display_name(&provider_id));
        }
        if provider_has_builtin_defaults(&provider_id) {
            row["is_local"] = json!(provider_is_local(&provider_id));
            row["needs_key"] = json!(provider_needs_key(&provider_id));
        }
        if row
            .get("api_key_env")
            .and_then(Value::as_str)
            .map(|value| clean_text(value, 120).is_empty())
            .unwrap_or(true)
        {
            row["api_key_env"] = json!(provider_api_key_env(&provider_id));
        }
        if row.get("base_url").and_then(Value::as_str).unwrap_or("").trim().is_empty() {
            row["base_url"] = json!(provider_base_url_default(&provider_id));
        }
        let base_url = clean_text(
            row.get("base_url").and_then(Value::as_str).unwrap_or(""),
            400,
        );
        let key_present = provider_key(root, &provider_id).is_some();
        let local_reachable = if provider_is_local(&provider_id) {
            local_provider_reachable(&provider_id, row)
        } else {
            row.get("reachable").and_then(Value::as_bool).unwrap_or(false)
        };
        if provider_is_local(&provider_id) {
            row["auth_status"] = json!(if provider_id == "claude-code" && !local_reachable {
                "not_set"
            } else {
                "configured"
            });
            row["reachable"] = json!(local_reachable);
        } else if key_present {
            row["auth_status"] = json!("configured");
        } else if !row.get("auth_status").and_then(Value::as_str).map(auth_status_configured).unwrap_or(false) {
            row["auth_status"] = json!("not_set");
        }
        row["supports_chat"] = json!(provider_supports_chat(&provider_id, &base_url));
        if row.get("model_profiles").and_then(Value::as_object).map(|obj| obj.is_empty()).unwrap_or(true) {
            row["model_profiles"] = Value::Object(model_profiles_for_provider(&provider_id));
        }
        if row
            .get("detected_models")
            .and_then(Value::as_array)
            .map(|rows| rows.is_empty())
            .unwrap_or(true)
        {
            let detected = row
                .get("model_profiles")
                .and_then(Value::as_object)
                .map(|obj| obj.keys().cloned().map(Value::String).collect::<Vec<_>>())
                .unwrap_or_default();
            row["detected_models"] = Value::Array(detected);
        }
        if provider_id == "google" {
            row["aliases"] = json!(["gemini"]);
        }
    }
    rows.sort_by(|a, b| {
        clean_text(a.get("id").and_then(Value::as_str).unwrap_or(""), 120)
            .cmp(&clean_text(
                b.get("id").and_then(Value::as_str).unwrap_or(""),
                120,
            ))
    });
    rows
}

pub fn providers_payload(root: &Path, snapshot: &Value) -> Value {
    json!({"ok": true, "providers": provider_rows(root, snapshot)})
}

pub fn save_provider_key(root: &Path, provider_id: &str, key: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let secret = clean_text(key, 4096);
    if provider.is_empty() || secret.is_empty() || provider == "auto" {
        return json!({"ok": false, "error": "provider_key_invalid"});
    }
    let mut secrets = load_secrets(root);
    if secrets.get("providers").is_none() || !secrets.get("providers").map(Value::is_object).unwrap_or(false) {
        secrets["providers"] = json!({});
    }
    secrets["providers"][provider.clone()] = json!({"key": secret, "updated_at": crate::now_iso()});
    save_secrets(root, secrets);

    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!("configured");
    row["key_prefix"] = json!(masked_prefix(key));
    row["key_last4"] = json!(masked_last4(key));
    row["key_hash"] = json!(crate::deterministic_receipt_hash(&json!({"provider": provider, "key": key})));
    row["key_set_at"] = json!(crate::now_iso());
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({
        "ok": true,
        "provider": provider,
        "auth_status": "configured",
        "switched_default": false
    })
}

pub fn remove_provider_key(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut secrets = load_secrets(root);
    if let Some(obj) = secrets.get_mut("providers").and_then(Value::as_object_mut) {
        obj.remove(&provider);
    }
    save_secrets(root, secrets);
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["auth_status"] = json!(if provider_is_local(&provider) {
        "configured"
    } else {
        "not_set"
    });
    row["key_prefix"] = json!("");
    row["key_last4"] = json!("");
    row["key_hash"] = json!("");
    row["key_set_at"] = json!("");
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider})
}

pub fn set_provider_url(root: &Path, provider_id: &str, base_url: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let cleaned = clean_text(base_url, 400);
    if provider.is_empty() || cleaned.is_empty() {
        return json!({"ok": false, "error": "provider_url_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    row["base_url"] = json!(cleaned);
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    let probe = test_provider(root, &provider);
    json!({
        "ok": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "provider": provider,
        "reachable": probe.get("status").and_then(Value::as_str) == Some("ok"),
        "latency_ms": probe.get("latency_ms").cloned().unwrap_or_else(|| json!(0)),
        "error": probe.get("error").cloned().unwrap_or(Value::Null)
    })
}

pub fn test_provider(root: &Path, provider_id: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let started = Instant::now();
    if provider == "claude-code" {
        let ok = Command::new("sh")
            .arg("-lc")
            .arg("command -v claude >/dev/null 2>&1")
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        return if ok {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "claude_code_cli_not_detected"})
        };
    }

    if provider == "auto" {
        let providers = provider_rows(root, &json!({}));
        let ready = providers.into_iter().any(|row| {
            row.get("is_local").and_then(Value::as_bool).unwrap_or(false)
                || auth_status_configured(row.get("auth_status").and_then(Value::as_str).unwrap_or(""))
        });
        return if ready {
            json!({"ok": true, "status": "ok", "provider": provider, "latency_ms": started.elapsed().as_millis() as i64})
        } else {
            json!({"ok": false, "status": "error", "provider": provider, "error": "no_configured_provider"})
        };
    }

    let row = provider_row(root, &provider);
    let base_url = clean_text(
        row.get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let mut headers = vec!["Content-Type: application/json".to_string()];
    let url = match provider.as_str() {
        "ollama" => format!("{base_url}/api/tags"),
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            format!("{base_url}/v1beta/models?key={key}")
        }
        "anthropic" => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("x-api-key: {key}"));
            headers.push("anthropic-version: 2023-06-01".to_string());
            format!("{base_url}/v1/models")
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return json!({"ok": false, "status": "error", "provider": provider, "error": "provider_key_missing"});
            };
            headers.push(format!("Authorization: Bearer {key}"));
            format!("{base_url}/models")
        }
    };

    match curl_json(&url, "GET", &headers, None, 20) {
        Ok((status, value)) if status >= 200 && status < 300 => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(true);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": true,
                "status": "ok",
                "provider": provider,
                "latency_ms": started.elapsed().as_millis() as i64,
                "detail": value
            })
        }
        Ok((status, value)) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({
                "ok": false,
                "status": "error",
                "provider": provider,
                "error": format!("http_{status}:{}", error_text_from_value(&value))
            })
        }
        Err(err) => {
            let mut registry = load_registry(root);
            let row = ensure_provider_row_mut(&mut registry, &provider);
            row["reachable"] = json!(false);
            row["updated_at"] = json!(crate::now_iso());
            save_registry(root, registry);
            json!({"ok": false, "status": "error", "provider": provider, "error": clean_text(&err, 280)})
        }
    }
}

pub fn discover_models(root: &Path, input: &str) -> Value {
    let cleaned = clean_text(input, 4096);
    if cleaned.is_empty() {
        return json!({"ok": false, "error": "discover_input_required"});
    }
    let candidate_path = PathBuf::from(&cleaned);
    if candidate_path.exists() {
        let provider = "local";
        let mut profiles = Map::<String, Value>::new();
        let mut local_paths = Vec::<Value>::new();
        if candidate_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&candidate_path) {
                for entry in entries.flatten().take(128) {
                    let name = clean_text(&entry.file_name().to_string_lossy(), 140);
                    if name.is_empty() {
                        continue;
                    }
                    profiles.insert(
                        name.clone(),
                        json!({
                            "power_rating": 3,
                            "cost_rating": 1,
                            "param_count_billion": 0,
                            "specialty": "general",
                            "specialty_tags": ["general"],
                            "deployment_kind": "local",
                            "local_download_path": entry.path().to_string_lossy().to_string(),
                            "download_available": true,
                            "updated_at": crate::now_iso()
                        }),
                    );
                    local_paths.push(json!(entry.path().to_string_lossy().to_string()));
                }
            }
        }
        let mut registry = load_registry(root);
        let row = ensure_provider_row_mut(&mut registry, provider);
        row["is_local"] = json!(true);
        row["needs_key"] = json!(false);
        row["auth_status"] = json!("configured");
        row["reachable"] = json!(true);
        row["local_model_root"] = json!(candidate_path.to_string_lossy().to_string());
        row["local_model_paths"] = json!(local_paths);
        row["model_profiles"] = Value::Object(profiles.clone());
        row["updated_at"] = json!(crate::now_iso());
        save_registry(root, registry);
        return json!({
            "ok": true,
            "provider": provider,
            "input_kind": "local_path",
            "model_count": profiles.len(),
            "models": profiles.keys().cloned().collect::<Vec<_>>()
        });
    }

    let provider = guess_provider_from_key(&cleaned);
    let saved = save_provider_key(root, &provider, &cleaned);
    if !saved.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return saved;
    }
    let row = provider_row(root, &provider);
    let models = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "ok": true,
        "provider": provider,
        "input_kind": "api_key",
        "model_count": models.len(),
        "models": models
    })
}

pub fn add_custom_model(
    root: &Path,
    provider_id: &str,
    model_id: &str,
    context_window: i64,
    max_output_tokens: i64,
) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut model = clean_text(model_id, 240);
    if model.contains('/') {
        let mut parts = model.splitn(2, '/');
        let maybe_provider = normalize_provider_id(parts.next().unwrap_or(""));
        let maybe_model = clean_text(parts.next().unwrap_or(""), 200);
        if !maybe_provider.is_empty() && !maybe_model.is_empty() {
            model = maybe_model;
        }
    }
    if provider.is_empty() || model.is_empty() {
        return json!({"ok": false, "error": "custom_model_invalid"});
    }
    let mut registry = load_registry(root);
    let row = ensure_provider_row_mut(&mut registry, &provider);
    if row.get("model_profiles").is_none() || !row.get("model_profiles").map(Value::is_object).unwrap_or(false) {
        row["model_profiles"] = json!({});
    }
    row["model_profiles"][model.clone()] = json!({
        "power_rating": 3,
        "cost_rating": if provider_is_local(&provider) { 1 } else { 3 },
        "param_count_billion": 0,
        "specialty": "general",
        "specialty_tags": ["general"],
        "deployment_kind": if provider_is_local(&provider) { "local" } else { "api" },
        "context_window": context_window.max(0),
        "max_output_tokens": max_output_tokens.max(0),
        "download_available": provider_is_local(&provider),
        "local_download_path": "",
        "custom": true,
        "updated_at": crate::now_iso()
    });
    row["updated_at"] = json!(crate::now_iso());
    save_registry(root, registry);
    json!({"ok": true, "provider": provider, "model": model})
}

pub fn delete_custom_model(root: &Path, model_ref: &str) -> Value {
    let cleaned = clean_text(model_ref, 240);
    if cleaned.is_empty() {
        return json!({"ok": false, "error": "custom_model_ref_required"});
    }
    let mut registry = load_registry(root);
    let mut removed = false;
    if let Some(providers) = registry.get_mut("providers").and_then(Value::as_object_mut) {
        for (provider_id, row) in providers.iter_mut() {
            let provider_id_clean = normalize_provider_id(provider_id);
            let target = if cleaned.starts_with(&(provider_id_clean.clone() + "/")) {
                clean_text(cleaned.split_once('/').map(|(_, tail)| tail).unwrap_or(""), 200)
            } else {
                cleaned.clone()
            };
            if let Some(models) = row.get_mut("model_profiles").and_then(Value::as_object_mut) {
                if models.remove(&target).is_some() {
                    removed = true;
                    row["updated_at"] = json!(crate::now_iso());
                    break;
                }
            }
        }
    }
    save_registry(root, registry);
    json!({"ok": removed, "removed": removed, "model": cleaned})
}

pub fn download_model(root: &Path, provider_id: &str, model_ref: &str) -> Value {
    let provider = normalize_provider_id(provider_id);
    let mut model = clean_text(model_ref, 240);
    if model.contains('/') {
        let mut parts = model.splitn(2, '/');
        let maybe_provider = normalize_provider_id(parts.next().unwrap_or(""));
        let maybe_model = clean_text(parts.next().unwrap_or(""), 200);
        if maybe_provider == "ollama" {
            return download_model(root, "ollama", &maybe_model);
        }
        if !maybe_model.is_empty() {
            model = maybe_model;
        }
    }
    if provider == "ollama" {
        let output = Command::new("ollama")
            .arg("pull")
            .arg(&model)
            .output();
        return match output {
            Ok(out) if out.status.success() => json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "method": "ollama_pull",
                "download_path": format!("ollama://{}", model)
            }),
            Ok(out) => json!({
                "ok": false,
                "error": clean_text(
                    &format!(
                        "{} {}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                    280
                )
            }),
            Err(err) => json!({"ok": false, "error": clean_text(&err.to_string(), 280)}),
        };
    }

    let row = provider_row(root, &provider);
    let path = row
        .get("model_profiles")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&model))
        .and_then(|profile| profile.get("local_download_path").and_then(Value::as_str))
        .map(|raw| clean_text(raw, 4000))
        .unwrap_or_default();
    if path.is_empty() {
        return json!({"ok": false, "error": "model_download_path_missing"});
    }
    let download_path = PathBuf::from(&path);
    let _ = fs::create_dir_all(&download_path);
    json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "method": "prepare_local_path",
        "download_path": download_path.to_string_lossy().to_string()
    })
}

fn invoke_chat_live(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let system = clean_text(system_prompt, 12_000);
    let mut messages = content_from_message_rows(session_messages);
    let user = clean_text(user_message, 16_000);
    if user.is_empty() {
        return Err("message_required".to_string());
    }
    messages.push(("user".to_string(), user.clone()));
    let base_url = clean_text(
        provider_row(root, &provider)
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or(&provider_base_url_default(&provider)),
        400,
    );
    let started = Instant::now();
    let context_window = model_context_window(root, &provider, &model);

    let response = match provider.as_str() {
        "ollama" => {
            let mut rows = Vec::<Value>::new();
            if !system.is_empty() {
                rows.push(json!({"role":"system","content": system}));
            }
            for (role, text) in &messages {
                rows.push(json!({"role": if role == "assistant" { "assistant" } else { "user" }, "content": text}));
            }
            let payload = json!({
                "model": model,
                "stream": false,
                "messages": rows
            });
            let (status, value) = curl_json(
                &format!("{base_url}/api/chat"),
                "POST",
                &["Content-Type: application/json".to_string()],
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = clean_text(
                value.pointer("/message/content").and_then(Value::as_str).unwrap_or(""),
                32_000,
            );
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.get("prompt_eval_count").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.get("eval_count").and_then(Value::as_i64).unwrap_or((text.len() / 4) as i64),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        "anthropic" => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let payload = json!({
                "model": model,
                "system": system,
                "max_tokens": 4096,
                "messages": messages.iter().map(|(role, text)| {
                    json!({
                        "role": if role == "assistant" { "assistant" } else { "user" },
                        "content": text
                    })
                }).collect::<Vec<_>>()
            });
            let headers = vec![
                "Content-Type: application/json".to_string(),
                format!("x-api-key: {key}"),
                "anthropic-version: 2023-06-01".to_string(),
            ];
            let (status, value) = curl_json(&format!("{base_url}/v1/messages"), "POST", &headers, Some(&payload), 180)?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_anthropic_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usage/input_tokens").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usage/output_tokens").and_then(Value::as_i64).unwrap_or(1.max((extract_anthropic_text(&value).len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        "google" => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let payload = json!({
                "system_instruction": if system.is_empty() { Value::Null } else { json!({"parts":[{"text": system}]}) },
                "contents": messages.iter().map(|(role, text)| {
                    json!({
                        "role": if role == "assistant" { "model" } else { "user" },
                        "parts": [{"text": text}]
                    })
                }).collect::<Vec<_>>()
            });
            let (status, value) = curl_json(
                &format!("{base_url}/v1beta/models/{}:generateContent?key={}", urlencoding::encode(&model), key),
                "POST",
                &["Content-Type: application/json".to_string()],
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_google_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usageMetadata/promptTokenCount").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usageMetadata/candidatesTokenCount").and_then(Value::as_i64).unwrap_or(1.max((text.len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
        _ => {
            let Some(key) = provider_key(root, &provider) else {
                return Err("couldn't reach a chat model backend: provider key missing".to_string());
            };
            let mut rows = Vec::<Value>::new();
            if !system.is_empty() {
                rows.push(json!({"role": "system", "content": system}));
            }
            for (role, text) in &messages {
                rows.push(json!({"role": if role == "assistant" { "assistant" } else { "user" }, "content": text}));
            }
            let payload = json!({
                "model": model,
                "stream": false,
                "messages": rows
            });
            let headers = vec![
                "Content-Type: application/json".to_string(),
                format!("Authorization: Bearer {key}"),
            ];
            let (status, value) = curl_json(
                &format!("{base_url}/chat/completions"),
                "POST",
                &headers,
                Some(&payload),
                180,
            )?;
            if !(200..300).contains(&status) {
                return Err(format!("model backend unavailable: {}", error_text_from_value(&value)));
            }
            let text = extract_openai_text(&value);
            json!({
                "ok": true,
                "provider": provider,
                "model": model,
                "runtime_model": model,
                "response": text,
                "input_tokens": value.pointer("/usage/prompt_tokens").and_then(Value::as_i64).unwrap_or(((system.len() + user.len()) / 4) as i64),
                "output_tokens": value.pointer("/usage/completion_tokens").and_then(Value::as_i64).unwrap_or(1.max((text.len() / 4) as i64)),
                "cost_usd": 0.0,
                "context_window": context_window,
                "latency_ms": started.elapsed().as_millis() as i64,
                "tools": []
            })
        }
    };
    let text = clean_text(
        response.get("response").and_then(Value::as_str).unwrap_or(""),
        32_000,
    );
    if text.is_empty() {
        return Err("model backend unavailable: empty_response".to_string());
    }
    Ok(response)
}

#[cfg(test)]
fn invoke_chat_impl(
    _root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    _session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let model = clean_text(model_name, 240);
    let system = clean_text(system_prompt, 1_000);
    let user = clean_text(user_message, 16_000);
    if user.is_empty() {
        return Err("message_required".to_string());
    }
    let response = if system.is_empty() {
        format!("[{provider}/{model}] {user}")
    } else {
        format!("[{provider}/{model}] {system} | {user}")
    };
    Ok(json!({
        "ok": true,
        "provider": provider,
        "model": model,
        "runtime_model": model,
        "response": response,
        "input_tokens": ((user.len() as i64) / 4).max(1),
        "output_tokens": ((response.len() as i64) / 4).max(1),
        "cost_usd": 0.0,
        "context_window": 0,
        "latency_ms": 1,
        "tools": []
    }))
}

#[cfg(not(test))]
fn invoke_chat_impl(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    invoke_chat_live(
        root,
        provider_id,
        model_name,
        system_prompt,
        session_messages,
        user_message,
    )
}

pub fn invoke_chat(
    root: &Path,
    provider_id: &str,
    model_name: &str,
    system_prompt: &str,
    session_messages: &[Value],
    user_message: &str,
) -> Result<Value, String> {
    invoke_chat_impl(
        root,
        provider_id,
        model_name,
        system_prompt,
        session_messages,
        user_message,
    )
}
