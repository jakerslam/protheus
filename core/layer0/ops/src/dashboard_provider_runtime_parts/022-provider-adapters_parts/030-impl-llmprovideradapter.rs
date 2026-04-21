impl LlmProviderAdapter for FrontierProviderAdapter {
    fn supports(&self, provider: &str) -> bool {
        provider == "frontier_provider"
    }
    fn invoke(&self, input: &ProviderInvokeInput<'_>) -> Result<Value, String> {
        let key = provider_key_or_error(input.root, input.provider)?;
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
            return Err(model_backend_unavailable(&value));
        }
        let text = extract_frontier_provider_text(&value);
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
        let key = provider_key_or_error(input.root, input.provider)?;
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
            return Err(model_backend_unavailable(&value));
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
        let key = provider_key_or_error(input.root, input.provider)?;
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
            return Err(model_backend_unavailable(&value));
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
    let frontier_provider = FrontierProviderAdapter;
    let google = GoogleAdapter;
    let openai = OpenAiCompatAdapter;
    let adapters: [&dyn LlmProviderAdapter; 4] = [&ollama, &frontier_provider, &google, &openai];
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

    #[test]
    fn provider_network_guard_allows_loopback_for_local_provider() {
        let root = tempfile::tempdir().expect("tempdir");
        let decision = provider_network_guard(root.path(), "ollama", "http://127.0.0.1:11434");
        assert!(decision.is_ok(), "loopback should stay available for local providers");
    }

    #[test]
    fn provider_network_guard_blocks_metadata_domain() {
        let root = tempfile::tempdir().expect("tempdir");
        let decision = provider_network_guard(
            root.path(),
            "openai",
            "https://metadata.google.internal/v1",
        );
        assert!(
            decision
                .err()
                .map(|err| err.contains("denied_domain"))
                .unwrap_or(false),
            "metadata host should remain fail-closed"
        );
    }

    #[test]
    fn provider_network_guard_allows_cloud_when_relaxed_test_mode_enabled() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut policy = default_provider_network_policy();
        policy["relaxed_test_mode"] = json!(true);
        write_json_pretty(&provider_network_policy_path(root.path()), &policy);
        let decision = provider_network_guard(root.path(), "openai", "https://api.openai.com/v1")
            .expect("relaxed mode should bypass local-first block");
        assert_eq!(
            decision.get("policy_bypass").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            decision.get("bypass_reason").and_then(Value::as_str),
            Some("web_tooling_relaxed_test_mode")
        );
    }
}
