
fn normalize_scope_rule(id: &str, raw_rule: &Map<String, Value>) -> ScopeRule {
    ScopeRule {
        id: normalize_token(id, 120),
        methods: clean_methods(raw_rule.get("methods")),
        domains: clean_domains(raw_rule.get("domains")),
        require_runtime_allowlist: raw_rule
            .get("require_runtime_allowlist")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        rate_caps: RateCaps {
            per_hour: raw_rule
                .get("rate_caps")
                .and_then(Value::as_object)
                .and_then(|row| to_u64(row.get("per_hour"))),
            per_day: raw_rule
                .get("rate_caps")
                .and_then(Value::as_object)
                .and_then(|row| to_u64(row.get("per_day"))),
        },
    }
}

fn load_policy_model(root: &Path, payload: &Map<String, Value>) -> (Policy, PathBuf) {
    let runtime = runtime_root(root, payload);
    let explicit = clean_text(
        payload.get("policy_path").or_else(|| payload.get("path")),
        520,
    );
    let policy_env = std::env::var("EGRESS_GATEWAY_POLICY_PATH").unwrap_or_default();
    let policy_path = resolve_path(
        &runtime,
        if explicit.is_empty() {
            &policy_env
        } else {
            &explicit
        },
        DEFAULT_POLICY_REL,
    );
    let src = read_json_or_default(&policy_path, json!({}));
    let src_obj = src.as_object().cloned().unwrap_or_default();
    let scopes_raw = src_obj
        .get("scopes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut scopes = BTreeMap::new();
    for (id, row) in scopes_raw {
        if let Some(rule) = row.as_object() {
            let normalized = normalize_scope_rule(id.as_str(), rule);
            if !normalized.id.is_empty() {
                scopes.insert(normalized.id.clone(), normalized);
            }
        }
    }
    (
        Policy {
            version: {
                let value = clean_text(src_obj.get("version"), 32);
                if value.is_empty() {
                    "1.0".to_string()
                } else {
                    value
                }
            },
            default_decision: {
                let value = normalize_token(&clean_text(src_obj.get("default_decision"), 12), 12);
                if value.is_empty() {
                    "deny".to_string()
                } else {
                    value
                }
            },
            global_rate_caps: RateCaps {
                per_hour: src_obj
                    .get("global_rate_caps")
                    .and_then(Value::as_object)
                    .and_then(|row| to_u64(row.get("per_hour"))),
                per_day: src_obj
                    .get("global_rate_caps")
                    .and_then(Value::as_object)
                    .and_then(|row| to_u64(row.get("per_day"))),
            },
            scopes,
        },
        policy_path,
    )
}

fn policy_to_value(policy: &Policy) -> Value {
    let scopes = policy
        .scopes
        .iter()
        .map(|(id, rule)| {
            (
                id.clone(),
                json!({
                    "id": rule.id,
                    "methods": rule.methods,
                    "domains": rule.domains,
                    "require_runtime_allowlist": rule.require_runtime_allowlist,
                    "rate_caps": {
                        "per_hour": rule.rate_caps.per_hour,
                        "per_day": rule.rate_caps.per_day,
                    }
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    json!({
        "version": policy.version,
        "default_decision": policy.default_decision,
        "global_rate_caps": {
            "per_hour": policy.global_rate_caps.per_hour,
            "per_day": policy.global_rate_caps.per_day,
        },
        "scopes": scopes,
    })
}

fn load_state_model(root: &Path, payload: &Map<String, Value>) -> (Value, PathBuf) {
    let runtime = runtime_root(root, payload);
    let explicit = clean_text(
        payload.get("state_path").or_else(|| payload.get("path")),
        520,
    );
    let state_env = std::env::var("EGRESS_GATEWAY_STATE_PATH").unwrap_or_default();
    let state_path = resolve_path(
        &runtime,
        if explicit.is_empty() {
            &state_env
        } else {
            &explicit
        },
        DEFAULT_STATE_REL,
    );
    let src = read_json_or_default(&state_path, json!({}));
    let src_obj = src.as_object().cloned().unwrap_or_default();
    let updated_at = {
        let value = clean_text(src_obj.get("updated_at"), 80);
        if value.is_empty() {
            Value::Null
        } else {
            Value::String(value)
        }
    };
    let per_hour = src_obj
        .get("per_hour")
        .cloned()
        .filter(|value| value.is_object())
        .unwrap_or_else(|| json!({}));
    let per_day = src_obj
        .get("per_day")
        .cloned()
        .filter(|value| value.is_object())
        .unwrap_or_else(|| json!({}));
    let state = json!({
        "schema_id": "egress_gateway_state",
        "schema_version": "1.0",
        "updated_at": updated_at,
        "per_hour": per_hour,
        "per_day": per_day,
    });
    (state, state_path)
}

fn audit_path(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let runtime = runtime_root(root, payload);
    let explicit = clean_text(payload.get("audit_path"), 520);
    let audit_env = std::env::var("EGRESS_GATEWAY_AUDIT_PATH").unwrap_or_default();
    resolve_path(
        &runtime,
        if explicit.is_empty() {
            &audit_env
        } else {
            &explicit
        },
        DEFAULT_AUDIT_REL,
    )
}

fn resolve_scope_rule<'a>(policy: &'a Policy, scope_id: &str) -> Option<&'a ScopeRule> {
    if let Some(rule) = policy.scopes.get(scope_id) {
        return Some(rule);
    }
    if scope_id.starts_with("sensory.collector.") {
        return policy.scopes.get("sensory.collector.dynamic");
    }
    None
}

fn count_key(scope_id: &str, epoch_key: &str) -> String {
    format!("{scope_id}:{epoch_key}")
}

fn counter_value(map: &Map<String, Value>, key: &str) -> u64 {
    to_u64(map.get(key)).unwrap_or(0)
}

fn check_cap(map: &Map<String, Value>, key: &str, cap: Option<u64>) -> bool {
    match cap {
        Some(limit) if limit > 0 => counter_value(map, key) < limit,
        _ => true,
    }
}

fn set_counter(map: &mut Map<String, Value>, key: &str, value: u64) {
    map.insert(
        key.to_string(),
        Value::Number(serde_json::Number::from(value)),
    );
}

fn increment_counter(map: &mut Map<String, Value>, key: &str) {
    let next = counter_value(map, key).saturating_add(1);
    set_counter(map, key, next);
}

fn iso_hour_key(now_ms: i64) -> String {
    chrono::Utc
        .timestamp_millis_opt(now_ms)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%dT%H")
        .to_string()
}

fn iso_day_key(now_ms: i64) -> String {
    chrono::Utc
        .timestamp_millis_opt(now_ms)
        .single()
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%d")
        .to_string()
}

fn load_policy_command(root: &Path, payload: &Map<String, Value>) -> Value {
    let (policy, policy_path) = load_policy_model(root, payload);
    json!({
        "ok": true,
        "policy": policy_to_value(&policy),
        "policy_path": policy_path.to_string_lossy(),
    })
}

fn load_state_command(root: &Path, payload: &Map<String, Value>) -> Value {
    let (state, state_path) = load_state_model(root, payload);
    json!({
        "ok": true,
        "state": state,
        "state_path": state_path.to_string_lossy(),
    })
}
