const PROVIDER_INFERENCE_RECEIPTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_inference_receipts.jsonl";
const PROVIDER_OUTBOUND_GUARD_RECEIPTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_outbound_guard_receipts.jsonl";
const PROVIDER_NETWORK_POLICY_REL: &str = "client/runtime/config/provider_network_policy.json";
const DEFAULT_TELEMETRY_BLOCKLIST: &[&str] = &[
    "segment.io",
    "sentry.io",
    "mixpanel.com",
    "amplitude.com",
    "datadoghq.com",
    "newrelic.com",
];
const DEFAULT_DENY_DOMAINS: &[&str] = &[
    "127.0.0.1",
    "localhost",
    "metadata.google.internal",
    "169.254.169.254",
];

fn provider_network_policy_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_NETWORK_POLICY_REL)
}

fn default_provider_network_policy() -> Value {
    json!({
        "type": "infring_provider_network_policy",
        "version": "v1",
        "local_first_default": true,
        "require_explicit_provider_consent": true,
        "telemetry_blocklist_enabled": true,
        "telemetry_blocklist_domains": DEFAULT_TELEMETRY_BLOCKLIST,
        "deny_domains": DEFAULT_DENY_DOMAINS,
        "allow_provider_ids": [],
        "updated_at": crate::now_iso(),
    })
}

fn provider_network_policy(root: &Path) -> Value {
    let path = provider_network_policy_path(root);
    if !path.exists() {
        write_json_pretty(&path, &default_provider_network_policy());
    }
    let mut policy = read_json(&path).unwrap_or_else(default_provider_network_policy);
    if !policy.is_object() {
        policy = default_provider_network_policy();
    }
    if policy.get("type").and_then(Value::as_str).unwrap_or("") != "infring_provider_network_policy" {
        policy["type"] = json!("infring_provider_network_policy");
    }
    if policy.get("version").and_then(Value::as_str).unwrap_or("").is_empty() {
        policy["version"] = json!("v1");
    }
    if policy.get("local_first_default").and_then(Value::as_bool).is_none() {
        policy["local_first_default"] = json!(true);
    }
    if policy
        .get("require_explicit_provider_consent")
        .and_then(Value::as_bool)
        .is_none()
    {
        policy["require_explicit_provider_consent"] = json!(true);
    }
    if policy
        .get("telemetry_blocklist_enabled")
        .and_then(Value::as_bool)
        .is_none()
    {
        policy["telemetry_blocklist_enabled"] = json!(true);
    }
    if !policy
        .get("telemetry_blocklist_domains")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        policy["telemetry_blocklist_domains"] = json!(DEFAULT_TELEMETRY_BLOCKLIST);
    }
    if !policy.get("deny_domains").map(Value::is_array).unwrap_or(false) {
        policy["deny_domains"] = json!(DEFAULT_DENY_DOMAINS);
    }
    if !policy
        .get("allow_provider_ids")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        policy["allow_provider_ids"] = json!([]);
    }
    policy
}

fn provider_inference_receipts_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_INFERENCE_RECEIPTS_REL)
}

fn provider_outbound_guard_receipts_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_OUTBOUND_GUARD_RECEIPTS_REL)
}

fn append_jsonl_row(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(format!("{raw}\n").as_bytes()));
    }
}

fn append_provider_inference_receipt(root: &Path, mut row: Value) {
    if !row.is_object() {
        row = json!({});
    }
    row["type"] = json!("infring_provider_inference_receipt");
    row["ts"] = json!(crate::now_iso());
    row["receipt_hash"] = json!(crate::deterministic_receipt_hash(&row));
    append_jsonl_row(&provider_inference_receipts_path(root), &row);
}

fn append_provider_outbound_guard_receipt(root: &Path, mut row: Value) {
    if !row.is_object() {
        row = json!({});
    }
    row["type"] = json!("infring_provider_outbound_guard_receipt");
    row["ts"] = json!(crate::now_iso());
    row["receipt_hash"] = json!(crate::deterministic_receipt_hash(&row));
    append_jsonl_row(&provider_outbound_guard_receipts_path(root), &row);
}

fn url_host(raw: &str) -> String {
    let cleaned = clean_text(raw, 500).to_ascii_lowercase();
    let trimmed = cleaned
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    clean_text(
        trimmed
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .split('@')
            .next_back()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .trim_matches('.'),
        220,
    )
    .to_ascii_lowercase()
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    let host_clean = clean_text(host, 220).to_ascii_lowercase();
    let domain_clean = clean_text(domain, 220).to_ascii_lowercase();
    if host_clean.is_empty() || domain_clean.is_empty() {
        return false;
    }
    host_clean == domain_clean || host_clean.ends_with(&format!(".{domain_clean}"))
}

fn provider_network_guard(root: &Path, provider_id: &str, base_url: &str) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let host = url_host(base_url);
    let policy = provider_network_policy(root);
    let denied = policy
        .get("deny_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|v| clean_text(v, 220)))
        .filter(|domain| !domain.is_empty())
        .any(|domain| host_matches_domain(&host, &domain));
    if denied {
        append_provider_outbound_guard_receipt(
            root,
            json!({
                "provider": provider,
                "host": host,
                "allowed": false,
                "reason": "denied_domain"
            }),
        );
        return Err("provider_network_policy_blocked:denied_domain".to_string());
    }
    let telemetry_blocked = policy
        .get("telemetry_blocklist_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        && policy
            .get("telemetry_blocklist_domains")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row.as_str().map(|v| clean_text(v, 220)))
            .filter(|domain| !domain.is_empty())
            .any(|domain| host_matches_domain(&host, &domain));
    if telemetry_blocked {
        append_provider_outbound_guard_receipt(
            root,
            json!({
                "provider": provider,
                "host": host,
                "allowed": false,
                "reason": "telemetry_blocklist_domain"
            }),
        );
        return Err("provider_network_policy_blocked:telemetry_blocklist_domain".to_string());
    }

    let local_first = policy
        .get("local_first_default")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let require_explicit_provider_consent = policy
        .get("require_explicit_provider_consent")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let allow_provider_ids = policy
        .get("allow_provider_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|v| normalize_provider_id(v)))
        .collect::<Vec<_>>();
    let has_provider_key = provider_key(root, &provider).is_some();
    let provider_explicitly_allowed = allow_provider_ids.iter().any(|row| row == &provider);
    let needs_cloud_opt_in = local_first
        && require_explicit_provider_consent
        && !provider_is_local(&provider)
        && !has_provider_key
        && !provider_explicitly_allowed;
    if needs_cloud_opt_in {
        append_provider_outbound_guard_receipt(
            root,
            json!({
                "provider": provider,
                "host": host,
                "allowed": false,
                "reason": "local_first_opt_in_required"
            }),
        );
        return Err("provider_network_policy_blocked:local_first_opt_in_required".to_string());
    }
    let decision = json!({
        "allowed": true,
        "provider": provider,
        "host": host,
        "local_first_default": local_first,
        "consent_via_provider_key": has_provider_key,
        "consent_via_allowlist": provider_explicitly_allowed
    });
    append_provider_outbound_guard_receipt(root, decision.clone());
    Ok(decision)
}

struct ProviderInvokeInput<'a> {
    root: &'a Path,
    provider: &'a str,
    model: &'a str,
    base_url: &'a str,
    system: &'a str,
    messages: &'a [(String, String)],
    prefill: &'a str,
    user: &'a str,
    context_window: i64,
    started: Instant,
}

trait LlmProviderAdapter {
    fn supports(&self, provider: &str) -> bool;
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String>;
}

fn openai_style_messages(system: &str, messages: &[(String, String)]) -> Vec<Value> {
    let mut rows = Vec::<Value>::new();
    if !system.is_empty() {
        rows.push(json!({"role": "system", "content": system}));
    }
    for (role, text) in messages {
        rows.push(json!({
            "role": if role == "assistant" { "assistant" } else { "user" },
            "content": text
        }));
    }
    rows
}

fn provider_response_row(
    input: &ProviderInvokeInput<'_>,
    text: &str,
    input_tokens: i64,
    output_tokens: i64,
    latency_ms: i64,
) -> Value {
    json!({
        "ok": true,
        "provider": input.provider,
        "model": input.model,
        "runtime_model": input.model,
        "response": clean_chat_text(text, 32_000),
        "input_tokens": input_tokens.max(1),
        "output_tokens": output_tokens.max(1),
        "cost_usd": 0.0,
        "context_window": input.context_window.max(0),
        "latency_ms": latency_ms.max(1),
        "tools": [],
        "assistant_prefill_used": !input.prefill.is_empty()
    })
}

struct OllamaAdapter;
impl LlmProviderAdapter for OllamaAdapter {
    fn supports(&self, provider: &str) -> bool {
        provider == "ollama"
    }
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
        let payload = json!({
            "model": input.model,
            "stream": false,
            "messages": openai_style_messages(input.system, input.messages)
        });
        let (status, value) = curl_json(
            &format!("{}/api/chat", input.base_url),
            "POST",
            &["Content-Type: application/json".to_string()],
            Some(&payload),
            180,
        )?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "model backend unavailable: {}",
                error_text_from_value(&value)
            ));
        }
        let text = clean_chat_text(
            value
                .pointer("/message/content")
                .and_then(Value::as_str)
                .unwrap_or(""),
            32_000,
        );
        Ok(provider_response_row(
            input,
            &text,
            value
                .get("prompt_eval_count")
                .and_then(Value::as_i64)
                .unwrap_or(((input.system.len() + input.user.len()) / 4) as i64),
            value
                .get("eval_count")
                .and_then(Value::as_i64)
                .unwrap_or((text.len() / 4) as i64),
            input.started.elapsed().as_millis() as i64,
        ))
    }
}

struct AnthropicAdapter;
impl LlmProviderAdapter for AnthropicAdapter {
    fn supports(&self, provider: &str) -> bool {
        provider == "anthropic"
    }
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
        let Some(key) = provider_key(input.root, input.provider) else {
            return Err("couldn't reach a chat model backend: provider key missing".to_string());
        };
        let payload = json!({
            "model": input.model,
            "system": input.system,
            "max_tokens": 4096,
            "messages": input.messages.iter().map(|(role, text)| {
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
        let (status, value) = curl_json(
            &format!("{}/v1/messages", input.base_url),
            "POST",
            &headers,
            Some(&payload),
            180,
        )?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "model backend unavailable: {}",
                error_text_from_value(&value)
            ));
        }
        let text = extract_anthropic_text(&value);
        Ok(provider_response_row(
            input,
            &text,
            value
                .pointer("/usage/input_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(((input.system.len() + input.user.len()) / 4) as i64),
            value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_i64)
                .unwrap_or((text.len() / 4) as i64),
            input.started.elapsed().as_millis() as i64,
        ))
    }
}

struct GoogleAdapter;
impl LlmProviderAdapter for GoogleAdapter {
    fn supports(&self, provider: &str) -> bool {
        provider == "google"
    }
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
        let Some(key) = provider_key(input.root, input.provider) else {
            return Err("couldn't reach a chat model backend: provider key missing".to_string());
        };
        let payload = json!({
            "system_instruction": if input.system.is_empty() { Value::Null } else { json!({"parts":[{"text": input.system}]}) },
            "contents": input.messages.iter().map(|(role, text)| {
                json!({
                    "role": if role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": text}]
                })
            }).collect::<Vec<_>>()
        });
        let (status, value) = curl_json(
            &format!(
                "{}/v1beta/models/{}:generateContent?key={}",
                input.base_url,
                urlencoding::encode(input.model),
                key
            ),
            "POST",
            &["Content-Type: application/json".to_string()],
            Some(&payload),
            180,
        )?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "model backend unavailable: {}",
                error_text_from_value(&value)
            ));
        }
        let text = extract_google_text(&value);
        Ok(provider_response_row(
            input,
            &text,
            value
                .pointer("/usageMetadata/promptTokenCount")
                .and_then(Value::as_i64)
                .unwrap_or(((input.system.len() + input.user.len()) / 4) as i64),
            value
                .pointer("/usageMetadata/candidatesTokenCount")
                .and_then(Value::as_i64)
                .unwrap_or((text.len() / 4) as i64),
            input.started.elapsed().as_millis() as i64,
        ))
    }
}

struct OpenAiCompatAdapter;
impl LlmProviderAdapter for OpenAiCompatAdapter {
    fn supports(&self, _provider: &str) -> bool {
        true
    }
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
        let Some(key) = provider_key(input.root, input.provider) else {
            return Err("couldn't reach a chat model backend: provider key missing".to_string());
        };
        let payload = json!({
            "model": input.model,
            "stream": false,
            "messages": openai_style_messages(input.system, input.messages)
        });
        let headers = vec![
            "Content-Type: application/json".to_string(),
            format!("Authorization: Bearer {key}"),
        ];
        let (status, value) = curl_json(
            &format!("{}/chat/completions", input.base_url),
            "POST",
            &headers,
            Some(&payload),
            180,
        )?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "model backend unavailable: {}",
                error_text_from_value(&value)
            ));
        }
        let text = extract_openai_text(&value);
        Ok(provider_response_row(
            input,
            &text,
            value
                .pointer("/usage/prompt_tokens")
                .and_then(Value::as_i64)
                .unwrap_or(((input.system.len() + input.user.len()) / 4) as i64),
            value
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_i64)
                .unwrap_or((text.len() / 4) as i64),
            input.started.elapsed().as_millis() as i64,
        ))
    }
}

fn invoke_provider_via_adapter(input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
    let policy_decision = provider_network_guard(input.root, input.provider, input.base_url)?;
    let ollama = OllamaAdapter;
    let anthropic = AnthropicAdapter;
    let google = GoogleAdapter;
    let openai = OpenAiCompatAdapter;
    let adapters: [&dyn LlmProviderAdapter; 4] = [&ollama, &anthropic, &google, &openai];
    for adapter in adapters {
        if !adapter.supports(input.provider) {
            continue;
        }
        let mut response = adapter.invoke(input)?;
        response["policy_decision"] = policy_decision;
        return Ok(response);
    }
    Err("model backend unavailable: provider_adapter_missing".to_string())
}

#[cfg(test)]
mod provider_adapter_tests {
    use super::*;

    #[test]
    fn provider_network_guard_blocks_unapproved_cloud_when_local_first() {
        let root = tempfile::tempdir().expect("tempdir");
        let decision = provider_network_guard(root.path(), "openai", "https://api.openai.com/v1");
        assert!(
            decision
                .err()
                .map(|err| err.contains("local_first_opt_in_required"))
                .unwrap_or(false),
            "cloud provider should be blocked until explicit consent exists"
        );
    }
}
