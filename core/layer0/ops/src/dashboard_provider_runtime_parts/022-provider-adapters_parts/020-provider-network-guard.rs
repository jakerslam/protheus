
fn provider_network_guard(root: &Path, provider_id: &str, base_url: &str) -> Result<Value, String> {
    let provider = normalize_provider_id(provider_id);
    let host = url_host(base_url);
    let host_loopback = host_is_loopback(&host);
    let policy = provider_network_policy(root);
    let relaxed_test_mode = policy
        .get("relaxed_test_mode")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || web_tooling_relaxed_test_mode_env_enabled();
    if relaxed_test_mode {
        let has_provider_key = provider_key(root, &provider).is_some();
        let decision = json!({
            "allowed": true,
            "provider": provider,
            "host": host,
            "host_is_loopback": host_loopback,
            "local_first_default": policy.get("local_first_default").and_then(Value::as_bool).unwrap_or(false),
            "consent_via_provider_key": has_provider_key,
            "consent_via_allowlist": true,
            "policy_bypass": true,
            "bypass_reason": "web_tooling_relaxed_test_mode"
        });
        append_provider_outbound_guard_receipt(root, decision.clone());
        return Ok(decision);
    }
    let denied = policy
        .get("deny_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|v| clean_text(v, 220)))
        .filter(|domain| !domain.is_empty())
        .any(|domain| !host_loopback && host_matches_domain(&host, &domain));
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
        "host_is_loopback": host_loopback,
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
            return Err(model_backend_unavailable(&value));
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

struct FrontierProviderAdapter;
