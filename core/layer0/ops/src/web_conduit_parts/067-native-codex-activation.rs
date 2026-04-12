const NATIVE_CODEX_PROVIDER_SECRETS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_secrets.json";
const NATIVE_CODEX_OPENAI_ENV_KEYS: &[&str] = &["OPENAI_API_KEY"];

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeCodexUserLocation {
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
    timezone: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeCodexConfig {
    enabled: bool,
    mode: String,
    allowed_domains: Vec<String>,
    context_size: Option<String>,
    user_location: Option<NativeCodexUserLocation>,
}

fn native_codex_provider_secrets_path(root: &Path) -> PathBuf {
    root.join(NATIVE_CODEX_PROVIDER_SECRETS_REL)
}

fn native_codex_policy_section<'a>(policy: &'a Value) -> &'a Value {
    policy
        .pointer("/web_conduit/native_codex_web_search")
        .or_else(|| policy.get("native_codex_web_search"))
        .unwrap_or(&Value::Null)
}

fn native_codex_optional_string(value: Option<&Value>, max_len: usize) -> Option<String> {
    value.and_then(Value::as_str).map(|raw| clean_text(raw, max_len)).and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn native_codex_config_value<'a>(section: &'a Value, snake: &str, camel: &str) -> Option<&'a Value> {
    section.get(snake).or_else(|| section.get(camel))
}

fn native_codex_bool_value(section: &Value, snake: &str, camel: &str, default: bool) -> bool {
    native_codex_config_value(section, snake, camel)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn native_codex_mode(value: Option<&Value>) -> String {
    match native_codex_optional_string(value, 24)
        .unwrap_or_else(|| "cached".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "live" => "live".to_string(),
        _ => "cached".to_string(),
    }
}

fn native_codex_allowed_domains(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(rows)) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        if let Some(domain) = native_codex_optional_string(Some(row), 240) {
            if !out.iter().any(|current| current == &domain) {
                out.push(domain);
            }
        }
    }
    out
}

fn native_codex_context_size(value: Option<&Value>) -> Option<String> {
    match native_codex_optional_string(value, 24)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "low" => Some("low".to_string()),
        "medium" => Some("medium".to_string()),
        "high" => Some("high".to_string()),
        _ => None,
    }
}

fn native_codex_user_location(value: Option<&Value>) -> Option<NativeCodexUserLocation> {
    let Some(Value::Object(map)) = value else {
        return None;
    };
    let location = NativeCodexUserLocation {
        country: native_codex_optional_string(map.get("country"), 120),
        region: native_codex_optional_string(map.get("region"), 120),
        city: native_codex_optional_string(map.get("city"), 120),
        timezone: native_codex_optional_string(map.get("timezone"), 120),
    };
    if location.country.is_some()
        || location.region.is_some()
        || location.city.is_some()
        || location.timezone.is_some()
    {
        Some(location)
    } else {
        None
    }
}

fn native_codex_effective_config(policy: &Value) -> NativeCodexConfig {
    let section = native_codex_policy_section(policy);
    NativeCodexConfig {
        enabled: native_codex_bool_value(section, "enabled", "enabled", false),
        mode: native_codex_mode(native_codex_config_value(section, "mode", "mode")),
        allowed_domains: native_codex_allowed_domains(
            native_codex_config_value(section, "allowed_domains", "allowedDomains"),
        ),
        context_size: native_codex_context_size(
            native_codex_config_value(section, "context_size", "contextSize"),
        ),
        user_location: native_codex_user_location(
            native_codex_config_value(section, "user_location", "userLocation"),
        ),
    }
}

fn native_codex_user_location_json(location: Option<&NativeCodexUserLocation>) -> Value {
    location.map_or(Value::Null, |current| {
        json!({
            "country": current.country,
            "region": current.region,
            "city": current.city,
            "timezone": current.timezone
        })
    })
}

fn native_codex_auth_status(root: &Path) -> Value {
    let env_available = NATIVE_CODEX_OPENAI_ENV_KEYS.iter().any(|key| {
        std::env::var(key)
            .ok()
            .map(|raw| !clean_text(&raw, 4096).is_empty())
            .unwrap_or(false)
    });
    let store_available = read_json_or(
        &native_codex_provider_secrets_path(root),
        json!({"providers": {}}),
    )
    .pointer("/providers/openai/key")
    .and_then(Value::as_str)
    .map(|raw| !clean_text(raw, 4096).is_empty())
    .unwrap_or(false);
    let source = if env_available {
        "env"
    } else if store_available {
        "provider_secret_store"
    } else {
        "missing"
    };
    json!({
        "available": env_available || store_available,
        "source": source,
        "env_keys": NATIVE_CODEX_OPENAI_ENV_KEYS,
        "provider_secret_path": format!("{NATIVE_CODEX_PROVIDER_SECRETS_REL}#/providers/openai/key")
    })
}

fn native_codex_model_provider(request: &Value) -> String {
    native_codex_optional_string(
        request.get("model_provider").or_else(|| request.get("provider")),
        80,
    )
    .unwrap_or_default()
    .to_ascii_lowercase()
}

fn native_codex_model_api(request: &Value) -> String {
    native_codex_optional_string(request.get("model_api").or_else(|| request.get("api")), 80)
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn native_codex_model_is_eligible(model_provider: &str, model_api: &str) -> bool {
    model_provider == "openai-codex" || model_api == "openai-codex-responses"
}

fn build_native_codex_web_search_tool(policy: &Value) -> Value {
    let config = native_codex_effective_config(policy);
    let mut tool = json!({
        "type": "web_search",
        "external_web_access": config.mode == "live"
    });
    if !config.allowed_domains.is_empty() {
        tool["filters"] = json!({
            "allowed_domains": config.allowed_domains
        });
    }
    if let Some(context_size) = config.context_size {
        tool["search_context_size"] = json!(context_size);
    }
    if let Some(location) = config.user_location {
        tool["user_location"] = json!({
            "type": "approximate",
            "country": location.country,
            "region": location.region,
            "city": location.city,
            "timezone": location.timezone
        });
    }
    tool
}

fn native_codex_payload_has_web_search_tool(payload: &Value) -> bool {
    payload
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools.iter().any(|tool| {
                tool.get("type")
                    .and_then(Value::as_str)
                    .map(|kind| kind == "web_search")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn patch_native_codex_payload_in_place(payload: &mut Value, policy: &Value) -> Value {
    if native_codex_payload_has_web_search_tool(payload) {
        return json!({ "status": "native_tool_already_present" });
    }
    let Some(payload_obj) = payload.as_object_mut() else {
        return json!({ "status": "payload_not_object" });
    };
    let mut tools = payload_obj
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    tools.push(build_native_codex_web_search_tool(policy));
    payload_obj.insert("tools".to_string(), Value::Array(tools));
    json!({ "status": "injected" })
}

fn native_codex_description(policy: &Value) -> Option<String> {
    if policy
        .pointer("/web_conduit/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
        == false
    {
        return None;
    }
    let config = native_codex_effective_config(policy);
    if !config.enabled {
        return None;
    }
    Some(format!(
        "Codex native search: {} for Codex-capable models",
        config.mode
    ))
}

fn native_codex_relevant(root: &Path, policy: &Value, request: &Value) -> bool {
    let config = native_codex_effective_config(policy);
    let auth = native_codex_auth_status(root);
    config.enabled
        || auth.get("available").and_then(Value::as_bool).unwrap_or(false)
        || native_codex_model_is_eligible(
            &native_codex_model_provider(request),
            &native_codex_model_api(request),
        )
}

fn resolve_native_codex_activation(root: &Path, policy: &Value, request: &Value) -> Value {
    let config = native_codex_effective_config(policy);
    let global_web_search_enabled = policy
        .pointer("/web_conduit/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let model_provider = native_codex_model_provider(request);
    let model_api = native_codex_model_api(request);
    let native_eligible = native_codex_model_is_eligible(&model_provider, &model_api);
    let auth = native_codex_auth_status(root);
    let has_required_auth =
        model_provider != "openai-codex" || auth.get("available").and_then(Value::as_bool) == Some(true);
    let inactive_reason = if !global_web_search_enabled {
        Some("globally_disabled")
    } else if !config.enabled {
        Some("codex_not_enabled")
    } else if !native_eligible {
        Some("model_not_eligible")
    } else if !has_required_auth {
        Some("codex_auth_missing")
    } else {
        None
    };
    json!({
        "global_web_search_enabled": global_web_search_enabled,
        "codex_native_enabled": config.enabled,
        "codex_mode": config.mode,
        "native_eligible": native_eligible,
        "has_required_auth": has_required_auth,
        "state": if inactive_reason.is_some() { "managed_only" } else { "native_active" },
        "inactive_reason": inactive_reason,
        "model_provider": model_provider,
        "model_api": model_api
    })
}

fn native_codex_public_contract(root: &Path, policy: &Value) -> Value {
    let config = native_codex_effective_config(policy);
    json!({
        "config": {
            "enabled": config.enabled,
            "mode": config.mode,
            "allowed_domains": if config.allowed_domains.is_empty() { Value::Null } else { json!(config.allowed_domains) },
            "context_size": config.context_size,
            "user_location": native_codex_user_location_json(config.user_location.as_ref())
        },
        "eligible_model_contract": {
            "provider": "openai-codex",
            "api": "openai-codex-responses"
        },
        "auth": native_codex_auth_status(root),
        "description": native_codex_description(policy),
        "tool_definition": build_native_codex_web_search_tool(policy),
        "supported_activation_states": ["managed_only", "native_active"],
        "inactive_reasons": [
            "globally_disabled",
            "codex_not_enabled",
            "model_not_eligible",
            "codex_auth_missing"
        ]
    })
}

pub fn api_native_codex(root: &Path, request: &Value) -> Value {
    let (policy, policy_path_value) = load_policy(root);
    let activation = resolve_native_codex_activation(root, &policy, request);
    let payload_patch = if let Some(payload) = request.get("payload") {
        if activation.get("state").and_then(Value::as_str) != Some("native_active") {
            json!({
                "status": "inactive",
                "inactive_reason": activation.get("inactive_reason").cloned().unwrap_or(Value::Null)
            })
        } else {
            let mut patched_payload = payload.clone();
            let patch = patch_native_codex_payload_in_place(&mut patched_payload, &policy);
            json!({
                "status": patch.get("status").and_then(Value::as_str).unwrap_or("payload_not_object"),
                "payload": patched_payload
            })
        }
    } else {
        Value::Null
    };
    json!({
        "ok": true,
        "type": "web_conduit_native_codex",
        "policy_path": policy_path_value.to_string_lossy().to_string(),
        "native_codex_web_search": native_codex_public_contract(root, &policy),
        "activation": activation,
        "relevant": native_codex_relevant(root, &policy, request),
        "suppress_managed_web_search_tool": activation.get("state").and_then(Value::as_str) == Some("native_active"),
        "payload_patch": payload_patch
    })
}
